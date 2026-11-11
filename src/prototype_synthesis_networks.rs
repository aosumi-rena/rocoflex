use crate::{Result, model::{ModelNode, ModelTree}, model_tags::decoded_hashes};
#[path="prototype_packed_float.rs"]
mod packed_float;
#[path="prototype_cepstral_pointwise.rs"]
mod pointwise_cepstral;
pub(crate) use pointwise_cepstral::PointwiseCepstralBands;
#[path="prototype_gate_activation.rs"]
pub mod gate_activation;

pub const SYNTHESIS_ACTIVATION_ARITHMETIC: crate::activation::ArithmeticMode =
    crate::activation::ArithmeticMode { fused_multiply_add: false };

pub const SYNTHESIS_PRIM4_OVERWRITE_FOLD: crate::quantized_matrix::OverwriteFold =
    crate::quantized_matrix::OverwriteFold::SaturateEach;

pub const SYNTHESIS_PRIM4_ADD_FOLD: crate::quantized_matrix::OverwriteFold =
    crate::quantized_matrix::OverwriteFold::SaturateEach;

#[path="prototype_ncwn_stream.rs"]
mod persistent_ncwn;
pub use persistent_ncwn::NcwnPersistentStream;

#[path="prototype_conditioning.rs"]
mod conditioning;
pub use conditioning::ConditioningNetwork;

#[path="prototype_filterbanks.rs"]
mod filterbanks;
pub use filterbanks::{AnalysisBank8, AnalysisBankProgress, SynthesisBank8, SynthesisBankProgress};

#[path="morph_sampling.rs"]
mod morph_sampling;

fn word(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(data.get(offset..offset + 4).ok_or("Truncated neural parameter")?.try_into().unwrap()))
}
fn floats(data: &[u8]) -> Result<Vec<f32>> {
    if data.len() % 4 != 0 { return Err("Incomplete f32 parameter".into()); }
    let result: Vec<_> = data.chunks_exact(4).map(|x| f32::from_le_bytes(x.try_into().unwrap())).collect();
    if result.iter().any(|x| !x.is_finite()) { return Err("Nonfinite model parameter".into()); }
    Ok(result)
}
fn is_tag(node: &ModelNode<'_>, tag: &str) -> bool {
    decoded_hashes(tag).contains(&(node.tag ^ 0x40dab9846f846a19))

        || (tag == "prim2" && [0xce8a79eed68f751b, 0x618fd1bc101c6bed,
            0x884331b14da58263].contains(&node.tag))
}
fn vector(node: &ModelNode<'_>, width: usize) -> Result<Vec<f32>> {
    if !is_tag(node, "prim1") || word(node.data, 0)? as usize != width {
        return Err("Neural bias dimensions differ".into());
    }
    let values = floats(&node.data[4..])?;
    if values.len() != width { return Err("Neural bias extent differs".into()); }
    Ok(values)
}
fn gru_work(width: usize) -> crate::modl3_values::Work {
    crate::modl3_values::Work { reset: vec![0.; width], update: vec![0.; width],
        candidate: vec![0.; width], stage: crate::modl3_values::Stage::Ready }
}

#[inline]
fn dot(input: &[f32], weights: &[f32]) -> f32 {
    debug_assert_eq!(input.len(), weights.len());
    #[cfg(target_arch="x86_64")]
    {
        use std::arch::x86_64::*;

        unsafe {
            let mut a=_mm_setzero_ps(); let mut b=_mm_setzero_ps(); let mut i=0;
            while i+8<=input.len() {
                let x=_mm_loadu_ps(input.as_ptr().add(i));
                let y=_mm_loadu_ps(input.as_ptr().add(i+4));
                a=_mm_add_ps(a,_mm_mul_ps(x,_mm_loadu_ps(weights.as_ptr().add(i))));
                b=_mm_add_ps(b,_mm_mul_ps(y,_mm_loadu_ps(weights.as_ptr().add(i+4))));
                i+=8;
            }
            let mut lanes=[0.;4]; _mm_storeu_ps(lanes.as_mut_ptr(),_mm_add_ps(a,b));
            let mut sum=lanes[0]+lanes[1]+lanes[2]+lanes[3];
            while i<input.len() {sum+=input[i]*weights[i];i+=1;}
            sum
        }
    }
    #[cfg(not(target_arch="x86_64"))]
    { input.iter().zip(weights).map(|(&x,&w)| x*w).sum() }
}

#[cfg(target_arch="x86_64")]
mod float_matrix_avx2 {
    use std::arch::x86_64::*;
    use super::dot;

#[inline]
#[target_feature(enable="avx2")]
unsafe fn reduce(v:__m256)->f32 {

    unsafe {
    let a=_mm256_castps256_ps128(v);let b=_mm256_extractf128_ps::<1>(v);
    let mut lanes=[0.;4];_mm_storeu_ps(lanes.as_mut_ptr(),_mm_add_ps(a,b));
    lanes[0]+lanes[1]+lanes[2]+lanes[3]
    }
}
#[inline(never)]
#[target_feature(enable="avx2")]
pub(super) unsafe fn apply<const ADD:bool>(x:&[f32],w:&[f32],out:&mut[f32]) {

    unsafe {
    let n=x.len();let tiles=out.len()/4*4;let mut o=0;
    while o<tiles {
        let p=w.as_ptr().add(o*n);
        let mut a=_mm256_setzero_ps();let mut b=_mm256_setzero_ps();
        let mut c=_mm256_setzero_ps();let mut d=_mm256_setzero_ps();let mut i=0;
        while i+8<=n {
            let v=_mm256_loadu_ps(x.as_ptr().add(i));
            a=_mm256_add_ps(a,_mm256_mul_ps(v,_mm256_loadu_ps(p.add(i))));
            b=_mm256_add_ps(b,_mm256_mul_ps(v,_mm256_loadu_ps(p.add(n+i))));
            c=_mm256_add_ps(c,_mm256_mul_ps(v,_mm256_loadu_ps(p.add(2*n+i))));
            d=_mm256_add_ps(d,_mm256_mul_ps(v,_mm256_loadu_ps(p.add(3*n+i))));
            i+=8;
        }
        let mut s0=reduce(a);let mut s1=reduce(b);let mut s2=reduce(c);let mut s3=reduce(d);
        while i<n {
            s0+=x[i]*w[o*n+i];s1+=x[i]*w[(o+1)*n+i];
            s2+=x[i]*w[(o+2)*n+i];s3+=x[i]*w[(o+3)*n+i];i+=1;
        }
        if ADD {out[o]+=s0;out[o+1]+=s1;out[o+2]+=s2;out[o+3]+=s3;}
        else {out[o]=s0;out[o+1]=s1;out[o+2]=s2;out[o+3]=s3;}
        o+=4;
    }
    for j in tiles..out.len() {
        let sum=dot(x,&w[j*n..(j+1)*n]);
        if ADD {out[j]+=sum;} else {out[j]=sum;}
    }
    }
}
}

