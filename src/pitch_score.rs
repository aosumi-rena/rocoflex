use crate::{
    Result,
    model::{ModelNode as Node, ModelTree},
    model_tags::decoded_hashes,
    quantized_matrix::QuantizedMatrix,
    timbre::{QuantizedLinear, runtime_silu},
};

#[path = "pitch_base_stream.rs"]
mod incremental;
pub use incremental::{PitchBaseBatch, PitchBaseStream, PitchBaseStreamFailure};
pub(crate) use incremental::PitchBaseState;

pub struct PitchScore {
    first: QuantizedLinear,
    last: QuantizedLinear,
}
pub struct PitchScoreStages {
    pub hidden: Vec<f32>,
    pub activated: Vec<f32>,
    pub scores: Vec<f32>,
}

impl PitchScore {
    pub(crate) fn into_linear_owners(self) -> (QuantizedLinear, QuantizedLinear) {
        (self.first, self.last)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let parent = root
            .children
            .get(1)
            .and_then(|x| x.children.get(10))
            .ok_or("pitch scorer parent missing")?;
        parent.require(0x7fca41ff, b"", &[0x7fb30f7f39ad76e0])?;
        if parent.data != [2, 0, 0, 0, 16, 0, 0, 0] || parent.children.len() != 5 {
            return Err("pitch scorer parent layout mismatch".into());
        }
        Self::from_validated_parent(parent, 8, 10)
    }

    pub(crate) fn from_validated_parent(parent: &Node<'_>, first_shift: u32, last_shift: u32)
        -> Result<Self>
    {
        if parent.children.len() != 5 { return Err("pitch scorer parent layout mismatch".into()); }
        let seq = &parent.children[4];
        seq.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
        if !seq.data.is_empty() || seq.children.len() != 3 {
            return Err("pitch scorer topology mismatch".into());
        }
        let act = &seq.children[1];
        act.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
        if !act.data.is_empty() || !act.children.is_empty() {
            return Err("pitch SiLU must be empty leaf".into());
        }
        fn linear(n: &Node<'_>, dims: (usize, usize), shift: u32) -> Result<QuantizedLinear> {
            n.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
            if !n.data.is_empty() || n.children.len() != 2 {
                return Err("pitch linear topology mismatch".into());
            }
            n.children[0].require(0x7fca40ff, b"prim4", decoded_hashes("prim4"))?;
            n.children[1].require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if n.children.iter().any(|p| !p.children.is_empty()) {
                return Err("pitch parameters must be leaves".into());
            }
            let p = QuantizedLinear::from_payloads(n.children[0].data, n.children[1].data, shift)?;
            if (p.in_dim, p.out_dim) != dims {
                return Err("pitch linear dimension mismatch".into());
            }
            Ok(p)
        }
        Ok(Self {
            first: linear(&seq.children[0], (272, 256), first_shift)?,
            last: linear(&seq.children[2], (256, 1), last_shift)?,
        })
    }

    pub fn forward(&self, src: &[f32]) -> Result<PitchScoreStages> {
        if src.is_empty() || src.len() % 272 != 0 {
            return Err("pitch input must be rows x272".into());
        }
        let mut hidden = Vec::new();
        let mut activated = Vec::new();
        let mut scores = Vec::new();
        for row in src.chunks_exact(272) {
            let h = self.first.forward_saturation(row, true)?;
            let a = h.iter().map(|&x| runtime_silu(x)).collect::<Vec<_>>();
            scores.extend(self.last.forward_saturation(&a, true)?);
            hidden.extend(h);
            activated.extend(a);
        }
        Ok(PitchScoreStages {
            hidden,
            activated,
            scores,
        })
    }
}

pub fn apply_pitch_scores(rows72: &[f32], scores: &[f32]) -> Result<Vec<f32>> {
    if rows72.len() % 72 != 0
        || rows72.len() / 72 != scores.len()
        || rows72.iter().chain(scores).any(|v| !v.is_finite())
    {
        return Err("lane71 row/score dimensions or finiteness mismatch".into());
    }
    let mut out = rows72.to_vec();
    for (row, &score) in out.chunks_exact_mut(72).zip(scores) {

        let hz = f64::from(score).exp() as f32;
        let ratio = hz / 440.0_f32;
        let logged = f64::from(ratio).ln() as f32;
        let scaled = logged * f32::from_bits(0x418a7fac);
        row[71] = scaled + 69.0_f32;
    }
    Ok(out)
}

pub struct PitchBase {
    first: crate::bank::Conv,
    second: crate::bank::Conv,
}

pub struct PitchProfile {
    matrix: QuantizedMatrix,
}
pub struct PitchProfileStages {
    pub profile128: Vec<f32>,
    pub adjusted_profile128: Vec<f32>,
    pub positions: Vec<usize>,
    pub source272: Vec<f32>,
}

impl PitchProfile {
    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let parent = root
            .children
            .get(1)
            .and_then(|x| x.children.get(10))
            .ok_or("pitch profile parent missing")?;
        parent.require(0x7fca41ff, b"", &[0x7fb30f7f39ad76e0])?;
        if parent.data != [2, 0, 0, 0, 16, 0, 0, 0] || parent.children.len() != 5 {
            return Err("pitch profile parent mismatch".into());
        }
        Self::from_validated_parent(parent, 8)
    }

