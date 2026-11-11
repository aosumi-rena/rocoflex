use super::AudioSpectrum;
use std::collections::VecDeque;

pub struct SpectrumStream<'a> {
    spectrum: &'a AudioSpectrum,
    samples: VecDeque<f32>,
    base: usize,
    seen: usize,
    next_row: usize,
    sample_limit: usize,
    ended: bool,
    halted: bool,
}

pub struct SpectrumBatch<'a> {
    pub first_row: usize,
    pub postlog513: Vec<f32>,
    pub final_input: bool,
    pub state: SpectrumStream<'a>,
}

pub struct SpectrumStreamFailure<'a> {
    pub reason: String,
    pub first_row: usize,
    pub completed_postlog513: Vec<f32>,
    pub advanced: SpectrumStream<'a>,
}

impl<'a> SpectrumStream<'a> {
    pub(super) fn new(spectrum: &'a AudioSpectrum, sample_limit: usize) -> crate::Result<Self> {
        if sample_limit == 0 || sample_limit > i32::MAX as usize {
            return Err("Spectral stream requires an explicit positive i32 sample limit".into());
        }
        Ok(Self {
            spectrum,
            samples: VecDeque::new(),
            base: 0,
            seen: 0,
            next_row: 0,
            sample_limit,
            ended: false,
            halted: false,
        })
    }

    pub fn accepted_samples(&self) -> usize { self.seen }
    pub fn emitted_rows(&self) -> usize { self.next_row }
    pub fn retained_samples(&self) -> usize { self.samples.len() }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    fn failure(
        self,
        reason: impl Into<String>,
        first_row: usize,
        completed_postlog513: Vec<f32>,
    ) -> SpectrumStreamFailure<'a> {
        SpectrumStreamFailure {
            reason: reason.into(), first_row, completed_postlog513, advanced: self,
        }
    }

    pub fn push(
        mut self,
        input: &[f32],
        final_input: bool,
    ) -> Result<SpectrumBatch<'a>, SpectrumStreamFailure<'a>> {
        let first_row = self.next_row;
        if self.ended || self.halted {
            return Err(self.failure("Spectral stream is already ended or halted", first_row, Vec::new()));
        }
        if input.iter().any(|v| !v.is_finite()) {
            return Err(self.failure("Spectral stream input must be finite", first_row, Vec::new()));
        }
        let Some(seen) = self.seen.checked_add(input.len()) else {
            return Err(self.failure("Spectral stream input count overflow", first_row, Vec::new()));
        };
        if seen > self.sample_limit {
            return Err(self.failure("Spectral stream sample limit exceeded", first_row, Vec::new()));
        }
        if final_input && seen == 0 {
            return Err(self.failure("Spectral stream requires nonempty final input", first_row, Vec::new()));
        }

        self.samples.extend(input.iter().copied());
        self.seen = seen;
        self.ended = final_input;
        let final_rows = (seen + 512).div_ceil(240);
        let mut output = Vec::new();
        loop {
            let center = self.next_row * 240;
            if final_input {
                if self.next_row >= final_rows { break; }
            } else if center + 512 > seen {
                break;
            }

            let mut frame = [0.0_f32; 1024];
            for (offset, value) in frame.iter_mut().enumerate() {
                let at = center + offset;
                if at >= 512 {
                    let absolute = at - 512;
                    if absolute < seen {
                        let Some(index) = absolute.checked_sub(self.base) else {
                            self.halted = true;
                            return Err(self.failure("Spectral history precedes its retained range", first_row, output));
                        };
                        let Some(sample) = self.samples.get(index) else {
                            self.halted = true;
                            return Err(self.failure("Spectral history is missing an admitted sample", first_row, output));
                        };
                        *value = *sample;
                    }
                }
            }

            let stages = match self.spectrum.from_window_samples(&frame) {
                Ok(stages) => stages,
                Err(reason) => {
                    self.halted = true;
                    return Err(self.failure(reason, first_row, output));
                }
            };
            output.extend(stages.postlog513);
            self.next_row += 1;

            let keep_from = (self.next_row * 240).saturating_sub(512).min(seen);
            let discard = keep_from.saturating_sub(self.base).min(self.samples.len());
            drop(self.samples.drain(..discard));
            self.base += discard;
        }
        Ok(SpectrumBatch { first_row, postlog513: output, final_input, state: self })
    }
}