enum MatrixStorage {
    Float(Vec<f32>),
    Quantized(crate::quantized_matrix::QuantizedMatrix),
}

pub struct DenseMatrix {
    pub input_width: usize,
    pub output_width: usize,
    storage: MatrixStorage,
}
impl DenseMatrix {
    pub fn from_node(node: &ModelNode<'_>) -> Result<Self> {
        let layout = crate::quantized_matrix::DenseLayout::Auto;
        Self::from_node_with_layout(node, layout)
    }
    fn from_node_with_layout(node: &ModelNode<'_>, layout: crate::quantized_matrix::DenseLayout)
        -> Result<Self>
    {
        if !node.children.is_empty() { return Err("Matrix parameter must be a leaf".into()); }
        let data = node.data;
        if is_tag(node, "prim0") || is_tag(node, "prim2") {
            let output_width = word(data, 0)? as usize;
            let input_width = word(data, 4)? as usize;
            let weights = floats(data.get(8..).ok_or("Missing float coefficients")?)?;
            if input_width == 0 || output_width == 0 || input_width.checked_mul(output_width) != Some(weights.len()) {
                return Err("Float matrix extent mismatch".into());
            }
            return Ok(Self { input_width, output_width, storage:MatrixStorage::Float(weights) });
        }
        if is_tag(node, "prim4") {
            let mut matrix = crate::quantized_matrix::QuantizedMatrix::from_payload(data, word(data, 4)?)?;
            matrix.prepare_dense_layout(layout)?;
            return Ok(Self { input_width:matrix.input_width(), output_width:matrix.output_width(),
                storage:MatrixStorage::Quantized(matrix) });
        }
        Err(format!("Unsupported neural matrix tag {:016x}; expected prim0/prim2/prim4", node.tag))
    }

    pub fn quantized_workspace_requirements(&self) -> Option<(usize, usize)> {
        match &self.storage {
            MatrixStorage::Float(_) => None,
            MatrixStorage::Quantized(matrix) => Some((matrix.input_width(), matrix.output_width())),
        }
    }
    pub fn overwrite(&self, input: &[f32], output: &mut [f32]) -> Result<()> {
        self.overwrite_using(input, output, None)
    }
    pub fn overwrite_with_workspace(&self, input: &[f32], output: &mut [f32],
        work: &mut crate::quantized_matrix::QuantizedWorkspace) -> Result<()>
    {
        self.overwrite_using(input, output, Some(work))
    }
    fn overwrite_using(&self, input: &[f32], output: &mut [f32],
        work: Option<&mut crate::quantized_matrix::QuantizedWorkspace>) -> Result<()>
    {
        if input.len() != self.input_width || output.len() != self.output_width {
            return Err("Neural matrix input/output width mismatch".into());
        }
        match &self.storage {
            MatrixStorage::Quantized(matrix) => {
                let saturate_each = matches!(SYNTHESIS_PRIM4_OVERWRITE_FOLD,
                    crate::quantized_matrix::OverwriteFold::SaturateEach);
                if let Some(work) = work {
                    matrix.forward_unbiased_saturation_into(input, output, saturate_each, work)?;
                } else {
                    let values = matrix.forward_unbiased_saturation(input, saturate_each)?;
                    output.copy_from_slice(&values);
                }
            }
            MatrixStorage::Float(weights) => {
                #[cfg(target_arch="x86_64")]
                if self.input_width >= 8 && self.output_width >= 4
                    && self.input_width.checked_mul(self.output_width) == Some(weights.len())
                    && std::is_x86_feature_detected!("avx2") {

                    unsafe { float_matrix_avx2::apply::<false>(input, weights, output); }
                    return Ok(());
                }
                for (destination, weights) in output.iter_mut().zip(weights.chunks_exact(self.input_width)) {
                    *destination=dot(input,weights);
                }
            }
        }
        Ok(())
    }

    pub fn add(&self, input: &[f32], output: &mut [f32]) -> Result<()> {
        self.add_using(input, output, None)
    }
    pub fn add_with_workspace(&self, input: &[f32], output: &mut [f32],
        work: &mut crate::quantized_matrix::QuantizedWorkspace) -> Result<()>
    {
        self.add_using(input, output, Some(work))
    }

    fn add_using(&self, input: &[f32], output: &mut [f32],
        work: Option<&mut crate::quantized_matrix::QuantizedWorkspace>) -> Result<()>
    {
        if input.len() != self.input_width || output.len() != self.output_width {
            return Err("Neural matrix input/output width mismatch".into());
        }
        match &self.storage {
            MatrixStorage::Quantized(matrix) => match work {
                Some(work) => matrix.forward_add_saturating_f32_with_workspace(input, output, work),
                None => matrix.forward_add_saturating_f32(input, output),
            },
            MatrixStorage::Float(weights) => {
                #[cfg(target_arch="x86_64")]
                if self.input_width >= 8 && self.output_width >= 4
                    && self.input_width.checked_mul(self.output_width) == Some(weights.len())
                    && std::is_x86_feature_detected!("avx2") {
                    unsafe { float_matrix_avx2::apply::<true>(input, weights, output); }
                    return Ok(());
                }
                for (destination, weights) in output.iter_mut().zip(weights.chunks_exact(self.input_width)) {
                    *destination+=dot(input,weights);
                }
                Ok(())
            }
        }
    }
}

struct AuxiliaryLinear { matrix:DenseMatrix, bias:Vec<f32> }
impl AuxiliaryLinear {
    fn from_node(node:&ModelNode<'_>,input:usize,output:usize)->Result<Self> {
        if !is_tag(node,"modl0") || !node.data.is_empty() || node.children.len()!=2 {
            return Err("Auxiliary head requires a biased linear operator".into());
        }
        let matrix=DenseMatrix::from_node(&node.children[0])?;
        if (matrix.input_width,matrix.output_width)!=(input,output) {
            return Err("Auxiliary head matrix dimensions differ".into());
        }
        let bias=vector(&node.children[1],output)?;
        Ok(Self {matrix,bias})
    }
    fn forward(&self,input:&[f32])->Result<Vec<f32>> {
        let mut output=vec![0.;self.matrix.output_width];
        self.matrix.overwrite(input,&mut output)?;
        for (value,bias) in output.iter_mut().zip(&self.bias) {*value+=bias;}
        Ok(output)
    }
}

pub struct HarmonicProjection { first: AuxiliaryLinear, last: AuxiliaryLinear }
impl HarmonicProjection {
    pub fn from_node(node: &ModelNode<'_>) -> Result<Self> {
        node.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
        if !node.data.is_empty() || node.children.len() != 3 {
            return Err("Harmonic projection requires the stored three-stage sequence".into());
        }
        let activation = &node.children[1];
        activation.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
        if !activation.data.is_empty() || !activation.children.is_empty() {
            return Err("Harmonic projection requires a SiLU leaf".into());
        }
        for linear in [&node.children[0], &node.children[2]] {
            linear.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
            if !linear.data.is_empty() || linear.children.len() != 2 {
                return Err("Harmonic projection requires stored matrix/bias pairs".into());
            }
            linear.children[0].require(0x7fca40ff, b"prim0", decoded_hashes("prim0"))?;
            linear.children[1].require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if !linear.children[1].children.is_empty() {
                return Err("Harmonic projection bias must be a leaf".into());
            }
        }
        Ok(Self { first: AuxiliaryLinear::from_node(&node.children[0], 256, 64)?,
            last: AuxiliaryLinear::from_node(&node.children[2], 64, 10)? })
    }

