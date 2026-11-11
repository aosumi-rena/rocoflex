use std::collections::VecDeque;
use super::{NcwnNetwork,NcwnCausalStream,NcwnBlock,NeuralConv,pointwise_into_using,require_pointwise};
use crate::{Result,quantized_matrix::QuantizedWorkspace};

enum Execution { Causal(NcwnCausalStream), Mixed(MixedNetwork) }

pub struct NcwnPersistentStream {
    pub input_width:usize, pub condition_width:usize, pub output_width:usize,
    execution:Execution, finished:bool, failure:Option<String>,
}
impl NcwnNetwork {
    pub fn into_persistent_stream(self)->Result<NcwnPersistentStream> {
        require_pointwise(&self.head)?;
        let lookahead=self.streaming_context()?.1;
        for block in &self.blocks {
            require_pointwise(&block.condition)?;
            require_pointwise(&block.residual)?;
            require_pointwise(&block.skip)?;
            let (left,right)=block.main_padding()?;
            let support=(block.main.taps.len()-1).checked_mul(block.main.dilation)
                .ok_or("Persistent convolution support overflow")?;
            if block.main.stride!=1 || left.checked_add(right)!=Some(support) {
                return Err("Persistent NCWN requires actual stride1, row-aligned main padding".into());
            }
            support.checked_add(1).and_then(|n|n.checked_mul(block.main.input_width))
                .ok_or("Persistent convolution history overflow")?;
        }
        let input_width=self.input_width;let output_width=self.output_width;let condition_width=self.condition_width;
        let all_causal=self.blocks.iter().all(|b|b.warmup_rows.is_some() && b.main.taps.len()==3);
        let execution=if all_causal {Execution::Causal(self.into_causal_stream()?)} else {
            let requirements=self.head.taps.iter().filter_map(|matrix|matrix.quantized_workspace_requirements())
                .reduce(|(input,output),(next_input,next_output)|
                    (input.max(next_input),output.max(next_output)));
            let head_work=requirements.map(|(input,output)|QuantizedWorkspace::new(input,output)).transpose()?;
            let blocks=self.blocks.into_iter().map(PersistentBlock::new).collect::<Result<_>>()?;
            Execution::Mixed(MixedNetwork {blocks,head:self.head,conditions:VecDeque::new(),
                outputs:VecDeque::new(),received:0,emitted:0,lookahead,head_work})
        };
        Ok(NcwnPersistentStream {input_width,condition_width,output_width,execution,finished:false,failure:None})
    }
}
impl NcwnPersistentStream {
    pub fn lookahead_rows(&self)->usize {
        match &self.execution {Execution::Causal(_)=>0,Execution::Mixed(s)=>s.lookahead}
    }

    pub fn retained_rows(&self)->usize {
        match &self.execution {Execution::Causal(_)=>0,Execution::Mixed(s)=>s.conditions.len()}
    }
    pub fn failure(&self)->Option<&str> {self.failure.as_deref()}

    pub fn push(&mut self,input:&[Vec<f32>],condition:&[Vec<f32>],final_input:bool,
        output:&mut Vec<Vec<f32>>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        if self.finished {return Err("Persistent NCWN already received final input".into());}
        let result:Result<()>=(|| {
            if input.len()!=condition.len() || input.iter().any(|x|x.len()!=self.input_width)
                || condition.iter().any(|x|x.len()!=self.condition_width) {
                return Err("Persistent NCWN input/condition dimensions differ".into());
            }
            output.reserve(input.len().saturating_add(if final_input {self.lookahead_rows()} else {0}));
            match &mut self.execution {
                Execution::Causal(stream)=>stream.process(input,condition,output)?,
                Execution::Mixed(stream)=>{
                    for (input,condition) in input.iter().zip(condition) {stream.accept(input,condition,output)?;}
                    if final_input {stream.finish(output)?;}
                }
            }
            self.finished=final_input;
            Ok(())
        })();
        if let Err(error)=&result {self.failure=Some(error.clone());}
        result
    }
}

