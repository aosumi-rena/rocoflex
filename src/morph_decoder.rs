use std::collections::VecDeque;
use crate::{Result, model::{ModelNode, ModelTree}, model_tags::decoded_hashes,
    normalization::MaxMinNormalizer,
    prototype_synthesis_networks::{DenseMatrix, FloatGru, NcwnNetwork,NcwnCausalStream,NcwnPersistentStream,
        SYNTHESIS_ACTIVATION_ARITHMETIC}};

#[path = "morph_sampling.rs"]
mod morph_sampling;

#[path = "morph_codebook.rs"]
mod codebook;
use codebook::Codebook;

fn tagged(node: &ModelNode<'_>, name: &str) -> bool {
    let decoded=node.tag ^ 0x40dab9846f846a19;

    if name=="moda2" {
        return [0xd932584fe0520b32,0xc0a5aaea285ed148,0xd6583d2397d0ee5b,
            0xa5245e9eec3c691c,0x24aafe3d31768758,0xe792723005e5316b,
            0xbbfdd0adc04da9a6,0xfb217a106462f38e,0xc5954015b4a53c78,
            0x99af7461e69ec4e3,0xfd0b354d5fe6ae8d,0xddd7e8788e475f69].contains(&decoded);
    }
    decoded_hashes(name).contains(&decoded)
}
fn dimension(data: &[u8], offset: usize) -> Result<usize> {
    Ok(u16::from_le_bytes(data.get(offset..offset+2).ok_or("Truncated decoder configuration")?
        .try_into().unwrap()) as usize)
}
enum Operation { Linear(DenseMatrix, Vec<f32>), Relu, Silu, Sigmoid, Identity }
struct Sequence {
    operations: Vec<Operation>,
    arithmetic:crate::activation::ArithmeticMode,

    quantized_work: Option<crate::quantized_matrix::QuantizedWorkspace>,

    row_work: Vec<f32>,
    next_work: Vec<f32>,
}
impl Sequence {
    fn load(node: &ModelNode<'_>) -> Result<Self> {
        let mut operations=Vec::new(); Self::append(node, &mut operations)?;
        let requirements=operations.iter().filter_map(|operation| match operation {
            Operation::Linear(matrix,_)=>matrix.quantized_workspace_requirements(),
            _=>None,
        }).reduce(|(input,output),(next_input,next_output)|
            (input.max(next_input),output.max(next_output)));
        let quantized_work=requirements.map(|(input,output)|
            crate::quantized_matrix::QuantizedWorkspace::new(input,output)).transpose()?;
        let row_width=operations.iter().filter_map(|operation| match operation {
            Operation::Linear(matrix,_)=>Some(matrix.input_width.max(matrix.output_width)),
            _=>None,
        }).max().unwrap_or(0);
        let mut row_work=Vec::new();
        row_work.try_reserve_exact(row_width).map_err(|error|error.to_string())?;
        let mut next_work=Vec::new();
        next_work.try_reserve_exact(row_width).map_err(|error|error.to_string())?;
        Ok(Self {operations,arithmetic:SYNTHESIS_ACTIVATION_ARITHMETIC,quantized_work,
            row_work,next_work})
    }
    fn with_activation_arithmetic(mut self,arithmetic:crate::activation::ArithmeticMode)->Self {
        self.arithmetic=arithmetic;
        self
    }
    fn append(node: &ModelNode<'_>, operations: &mut Vec<Operation>) -> Result<()> {
        if tagged(node,"modm0") {
            for child in &node.children {Self::append(child, operations)?;}
        } else if tagged(node,"modl0") && node.children.len()==2 {
            let matrix=DenseMatrix::from_node(&node.children[0])?;
            let bias=&node.children[1];
            if !tagged(bias,"prim1") || bias.data.len()!=4+4*matrix.output_width
                || u32::from_le_bytes(bias.data[..4].try_into().unwrap()) as usize!=matrix.output_width {
                return Err("Decoder linear bias dimensions differ".into());
            }
            let values=bias.data[4..].chunks_exact(4)
                .map(|x|f32::from_le_bytes(x.try_into().unwrap())).collect::<Vec<_>>();
            if values.iter().any(|x|!x.is_finite()) {return Err("Nonfinite decoder bias".into());}
            operations.push(Operation::Linear(matrix,values));
        } else if node.children.is_empty() && node.data.is_empty() {
            operations.push(if tagged(node,"moda0") {Operation::Relu}
                else if tagged(node,"moda7") {Operation::Silu}
                else if tagged(node,"moda2") {Operation::Sigmoid}
                else if tagged(node,"moda5") {Operation::Identity}
                else {return Err(format!("Unknown decoder activation {:016x}",node.tag));});
        } else {return Err(format!("Unsupported decoder sequence {:016x}",node.tag));}
        Ok(())
    }
    fn forward(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        let Self {operations,arithmetic,quantized_work,row_work:row,next_work:next}=self;
        row.clear();
        row.extend_from_slice(input);
        for operation in operations.iter() {
            match operation {
                Operation::Linear(matrix,bias)=>{
                    next.resize(bias.len(),0.);

                    next.fill(0.);
                    if let Some(work)=quantized_work.as_mut() {
                        matrix.overwrite_with_workspace(row,next,work)?;
                    } else {
                        matrix.overwrite(row,next)?;
                    }
                    for (value,bias) in next.iter_mut().zip(bias) {*value+=*bias;}
                    std::mem::swap(row,next);
                }
                Operation::Relu=>row.iter_mut().for_each(|x|*x=x.max(0.)),
                Operation::Silu=>row.iter_mut().for_each(|x|*x*=arithmetic.sigmoid(*x)),
                Operation::Sigmoid=>row.iter_mut().for_each(|x|*x=arithmetic.sigmoid(*x)),
                Operation::Identity=>{}
            }
        }
        if row.iter().any(|x|!x.is_finite()) {return Err("Nonfinite decoder sequence output".into());}
        Ok(row.clone())
    }
}

