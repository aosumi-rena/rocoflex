use crate::{
    Result,
    model::{ModelNode as Node, ModelTree},
    model_tags::decoded_hashes,
    reduction::runtime_tanh,
    sparse_linear::SparseLinear,
    timbre::{runtime_sigmoid, runtime_silu, u32_at},
};

#[path = "pitch_temporal_stream.rs"]
mod incremental;
pub use incremental::{PitchTemporalBatch, PitchTemporalStartFailure, PitchTemporalStream,
    PitchTemporalStreamFailure};

fn sparse_workspace<'a>(matrices: impl Iterator<Item = &'a SparseLinear>)
    -> Result<crate::quantized_matrix::QuantizedWorkspace>
{
    let (input, output) = matrices.fold((0_usize, 0_usize), |(input, output), matrix|
        (input.max(matrix.in_dim), output.max(matrix.out_dim)));
    crate::quantized_matrix::QuantizedWorkspace::new(input, output)
}

struct SparseGru {
    p: Vec<SparseLinear>,
    input: usize,
}

impl SparseGru {
    fn from_node(n: &Node<'_>, input: usize) -> Result<Self> {
        n.require(0x7fca41ff, b"modl3", decoded_hashes("modl3"))?;
        if !n.data.is_empty() || n.children.len() != 12 {
            return Err("sparse GRU layout".into());
        }
        let mut p = Vec::new();
        for i in 0..6 {
            let q = SparseLinear::from_nodes(&n.children[2 * i], &n.children[2 * i + 1])?;
            if (q.in_dim, q.out_dim) != (if i < 3 { input } else { 1024 }, 1024) {
                return Err("sparse GRU dimension mismatch".into());
            }
            p.push(q);
        }
        Ok(Self { p, input })
    }

    fn forward(&self, x: &[f32], state: &mut [f32]) -> Result<Vec<f32>> {
        self.forward_with_workspace(x, state, &mut PitchTemporalWorkspace::default())
    }

    fn forward_with_workspace(&self, x: &[f32], state: &mut [f32],
        work: &mut PitchTemporalWorkspace) -> Result<Vec<f32>>
    {
        if x.len() % self.input != 0 || state.len() != 1024 || state.iter().any(|v| !v.is_finite())
        {
            return Err("sparse GRU input/state shape".into());
        }
        let mut out = Vec::new();
        for row in x.chunks_exact(self.input) {
            if work.sparse.is_none() {
                work.sparse = Some(sparse_workspace(self.p.iter())?);
            }
            let matrix_work = work.sparse.as_mut().ok_or("Missing sparse GRU workspace")?;
            work.reset.resize(1024, 0.0);
            let reset = &mut work.reset;
            for j in 0..1024 {
                reset[j] = self.p[0].bias[j] + self.p[3].bias[j];
            }
            self.p[0].forward_add_with_workspace(row, reset, matrix_work)?;
            self.p[3].forward_add_with_workspace(state, reset, matrix_work)?;
            for v in reset.iter_mut() {
                *v = runtime_sigmoid(*v);
            }
            work.update.resize(1024, 0.0);
            let update = &mut work.update;
            for j in 0..1024 {
                update[j] = self.p[1].bias[j] + self.p[4].bias[j];
            }
            self.p[1].forward_add_with_workspace(row, update, matrix_work)?;
            self.p[4].forward_add_with_workspace(state, update, matrix_work)?;
            for v in update.iter_mut() {
                *v = runtime_sigmoid(*v);
            }
            work.candidate.resize(1024, 0.0);
            let candidate = &mut work.candidate;
            candidate.copy_from_slice(&self.p[5].bias);
            self.p[5].forward_add_with_workspace(state, candidate, matrix_work)?;
            for j in 0..1024 {
                candidate[j] *= reset[j];
            }
            self.p[2].forward_add_with_workspace(row, candidate, matrix_work)?;
            for j in 0..1024 {
                candidate[j] += self.p[2].bias[j];
                candidate[j] = runtime_tanh(candidate[j]);
                let previous = state[j] * update[j];
                let inverse = 1.0_f32 - update[j];
                let next = candidate[j] * inverse;
                state[j] = previous + next;
            }
            out.extend_from_slice(state);
        }
        Ok(out)
    }
}

pub struct PitchTemporal {
    gru: Vec<SparseGru>,
    main: Vec<SparseLinear>,
    bypass: Vec<SparseLinear>,
}

#[derive(Default)]
pub struct PitchTemporalWorkspace {
    reset: Vec<f32>,
    update: Vec<f32>,
    candidate: Vec<f32>,

    sparse: Option<crate::quantized_matrix::QuantizedWorkspace>,
}

pub struct PitchTemporalState {
    pub first: Vec<f32>,
    pub second: Vec<f32>,
}
impl Default for PitchTemporalState {
    fn default() -> Self {
        Self {
            first: vec![0.0; 1024],
            second: vec![0.0; 1024],
        }
    }
}

pub struct PitchTemporalStages {
    pub first_gru: Vec<f32>,
    pub first_main: Vec<f32>,
    pub first_bypass: Vec<f32>,
    pub first_sum: Vec<f32>,
    pub second_gru: Vec<f32>,
    pub second_main: Vec<f32>,
    pub second_bypass: Vec<f32>,
    pub output384: Vec<f32>,
}