    pub fn forward(&self, second256: &[f32]) -> Result<[f32; 10]> {
        if second256.len() != 256 || second256.iter().any(|x| !x.is_finite()) {
            return Err("Harmonic projection requires one finite second-network 256-value row".into());
        }
        let mut hidden = [0.; 64];
        self.first.matrix.overwrite(second256, &mut hidden)?;
        for (value,bias) in hidden.iter_mut().zip(&self.first.bias) {*value+=bias;}
        for value in &mut hidden { *value *= SYNTHESIS_ACTIVATION_ARITHMETIC.sigmoid(*value); }
        let mut output = [0.; 10];
        self.last.matrix.overwrite(&hidden, &mut output)?;
        for (value,bias) in output.iter_mut().zip(&self.last.bias) {*value+=bias;}
        if output.iter().any(|x| !x.is_finite()) {
            return Err("Harmonic projection output is nonfinite".into());
        }
        Ok(output)
    }
}

fn auxiliary_leaf(node:&ModelNode<'_>,tags:&[u64])->Result<()> {
    if node.kind!=0x7fca41ff || !node.data.is_empty() || !node.children.is_empty() || !tags.contains(&node.tag) {
        return Err("Unrecognized auxiliary head activation".into());
    }
    Ok(())
}
fn auxiliary_sequence(node:&ModelNode<'_>,count:usize)->Result<()> {
    if !is_tag(node,"modm0") || !node.data.is_empty() || node.children.len()!=count {
        return Err("Auxiliary head sequence layout differs".into());
    }
    Ok(())
}
fn log_softmax(values:&mut [f32]) {
    let maximum=values.iter().copied().fold(f32::NEG_INFINITY,f32::max);
    let log_sum=values.iter().map(|x|(*x-maximum).exp()).sum::<f32>().ln();
    for value in values {*value=(*value-maximum)-log_sum;}
}

struct Root1AuxHead {small:AuxiliaryLinear,large_first:AuxiliaryLinear,large_last:AuxiliaryLinear}

pub struct Root1AuxHeads {heads:[Root1AuxHead;3]}
impl Root1AuxHeads {
    pub fn from_dnni(bytes:&[u8])->Result<Self> {
        let tree=ModelTree::parse(bytes)?;
        if tree.version!=2 {return Err("Auxiliary heads require the version2 model".into());}
        let root=tree.root.children.get(1).ok_or("Missing mode1 root1 model")?;
        if root.data.len()!=76 || root.children.len()!=14 || word(root.data,4)?!=5
            || word(root.data,8)?!=107 || word(root.data,24)?!=3 {
            return Err("Mode1 auxiliary head configuration differs".into());
        }
        let head_tags=[0xb5c97a3d7a219af5,0x05696048b331f7d8,0xf9a98d90200a23d5];
        let log_softmax_tags=[0xe3176622051dab0f,0x1b70b29be40314ad,0x594538e1a08bd0d8];
        let glu_tags=[0x82afc253a00121c4,0xbfa170d62a009c57,0xfb505bf192b0b6b7];
        let mut heads=Vec::with_capacity(3);
        for (ordinal,node) in root.children[11..14].iter().enumerate() {
            if node.kind!=0x7fca41ff || node.tag!=head_tags[ordinal]
                || !node.data.is_empty() || node.children.len()!=2 {
                return Err("Mode1 auxiliary head binding differs".into());
            }
            let small=&node.children[0];let large=&node.children[1];
            auxiliary_sequence(small,2)?;auxiliary_sequence(large,4)?;
            auxiliary_leaf(&small.children[1],&log_softmax_tags)?;
            auxiliary_leaf(&large.children[1],&glu_tags)?;
            auxiliary_leaf(&large.children[3],&log_softmax_tags)?;
            heads.push(Root1AuxHead {small:AuxiliaryLinear::from_node(&small.children[0],384,5)?,
                large_first:AuxiliaryLinear::from_node(&large.children[0],384,512)?,
                large_last:AuxiliaryLinear::from_node(&large.children[2],256,107)?});
        }
        Ok(Self {heads:heads.try_into().map_err(|_|"Auxiliary head count differs")?})
    }