struct Head { scores: Sequence, codebook: Codebook, feedback: Option<Sequence> }
struct RecurrentCodes {
    gru: FloatGru, projection: Sequence, heads: Vec<Head>,
    persistent: Vec<f32>, random: u64,
}
impl RecurrentCodes {
    fn load(node: &ModelNode<'_>, seed: u64) -> Result<Self> {
        if node.children.len()!=2 || node.data!=384u16.to_le_bytes() {
            return Err("Expected decoder external recurrent width384".into());
        }
        let recurrent=&node.children[0]; let sampler=&node.children[1];
        if recurrent.children.len()!=3 || sampler.data.len()!=8
            || (0..4).map(|i|dimension(sampler.data,2*i)).collect::<Result<Vec<_>>>()?!=[4,128,64,384]
            || sampler.children.len()!=11 {
            return Err("Decoder recurrent/codebook topology differs".into());
        }
        let gru=FloatGru::from_node(&recurrent.children[0])?;
        if gru.input_width()!=640 || gru.hidden().len()!=512 || !tagged(&recurrent.children[2],"moda0") {
            return Err("Decoder GRU dimensions or postactivation differ".into());
        }
        let mut projection=Sequence::load(&recurrent.children[1])?;
        projection.operations.push(Operation::Relu);
        let mut heads=Vec::with_capacity(4);
        for head in 0..4 {
            let code=&sampler.children[3*head+1];
            if code.children.len()!=1 {return Err("Expected decoder codebook matrix".into());}
            let codebook=Codebook::from_node(&code.children[0])?;
            heads.push(Head {scores:Sequence::load(&sampler.children[3*head])?,codebook,
                feedback:if head<3 {Some(Sequence::load(&sampler.children[3*head+2])?)} else {None}});
        }
        Ok(Self {gru,projection,heads,persistent:vec![0.;640],random:seed.max(1)})
    }

