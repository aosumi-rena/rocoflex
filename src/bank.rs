use crate::{
    Result,
    model::ModelNode as Node,
    model_tags::decoded_hashes,
    timbre::{QuantizedLinear, u32_at},
    quantized_matrix::QuantizedMatrix,
};

#[path = "convolution_stream.rs"]
pub(crate) mod streaming;
#[path = "bank_stream.rs"]
mod incremental;
pub use incremental::{BankBatch, BankStream, BankStreamFailure};

fn require(node: &Node<'_>, kind: u32, tag: &str) -> Result<()> {
    node.require(kind, tag.as_bytes(), decoded_hashes(tag))
}

pub(crate) fn vector(node: &Node<'_>, width: usize) -> Result<Vec<f32>> {
    require(node, 0x7fca40ff, "prim1")?;
    if !node.children.is_empty()
        || u32_at(node.data, 0)? as usize != width
        || width.checked_mul(4).and_then(|bytes| bytes.checked_add(4)) != Some(node.data.len())
    {
        return Err("vector layout mismatch".into());
    }
    let out = node.data[4..]
        .chunks_exact(4)
        .map(|x| f32::from_le_bytes(x.try_into().unwrap()))
        .collect::<Vec<_>>();
    if out.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite vector parameter".into());
    }
    Ok(out)
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct ConvPublicationLayout {
    pub input_width:usize,pub output_width:usize,pub taps:usize,
    pub stride:usize,pub padding:usize,pub dilation:usize,
}
pub(crate) struct Conv {
    taps: Vec<QuantizedMatrix>,
    bias: Vec<f32>,
    in_dim: usize,
    out_dim: usize,
    stride: usize,
    padding: usize,
    dilation: usize,
}

impl Conv {
    pub(crate) fn publication_layout(&self)->ConvPublicationLayout {
        ConvPublicationLayout {input_width:self.in_dim,output_width:self.out_dim,
            taps:self.taps.len(),stride:self.stride,padding:self.padding,dilation:self.dilation}
    }

    pub(crate) fn into_parameter_owners(self) -> (Vec<QuantizedMatrix>, Vec<f32>) {
        (self.taps, self.bias)
    }

    pub(crate) fn load(
        node: &Node<'_>,
        expected: [u32; 5],
        in_dim: usize,
        out_dim: usize,
    ) -> Result<Self> {
        require(node, 0x7fca41ff, "modl1")?;
        if node.data.len() != 20
            || (0..5).any(|i| u32_at(node.data, 4 * i).ok() != Some(expected[i]))
        {
            return Err("unqualified sourcebank conv config".into());
        }
        let k = expected[0] as usize;
        if node.children.len() != k + 1 {
            return Err("conv child count mismatch".into());
        }
        let mut taps = Vec::new();
        for m in &node.children[..k] {
            require(m, 0x7fca40ff, "prim4")?;
            if !m.children.is_empty() {
                return Err("conv matrix must be leaf".into());
            }
            let shift = u32_at(m.data, 4)?;
            let p = QuantizedMatrix::from_payload(m.data, shift)?;
            if (p.in_dim, p.out_dim) != (in_dim, out_dim) {
                return Err("conv tap dimensions mismatch".into());
            }
            taps.push(p);
        }
        let bias = vector(&node.children[k], out_dim)?;
        Ok(Self {
            taps,
            bias,
            in_dim,
            out_dim,
            stride: expected[1] as usize,
            padding: expected[2] as usize,
            dilation: expected[3] as usize,
        })
    }

    pub(crate) fn forward(&self, input: &[f32]) -> Result<Vec<f32>> {
        if input.len() % self.in_dim != 0 {
            return Err("conv input width mismatch".into());
        }
        let n = input.len() / self.in_dim;
        let padded = n
            .checked_add(2 * self.padding)
            .ok_or("conv length overflow")?;
        let effective = self.dilation * (self.taps.len() - 1) + 1;
        if padded < effective {
            return Err("nonpositive conv output length".into());
        }
        let out_rows = (padded - effective) / self.stride + 1;
        let mut output = vec![0.0_f32; out_rows * self.out_dim];
        let zero = vec![0.0_f32; self.in_dim];
        for (tap, p) in self.taps.iter().enumerate() {
            for row in 0..out_rows {
                let pos = row * self.stride + tap * self.dilation;
                let source = match pos.checked_sub(self.padding).filter(|&src|src<n) {
                    Some(src)=>&input[src*self.in_dim..(src+1)*self.in_dim],
                    None=>zero.as_slice(),
                };
                let target = &mut output[row*self.out_dim..(row+1)*self.out_dim];
                if tap==0 {
                    target.copy_from_slice(&p.forward_unbiased_saturation(source,true)?);
                } else {
                    p.forward_add_saturating_f32(source,target)?;
                }
            }
        }
        for row in output.chunks_exact_mut(self.out_dim) {
            for (v, &b) in row.iter_mut().zip(&self.bias) {
                *v += b;
            }
        }
        Ok(output)
    }
}