    pub fn forward(&self,input384:&[f32])->Result<(Vec<f32>,Vec<f32>)> {
        if input384.len()!=384 || input384.iter().any(|x|!x.is_finite()) {
            return Err("Auxiliary heads require one finite384-value temporal row".into());
        }
        let mut large_output=Vec::with_capacity(321);let mut small_output=Vec::with_capacity(15);
        for head in &self.heads {
            let mut small=head.small.forward(input384)?;log_softmax(&mut small);
            small_output.extend(small);
            let mut hidden=head.large_first.forward(input384)?;
            let (first,second)=hidden.split_at_mut(256);

            for (value,gate) in first.iter_mut().zip(second) {*value*=SYNTHESIS_ACTIVATION_ARITHMETIC.sigmoid(*gate);}
            let mut large=head.large_last.forward(first)?;log_softmax(&mut large);
            large_output.extend(large);
        }
        if large_output.iter().chain(&small_output).any(|x|!x.is_finite()) {
            return Err("Auxiliary head log probabilities are nonfinite".into());
        }
        let small: &mut [f32; 15] = small_output.as_mut_slice().try_into()
            .map_err(|_| "Auxiliary Small15 extent differs")?;
        let large: &mut [f32; 321] = large_output.as_mut_slice().try_into()
            .map_err(|_| "Auxiliary Large321 extent differs")?;
        morph_sampling::normalize_auxiliary(small, large)?;
        Ok((large_output,small_output))
    }
}

pub struct FloatGru {
    matrices: [DenseMatrix; 6],
    biases: [Vec<f32>; 6],
    reset_bias: Vec<f32>, update_bias: Vec<f32>,
    hidden: Vec<f32>, work: crate::modl3_values::Work,
    quantized_work: Option<crate::quantized_matrix::QuantizedWorkspace>,
}
impl FloatGru {
    pub fn from_node(node: &ModelNode<'_>) -> Result<Self> {
        if !is_tag(node, "modl3") || node.children.len() != 12 { return Err("Expected six-matrix modl3 GRU".into()); }
        let mut matrices = Vec::new(); let mut biases = Vec::new();
        for pair in node.children.chunks_exact(2) {
            let matrix = DenseMatrix::from_node(&pair[0])?;
            let bias = vector(&pair[1], matrix.output_width)?;
            matrices.push(matrix); biases.push(bias);
        }
        let matrices: [DenseMatrix; 6] = matrices.try_into().map_err(|_| "GRU matrix count")?;
        let biases: [Vec<f32>; 6] = biases.try_into().map_err(|_| "GRU bias count")?;
        let width = matrices[0].output_width;
        if matrices.iter().enumerate().any(|(i, m)| m.output_width != width
            || m.input_width != if i < 3 { matrices[0].input_width } else { width }) {
            return Err("GRU matrix dimensions do not connect".into());
        }
        let reset_bias = biases[0].iter().zip(&biases[3]).map(|(a,b)| a+b).collect();
        let update_bias = biases[1].iter().zip(&biases[4]).map(|(a,b)| a+b).collect();
        let requirements = matrices.iter().filter_map(DenseMatrix::quantized_workspace_requirements)
            .reduce(|(input, output), (next_input, next_output)|
                (input.max(next_input), output.max(next_output)));
        let quantized_work = requirements.map(|(input, output)|
            crate::quantized_matrix::QuantizedWorkspace::new(input, output)).transpose()?;
        Ok(Self { matrices, biases, reset_bias, update_bias, hidden: vec![0.; width],
            work: gru_work(width), quantized_work })
    }
    pub fn input_width(&self) -> usize { self.matrices[0].input_width }
    pub fn hidden(&self) -> &[f32] { &self.hidden }
    pub fn reset(&mut self) {
        self.hidden.fill(0.);
        self.work.reset.fill(0.);
        self.work.update.fill(0.);
        self.work.candidate.fill(0.);
        self.work.stage = crate::modl3_values::Stage::Ready;
    }
    pub fn step(&mut self, input: &[f32]) -> Result<&[f32]> {
        if input.len() != self.input_width() || input.iter().any(|x| !x.is_finite()) { return Err("Invalid GRU input".into()); }
        let Self { matrices, biases, reset_bias, update_bias, hidden, work, quantized_work } = self;
        let biases = std::array::from_fn(|i| biases[i].as_slice());
        crate::modl3_values::forward_row(input, hidden, biases, reset_bias,
            update_bias, work,
            &mut |i, x, y| match quantized_work.as_mut() {
                Some(scratch) => matrices[i].add_with_workspace(x, y, scratch),
                None => matrices[i].add(x, y),
            },
            &mut |kind, row| -> Result<()> {
                for x in row { *x = match kind {
                    crate::modl3_values::Activation::Sigmoid => SYNTHESIS_ACTIVATION_ARITHMETIC.sigmoid(*x),
                    crate::modl3_values::Activation::Tanh => SYNTHESIS_ACTIVATION_ARITHMETIC.tanh(*x),
                }; } Ok(())
            }).map_err(|_| "Float GRU execution failed".to_owned())?;
        Ok(hidden.as_slice())
    }
}

pub struct NeuralConv {
    taps: Vec<DenseMatrix>, bias: Vec<f32>,
    pub input_width: usize, pub output_width: usize,
    stride: usize, padding: usize, dilation: usize,
    packed_float:Option<packed_float::PackedFloatConvolution>,
}
impl NeuralConv {
    fn streaming_context(&self)->Result<(usize,usize)> {
        if self.stride!=1 {return Err("Streaming NCWN requires unit convolution stride".into());}
        let span=(self.taps.len()-1).checked_mul(self.dilation).ok_or("Convolution context overflow")?;
        let right=span.checked_sub(self.padding).ok_or("Convolution padding exceeds support")?;
        Ok((self.padding,right))
    }
    pub fn from_node(node: &ModelNode<'_>) -> Result<Self> {
        if !is_tag(node, "modl1") || node.data.len() != 20 { return Err("Expected modl1 convolution".into()); }
        let count = word(node.data, 0)? as usize;
        if count == 0 || node.children.len() != count + 1 { return Err("Convolution tap count mismatch".into()); }
        let taps = node.children[..count].iter().map(DenseMatrix::from_node).collect::<Result<Vec<_>>>()?;
        let input_width = taps[0].input_width; let output_width = taps[0].output_width;
        if taps.iter().any(|m| m.input_width != input_width || m.output_width != output_width) {
            return Err("Convolution tap width mismatch".into());
        }
        let bias = vector(&node.children[count], output_width)?;
        let stride = word(node.data, 4)? as usize; let dilation = word(node.data, 12)? as usize;
        if stride == 0 || dilation == 0 { return Err("Invalid convolution stride/dilation".into()); }
        Ok(Self { taps, bias, input_width, output_width, stride,
            padding: word(node.data, 8)? as usize, dilation,packed_float:None })
    }
    fn enable_packed_float(&mut self)->Result<()> {
        self.packed_float=Some(packed_float::PackedFloatConvolution::prepare(&self.taps)?);Ok(())
    }
    fn apply_taps_using<'a>(&self,mut row:impl FnMut(usize)->&'a[f32],output:&mut[f32],
        mut work:Option<&mut crate::quantized_matrix::QuantizedWorkspace>)->Result<()> {
        if let Some(packed)=&self.packed_float {packed.apply(&mut row,output)?;}
        else {
            for (tap,matrix) in self.taps.iter().enumerate() {
                let input=row(tap);
                if tap==0 {matrix.overwrite_using(input,output,work.as_deref_mut())?;}
                else {matrix.add_using(input,output,work.as_deref_mut())?;}
            }
        }
        for (value,bias) in output.iter_mut().zip(&self.bias) {*value+=bias;}
        Ok(())
    }