    fn uniform(&mut self)->f32 {
        let old=self.random;
        let shifted=((old>>45) as u32)^((old>>27) as u32);
        let output=shifted.rotate_right((old>>59) as u32);
        self.random=old.wrapping_mul(0x5851_f42d_4c95_7f2d)
            .wrapping_add(0xda3e_39cb_94b9_5bdb);

        (output as f32)*f32::from_bits(0x2f80_0000)
    }
    fn step(&mut self, external:&[f32], scale:f32)->Result<Vec<f32>> {
        if external.len()!=384 || !scale.is_finite() {return Err("Invalid recurrent decoder input".into());}
        self.persistent[..384].copy_from_slice(external);
        let hidden=self.gru.step(&self.persistent)?;
        let mut condition=self.projection.forward(hidden)?;
        if condition.len()!=384 {return Err("Decoder recurrent projection width differs".into());}
        let mut output=vec![0.;256];
        for head in 0..4 {
            let mut logits=self.heads[head].scores.forward(&condition)?;
            let sampling_result=(||->Result<[f32;64]> {
                let probabilities:&mut[f32;128]=logits.as_mut_slice().try_into()
                    .map_err(|_|"Decoder categorical width differs")?;
                if scale!=1. {for x in probabilities.iter_mut() {*x*=scale;}}
                morph_sampling::normalize(probabilities)?;
                let mut draw=self.uniform(); let mut selected=127;
                for (index,&probability) in probabilities.iter().enumerate() {
                    draw-=probability; if draw<0. {selected=index;break;}
                }
                let vector=*self.heads[head].codebook.column(selected)?;
                self.persistent[384+64*head..384+64*(head+1)].copy_from_slice(&vector);
                for word in 0..64 {output[4*word+head]=vector[word];}
                Ok(vector)
            })();
            let vector=sampling_result?;
            if let Some(feedback)=&mut self.heads[head].feedback {
                let addition=feedback.forward(&vector)?;
                if addition.len()!=384 {return Err("Decoder inter-head feedback width differs".into());}
                for (x,y) in condition.iter_mut().zip(addition) {*x+=y;}
            }
        }
        Ok(output)
    }
}

pub(crate) struct DecoderInput<'a> {
    pub large321:&'a [f32], pub small15:&'a [f32], pub scalar_encoding4:&'a [f32],
    pub auxiliary2:&'a [f32], pub target128:&'a [f32],
}
pub(crate) struct DecoderFrontFrame {
    pub main256:Vec<f32>, pub sampled256:Vec<f32>, pub target128:Vec<f32>,
}
struct Pending {target:Vec<f32>}
pub(crate) struct DecoderFront {
    scalar:MaxMinNormalizer, prefix:Sequence, main:NcwnPersistentStream, recurrent:RecurrentCodes,
    pending:VecDeque<Pending>, sampling_scale:f32, failure:Option<String>,
}
impl DecoderFront {
    pub(crate) fn from_dnni(bytes:&[u8],branch:usize,seed:u64,sampling_scale:f32)->Result<Self> {
        if ![4,5].contains(&branch) || !sampling_scale.is_finite() {
            return Err("Select an actual decoder branch4 or5 and finite sampling scale".into());
        }
        let tree=ModelTree::parse(bytes)?;
        let node=tree.root.children.get(branch).ok_or("Missing paired decoder")?;
        if node.children.len()!=4 || node.children[0].children.len()!=2 {
            return Err("Paired decoder topology differs".into());
        }
        let main=NcwnNetwork::from_node(&node.children[1])?;
        if (main.input_width,main.output_width,main.condition_width)!=(256,256,384) {
            return Err("Paired decoder main dimensions differ".into());
        }

        #[cfg(target_arch="x86_64")]
        let fused_multiply_add=std::is_x86_feature_detected!("fma");
        #[cfg(not(target_arch="x86_64"))]
        let fused_multiply_add=false;
        let main=main.with_activation_arithmetic(crate::activation::ArithmeticMode {fused_multiply_add});
        Ok(Self {scalar:MaxMinNormalizer::from_node(&node.children[0].children[0],1)?,
            prefix:Sequence::load(&node.children[0].children[1])?,
            main:main.into_persistent_stream()?,recurrent:RecurrentCodes::load(&node.children[2],seed)?,
            pending:VecDeque::new(),sampling_scale,failure:None})
    }

