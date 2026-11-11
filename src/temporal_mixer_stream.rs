use super::TemporalMixer;
use crate::{bank::streaming::ConvolutionCursor, reduction::runtime_tanh,
    timbre::runtime_sigmoid};
use std::collections::VecDeque;

pub struct TemporalMixerStream<'a> {
    mixer: &'a TemporalMixer,
    gates: Vec<ConvolutionCursor>,
    residual: Vec<VecDeque<f32>>,
    skips: Vec<VecDeque<f32>>,
    row_limit: usize,
    accepted: usize,
    emitted: usize,
    ended: bool,
    halted: bool,
}

pub struct TemporalMixerBatch<'a> {
    pub first_row: usize,
    pub left256: Vec<f32>,
    pub final_input: bool,
    pub state: TemporalMixerStream<'a>,
}

pub struct TemporalMixerStreamFailure<'a> {
    pub reason: String,
    pub block: Option<usize>,
    pub input_rows: Vec<f32>,
    pub inner512: Vec<f32>,
    pub gated256: Vec<f32>,
    pub skip256: Vec<f32>,
    pub advanced: TemporalMixerStream<'a>,
}

struct Work {
    block: Option<usize>, input: Vec<f32>, inner: Vec<f32>,
    gated: Vec<f32>, skip: Vec<f32>,
}

impl Work {
    fn empty() -> Self {
        Self { block: None, input: Vec::new(), inner: Vec::new(),
            gated: Vec::new(), skip: Vec::new() }
    }
}

impl<'a> TemporalMixerStream<'a> {
    pub(super) fn new(mixer: &'a TemporalMixer, row_limit: usize) -> crate::Result<Self> {
        if mixer.blocks.len() != 4 || row_limit == 0
            || row_limit > i32::MAX as usize
            || row_limit.checked_mul(512).is_none()
        {
            return Err("Temporal stream requires four blocks and a positive i32 row limit".into());
        }
        let gates = mixer.blocks.iter()
            .map(|b| ConvolutionCursor::new(&b.gate, row_limit))
            .collect::<crate::Result<Vec<_>>>()?;
        Ok(Self {
            mixer, gates, residual: (0..4).map(|_| VecDeque::new()).collect(),
            skips: (0..4).map(|_| VecDeque::new()).collect(),
            row_limit, accepted: 0, emitted: 0, ended: false, halted: false,
        })
    }

    pub fn accepted_rows(&self) -> usize { self.accepted }
    pub fn emitted_rows(&self) -> usize { self.emitted }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    fn failure(self, reason: impl Into<String>, work: Work) -> TemporalMixerStreamFailure<'a> {
        TemporalMixerStreamFailure {
            reason: reason.into(), block: work.block, input_rows: work.input,
            inner512: work.inner, gated256: work.gated, skip256: work.skip,
            advanced: self,
        }
    }

    pub fn push(mut self, input: &[f32], final_input: bool)
        -> Result<TemporalMixerBatch<'a>, TemporalMixerStreamFailure<'a>>
    {
        if self.ended || self.halted {
            return Err(self.failure("Temporal stream is already ended or halted", Work::empty()));
        }
        if input.len() % 384 != 0 || input.iter().any(|v| !v.is_finite()) {
            return Err(self.failure("Temporal stream requires finite complete 384-lane rows", Work::empty()));
        }
        let Some(accepted) = self.accepted.checked_add(input.len() / 384)
            .filter(|&n| n <= self.row_limit) else {
            return Err(self.failure("Temporal stream row limit exceeded", Work::empty()));
        };
        if final_input && accepted == 0 {
            return Err(self.failure("Temporal stream requires nonempty final input", Work::empty()));
        }
        self.accepted = accepted;
        self.ended = final_input;
        let first_row = self.emitted;
        let mut work = Work { input: input.to_vec(), ..Work::empty() };
        if let Err(reason) = self.advance(final_input, &mut work) {
            self.halted = true;
            return Err(self.failure(reason, work));
        }
        let rows = self.skips.iter().map(|q| q.len() / 256).min().unwrap_or(0);
        if final_input && (self.gates.iter().any(|g| g.emitted_rows() != accepted)
            || self.residual.iter().any(|q| !q.is_empty())
            || self.skips.iter().any(|q| q.len() != rows * 256)
            || self.emitted + rows != accepted)
        {
            self.halted = true;
            return Err(self.failure("Temporal final histories have unequal row counts", work));
        }
        let mut left256 = vec![0.0_f32; rows * 256];
        for queue in &mut self.skips {
            for (value, skip) in left256.iter_mut().zip(queue.drain(..rows * 256)) {
                *value += skip;
            }
        }
        self.emitted += rows;
        Ok(TemporalMixerBatch { first_row, left256, final_input, state: self })
    }

    fn advance(&mut self, final_input: bool, work: &mut Work) -> crate::Result<()> {
        for (i, block) in self.mixer.blocks.iter().enumerate() {
            work.block = Some(i);
            work.inner.clear(); work.gated.clear(); work.skip.clear();
            let residual = if work.input.is_empty() { Vec::new() }
                else { block.residual.forward(&work.input)? };
            self.residual[i].extend(residual);
            self.gates[i].append(&block.gate, &work.input, final_input, &mut work.inner)?;
            for row in work.inner.chunks_exact(512) {
                for j in 0..256 {
                    work.gated.push(runtime_tanh(row[j]) * runtime_sigmoid(row[256 + j]));
                }
            }
            if self.residual[i].len() < work.gated.len() {
                return Err("Temporal residual history is incomplete".into());
            }
            work.skip = if work.gated.is_empty() { Vec::new() }
                else { block.skip.forward(&work.gated)? };
            self.skips[i].extend(work.skip.iter().copied());
            let mut next = Vec::with_capacity(work.gated.len());
            for &gated in &work.gated {
                let residual = self.residual[i].pop_front()
                    .ok_or("Temporal residual history is incomplete")?;
                if i < 3 { next.push(gated + residual); }
            }
            work.input = next;
        }
        work.block = None;
        Ok(())
    }
}
