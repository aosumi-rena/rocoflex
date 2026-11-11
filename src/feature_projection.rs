use crate::{
    Result,
    model::ModelTree,
    model_tags::decoded_hashes,
    timbre::{QuantizedLinear, runtime_silu},
};

pub struct FeatureProjection {
    linear: QuantizedLinear,
}
pub struct FeatureProjectionStages {
    pub activated256: Vec<f32>,
    pub features72: Vec<f32>,
}

impl FeatureProjection {
    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let seq = root
            .children
            .first()
            .and_then(|n| n.children.get(3))
            .ok_or("features72 branch missing")?;
        seq.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
        if !seq.data.is_empty() || seq.children.len() != 2 {
            return Err("features72 topology mismatch".into());
        }
        let act = &seq.children[0];
        act.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
        if !act.children.is_empty() || !act.data.is_empty() {
            return Err("features72 SiLU must be empty leaf".into());
        }
        let n = &seq.children[1];
        n.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
        if !n.data.is_empty() || n.children.len() != 2 {
            return Err("features72 Linear topology mismatch".into());
        }
        n.children[0].require(0x7fca40ff, b"prim4", decoded_hashes("prim4"))?;
        n.children[1].require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
        if n.children.iter().any(|p| !p.children.is_empty()) {
            return Err("features72 parameters must be leaves".into());
        }
        let linear = QuantizedLinear::from_payloads(n.children[0].data, n.children[1].data, 11)?;
        if (linear.in_dim, linear.out_dim) != (256, 72) {
            return Err("features72 dimensions mismatch".into());
        }
        Ok(Self { linear })
    }

    pub fn forward256(&self, src: &[f32]) -> Result<FeatureProjectionStages> {
        if src.is_empty() || src.len() % 256 != 0 || src.iter().any(|x| !x.is_finite()) {
            return Err("features72 input must be finite rows x256".into());
        }
        let mut activated256 = Vec::new();
        let mut features72 = Vec::new();
        for row in src.chunks_exact(256) {
            let act = row.iter().map(|&x| runtime_silu(x)).collect::<Vec<_>>();
            features72.extend(self.linear.forward_saturation(&act, true)?);
            activated256.extend(act);
        }
        Ok(FeatureProjectionStages {
            activated256,
            features72,
        })
    }
}
