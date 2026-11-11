use crate::{
    Result,
    bank::Conv,
    model::{ModelNode as Node, ModelTree},
    model_tags::decoded_hashes,
    reduction::runtime_tanh,
    timbre::{runtime_sigmoid, u32_at},
};

#[path = "temporal_mixer_stream.rs"]
mod streaming;
pub use streaming::{TemporalMixerBatch, TemporalMixerStream, TemporalMixerStreamFailure};

struct TemporalBlock {
    gate: Conv,
    residual: Conv,
    skip: Conv,
}

pub struct TemporalMixer {
    blocks: Vec<TemporalBlock>,
}

pub struct TemporalMixerStages {
    pub residuals256: Vec<Vec<f32>>,
    pub inner512: Vec<Vec<f32>>,
    pub gated256: Vec<Vec<f32>>,
    pub skips256: Vec<Vec<f32>>,
    pub sources256: Vec<Vec<f32>>,
    pub left256: Vec<f32>,
}

impl TemporalMixer {

    pub fn stream_with_limit(&self, maximum_rows: usize) -> Result<TemporalMixerStream<'_>> {
        TemporalMixerStream::new(self, maximum_rows)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let n = root
            .children
            .first()
            .and_then(|n| n.children.get(13))
            .ok_or("temporal mixer missing")?;
        fn config(n: &Node<'_>, tag: &str, values: &[u32], children: usize) -> Result<()> {
            n.require(0x7fca41ff, tag.as_bytes(), decoded_hashes(tag))?;
            if n.data.len() != values.len() * 4 || n.children.len() != children {
                return Err("temporal topology mismatch".into());
            }
            for (i, &v) in values.iter().enumerate() {
                if u32_at(n.data, 4 * i)? != v {
                    return Err("temporal configuration mismatch".into());
                }
            }
            Ok(())
        }
        config(n, "_ncwnv0", &[384, 256, 0], 3)?;
        for list in &n.children {
            config(list, "cmpg1", &[], 4)?;
        }
        let mut blocks = Vec::new();
        for (i, dilation) in [1, 3, 1, 3].into_iter().enumerate() {
            let in_dim = if i == 0 { 384 } else { 256 };
            let gated = &n.children[0].children[i];
            config(gated, "_gnc1v0", &[in_dim as u32, 256, 0], 1)?;
            blocks.push(TemporalBlock {
                gate: Conv::load(
                    &gated.children[0],
                    [3, 1, dilation, dilation, 1],
                    in_dim,
                    512,
                )?,
                residual: Conv::load(&n.children[2].children[i], [1, 1, 0, 1, 1], in_dim, 256)?,
                skip: Conv::load(&n.children[1].children[i], [1, 1, 0, 1, 1], 256, 256)?,
            });
        }
        Ok(Self { blocks })
    }

    pub fn forward384(&self, src: &[f32]) -> Result<TemporalMixerStages> {
        if src.is_empty() || src.len() % 384 != 0 || src.iter().any(|v| !v.is_finite()) {
            return Err("temporal mixer requires finite rows x384".into());
        }
        let mut source = src.to_vec();
        let mut s = TemporalMixerStages {
            residuals256: Vec::new(),
            inner512: Vec::new(),
            gated256: Vec::new(),
            skips256: Vec::new(),
            sources256: Vec::new(),
            left256: vec![0.0; src.len() / 384 * 256],
        };
        for (i, block) in self.blocks.iter().enumerate() {
            let residual = block.residual.forward(&source)?;
            let inner = block.gate.forward(&source)?;
            let mut gated = Vec::with_capacity(residual.len());
            for row in inner.chunks_exact(512) {
                for j in 0..256 {
                    gated.push(runtime_tanh(row[j]) * runtime_sigmoid(row[256 + j]));
                }
            }
            let skip = block.skip.forward(&gated)?;
            for (a, &v) in s.left256.iter_mut().zip(&skip) {
                *a += v;
            }
            if i < 3 {
                source = gated.iter().zip(&residual).map(|(a, b)| a + b).collect();
                s.sources256.push(source.clone());
            }
            s.residuals256.push(residual);
            s.inner512.push(inner);
            s.gated256.push(gated);
            s.skips256.push(skip);
        }
        Ok(s)
    }
}
