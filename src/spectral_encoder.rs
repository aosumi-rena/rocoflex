use crate::{
    Result,
    model::{ModelNode as Node, ModelTree},
    model_tags::decoded_hashes,
    timbre::{QuantizedLinear, runtime_silu, u32_at},
};

pub struct SpectralEncoder {
    branches: Vec<[QuantizedLinear; 2]>,
    right: QuantizedLinear,
}
pub struct SpectralEncoderStages {
    pub items256: Vec<Vec<f32>>,
    pub spectral256: Vec<f32>,
    pub right256: Vec<f32>,
}

impl SpectralEncoder {
    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let owner = root
            .children
            .first()
            .ok_or("spectral encoder owner missing")?;
        if owner.children.len() != 14 {
            return Err("spectral encoder owner topology mismatch".into());
        }
        fn linear(n: &Node<'_>, dims: (usize, usize)) -> Result<QuantizedLinear> {
            n.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
            if !n.data.is_empty() || n.children.len() != 2 {
                return Err("spectral Linear topology mismatch".into());
            }
            let m = &n.children[0];
            let b = &n.children[1];
            m.require(0x7fca40ff, b"prim4", decoded_hashes("prim4"))?;
            b.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if !m.children.is_empty() || !b.children.is_empty() {
                return Err("spectral parameters must be leaves".into());
            }
            let p = QuantizedLinear::from_payloads(m.data, b.data, u32_at(m.data, 4)?)?;
            if (p.in_dim, p.out_dim) != dims {
                return Err("spectral matrix dimensions mismatch".into());
            }
            Ok(p)
        }
        let mut branches = Vec::new();
        for i in 5..10 {
            let seq = &owner.children[i];
            seq.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
            if !seq.data.is_empty() || seq.children.len() != 4 {
                return Err("spectral MLP topology mismatch".into());
            }
            for j in [1, 3] {
                let n = &seq.children[j];
                n.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
                if !n.data.is_empty() || !n.children.is_empty() {
                    return Err("spectral SiLU must be empty leaf".into());
                }
            }
            branches.push([
                linear(&seq.children[0], (128, 256))?,
                linear(&seq.children[2], (256, 256))?,
            ]);
        }
        Ok(Self {
            branches,
            right: linear(&owner.children[1], (256, 256))?,
        })
    }

    pub fn forward_right256(&self, src: &[f32]) -> Result<Vec<f32>> {
        if src.is_empty() || src.len() % 256 != 0 || src.iter().any(|x| !x.is_finite()) {
            return Err("right encoder requires finite rows x256".into());
        }
        let mut out = Vec::with_capacity(src.len());
        for row in src.chunks_exact(256) {
            out.extend(self.right.forward_saturation(row, true)?);
        }
        Ok(out)
    }

    pub fn forward513(&self, src: &[f32]) -> Result<SpectralEncoderStages> {
        if src.is_empty() || src.len() % 513 != 0 || src.iter().any(|x| !x.is_finite()) {
            return Err("spectral encoder requires finite rows x513".into());
        }
        let mut items256 = (0..5).map(|_| Vec::new()).collect::<Vec<_>>();
        let mut spectral256 = Vec::new();
        let mut right256 = Vec::new();
        for row in src.chunks_exact(513) {
            let mut acc = vec![0.0_f32; 256];
            for (i, branch) in self.branches.iter().enumerate() {
                let first = branch[0].forward_saturation(&row[i * 96..i * 96 + 128], true)?;
                let act = first.iter().map(|&x| runtime_silu(x)).collect::<Vec<_>>();
                let last = branch[1].forward_saturation(&act, true)?;
                let item = last.iter().map(|&x| runtime_silu(x)).collect::<Vec<_>>();
                for j in 0..256 {
                    acc[j] += item[j];
                }
                items256[i].extend(item);
            }
            right256.extend(self.right.forward_saturation(&acc, true)?);
            spectral256.extend(acc);
        }
        Ok(SpectralEncoderStages {
            items256,
            spectral256,
            right256,
        })
    }
}