    pub(crate) fn process_source(&mut self,large321:&[f32],small15:&[f32],
        auxiliary2:&[f32],target128:&[f32],log_pitch:f32,
        output:&mut Vec<DecoderFrontFrame>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let normalized=match self.scalar.forward(&[log_pitch],true) {
            Ok(row)=>row[0],Err(error)=>{self.failure=Some(error.clone());return Err(error);}
        };
        let encoding=scalar_encoding(normalized);
        self.process(DecoderInput {large321,small15,scalar_encoding4:&encoding,auxiliary2,target128},output)
    }
    pub(crate) fn process(&mut self,input:DecoderInput<'_>,output:&mut Vec<DecoderFrontFrame>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.advance(input,output);
        if let Err(error)=&result {self.failure=Some(error.clone());} result
    }
    fn advance(&mut self,input:DecoderInput<'_>,output:&mut Vec<DecoderFrontFrame>)->Result<()> {
        for (row,width) in [(input.large321,321),(input.small15,15),(input.scalar_encoding4,4),
            (input.auxiliary2,2),(input.target128,128)] {
            if row.len()!=width || row.iter().any(|x|!x.is_finite()) {
                return Err("Decoder requires the actual321/15/4/2 source rows and128 target".into());
            }
        }
        let mut assembled=Vec::with_capacity(342);
        for row in [input.large321,input.small15,input.scalar_encoding4,input.auxiliary2] {assembled.extend_from_slice(row);}
        let projected=self.prefix.forward(&assembled)?;
        if projected.len()!=256 {return Err("Decoder prefix output width differs".into());}
        let mut condition=projected.clone(); condition.extend_from_slice(input.target128);
        self.pending.push_back(Pending {target:input.target128.to_vec()});
        let mut completed=Vec::new();
        self.main.push(&[projected],&[condition],false,&mut completed)?;
        for main in completed {
            let pending=self.pending.pop_front().ok_or("Decoder target queue is empty")?;
            let mut external=main.clone();external.extend_from_slice(&pending.target);
            let sampled=self.recurrent.step(&external,self.sampling_scale)?;
            output.push(DecoderFrontFrame {main256:main,sampled256:sampled,target128:pending.target});
        }
        Ok(())
    }
}

#[derive(Clone,Copy)]
struct Pitch {value:f32,valid:bool,ordinal:u64}
pub(crate) struct DecoderFrame {pub ordinal:u64,pub feature72:[f32;72]}

