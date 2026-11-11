use super::{PitchSpectralEncoder, PitchSpectralStages};
use std::collections::VecDeque;

const WIDTHS: [usize; 3] = [257, 513, 1025];

pub struct PitchEncoderStream<'a> {
    encoder: &'a PitchSpectralEncoder,
    pending: [VecDeque<f32>; 3],
    accepted: [usize; 3],
    emitted: usize,
    row_limit: usize,
    ended: bool,
    halted: bool,
}
pub(crate) struct PitchEncoderState {
    pending:[VecDeque<f32>;3],accepted:[usize;3],emitted:usize,
    row_limit:usize,ended:bool,halted:bool,
}

pub struct PitchEncoderBatch<'a> {
    pub first_row: usize,
    pub stages: Option<PitchSpectralStages>,
    pub final_input: bool,
    pub state: PitchEncoderStream<'a>,
}

pub struct PitchEncoderStreamFailure<'a> {
    pub reason: String,
    pub advanced: PitchEncoderStream<'a>,
}

impl<'a> PitchEncoderStream<'a> {
    pub(crate) fn into_state(self)->PitchEncoderState {
        let Self {pending,accepted,emitted,row_limit,ended,halted,..}=self;
        PitchEncoderState {pending,accepted,emitted,row_limit,ended,halted}
    }
    pub(crate) fn from_state(encoder:&'a PitchSpectralEncoder,state:PitchEncoderState)->Self {
        let PitchEncoderState {pending,accepted,emitted,row_limit,ended,halted}=state;
        Self {encoder,pending,accepted,emitted,row_limit,ended,halted}
    }
    pub(super) fn new(encoder: &'a PitchSpectralEncoder, row_limit: usize) -> crate::Result<Self> {
        if row_limit == 0 || row_limit > i32::MAX as usize
            || row_limit.checked_mul(1025).is_none()
        {
            return Err("Pitch encoder stream requires a positive i32 row limit".into());
        }
        Ok(Self { encoder, pending: std::array::from_fn(|_| VecDeque::new()),
            accepted: [0; 3], emitted: 0, row_limit, ended: false, halted: false })
    }

    pub fn accepted_rows(&self) -> [usize; 3] { self.accepted }
    pub fn emitted_rows(&self) -> usize { self.emitted }
    pub fn retained_row_counts(&self) -> [usize; 3] {
        std::array::from_fn(|i| self.pending[i].len() / WIDTHS[i])
    }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    pub fn into_unmatched(self) -> [VecDeque<f32>; 3] { self.pending }

    fn failure(self, reason: impl Into<String>) -> PitchEncoderStreamFailure<'a> {
        PitchEncoderStreamFailure { reason: reason.into(), advanced: self }
    }

    pub fn push(mut self, spectra: [&[f32]; 3], final_input: bool)
        -> Result<PitchEncoderBatch<'a>, PitchEncoderStreamFailure<'a>>
    {
        if self.ended || self.halted {
            return Err(self.failure("Pitch encoder stream is already ended or halted"));
        }
        let mut accepted = self.accepted;
        for i in 0..3 {
            if spectra[i].len() % WIDTHS[i] != 0
                || spectra[i].iter().any(|v| !v.is_finite())
            {
                return Err(self.failure("Pitch encoder stream requires finite complete spectral rows"));
            }
            let Some(rows) = accepted[i].checked_add(spectra[i].len() / WIDTHS[i])
                .filter(|&n| n <= self.row_limit) else {
                return Err(self.failure("Pitch encoder stream row limit exceeded"));
            };
            accepted[i] = rows;
        }
        for i in 0..3 { self.pending[i].extend(spectra[i].iter().copied()); }
        self.accepted = accepted;
        self.ended = final_input;
        let first_row = self.emitted;
        let rows = self.retained_row_counts().into_iter().min().unwrap_or(0);
        let stages = if rows == 0 { None } else {
            let paired: [Vec<f32>; 3] = std::array::from_fn(|i|
                self.pending[i].iter().take(rows * WIDTHS[i]).copied().collect());
            let stages = match self.encoder.forward([&paired[0], &paired[1], &paired[2]]) {
                Ok(stages) => stages,
                Err(reason) => {
                    self.halted = true;
                    return Err(self.failure(reason));
                }
            };
            for i in 0..3 { drop(self.pending[i].drain(..rows * WIDTHS[i])); }
            self.emitted += rows;
            Some(stages)
        };
        Ok(PitchEncoderBatch { first_row, stages, final_input, state: self })
    }
}
