use super::FrontendProducer;
use crate::{feature_projection::FeatureProjectionStages,
    temporal_mixer::{TemporalMixerStream, TemporalMixerStreamFailure}};
use std::collections::VecDeque;

pub struct FrontendStream<'a> {
    producer: &'a FrontendProducer,
    hidden384: Vec<f32>,
    temporal: Option<TemporalMixerStream<'a>>,
    right: VecDeque<f32>,
    accepted: usize,
    emitted: usize,
    row_limit: usize,
    ended: bool,
    halted: bool,
}
pub struct FrontendStartFailure { pub reason: String, pub hidden384: Vec<f32> }
pub struct FrontendBatch<'a> {
    pub first_row: usize,
    pub combined256: Vec<f32>,
    pub projection: Option<FeatureProjectionStages>,
    pub final_input: bool,
    pub state: FrontendStream<'a>,
}
pub struct FrontendStreamFailure<'a> {
    pub reason: String,

    pub pending_hidden384: Option<Vec<f32>>,
    pub recurrent384: Vec<f32>,
    pub left256: Vec<f32>,
    pub combined256: Vec<f32>,
    pub temporal_failure: Option<TemporalMixerStreamFailure<'a>>,
    pub advanced: FrontendStream<'a>,
}

impl<'a> FrontendStream<'a> {
    pub(super) fn new(producer: &'a FrontendProducer, hidden384: Vec<f32>, row_limit: usize)
        -> Result<Self, FrontendStartFailure>
    {
        if hidden384.len() != 384 || hidden384.iter().any(|v| !v.is_finite()) {
            return Err(FrontendStartFailure {
                reason: "Frontend stream requires an explicit finite 384-lane hidden vector".into(), hidden384,
            });
        }
        let temporal = match producer.temporal.stream_with_limit(row_limit) {
            Ok(state) => state,
            Err(reason) => return Err(FrontendStartFailure { reason, hidden384 }),
        };
        Ok(Self { producer, hidden384, temporal: Some(temporal), right: VecDeque::new(),
            accepted: 0, emitted: 0, row_limit, ended: false, halted: false })
    }

    pub fn hidden_state(&self) -> &[f32] { &self.hidden384 }
    pub fn accepted_rows(&self) -> usize { self.accepted }
    pub fn emitted_rows(&self) -> usize { self.emitted }
    pub fn retained_right_rows(&self) -> usize { self.right.len() / 256 }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    fn failure(self, reason: impl Into<String>, recurrent384: Vec<f32>, left256: Vec<f32>,
        combined256: Vec<f32>, temporal_failure: Option<TemporalMixerStreamFailure<'a>>)
        -> FrontendStreamFailure<'a>
    {
        FrontendStreamFailure { reason: reason.into(), pending_hidden384: None, recurrent384, left256, combined256,
            temporal_failure, advanced: self }
    }

    pub fn push(mut self, input256: &[f32], final_input: bool)
        -> Result<FrontendBatch<'a>, FrontendStreamFailure<'a>>
    {
        if self.ended || self.halted || self.temporal.is_none() {
            return Err(self.failure("Frontend stream is already ended or halted",
                Vec::new(), Vec::new(), Vec::new(), None));
        }
        if input256.len() % 256 != 0 || input256.iter().any(|v| !v.is_finite()) {
            return Err(self.failure("Frontend stream requires finite complete 256-lane rows",
                Vec::new(), Vec::new(), Vec::new(), None));
        }
        let Some(accepted) = self.accepted.checked_add(input256.len() / 256)
            .filter(|&n| n <= self.row_limit) else {
            return Err(self.failure("Frontend stream row limit exceeded",
                Vec::new(), Vec::new(), Vec::new(), None));
        };
        if final_input && accepted == 0 {
            return Err(self.failure("Frontend stream requires nonempty final input",
                Vec::new(), Vec::new(), Vec::new(), None));
        }
        let mut next_hidden = self.hidden384.clone();

        let mut recurrent = Vec::with_capacity(input256.len() / 256 * 384);
        for row in input256.chunks_exact(256) {
            match self.producer.gru.forward256(row, &mut next_hidden) {
                Ok(rows) => recurrent.extend(rows),
                Err(reason) => {
                    self.halted = true;
                    let mut failure = self.failure(reason, recurrent, Vec::new(), Vec::new(), None);
                    failure.pending_hidden384 = Some(next_hidden);
                    return Err(failure);
                }
            }
            if next_hidden.iter().any(|v| !v.is_finite()) {
                self.halted = true;
                let mut failure = self.failure("Frontend recurrent state left the finite domain",
                    recurrent, Vec::new(), Vec::new(), None);
                failure.pending_hidden384 = Some(next_hidden);
                return Err(failure);
            }
        }
        if next_hidden.iter().chain(&recurrent).any(|v| !v.is_finite()) {
            self.halted = true;
            let mut failure = self.failure("Frontend recurrent state left the finite domain",
                recurrent, Vec::new(), Vec::new(), None);
            failure.pending_hidden384 = Some(next_hidden);
            return Err(failure);
        }
        let right = if input256.is_empty() { Vec::new() } else {
            match self.producer.encoder.forward_right256(input256) {
                Ok(rows) => rows,
                Err(reason) => {
                    self.halted = true;
                    let mut failure = self.failure(reason, recurrent, Vec::new(), Vec::new(), None);
                    failure.pending_hidden384 = Some(next_hidden);
                    return Err(failure);
                }
            }
        };

        self.hidden384 = next_hidden;
        self.accepted = accepted;
        self.ended = final_input;
        self.right.extend(right);
        let temporal = self.temporal.take().expect("admitted temporal owner");
        let batch = match temporal.push(&recurrent, final_input) {
            Ok(batch) => batch,
            Err(failure) => {
                let reason = failure.reason.clone();
                self.halted = true;
                return Err(self.failure(reason, recurrent, Vec::new(), Vec::new(), Some(failure)));
            }
        };
        let left = batch.left256;
        let first_row = batch.first_row;
        self.temporal = Some(batch.state);
        if first_row != self.emitted || left.len() % 256 != 0 || self.right.len() < left.len()
            || (final_input && self.right.len() != left.len())
        {
            self.halted = true;
            return Err(self.failure("Frontend branch histories have unequal row ranges",
                recurrent, left, Vec::new(), None));
        }
        let combined = left.iter().zip(&self.right).map(|(a, b)| a + b).collect::<Vec<_>>();
        let projection = if combined.is_empty() { None } else {
            match self.producer.features.forward256(&combined) {
                Ok(rows) => Some(rows),
                Err(reason) => {
                    self.halted = true;
                    return Err(self.failure(reason, recurrent, left, combined, None));
                }
            }
        };
        drop(self.right.drain(..left.len()));
        self.emitted += left.len() / 256;
        Ok(FrontendBatch { first_row, combined256: combined, projection, final_input, state: self })
    }
}