fn linear(n: &Node<'_>, input: usize, output: usize) -> Result<SparseLinear> {
    n.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
    if !n.data.is_empty() || n.children.len() != 2 {
        return Err("sparse Linear layout".into());
    }
    let q = SparseLinear::from_nodes(&n.children[0], &n.children[1])?;
    if (q.in_dim, q.out_dim) != (input, output) {
        return Err("sparse Linear shape".into());
    }
    Ok(q)
}

impl PitchTemporal {

    pub fn workspace(&self) -> Result<PitchTemporalWorkspace> {
        let mut work = PitchTemporalWorkspace {
            sparse: Some(sparse_workspace(self.gru.iter().flat_map(|gru| gru.p.iter()))?),
            ..PitchTemporalWorkspace::default()
        };
        for gate in [&mut work.reset, &mut work.update, &mut work.candidate] {
            gate.try_reserve_exact(1024).map_err(|error| error.to_string())?;
            gate.resize(1024, 0.0);
        }
        Ok(work)
    }

    pub fn stream_from_explicit_state(&self, state: PitchTemporalState, maximum_rows: usize)
        -> std::result::Result<PitchTemporalStream<'_>, PitchTemporalStartFailure>
    {
        PitchTemporalStream::new(self, state, maximum_rows)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        if u32_at(model, 0)? != 0x7fca00ff || u32_at(model, 4)? != 2 {
            return Err("pitch temporal requires v2".into());
        }
        let tree = ModelTree::parse(model)?;
        let root = tree.root;
        let pitch = root.children.get(1).ok_or("missing pitch")?;
        let seq = pitch.children.get(7).ok_or("missing pitch temporal")?;
        seq.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
        if !seq.data.is_empty() || seq.children.len() != 2 {
            return Err("pitch temporal sequence".into());
        }
        let mut gru = Vec::new();
        let mut main = Vec::new();
        let mut bypass = Vec::new();
        for (i, w) in seq.children.iter().enumerate() {
            let rawtag = [1355573478173270741_u64, 2302632375832338006][i];
            w.require(
                0x7fca41ff,
                b"",
                &[rawtag ^ 0xf22ee5adeedbda94 ^ 0xb2f45c29815fb08d],
            )?;
            let (input, output) = [(384, 1024), (1024, 384)][i];
            if w.data.len() != 8
                || u32_at(w.data, 0)? as usize != input
                || u32_at(w.data, 4)? as usize != output
                || w.children.len() != 2
            {
                return Err("pitch temporal residual layout".into());
            }
            let m = &w.children[0];
            m.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
            if !m.data.is_empty() || m.children.len() != 3 {
                return Err("pitch temporal main sequence".into());
            }
            let a = &m.children[2];
            a.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
            if !a.data.is_empty() || !a.children.is_empty() {
                return Err("pitch temporal activation layout".into());
            }
            gru.push(SparseGru::from_node(&m.children[0], input)?);
            main.push(linear(&m.children[1], 1024, output)?);
            bypass.push(linear(&w.children[1], input, output)?);
        }
        Ok(Self { gru, main, bypass })
    }

    pub fn forward384(
        &self,
        src: &[f32],
        state: &mut PitchTemporalState,
    ) -> Result<PitchTemporalStages> {
        self.forward384_with_workspace(src, state, &mut PitchTemporalWorkspace::default())
    }

    pub fn forward384_with_workspace(&self, src: &[f32], state: &mut PitchTemporalState,
        work: &mut PitchTemporalWorkspace) -> Result<PitchTemporalStages>
    {
        let first_gru = self.gru[0].forward_with_workspace(src, &mut state.first, work)?;
        let (first_main, first_bypass, first_sum) = self.block(0, src, &first_gru)?;
        let second_gru = self.gru[1].forward_with_workspace(&first_sum, &mut state.second, work)?;
        let (second_main, second_bypass, output384) = self.block(1, &first_sum, &second_gru)?;
        Ok(PitchTemporalStages {
            first_gru,
            first_main,
            first_bypass,
            first_sum,
            second_gru,
            second_main,
            second_bypass,
            output384,
        })
    }

    pub fn main_from_gru(&self, block: usize, src: &[f32]) -> Result<Vec<f32>> {
        if block > 1 || src.len() % 1024 != 0 {
            return Err("pitch main source shape".into());
        }
        let mut out = Vec::new();
        for row in src.chunks_exact(1024) {
            out.extend(
                self.main[block]
                    .forward(row)?
                    .iter()
                    .map(|&v| runtime_silu(v)),
            );
        }
        Ok(out)
    }

    fn block(
        &self,
        i: usize,
        src: &[f32],
        hidden: &[f32],
    ) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>)> {
        let main = self.main_from_gru(i, hidden)?;
        let mut bypass = Vec::new();
        for row in src.chunks_exact(self.bypass[i].in_dim) {
            bypass.extend(self.bypass[i].forward(row)?);
        }
        if main.len() != bypass.len() {
            return Err("pitch residual row mismatch".into());
        }
        let sum = bypass.iter().zip(&main).map(|(b, m)| b + m).collect();
        Ok((main, bypass, sum))
    }
}