pub(crate) struct Decoder {
    front:DecoderFront, first_tail:NcwnCausalStream, second_tail:NcwnPersistentStream,
    head:Sequence, output:DecoderOutput, front_pitch:VecDeque<Pitch>,
    second_pending:VecDeque<Pitch>,
    accepted:u64, failure:Option<String>,
}
impl Decoder {
    pub(crate) fn from_dnni(bytes:&[u8],branch:usize,seed:u64)->Result<Self> {
        let front=DecoderFront::from_dnni(bytes,branch,seed,1.)?;
        let tree=ModelTree::parse(bytes)?;
        let tail=&tree.root.children[branch].children[3];
        if tail.data.len()!=8 || (0..4).map(|i|dimension(tail.data,2*i))
            .collect::<Result<Vec<_>>>()?!=[64,71,256,128] || tail.children.len()!=3 {
            return Err("Decoder tail configuration differs".into());
        }
        let first=NcwnNetwork::from_node(&tail.children[0])?;
        let second=NcwnNetwork::from_node(&tail.children[1])?;
        for network in [&first,&second] {
            if (network.input_width,network.output_width,network.condition_width)!=(256,256,384) {
                return Err("Decoder tail network dimensions differ".into());
            }
        }

        #[cfg(target_arch="x86_64")]
        let fused_multiply_add=std::is_x86_feature_detected!("fma");
        #[cfg(not(target_arch="x86_64"))]
        let fused_multiply_add=false;
        let first=first.with_activation_arithmetic(crate::activation::ArithmeticMode {fused_multiply_add});
        let second=second.with_activation_arithmetic(crate::activation::ArithmeticMode {fused_multiply_add});
        Ok(Self {front,first_tail:first.into_causal_stream()?,second_tail:second.into_persistent_stream()?,
            head:Sequence::load(&tail.children[2])?,output:DecoderOutput::from_dnni(bytes)?,
            front_pitch:VecDeque::new(),second_pending:VecDeque::new(),
            accepted:0,failure:None})
    }
    pub(crate) fn prepared_count_inputs(&self)->super::count_plan::Query<super::count_plan::DecoderCounts> {
        use super::count_plan::{self as count,DecoderCounts};
        count::cold(self.accepted!=0 || !self.front_pitch.is_empty() || !self.second_pending.is_empty()
            || !self.front.pending.is_empty(),self.failure.is_some() || self.front.failure.is_some())?;

        Ok(DecoderCounts {front_right_rows:count::checked_rows(self.front.main.lookahead_rows())?,
            tail_right_rows:count::checked_rows(self.second_tail.lookahead_rows())?})
    }
    pub(crate) fn process(&mut self,large321:&[f32],small15:&[f32],auxiliary2:&[f32],
        target128:&[f32],pitch_value:f32,pitch_valid:bool,output:&mut Vec<DecoderFrame>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.advance(large321,small15,auxiliary2,target128,pitch_value,pitch_valid,output);
        if let Err(error)=&result {self.failure=Some(error.clone());} result
    }
    fn advance(&mut self,large321:&[f32],small15:&[f32],auxiliary2:&[f32],target128:&[f32],
        pitch_value:f32,pitch_valid:bool,output:&mut Vec<DecoderFrame>)->Result<()> {
        if !pitch_value.is_finite() {return Err("Nonfinite decoder pitch record".into());}
        self.front_pitch.push_back(Pitch {value:pitch_value,valid:pitch_valid,ordinal:self.accepted});
        self.accepted=self.accepted.checked_add(1).ok_or("Decoder ordinal overflow")?;
        let mut front=Vec::new();
        let scalar=if pitch_valid {pitch_value} else {pitch_value.ln()};
        if !scalar.is_finite() {return Err("Invalid decoder logarithmic pitch scalar".into());}
        self.front.process_source(large321,small15,auxiliary2,target128,scalar,&mut front)?;
        for frame in front {
            let pitch=self.front_pitch.pop_front().ok_or("Decoder front pitch queue is empty")?;
            let mut condition=frame.sampled256;condition.extend_from_slice(&frame.target128);
            let first=self.first_tail.process_row(&frame.main256,&condition)?.to_vec();
            self.second_pending.push_back(pitch);
            let mut second=Vec::new();
            self.second_tail.push(&[first],&[condition],false,&mut second)?;
                for row in second {
                    let pitch=self.second_pending.pop_front().ok_or("Decoder second-tail queue is empty")?;
                    let decoded=self.head.forward(&row)?;
                    let feature72=self.output.pack(&decoded,pitch.value,pitch.valid)?;
                    output.push(DecoderFrame {ordinal:pitch.ordinal,feature72});
                }
        }
        Ok(())
    }
}

fn scalar_encoding(normalized:f32)->[f32;4] {
    std::array::from_fn(|j| {
        let center=(j as f32-1.)/4.;
        let angle=(normalized-center)*f32::from_bits(0x40c90fdb);
        ((angle.cos() as f64)*0.5+0.5) as f32
    })
}

pub(crate) struct DecoderOutput {normalizer:MaxMinNormalizer}
impl DecoderOutput {
    pub(crate) fn from_dnni(bytes:&[u8])->Result<Self> {
        let tree=ModelTree::parse(bytes)?;
        let node=tree.root.children.get(6).ok_or("Missing decoder71 inverse normalizer")?;
        Ok(Self {normalizer:MaxMinNormalizer::from_node(node,71)?})
    }
    pub(crate) fn pack(&self,decoded71:&[f32],pitch_value:f32,pitch_valid:bool)->Result<[f32;72]> {
        if decoded71.len()!=71 || !pitch_value.is_finite() {
            return Err("Invalid actual decoder71/pitch record".into());
        }
        let inverse=self.normalizer.inverse(decoded71)?;
        let pitch=if pitch_valid {pitch_value.exp()} else {pitch_value};
        if !pitch.is_finite() {return Err("Nonfinite decoder pitch frequency".into());}
        let mut feature=[0.;72];feature[0]=inverse[0];
        feature[1]=pitch.max(70.);feature[2..].copy_from_slice(&inverse[1..]);
        Ok(feature)
    }
}

#[path="morph_alternate_inner.rs"]
pub(crate) mod alternate_inner;