    pub fn forward(&self, input: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
        self.forward_with_padding(input,self.padding,self.padding)
    }
    fn forward_with_padding(&self,input:&[Vec<f32>],left:usize,right:usize)->Result<Vec<Vec<f32>>> {
        if input.iter().any(|row| row.len() != self.input_width) { return Err("Convolution row width mismatch".into()); }
        let effective = (self.taps.len()-1)*self.dilation+1;
        let padded = input.len().checked_add(left).and_then(|n|n.checked_add(right)).ok_or("Convolution extent overflow")?;
        if padded < effective { return Ok(Vec::new()); }
        let rows = (padded-effective)/self.stride+1;
        let mut output = vec![vec![0.;self.output_width]; rows];
        let padding = vec![0.;self.input_width];
        for (r, destination) in output.iter_mut().enumerate() {
            self.apply_taps_using(|tap| {
                let position = r*self.stride+tap*self.dilation;
                position.checked_sub(left).and_then(|p| input.get(p))
                    .map(Vec::as_slice).unwrap_or(&padding)
            },destination,None)?;
        }
        Ok(output)
    }
}

struct NcwnBlock {
    main: NeuralConv, condition: NeuralConv, skip: NeuralConv, residual: NeuralConv,
    warmup_rows: Option<usize>,
    activation_arithmetic: crate::activation::ArithmeticMode,
}
impl NcwnBlock {
    fn main_padding(&self)->Result<(usize,usize)> {
        match self.warmup_rows {
            None=>Ok((self.main.padding,self.main.padding)),
            Some(warmup)=>{
                let support=(self.main.taps.len()-1).checked_mul(self.main.dilation)
                    .ok_or("NCWN warmup support overflow")?;
                if self.main.stride!=1 || self.main.padding!=0 || warmup!=support {
                    return Err("NCWN20-byte main warmup must equal support with stride1/pad0".into());
                }

                Ok((warmup,0))
            }
        }
    }
    fn main_context(&self)->Result<(usize,usize)> {
        if self.warmup_rows.is_some() {self.main_padding()}
        else {self.main.streaming_context()}
    }
    fn main_forward(&self,input:&[Vec<f32>])->Result<Vec<Vec<f32>>> {
        let (left,right)=self.main_padding()?;
        self.main.forward_with_padding(input,left,right)
    }
}
pub struct NcwnNetwork {
    pub input_width: usize, pub output_width: usize, pub condition_width: usize,
    blocks: Vec<NcwnBlock>, head: NeuralConv,
}
impl NcwnNetwork {

    pub fn with_activation_arithmetic(mut self, arithmetic:crate::activation::ArithmeticMode)->Self {
        for block in &mut self.blocks {block.activation_arithmetic=arithmetic;}
        self
    }

    pub fn with_packed_float_convolutions(mut self)->Result<Self> {
        self.head.enable_packed_float()?;
        for block in &mut self.blocks {
            block.main.enable_packed_float()?;block.condition.enable_packed_float()?;
            block.residual.enable_packed_float()?;block.skip.enable_packed_float()?;
        }
        Ok(self)
    }

    pub fn streaming_context(&self)->Result<(usize,usize)> {
        let mut current=(0usize,0usize); let mut output=self.head.streaming_context()?;
        let add=|a:(usize,usize),b:(usize,usize)|->Result<(usize,usize)> {
            Ok((a.0.checked_add(b.0).ok_or("Network context overflow")?,a.1.checked_add(b.1).ok_or("Network context overflow")?))
        };
        let union=|a:(usize,usize),b:(usize,usize)|(a.0.max(b.0),a.1.max(b.1));
        for block in &self.blocks {
            let main=add(current,block.main_context()?)?;
            let gated=union(main,block.condition.streaming_context()?);
            output=union(output,add(gated,block.skip.streaming_context()?)?);
            current=union(gated,add(current,block.residual.streaming_context()?)?);
        }
        Ok(output)
    }
    pub fn from_node(node: &ModelNode<'_>) -> Result<Self> {
        if !is_tag(node,"_ncwnv0") || node.data.len()!=12 || node.children.len()!=4 {
            return Err("NCWN topology/configuration mismatch".into());
        }
        let input_width=word(node.data,0)? as usize; let output_width=word(node.data,4)? as usize;
        let condition_width=word(node.data,8)? as usize;
        let gates=&node.children[0].children; let skips=&node.children[1].children; let residuals=&node.children[2].children;
        if input_width==0 || output_width==0 || condition_width==0 || gates.is_empty() || gates.len()!=skips.len() || gates.len()!=residuals.len() {
            return Err("NCWN layer counts/dimensions differ".into());
        }
        let mut blocks=Vec::new();
        let mut current_width=input_width;
        for (i,gate) in gates.iter().enumerate() {
            if ![12,20].contains(&gate.data.len()) || gate.children.len()!=2 {
                return Err("NCWN gate requires main and condition convolutions".into());
            }
            if gate.data.len()==20 && word(gate.data,12)?!=0 { return Err("NCWN auxiliary operator is not implemented".into()); }
            let half_width=word(gate.data,4)? as usize;
            let double_width=half_width.checked_mul(2).filter(|&width|width>0).ok_or("Invalid NCWN hidden width")?;
            if word(gate.data,0)? as usize!=current_width || word(gate.data,8)? as usize!=condition_width {
                return Err("NCWN gate configuration dimensions differ".into());
            }
            let block=NcwnBlock {main:NeuralConv::from_node(&gate.children[0])?,condition:NeuralConv::from_node(&gate.children[1])?,
                warmup_rows: if gate.data.len()==20 {Some(word(gate.data,16)? as usize)} else {None},
                activation_arithmetic:SYNTHESIS_ACTIVATION_ARITHMETIC,
                skip:NeuralConv::from_node(&skips[i])?,residual:NeuralConv::from_node(&residuals[i])?};
            if block.main.input_width!=current_width || block.main.output_width!=double_width || block.condition.input_width!=condition_width
                || block.condition.output_width!=double_width || block.skip.input_width!=half_width || block.skip.output_width!=output_width
                || block.residual.input_width!=current_width || block.residual.output_width!=half_width {
                return Err("NCWN parameter matrix dimensions differ".into());
            }
            blocks.push(block);
            current_width=half_width;
        }
        let head=NeuralConv::from_node(&node.children[3])?;
        if head.input_width!=condition_width || head.output_width!=output_width { return Err("NCWN head dimensions differ".into()); }
        Ok(Self {input_width,output_width,condition_width,blocks,head})
    }
    pub fn forward(&self, input: &[Vec<f32>], condition: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
        self.forward_using(input,condition,None)
    }

