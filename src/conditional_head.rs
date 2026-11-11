use crate::{
    Result,
    model::{ModelNode as Node, ModelTree},
    model_tags::decoded_hashes,
    reduction::FloatLinear,
    timbre::{f32_at, runtime_silu, u32_at},
};

pub struct ConditionalHead128 {
    min: f32,
    max: f32,
    out_min: f32,
    out_max: f32,
    layers: [FloatLinear; 3],
}

pub struct ConditionalHeadStages {
    pub normalized_p0: f32,
    pub packed130: Vec<f32>,
    pub matrix: [Vec<f32>; 3],
    pub biased: [Vec<f32>; 3],
    pub activated: [Vec<f32>; 2],
    pub output128: Vec<f32>,
}

pub fn map_tone_control(p0: f32) -> Result<f32> {
    if !p0.is_finite() {
        return Err("nonfinite tone control".into());
    }
    let scaled = p0 * 36.0_f32;
    Ok(scaled + 42.0_f32)
}

fn vector(n: &Node<'_>, size: usize) -> Result<Vec<f32>> {
    n.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
    if !n.children.is_empty() || n.data.len() != 4 + 4 * size || u32_at(n.data, 0)? as usize != size
    {
        return Err("conditional normalizer vector layout".into());
    }
    let v = (0..size)
        .map(|i| f32_at(n.data, 4 + 4 * i))
        .collect::<Result<Vec<_>>>()?;
    if v.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite conditional parameter".into());
    }
    Ok(v)
}

fn normalize_scalar(mapped: f32, min: f32, max: f32, out_min: f32, out_max: f32) -> Result<f32> {
    if !mapped.is_finite() {
        return Err("nonfinite mapped tone control".into());
    }
    let shifted = mapped - min;
    let span = max - min;
    let denominator = span + f32::from_bits(0x3727c5ac);
    let normalized = (shifted / denominator).clamp(-1.0_f32, 1.0_f32);
    let out_span = out_max - out_min;
    let scaled = out_span * normalized;
    Ok(scaled + out_min)
}

impl ConditionalHead128 {
    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("conditional head requires supported model version".into());
        }
        let root = tree.root;
        if root.kind != 0x7fca40ff
            || root.tag != 5416372951164482264_u64
            || root.data.len() != 18
            || u32_at(root.data, 0)? != 1
            || u32_at(root.data, 4)? != 48000
            || u32_at(root.data, 8)? != 1024
            || u16::from_le_bytes(root.data[12..14].try_into().unwrap()) != 240
            || u16::from_le_bytes(root.data[14..16].try_into().unwrap()) != 72
        {
            return Err("conditional feature root/config mismatch".into());
        }
        let count = u16::from_le_bytes(root.data[16..18].try_into().unwrap()) as usize;
        if count != 2 || root.children.len() != count + 7 {
            return Err("conditional root auxiliary count".into());
        }
        let head_index = 1 + 2 + count + 2;
        let wrapper = &root.children[head_index];
        if wrapper.kind != 0x7fca41ff
            || wrapper.tag != 0x9fb99b57d3f138f7
            || wrapper.data != [128, 0]
            || wrapper.children.len() != 2
        {
            return Err("conditional head wrapper/noise dimension mismatch".into());
        }
        let norm = &wrapper.children[0];
        norm.require(0x7fca40ff, b"cmpu0", decoded_hashes("cmpu0"))?;
        if !norm.data.is_empty() || norm.children.len() != 3 {
            return Err("conditional normalizer topology".into());
        }
        let min = vector(&norm.children[0], 1)?[0];
        let max = vector(&norm.children[1], 1)?[0];
        let range = vector(&norm.children[2], 2)?;
        if min >= max || range[0] >= range[1] {
            return Err("conditional normalizer range".into());
        }
        let seq = &wrapper.children[1];
        seq.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
        if !seq.data.is_empty() || seq.children.len() != 5 {
            return Err("conditional MLP sequence".into());
        }
        for i in [1, 3] {
            let a = &seq.children[i];
            a.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
            if !a.data.is_empty() || !a.children.is_empty() {
                return Err("conditional SiLU leaf".into());
            }
        }
        let mut layers = Vec::new();
        for (i, dims) in [(130, 384), (384, 384), (384, 128)].into_iter().enumerate() {
            let n = &seq.children[2 * i];
            n.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
            if !n.data.is_empty() || n.children.len() != 2 {
                return Err("conditional Linear topology".into());
            }
            n.children[0].require(0x7fca40ff, b"prim0", decoded_hashes("prim0"))?;
            n.children[1].require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if !n.children[0].children.is_empty() || !n.children[1].children.is_empty() {
                return Err("conditional parameter leaf".into());
            }
            let layer = FloatLinear::from_payloads(n.children[0].data, n.children[1].data)?;
            if (layer.in_dim, layer.out_dim) != dims {
                return Err("conditional Linear dimensions".into());
            }
            layers.push(layer);
        }
        Ok(Self {
            min,
            max,
            out_min: range[0],
            out_max: range[1],
            layers: layers.try_into().map_err(|_| "conditional layers")?,
        })
    }

    pub fn normalize_control(&self, mapped_p0: f32) -> Result<f32> {
        normalize_scalar(mapped_p0, self.min, self.max, self.out_min, self.out_max)
    }

    pub fn forward(
        &self,
        mapped_p0: f32,
        p1: f32,
        noise128: &[f32],
        gain: f32,
    ) -> Result<ConditionalHeadStages> {
        if noise128.len() != 128
            || !p1.is_finite()
            || !gain.is_finite()
            || noise128.iter().any(|v| !v.is_finite())
        {
            return Err("conditional controls/noise must be finite".into());
        }
        let normalized_p0 = self.normalize_control(mapped_p0)?;
        let mut packed130 = noise128.iter().map(|v| v * gain).collect::<Vec<_>>();
        packed130.push(normalized_p0);
        packed130.push(p1);
        let mut matrix = std::array::from_fn(|_| Vec::new());
        let mut biased = std::array::from_fn(|_| Vec::new());
        let mut activated: [Vec<f32>; 2] = std::array::from_fn(|_| Vec::new());
        for i in 0..3 {
            let src = if i == 0 {
                &packed130
            } else {
                &activated[i - 1]
            };
            let (m, b) = self.layers[i].forward_stages(src)?;
            if i < 2 {
                activated[i] = b.iter().map(|&v| runtime_silu(v)).collect();
            }
            matrix[i] = m;
            biased[i] = b;
        }
        let output128 = biased[2].clone();
        Ok(ConditionalHeadStages {
            normalized_p0,
            packed130,
            matrix,
            biased,
            activated,
            output128,
        })
    }
}