struct MainConvolution {
    parameters:NeuralConv, history:Vec<f32>, ring_rows:usize, next:usize,
    actual_rows:usize, fed_rows:usize, right:usize, flush_remaining:usize, closing:bool,
    output:Vec<f32>,
}
impl MainConvolution {
    fn new(parameters:NeuralConv,left:usize,right:usize)->Result<Self> {
        let support=(parameters.taps.len()-1).checked_mul(parameters.dilation)
            .ok_or("Persistent convolution support overflow")?;
        if left.checked_add(right)!=Some(support) {return Err("Persistent convolution padding differs".into());}
        let ring_rows=support+1;
        let history=vec![0.;ring_rows*parameters.input_width];
        let output=vec![0.;parameters.output_width];
        Ok(Self {parameters,history,ring_rows,next:0,actual_rows:0,fed_rows:0,right,
            flush_remaining:0,closing:false,output})
    }
    fn push(&mut self,row:&[f32],work:Option<&mut QuantizedWorkspace>)->Result<Option<usize>> {
        if self.closing {return Err("Persistent main convolution already closed".into());}
        self.actual_rows=self.actual_rows.checked_add(1).ok_or("Persistent convolution row overflow")?;
        self.feed(Some(row),work)
    }
    fn flush_next(&mut self,mut work:Option<&mut QuantizedWorkspace>)->Result<Option<usize>> {
        if !self.closing {self.closing=true;self.flush_remaining=self.right;}
        while self.flush_remaining>0 {
            self.flush_remaining-=1;
            if let Some(row)=self.feed(None,work.as_deref_mut())? {return Ok(Some(row));}
        }
        Ok(None)
    }
    fn feed(&mut self,row:Option<&[f32]>,mut work:Option<&mut QuantizedWorkspace>)->Result<Option<usize>> {
        let width=self.parameters.input_width;
        let destination=&mut self.history[self.next*width..(self.next+1)*width];
        if let Some(row)=row {destination.copy_from_slice(row);} else {destination.fill(0.);}
        self.next=(self.next+1)%self.ring_rows;
        let index=self.fed_rows;
        self.fed_rows=self.fed_rows.checked_add(1).ok_or("Persistent convolution position overflow")?;
        let Some(ordinal)=index.checked_sub(self.right) else {return Ok(None);};
        if ordinal>=self.actual_rows {return Ok(None);}
        let history=&self.history;let next=self.next;let rows=self.ring_rows;
        self.parameters.apply_taps_using(|tap| {
            let row=(next+tap*self.parameters.dilation)%rows;&history[row*width..(row+1)*width]
        },&mut self.output,work.as_deref_mut())?;
        Ok(Some(ordinal))
    }
}

struct PersistentBlock {
    main:MainConvolution, condition:NeuralConv, residual:NeuralConv, skip:NeuralConv,
    activation_arithmetic:crate::activation::ArithmeticMode,
    residual_rows:VecDeque<(usize,Vec<f32>)>, condition_work:Vec<f32>, gated:Vec<f32>, skip_work:Vec<f32>,

    quantized_work:Option<QuantizedWorkspace>,
}
impl PersistentBlock {
    fn new(block:NcwnBlock)->Result<Self> {
        let (left,right)=block.main_padding()?;
        let double=block.main.output_width;let half=block.residual.output_width;let skip_width=block.skip.output_width;
        let requirements=[&block.main,&block.condition,&block.residual,&block.skip].into_iter()
            .flat_map(|conv|conv.taps.iter())
            .filter_map(|matrix|matrix.quantized_workspace_requirements())
            .reduce(|(input,output),(next_input,next_output)|
                (input.max(next_input),output.max(next_output)));
        let quantized_work=requirements.map(|(input,output)|QuantizedWorkspace::new(input,output)).transpose()?;
        Ok(Self {main:MainConvolution::new(block.main,left,right)?,condition:block.condition,
            residual:block.residual,skip:block.skip,residual_rows:VecDeque::with_capacity(right+1),
            condition_work:vec![0.;double],gated:vec![0.;half],skip_work:vec![0.;skip_width],quantized_work,
            activation_arithmetic:block.activation_arithmetic})
    }
    fn push(&mut self,ordinal:usize,input:&[f32],conditions:&VecDeque<Vec<f32>>,first:usize)
        ->Result<Option<(usize,Vec<f32>)>> {
        if ordinal!=self.main.actual_rows {return Err("Persistent NCWN main input ordinal differs".into());}
        let mut residual=vec![0.;self.residual.output_width];
        pointwise_into_using(&self.residual,input,&mut residual,self.quantized_work.as_mut())?;
        self.residual_rows.push_back((ordinal,residual));
        match self.main.push(input,self.quantized_work.as_mut())? {
            Some(ordinal)=>self.complete(ordinal,conditions,first).map(Some),None=>Ok(None),
        }
    }
    fn flush_next(&mut self,conditions:&VecDeque<Vec<f32>>,first:usize)->Result<Option<(usize,Vec<f32>)>> {
        match self.main.flush_next(self.quantized_work.as_mut())? {
            Some(ordinal)=>self.complete(ordinal,conditions,first).map(Some),None=>Ok(None),
        }
    }
    fn complete(&mut self,ordinal:usize,conditions:&VecDeque<Vec<f32>>,first:usize)->Result<(usize,Vec<f32>)> {
        let position=ordinal.checked_sub(first).ok_or("Persistent condition ordinal already retired")?;
        let condition=conditions.get(position).ok_or("Missing persistent NCWN condition row")?;
        pointwise_into_using(&self.condition,condition,&mut self.condition_work,self.quantized_work.as_mut())?;
        super::gate_activation::apply(self.activation_arithmetic,&self.main.output,&self.condition_work,&mut self.gated)?;
        pointwise_into_using(&self.skip,&self.gated,&mut self.skip_work,self.quantized_work.as_mut())?;
        let (residual_ordinal,mut current)=self.residual_rows.pop_front().ok_or("Missing persistent residual row")?;
        if residual_ordinal!=ordinal {return Err("Persistent residual ordinal differs".into());}
        for (value,gated) in current.iter_mut().zip(&self.gated) {*value+=gated;}
        Ok((ordinal,current))
    }
}