    fn forward_using(&self,input:&[Vec<f32>],condition:&[Vec<f32>],
        mut check:Option<&mut gate_activation::GateCheck>)->Result<Vec<Vec<f32>>> {
        let mut current = input.to_vec(); let mut skips = Vec::new();
        for block in &self.blocks {
            let mut gate = block.main_forward(&current)?;
            let projected_condition = block.condition.forward(condition)?;
            let residual = block.residual.forward(&current)?;
            let n = gate.len().min(projected_condition.len()).min(residual.len());
            gate.truncate(n);
            let half = block.skip.input_width;
            let mut gated = Vec::with_capacity(n);
            for (g, c) in gate.iter().zip(&projected_condition) {
                if g.len() != 2*half || c.len() != g.len() { return Err("NCWN gate dimensions differ".into()); }
                let mut row=vec![0.;half];
                gate_activation::apply_checked(block.activation_arithmetic,g,c,&mut row,check.as_deref_mut())?;
                gated.push(row);
            }
            skips.push(block.skip.forward(&gated)?);
            for (row, r) in gated.iter_mut().zip(&residual) { for (x, y) in row.iter_mut().zip(r) { *x += y; } }
            current = gated;
        }
        let mut output = self.head.forward(condition)?;
        let count = skips.iter().map(Vec::len).fold(output.len(), usize::min); output.truncate(count);
        for skip in skips { for (row, other) in output.iter_mut().zip(skip) { for (x,y) in row.iter_mut().zip(other) { *x += y; } } }
        Ok(output)
    }

    pub fn into_causal_stream(self) -> Result<NcwnCausalStream> {
        require_pointwise(&self.head)?;
        for block in &self.blocks {
            let support=(block.main.taps.len()-1).checked_mul(block.main.dilation)
                .ok_or("Causal convolution support overflow")?;
            if block.main.taps.len()!=3 || block.main.stride!=1 || block.main.padding!=0 || block.warmup_rows!=Some(support) {
                return Err("Causal NCWN requires explicit 20-byte gate warmup equal to main support, stride1/pad0".into());
            }
            support.checked_add(1).and_then(|n|n.checked_mul(block.main.input_width))
                .ok_or("Causal convolution history extent overflow")?;
            require_pointwise(&block.condition)?;
            require_pointwise(&block.residual)?;
            require_pointwise(&block.skip)?;
        }

        let requirements=std::iter::once(&self.head)
            .chain(self.blocks.iter().flat_map(|block|
                [&block.main,&block.condition,&block.residual,&block.skip]))
            .flat_map(|conv|conv.taps.iter())
            .filter_map(DenseMatrix::quantized_workspace_requirements)
            .reduce(|(input,output),(next_input,next_output)|
                (input.max(next_input),output.max(next_output)));
        let quantized_work=requirements.map(|(input,output)|
            crate::quantized_matrix::QuantizedWorkspace::new(input,output)).transpose()?;
        let blocks=self.blocks.into_iter().map(CausalNcwnBlock::new).collect();
        Ok(NcwnCausalStream {input_width:self.input_width,output_width:self.output_width,
            condition_width:self.condition_width,blocks,head:self.head,
            output:vec![0.;self.output_width],quantized_work})
    }
}

fn require_pointwise(conv: &NeuralConv) -> Result<()> {
    if conv.taps.len()!=1 || conv.stride!=1 || conv.padding!=0 {
        return Err("Causal NCWN parallel convolutions must be pointwise stride1/pad0".into());
    }
    Ok(())
}

fn pointwise_into_using(conv: &NeuralConv, input: &[f32], output: &mut [f32],
    work: Option<&mut crate::quantized_matrix::QuantizedWorkspace>) -> Result<()>
{
    conv.apply_taps_using(|_|input,output,work)
}

struct CausalNcwnBlock {
    parameters: NcwnBlock,
    history: Vec<f32>, next: usize, history_rows: usize,
    gate: Vec<f32>, condition: Vec<f32>, residual: Vec<f32>,
    gated: Vec<f32>, output: Vec<f32>, skip: Vec<f32>,
}
impl CausalNcwnBlock {
    fn new(parameters: NcwnBlock) -> Self {

        let history_rows=parameters.warmup_rows.unwrap()+1;
        let width=parameters.main.input_width;
        let double=parameters.main.output_width;
        let half=parameters.residual.output_width;
        let skip=parameters.skip.output_width;
        Self {parameters,history:vec![0.;history_rows*width],next:0,history_rows,
            gate:vec![0.;double],condition:vec![0.;double],residual:vec![0.;half],
            gated:vec![0.;half],output:vec![0.;half],skip:vec![0.;skip]}
    }
    fn step(&mut self, input: &[f32], condition: &[f32],
        mut work: Option<&mut crate::quantized_matrix::QuantizedWorkspace>) -> Result<(&[f32], &[f32])>
    {
        let main=&self.parameters.main;
        let width=main.input_width;
        self.history[self.next*width..(self.next+1)*width].copy_from_slice(input);
        self.next=(self.next+1)%self.history_rows;

        let history=&self.history;let next=self.next;let rows=self.history_rows;
        main.apply_taps_using(|tap| {
            let row=(next+tap*main.dilation)%rows;&history[row*width..(row+1)*width]
        },&mut self.gate,work.as_deref_mut())?;
        pointwise_into_using(&self.parameters.condition,condition,&mut self.condition,work.as_deref_mut())?;
        pointwise_into_using(&self.parameters.residual,input,&mut self.residual,work.as_deref_mut())?;
        let half=self.gated.len();
        gate_activation::apply(self.parameters.activation_arithmetic,&self.gate,&self.condition,&mut self.gated)?;
        pointwise_into_using(&self.parameters.skip,&self.gated,&mut self.skip,work.as_deref_mut())?;
        for i in 0..half {self.output[i]=self.gated[i]+self.residual[i];}
        Ok((&self.output,&self.skip))
    }
}