    pub(crate) fn from_validated_parent(parent: &Node<'_>, shift: u32) -> Result<Self> {
        if parent.children.len() != 5 { return Err("pitch profile parent mismatch".into()); }
        let n = &parent.children[1];
        n.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
        if !n.data.is_empty() || n.children.len() != 1 {
            return Err("pitch profile must be bias-free Linear".into());
        }
        let matrix = &n.children[0];
        matrix.require(0x7fca40ff, b"prim4", decoded_hashes("prim4"))?;
        if !matrix.children.is_empty() {
            return Err("pitch profile matrix must be leaf".into());
        }
        let matrix = QuantizedMatrix::from_payload(matrix.data, shift)?;
        if (matrix.input_width(), matrix.output_width()) != (256, 128) {
            return Err("pitch profile dimension mismatch".into());
        }
        Ok(Self { matrix })
    }

    pub fn forward256(&self, base: &[f32]) -> Result<PitchProfileStages> {
        self.forward256_stateful(base, &mut -1)
    }

    pub fn forward256_stateful(
        &self,
        base: &[f32],
        previous: &mut i32,
    ) -> Result<PitchProfileStages> {
        if *previous < -1 || *previous >= 128 {
            return Err("invalid previous pitch position".into());
        }
        if base.is_empty() || base.len() % 256 != 0 {
            return Err("pitch profile requires rows x256".into());
        }
        let mut profile128 = Vec::new();
        let mut adjusted_profile128 = Vec::new();
        let mut positions = Vec::new();
        let mut source272 = Vec::new();
        let denom = (0..8)
            .map(|i| 10000.0_f32.powf(i as f32 / 8.0_f32))
            .collect::<Vec<_>>();
        for row in base.chunks_exact(256) {
            let mut profile = self.matrix.forward_unbiased_saturation(row, true)?;
            profile128.extend(&profile);
            if *previous >= 0 {
                for (i, x) in profile.iter_mut().enumerate() {
                    let distance = (i as i32 - *previous).abs();
                    let excess = (distance - 5).max(0);
                    let penalty = f64::from(excess) * -0.2_f64;
                    *x = (f64::from(*x) + penalty) as f32;
                }
            }
            let mut pos = 0;
            for i in 1..128 {
                if profile[i] > profile[pos] {
                    pos = i;
                }
            }
            positions.push(pos);
            *previous = pos as i32;
            adjusted_profile128.extend(profile);
            source272.extend(row);
            let angles = denom.iter().map(|&d| pos as f32 / d).collect::<Vec<_>>();
            source272.extend(angles.iter().map(|a| a.sin()));
            source272.extend(angles.iter().map(|a| a.cos()));
        }
        Ok(PitchProfileStages {
            profile128,
            adjusted_profile128,
            positions,
            source272,
        })
    }
}
pub struct PitchBaseStages {
    pub conv0: Vec<f32>,
    pub activated0: Vec<f32>,
    pub conv1: Vec<f32>,
    pub base256: Vec<f32>,
}

impl PitchBase {
    pub(crate) fn publication_right_rows_from_layout(
        first:crate::bank::ConvPublicationLayout,second:crate::bank::ConvPublicationLayout)->Option<usize>
    {
        fn right(p:crate::bank::ConvPublicationLayout,input:usize,output:usize)->Option<usize> {

            if (p.input_width,p.output_width,p.taps,p.stride,p.padding,p.dilation)
                !=(input,output,3,1,1,1) {return None;}
            p.taps.checked_sub(1)?.checked_mul(p.dilation)?.checked_sub(p.padding)
        }
        right(first,384,256)?.checked_add(right(second,256,256)?)
    }
    pub(crate) fn prepared_right_rows(&self)->Option<usize> {
        Self::publication_right_rows_from_layout(self.first.publication_layout(),self.second.publication_layout())
    }
    pub fn stream_with_limit(&self, maximum_rows: usize) -> Result<PitchBaseStream<'_>> {
        PitchBaseStream::new(self, maximum_rows)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let parent = root
            .children
            .get(1)
            .and_then(|x| x.children.get(10))
            .ok_or("pitch base parent missing")?;
        parent.require(0x7fca41ff, b"", &[0x7fb30f7f39ad76e0])?;
        if parent.data != [2, 0, 0, 0, 16, 0, 0, 0] || parent.children.len() != 5 {
            return Err("pitch base parent mismatch".into());
        }
        let seq = &parent.children[0];
        seq.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
        if !seq.data.is_empty() || seq.children.len() != 6 {
            return Err("pitch base topology mismatch".into());
        }
        for i in [2, 4] {
            let n = &seq.children[i];
            n.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
            if !n.children.is_empty() || !n.data.is_empty() {
                return Err("pitch base SiLU must be empty leaf".into());
            }
        }
        for i in [0, 5] {
            let n = &seq.children[i];
            n.require(0x7fca41ff, b"moda5", decoded_hashes("moda5"))?;
            if !n.children.is_empty() || !n.data.is_empty() {
                return Err("pitch identity must be empty leaf".into());
            }
        }
        Ok(Self {
            first: crate::bank::Conv::load(&seq.children[1], [3, 1, 1, 1, 1], 384, 256)?,
            second: crate::bank::Conv::load(&seq.children[3], [3, 1, 1, 1, 1], 256, 256)?,
        })
    }

    pub fn forward384(&self, src: &[f32]) -> Result<PitchBaseStages> {
        if src.is_empty() || src.len() % 384 != 0 || src.iter().any(|x| !x.is_finite()) {
            return Err("pitch base requires finite rows x384".into());
        }
        let conv0 = self.first.forward(src)?;
        let activated0 = conv0.iter().map(|&x| runtime_silu(x)).collect::<Vec<_>>();
        let conv1 = self.second.forward(&activated0)?;
        let base256 = conv1.iter().map(|&x| runtime_silu(x)).collect::<Vec<_>>();
        Ok(PitchBaseStages {
            conv0,
            activated0,
            conv1,
            base256,
        })
    }
}