struct Block {
    conv: Conv,
    scale: Vec<f32>,
    bias: Vec<f32>,
}

pub struct BankProducer {
    blocks: Vec<Block>,
    head8: QuantizedLinear,
    head384: QuantizedLinear,
}
pub struct BankProducerStages {
    pub work384: Vec<f32>,
    pub bank8: Vec<f32>,
    pub bank384: Vec<f32>,
}

impl BankProducer {
    pub fn stream_with_limit(&self, maximum_rows: usize) -> Result<BankStream<'_>> {
        BankStream::new(self, maximum_rows)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = crate::model::ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported generator model version".into());
        }
        let root = tree.root;
        let producer = root
            .children
            .get(3)
            .and_then(|x| x.children.get(1))
            .ok_or("bank producer branch missing")?;
        if producer.children.len() != 4 {
            return Err("bank producer topology mismatch".into());
        }
        let seq = &producer.children[0];
        require(seq, 0x7fca41ff, "modm0")?;
        if !seq.data.is_empty() || seq.children.len() != 15 {
            return Err("expected five conv/scaling/relu blocks".into());
        }
        let mut blocks = Vec::new();
        for i in 0..5 {
            let cfg = if i == 0 {
                [3, 1, 1, 1, 1]
            } else {
                [4, 2, 2, 1, 1]
            };
            let conv = Conv::load(
                &seq.children[3 * i],
                cfg,
                if i == 0 { 60 } else { 384 },
                384,
            )?;
            let scale = &seq.children[3 * i + 1];
            require(scale, 0x7fca41ff, "modl5")?;
            if !scale.data.is_empty() || scale.children.len() != 2 {
                return Err("scaling topology mismatch".into());
            }
            let bias = vector(&scale.children[1], 384)?;
            let scale = vector(&scale.children[0], 384)?;
            let relu = &seq.children[3 * i + 2];
            require(relu, 0x7fca41ff, "moda0")?;
            if !relu.data.is_empty() || !relu.children.is_empty() {
                return Err("ReLU must be empty leaf".into());
            }
            blocks.push(Block { conv, scale, bias });
        }
        fn head(node: &Node<'_>, out: usize) -> Result<QuantizedLinear> {
            require(node, 0x7fca41ff, "modl0")?;
            if !node.data.is_empty() || node.children.len() != 2 {
                return Err("bank head topology mismatch".into());
            }
            let m = &node.children[0];
            require(m, 0x7fca40ff, "prim4")?;
            if !m.children.is_empty() {
                return Err("bank head matrix must be leaf".into());
            }
            vector(&node.children[1], out)?;
            let p =
                QuantizedLinear::from_payloads(m.data, node.children[1].data, u32_at(m.data, 4)?)?;
            if (p.in_dim, p.out_dim) != (384, out) {
                return Err("bank head dimensions mismatch".into());
            }
            Ok(p)
        }
        Ok(Self {
            blocks,
            head8: head(&producer.children[1], 8)?,
            head384: head(&producer.children[2], 384)?,
        })
    }

    pub fn forward_input60(&self, input: &[f32]) -> Result<BankProducerStages> {
        if input.is_empty() || input.len() % 60 != 0 || input.iter().any(|v| !v.is_finite()) {
            return Err("expected finite rows x60".into());
        }
        let mut work = input.to_vec();
        for block in &self.blocks {
            work = block.conv.forward(&work)?;
            for row in work.chunks_exact_mut(384) {
                for j in 0..384 {
                    let scaled = row[j] * block.scale[j];
                    row[j] = (scaled + block.bias[j]).max(0.0);
                }
            }
        }
        let mut bank8 = Vec::new();
        let mut bank384 = Vec::new();
        for row in work.chunks_exact(384) {
            bank8.extend(self.head8.forward_saturation(row, true)?);
            bank384.extend(self.head384.forward_saturation(row, true)?);
        }
        Ok(BankProducerStages {
            work384: work,
            bank8,
            bank384,
        })
    }

    pub fn forward_vec90(&self, rows72: &[f32]) -> Result<BankProducerStages> {
        if rows72.is_empty() || rows72.len() % 72 != 0 || rows72.iter().any(|v| !v.is_finite()) {
            return Err("expected finite rows x72".into());
        }
        let input = rows72
            .chunks_exact(72)
            .flat_map(|row| row[..60].iter().copied())
            .collect::<Vec<_>>();
        self.forward_input60(&input)
    }
}