pub struct NcwnCausalStream {
    pub input_width: usize, pub output_width: usize, pub condition_width: usize,
    blocks: Vec<CausalNcwnBlock>, head: NeuralConv, output: Vec<f32>,
    quantized_work: Option<crate::quantized_matrix::QuantizedWorkspace>,
}
impl NcwnCausalStream {

    pub fn process_row(&mut self, input: &[f32], condition: &[f32]) -> Result<&[f32]> {
        if input.len()!=self.input_width || condition.len()!=self.condition_width {
            return Err("Causal NCWN input/condition width mismatch".into());
        }
        pointwise_into_using(&self.head,condition,&mut self.output,self.quantized_work.as_mut())?;
        let mut current=input;
        for block in &mut self.blocks {
            let (next,skip)=block.step(current,condition,self.quantized_work.as_mut())?;
            for (destination,value) in self.output.iter_mut().zip(skip) {*destination+=value;}
            current=next;
        }
        Ok(&self.output)
    }

    pub fn process(&mut self, input: &[Vec<f32>], condition: &[Vec<f32>], output: &mut Vec<Vec<f32>>) -> Result<()> {
        if input.len()!=condition.len() || input.iter().any(|r|r.len()!=self.input_width)
            || condition.iter().any(|r|r.len()!=self.condition_width) {
            return Err("Causal NCWN batch input/condition dimensions differ".into());
        }
        output.reserve(input.len());
        for (row,condition) in input.iter().zip(condition) {output.push(self.process_row(row,condition)?.to_vec());}
        Ok(())
    }
}

pub struct SynthesisNetworks {
    pub first: NcwnNetwork, pub second: NcwnNetwork, pub subgroup: NcwnNetwork,
    pub cepstral: CepstralBands,
    pub down8: AnalysisBank8, pub up8: SynthesisBank8, pub post: SynthesisPost,
    pub conditioning: ConditioningNetwork,
    pub projection: HarmonicProjection,
    pub frequencies: [f32; 5],
}

impl SynthesisNetworks {
    pub fn from_userdata(bytes: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(bytes)?;
        if tree.version != 2 { return Err("Synthesis model must be DNNI version2".into()); }
        let root = tree.root.children.first().ok_or("Missing primary synthesis module")?;
        if root.children.len() != 13 { return Err("Primary synthesis model topology mismatch".into()); }
        let condition = &root.children[12];
        if condition.data.len() != 12 || condition.children.len() != 3 { return Err("Conditioning topology mismatch".into()); }
        let prefix = u16::from_le_bytes(condition.data[0..2].try_into().unwrap()) as usize;
        let period = u16::from_le_bytes(condition.data[2..4].try_into().unwrap());
        let mode = word(condition.data, 4)?;
        let feature_width = word(condition.data, 8)? as usize;
        if prefix == 0 || period == 0 || mode > 1 { return Err("Invalid conditioning configuration".into()); }
        let frequencies = vector(&condition.children[0], word(condition.children[0].data,0)? as usize)?;
        let dct = condition.children[1].children.first().ok_or("Missing conditioning DCT")?;
        let conditioning_dct = DenseMatrix::from_node(dct)?;
        let conditioning_gru = FloatGru::from_node(&condition.children[2])?;
        let conditioning=ConditioningNetwork::from_parts(conditioning_gru,conditioning_dct,
            frequencies,prefix,period,mode,feature_width)?;
        let harmonic_frequencies = &root.children[4];
        harmonic_frequencies.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
        if !harmonic_frequencies.children.is_empty() {
            return Err("Harmonic frequencies must be a stored vector leaf".into());
        }
        let harmonic_frequencies = vector(harmonic_frequencies, 5)?.try_into()
            .map_err(|_| "Harmonic frequency vector width differs")?;
        Ok(Self { first: NcwnNetwork::from_node(&root.children[1])?, second: NcwnNetwork::from_node(&root.children[2])?,
            subgroup: NcwnNetwork::from_node(&root.children[10])?,
            cepstral: CepstralBands::from_node(&root.children[3])?,

            down8: AnalysisBank8::from_node(&root.children[9])?,
            up8: SynthesisBank8::from_node(&root.children[8])?,
            post: SynthesisPost::from_node(&root.children[11])?,
            projection: HarmonicProjection::from_node(&root.children[5])?,
            frequencies: harmonic_frequencies,
            conditioning })
    }
}

enum BandOperator { Identity, Silu, Relu, Conv(NeuralConv) }
fn cepstral_silu(row:&mut[f32]) {
    for x in row { *x *= SYNTHESIS_ACTIVATION_ARITHMETIC.sigmoid(*x); }
}
fn cepstral_relu(row:&mut[f32]) {
    for x in row { *x=x.max(0.); }
}
struct CepstralBand { range: crate::cepstral::BandRange, operators: Vec<BandOperator> }

pub struct CepstralBands {
    bands: Vec<CepstralBand>,
    pub input_width: usize,
    pub output_width: usize,
}

pub struct SynthesisPost {first:DenseMatrix,first_bias:Vec<f32>,last:DenseMatrix,last_bias:Vec<f32>}
impl SynthesisPost {
    pub fn from_node(node:&ModelNode<'_>)->Result<Self> {
        if !is_tag(node,"modm0") || node.children.len()!=4 || !is_tag(&node.children[0],"moda0")
            || !is_tag(&node.children[2],"moda0") {return Err("Synthesis post-network topology differs".into());}
        fn linear(node:&ModelNode<'_>)->Result<(DenseMatrix,Vec<f32>)> {
            if !is_tag(node,"modl0") || node.children.len()!=2 {return Err("Expected post-network linear".into());}
            let matrix=DenseMatrix::from_node(&node.children[0])?;let bias=vector(&node.children[1],matrix.output_width)?;
            Ok((matrix,bias))
        }
        let (first,first_bias)=linear(&node.children[1])?;let (last,last_bias)=linear(&node.children[3])?;
        if first.input_width!=64 || first.output_width!=64 || last.input_width!=64 || last.output_width!=8 {
            return Err("Synthesis post-network must map64 to64 to8".into());
        }
        Ok(Self {first,first_bias,last,last_bias})
    }

