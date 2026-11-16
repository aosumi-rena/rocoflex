use crate::Result;
use super::count_plan;

#[path = "legacy_vocoder_model.rs"]
mod model;
#[path = "legacy_vocoder_signal.rs"]
mod signal;

pub(super) struct LegacyVocoder {
    model: model::Model,
    signal: signal::Signal,
    frames: Vec<model::NeuralFrame>,
    delay: usize,
    accepted: u64,
    failure: Option<String>,
}

impl LegacyVocoder {
    pub(super) fn new(bytes: &[u8], seed: u32) -> Result<Self> {
        let model::Prepared {
            features, down, up, residual, post, log_gain, modulation_bias, gate_noise, ratios,
        } = model::Model::prepare(bytes)?;
        let signal = signal::Signal::new(
            down, up, residual, post, log_gain, modulation_bias, gate_noise, ratios, seed,
        )?;
        let delay = features.lookahead_rows().checked_add(signal.delay_rows())
            .ok_or("Legacy vocoder delay exceeds the supported range")?;
        Ok(Self { model: features, signal, frames: Vec::with_capacity(1), delay,
            accepted: 0, failure: None })
    }

    pub(super) fn prepared_count_inputs(&self) -> count_plan::Query<u64> {
        count_plan::cold(self.accepted != 0, self.failure.is_some())?;
        count_plan::checked_rows(self.delay)
    }

    pub(super) fn process(&mut self, raw: &[f32], output: &mut Vec<f32>) -> Result<()> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let begin = output.len();
        let result: Result<()> = (|| {
            self.frames.clear();
            self.model.process(raw, &mut self.frames)?;
            for frame in self.frames.drain(..) {
                self.signal.process(&frame, output)?;
            }
            let expected = if self.accepted >= self.delay as u64 { 240 } else { 0 };
            if output.len() - begin != expected {
                return Err("Legacy vocoder produced an unexpected sample count".into());
            }
            self.accepted = self.accepted.checked_add(1).ok_or("Legacy frame count overflow")?;
            Ok(())
        })();
        if let Err(error) = &result {
            output.truncate(begin);
            self.failure = Some(error.clone());
        }
        result
    }
}