struct MixedNetwork {
    blocks:Vec<PersistentBlock>,head:NeuralConv,
    conditions:VecDeque<Vec<f32>>,outputs:VecDeque<Vec<f32>>,
    received:usize,emitted:usize,lookahead:usize,head_work:Option<QuantizedWorkspace>,
}
impl MixedNetwork {
    fn accept(&mut self,input:&[f32],condition:&[f32],output:&mut Vec<Vec<f32>>)->Result<()> {
        let ordinal=self.received;
        self.received=self.received.checked_add(1).ok_or("Persistent network row overflow")?;
        let mut head=vec![0.;self.head.output_width];pointwise_into_using(&self.head,condition,&mut head,self.head_work.as_mut())?;
        self.conditions.push_back(condition.to_vec());self.outputs.push_back(head);
        self.propagate(0,ordinal,input.to_vec(),output)
    }
    fn propagate(&mut self,start:usize,mut ordinal:usize,mut current:Vec<f32>,output:&mut Vec<Vec<f32>>)->Result<()> {
        for block in self.blocks.iter_mut().skip(start) {
            let Some((next,next_current))=block.push(ordinal,&current,&self.conditions,self.emitted)? else {return Ok(());};
            let pending=self.outputs.get_mut(next.checked_sub(self.emitted).ok_or("Persistent output already retired")?)
                .ok_or("Missing persistent output row")?;
            for (destination,skip) in pending.iter_mut().zip(&block.skip_work) {*destination+=skip;}
            ordinal=next;current=next_current;
        }
        self.publish(ordinal,output)
    }
    fn publish(&mut self,ordinal:usize,output:&mut Vec<Vec<f32>>)->Result<()> {
        if ordinal!=self.emitted {return Err("Persistent output ordinal differs".into());}
        let row=self.outputs.pop_front().ok_or("Missing completed persistent output")?;
        self.conditions.pop_front().ok_or("Missing completed persistent condition")?;
        output.push(row);self.emitted+=1;Ok(())
    }
    fn finish(&mut self,output:&mut Vec<Vec<f32>>)->Result<()> {

        for index in 0..self.blocks.len() {
            loop {
                let Some((ordinal,current))=self.blocks[index].flush_next(&self.conditions,self.emitted)? else {break;};
                let pending=self.outputs.get_mut(ordinal.checked_sub(self.emitted).ok_or("Persistent flush ordinal retired")?)
                    .ok_or("Missing persistent flush output")?;
                for (destination,skip) in pending.iter_mut().zip(&self.blocks[index].skip_work) {*destination+=skip;}
                self.propagate(index+1,ordinal,current,output)?;
            }
        }
        if self.emitted!=self.received || !self.outputs.is_empty() || !self.conditions.is_empty() {
            return Err("Persistent NCWN final row association incomplete".into());
        }
        Ok(())
    }
}
