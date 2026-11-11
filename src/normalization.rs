use crate::{
    Result,
    model::{ModelNode as Node, ModelTree},
    model_tags::decoded_hashes,
    timbre::{f32_at, u32_at},
};

#[derive(Clone)]
pub struct MaxMinNormalizer {
    lo: Vec<f32>,
    hi: Vec<f32>,
    out_min: f32,
    out_max: f32,
}

impl MaxMinNormalizer {
    pub(crate) fn from_node(node: &Node<'_>, width: usize) -> Result<Self> {
        node.require(0x7fca40ff, b"cmpu0", decoded_hashes("cmpu0"))?;
        if !node.data.is_empty() || node.children.len() != 3 {
            return Err("normalizer topology mismatch".into());
        }
        fn vector(node: &Node<'_>, width: usize) -> Result<Vec<f32>> {
            node.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if !node.children.is_empty()
                || u32_at(node.data, 0)? as usize != width
                || node.data.len() != 4 + 4 * width
            {
                return Err("normalizer vector layout mismatch".into());
            }
            let v = (0..width)
                .map(|i| f32_at(node.data, 4 + 4 * i))
                .collect::<Result<Vec<_>>>()?;
            if v.iter().any(|x| !x.is_finite()) {
                return Err("nonfinite normalizer parameter".into());
            }
            Ok(v)
        }
        let lo = vector(&node.children[0], width)?;
        let hi = vector(&node.children[1], width)?;
        let range = vector(&node.children[2], 2)?;
        if range[0] >= range[1] || lo.iter().zip(&hi).any(|(a, b)| a >= b) {
            return Err("invalid normalizer range".into());
        }
        Ok(Self {
            lo,
            hi,
            out_min: range[0],
            out_max: range[1],
        })
    }

    fn check(&self, src: &[f32]) -> Result<()> {
        if src.len() % self.lo.len() != 0 || src.iter().any(|x| !x.is_finite()) {
            return Err("normalizer input must be finite whole rows".into());
        }
        Ok(())
    }

    pub fn inverse(&self, src: &[f32]) -> Result<Vec<f32>> {
        self.check(src)?;
        let width = self.lo.len();
        let denom = self.out_max - self.out_min;
        Ok(src
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let j = i % width;
                let t = x - self.out_min;
                let span = self.hi[j] - self.lo[j];
                let prod = span * t;
                let scaled = prod / denom;
                scaled + self.lo[j]
            })
            .collect())
    }

    pub fn forward(&self, src: &[f32], clamp: bool) -> Result<Vec<f32>> {
        self.check(src)?;
        let width = self.lo.len();
        let out_span = self.out_max - self.out_min;
        Ok(src
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let j = i % width;
                let shifted = x - self.lo[j];
                let span = self.hi[j] - self.lo[j];
                let denom = span + f32::from_bits(0x3727c5ac);
                let norm = shifted / denom;
                let norm = if clamp { norm.clamp(-1.0, 1.0) } else { norm };
                let scaled = out_span * norm;
                scaled + self.out_min
            })
            .collect())
    }
}

#[derive(Clone)]
pub struct NormalizationBridge {
    tail: MaxMinNormalizer,
    vec90: MaxMinNormalizer,
}
pub struct NormalizationStages {
    pub post1256: Vec<f32>,
    pub vec3b0_before_lane71: Vec<f32>,
}

impl NormalizationBridge {
    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let tail = root
            .children
            .first()
            .and_then(|x| x.children.get(4))
            .ok_or("tail normalizer missing")?;
        let vec90 = root.children.get(2).ok_or("vec90 normalizer missing")?;
        Ok(Self {
            tail: MaxMinNormalizer::from_node(tail, 72)?,
            vec90: MaxMinNormalizer::from_node(vec90, 72)?,
        })
    }

    pub fn vec90_from_vec3b0(&self, src: &[f32], clamp: bool) -> Result<Vec<f32>> {
        self.vec90.forward(src, clamp)
    }

    pub fn inverse_source188(&self, src: &[f32]) -> Result<Vec<f32>> {
        self.tail.inverse(src)
    }

    pub fn prepare_source188(&self, src: &[f32]) -> Result<NormalizationStages> {
        if src.len() < 72 {
            return Err("source188 requires at least one row".into());
        }
        let post1256 = self.tail.inverse(src)?;
        let vec3b0_before_lane71 = post1256[..post1256.len() - 72].to_vec();
        Ok(NormalizationStages {
            post1256,
            vec3b0_before_lane71,
        })
    }
}