    pub fn quantized_workspace_requirements(&self)->Option<(usize,usize)> {
        [&self.first,&self.last].into_iter()
            .filter_map(DenseMatrix::quantized_workspace_requirements)
            .reduce(|(input,output),(next_input,next_output)|
                (input.max(next_input),output.max(next_output)))
    }
    pub fn forward(&self,input:&[Vec<f32>])->Result<Vec<Vec<f32>>> {
        let mut output=Vec::with_capacity(input.len());
        for row in input {
            let mut values=[0.;8];
            self.forward_row(row,&mut values)?;
            output.push(values.to_vec());
        }
        Ok(output)
    }

    pub fn forward_row(&self,input:&[f32],output:&mut[f32;8])->Result<()> {
        self.forward_row_using(input,output,None)
    }

    pub fn forward_row_using(&self,input:&[f32],output:&mut[f32;8],
        mut work:Option<&mut crate::quantized_matrix::QuantizedWorkspace>)->Result<()>
    {
        if input.len()!=64 {return Err("Synthesis post-network requires64values".into());}
        let activated:[f32;64]=std::array::from_fn(|i|input[i].max(0.));
        let mut hidden=[0.;64];
        self.first.overwrite_using(&activated,&mut hidden,work.as_deref_mut())?;
        for (value,bias) in hidden.iter_mut().zip(&self.first_bias) {*value+=bias;}
        for x in &mut hidden {*x=x.max(0.);}
        let mut values=[0.;8];
        self.last.overwrite_using(&hidden,&mut values,work.as_deref_mut())?;
        for (value,bias) in values.iter_mut().zip(&self.last_bias) {*value+=bias;}
        if values.iter().any(|x|!x.is_finite()) {return Err("Nonfinite post-network output".into());}
        output.copy_from_slice(&values);
        Ok(())
    }
}
impl CepstralBands {
    pub fn streaming_context(&self)->Result<(usize,usize)> {
        let mut result=(0usize,0usize);
        for band in &self.bands {
            let mut context=(0usize,0usize);
            for operation in &band.operators {
                if let BandOperator::Conv(conv)=operation {
                    let (left,right)=conv.streaming_context()?;
                    context.0=context.0.checked_add(left).ok_or("Cepstral context overflow")?;
                    context.1=context.1.checked_add(right).ok_or("Cepstral context overflow")?;
                }
            }
            result.0=result.0.max(context.0);result.1=result.1.max(context.1);
        }
        Ok(result)
    }
    pub fn from_node(node: &ModelNode<'_>) -> Result<Self> {
        if node.data.len()<8 || word(node.data,0)?!=2 || node.children.len()!=1 {
            return Err("Expected mode2 cepstral model".into());
        }
        let count=word(node.data,4)? as usize;
        let projections=&node.children[0].children;
        if count==0 || projections.len()!=count || node.data.len()!=8+16*count {
            return Err("Cepstral band records/projections differ".into());
        }
        let mut bands=Vec::with_capacity(count); let mut output_width=0; let mut input_width=0;
        for (i,projection) in projections.iter().enumerate() {
            if !is_tag(projection,"modm0") || !projection.data.is_empty() { return Err("Expected cepstral projection sequence".into()); }
            let base=8+16*i;
            let range=crate::cepstral::BandRange {start:word(node.data,base)? as usize,end:word(node.data,base+4)? as usize,
                left_fade:word(node.data,base+8)? as usize,right_fade:word(node.data,base+12)? as usize};
            let mut operators=Vec::new(); let mut width=None;
            for child in &projection.children {
                let operation=if is_tag(child,"moda5") { BandOperator::Identity }
                    else if is_tag(child,"moda7") { BandOperator::Silu }
                    else if is_tag(child,"moda0") { BandOperator::Relu }
                    else if is_tag(child,"modl1") {
                        let convolution=NeuralConv::from_node(child)?;
                        if let Some(previous)=width { if previous!=convolution.input_width {return Err("Cepstral layer widths differ".into());} }
                        else if input_width==0 {input_width=convolution.input_width;}
                        else if input_width!=convolution.input_width {return Err("Cepstral band input widths differ".into());}
                        width=Some(convolution.output_width); BandOperator::Conv(convolution)
                    } else {return Err(format!("Unsupported cepstral operator {:016x}",child.tag));};
                operators.push(operation);
            }
            let band_width=range.end.checked_sub(range.start).ok_or("Invalid cepstral range")?;
            if width!=band_width.checked_mul(2) {return Err("Cepstral projection must emit two contiguous band rows".into());}
            output_width=output_width.max(range.end); bands.push(CepstralBand {range,operators});
        }
        if bands.iter().any(|band|!band.range.admit(output_width)) {return Err("Invalid cepstral band fades".into());}
        Ok(Self {bands,input_width,output_width})
    }

    pub fn forward(&self,input:&[Vec<f32>])->Result<[Vec<Vec<f32>>;2]> {
        if input.iter().any(|row|row.len()!=self.input_width) {return Err("Cepstral input width differs".into());}
        let mut projected=Vec::with_capacity(self.bands.len());
        for band in &self.bands {
            let mut rows=input.to_vec();
            for operation in &band.operators {
                match operation {
                    BandOperator::Identity=>{},
                    BandOperator::Silu=>{for row in &mut rows {cepstral_silu(row);}},
                    BandOperator::Relu=>{for row in &mut rows {cepstral_relu(row);}},
                    BandOperator::Conv(conv)=>rows=conv.forward(&rows)?,
                }
            }
            projected.push(rows);
        }
        let count=projected.iter().map(Vec::len).min().unwrap_or(0);
        let mut outputs:[Vec<Vec<f32>>;2]=std::array::from_fn(|_|Vec::with_capacity(count));
        for row in 0..count {
            let mut combined:[Vec<f32>;2]=std::array::from_fn(|_|vec![0.;self.output_width]);
            for (band,rows) in self.bands.iter().zip(&projected) {
                let width=band.range.end-band.range.start;
                if rows[row].len()!=2*width {return Err("Cepstral projected width differs".into());}
                for which in 0..2 {
                    band.range.blend(&rows[row][which*width..(which+1)*width],&mut combined[which])
                        .map_err(|e|format!("Cepstral blend: {e:?}"))?;
                }
            }
            for (output,values) in outputs.iter_mut().zip(combined) {output.push(values);}
        }
        Ok(outputs)
    }
}

#[path="prototype_alternate_common.rs"]
mod alternate_common;
pub(crate) use alternate_common::AlternateCommon;
