use crate::{Result, timbre::QuantizedLinear};

pub fn current_windows(bank_rows: usize) -> Vec<(usize, usize)> {
    if bank_rows < 3 {
        return Vec::new();
    }
    let offsets = [0_usize, 2, 3, 4];
    let mut out = Vec::new();
    let mut row = 0;
    loop {
        let first = row * 25 / 4;
        let full = (bank_rows - 3 + offsets[row % 4]) / 3;
        let available = (bank_rows - 2).saturating_sub(first);
        let count = full.min(available);
        if count < 7 {
            break;
        }
        out.push((first, count));
        row += 1;
    }
    out
}

pub struct SourcebankStages {
    pub weights: Vec<f32>,
    pub accum384: Vec<f32>,
    pub tail120: Vec<f32>,
}

pub struct SourcebankReducer {
    finalizer: QuantizedLinear,
}

impl SourcebankReducer {
    pub fn from_finalizer(finalizer: QuantizedLinear) -> Result<Self> {
        if (finalizer.in_dim, finalizer.out_dim) != (384, 120) {
            return Err("sourcebank finalizer must be384->120".into());
        }
        Ok(Self { finalizer })
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = crate::model::ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported generator model version".into());
        }
        let root = tree.root;
        let node = root
            .children
            .get(3)
            .and_then(|n| n.children.get(1))
            .and_then(|n| n.children.get(3))
            .ok_or("sourcebank finalizer missing")?;
        node.require(0x7fca41ff, b"modl0", &[0xfcdc8e10646bd921])?;
        if !node.data.is_empty() || node.children.len() != 2 {
            return Err("sourcebank linear topology mismatch".into());
        }
        let m = &node.children[0];
        let b = &node.children[1];
        m.require(0x7fca40ff, b"prim4", &[0x624730cb4720f1a3])?;
        b.require(0x7fca40ff, b"prim1", &[0x18d43ba270395308])?;
        if !m.children.is_empty() || !b.children.is_empty() {
            return Err("sourcebank parameters must be leaves".into());
        }
        Self::from_finalizer(QuantizedLinear::from_payloads(m.data, b.data, 13)?)
    }

    pub fn forward(&self, bank8: &[f32], bank384: &[f32]) -> Result<SourcebankStages> {
        if bank8.len() % 8 != 0
            || bank384.len() != bank8.len() / 8 * 384
            || bank8.iter().chain(bank384).any(|v| !v.is_finite())
        {
            return Err("expected finite same-count bank8/bank384 rows".into());
        }
        let rows = bank8.len() / 8;
        let mut weights = vec![0.0_f32; bank8.len()];

        for lane in 0..if rows == 0 { 0 } else { 8 } {
            let mut maximum = f32::NEG_INFINITY;
            for row in 0..rows {
                maximum = maximum.max(bank8[row * 8 + lane]);
            }
            let mut denom = 0.0_f32;
            for row in 0..rows {
                let shifted = bank8[row * 8 + lane] - maximum;
                denom += (f64::from(shifted).exp()) as f32;
            }
            let log_denom = f64::from(denom).ln() as f32;
            for row in 0..rows {
                let shifted = bank8[row * 8 + lane] - maximum;
                let log_weight = shifted - log_denom;
                weights[row * 8 + lane] = f64::from(log_weight).exp() as f32;
            }
        }
        let mut accum384 = vec![0.0_f32; 384];
        for row in 0..rows {
            for dim in 0..384 {
                let prod = bank384[row * 384 + dim] * weights[row * 8 + dim / 48];
                accum384[dim] += prod;
            }
        }
        let tail120 = self.finalizer.forward_saturation(&accum384, true)?;
        Ok(SourcebankStages {
            weights,
            accum384,
            tail120,
        })
    }
}

