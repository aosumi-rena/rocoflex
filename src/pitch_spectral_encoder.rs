use crate::{
    Result,
    model::{ModelNode as Node, ModelTree},
    model_tags::decoded_hashes,
    timbre::{QuantizedLinear, runtime_silu, u32_at},
};

#[path = "pitch_spectral_stream.rs"]
mod incremental;
pub use incremental::{PitchEncoderBatch, PitchEncoderStream, PitchEncoderStreamFailure};
pub(crate) use incremental::PitchEncoderState;

pub struct PitchSpectralEncoder {
    branches: Vec<[QuantizedLinear; 2]>,
    slices: Vec<(usize, usize, usize)>,
}

pub struct PitchSpectralStages {
    pub first256: Vec<Vec<f32>>,
    pub activated256: Vec<Vec<f32>>,
    pub items384: Vec<Vec<f32>>,
    pub summed384: Vec<f32>,
}

impl PitchSpectralEncoder {
    pub fn stream_with_limit(&self, maximum_rows_per_spectrum: usize)
        -> Result<PitchEncoderStream<'_>>
    {
        PitchEncoderStream::new(self, maximum_rows_per_spectrum)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("pitch spectra require v2".into());
        }
        let root = tree.root;
        let owner = root.children.get(1).ok_or("pitch frontend missing")?;
        owner.require(
            0x7fca41ff,
            b"",
            &[560151423416826003 ^ 0xf22ee5adeedbda94 ^ 0xb2f45c29815fb08d],
        )?;
        let config = [
            981668463_u32,
            5,
            107,
            3,
            4,
            3,
            3,
            0,
            0,
            256,
            1,
            0,
            128,
            1,
            96,
            128,
            2,
            0,
            128,
        ];
        if owner.children.len() != 14 || owner.data.len() != config.len() * 4 {
            return Err("pitch frontend layout".into());
        }
        for (i, v) in config.into_iter().enumerate() {
            if u32_at(owner.data, i * 4)? != v {
                return Err("pitch frontend config".into());
            }
        }
        fn linear(n: &Node<'_>, input: usize, output: usize) -> Result<QuantizedLinear> {
            n.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
            if !n.data.is_empty() || n.children.len() != 2 {
                return Err("pitch MLP Linear layout".into());
            }
            let m = &n.children[0];
            let b = &n.children[1];
            m.require(0x7fca40ff, b"prim4", decoded_hashes("prim4"))?;
            b.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if !m.children.is_empty() || !b.children.is_empty() {
                return Err("pitch MLP parameter leaf".into());
            }
            let q = QuantizedLinear::from_payloads(m.data, b.data, u32_at(m.data, 4)?)?;
            if (q.in_dim, q.out_dim) != (input, output) {
                return Err("pitch MLP shape".into());
            }
            Ok(q)
        }
        let mut branches = Vec::new();
        let mut slices = Vec::new();
        for i in 0..4 {
            let off = 28 + i * 12;
            let stream = u32_at(owner.data, off)? as usize;
            let start = u32_at(owner.data, off + 4)? as usize;
            let width = u32_at(owner.data, off + 8)? as usize;
            let seq = &owner.children[3 + i];
            seq.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
            if !seq.data.is_empty() || seq.children.len() != 3 {
                return Err("pitch MLP sequence".into());
            }
            let a = &seq.children[1];
            a.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
            if !a.data.is_empty() || !a.children.is_empty() {
                return Err("pitch MLP activation".into());
            }
            branches.push([
                linear(&seq.children[0], width, 256)?,
                linear(&seq.children[2], 256, 384)?,
            ]);
            slices.push((stream, start, width));
        }
        Ok(Self { branches, slices })
    }

    pub fn forward(&self, spectra: [&[f32]; 3]) -> Result<PitchSpectralStages> {
        let widths = [257, 513, 1025];
        if spectra
            .iter()
            .zip(widths)
            .any(|(v, w)| v.is_empty() || v.len() % w != 0 || v.iter().any(|x| !x.is_finite()))
        {
            return Err("pitch spectrum row count or finiteness".into());
        }

        let rows = spectra
            .iter()
            .zip(widths)
            .map(|(v, w)| v.len() / w)
            .min()
            .unwrap();
        let mut items384 = (0..4).map(|_| Vec::new()).collect::<Vec<_>>();
        let mut first256 = (0..4).map(|_| Vec::new()).collect::<Vec<_>>();
        let mut activated256 = (0..4).map(|_| Vec::new()).collect::<Vec<_>>();
        let mut summed384 = Vec::new();
        for row in 0..rows {
            let mut sum = vec![0.0_f32; 384];
            for (i, branch) in self.branches.iter().enumerate() {
                let (stream, start, width) = self.slices[i];
                let begin = row * widths[stream] + start;

                let first =
                    branch[0].forward_saturation(&spectra[stream][begin..begin + width], true)?;
                let act = first.iter().map(|&v| runtime_silu(v)).collect::<Vec<_>>();
                first256[i].extend_from_slice(&first);
                activated256[i].extend_from_slice(&act);
                let last = branch[1].forward_saturation(&act, true)?;
                if i == 0 {

                    sum.copy_from_slice(&last);
                } else {
                    for j in 0..384 {
                        sum[j] += last[j];
                    }
                }
                items384[i].extend(last);
            }
            summed384.extend(sum);
        }
        Ok(PitchSpectralStages {
            first256,
            activated256,
            items384,
            summed384,
        })
    }
}
