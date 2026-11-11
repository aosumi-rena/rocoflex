use super::PitchAudioSpectrum;
use std::collections::VecDeque;

pub struct PitchSpectrumStream<'a> {
    spectrum: &'a PitchAudioSpectrum,
    samples: VecDeque<f32>,
    base: usize,
    seen: usize,
    next_rows: [usize; 3],
    sample_limit: usize,
    ended: bool,
    halted: bool,
}
pub(crate) struct PitchSpectrumState {
    samples:VecDeque<f32>,base:usize,seen:usize,next_rows:[usize;3],
    sample_limit:usize,ended:bool,halted:bool,
}

impl PitchSpectrumState {

    pub(crate) fn diagnostic_progress(&self)->(usize,[usize;3],usize) {
        (self.seen,self.next_rows,self.samples.len())
    }
}

pub struct PitchSpectrumBatch<'a> {
    pub first_rows: [usize; 3],
    pub spectra: [Vec<f32>; 3],
    pub final_input: bool,
    pub state: PitchSpectrumStream<'a>,
}

pub struct PitchSpectrumStreamFailure<'a> {
    pub reason: String,
    pub first_rows: [usize; 3],
    pub completed_spectra: [Vec<f32>; 3],
    pub advanced: PitchSpectrumStream<'a>,
}

impl<'a> PitchSpectrumStream<'a> {
    pub(crate) fn into_state(self)->PitchSpectrumState {
        let Self {samples,base,seen,next_rows,sample_limit,ended,halted,..}=self;
        PitchSpectrumState {samples,base,seen,next_rows,sample_limit,ended,halted}
    }
    pub(crate) fn from_state(spectrum:&'a PitchAudioSpectrum,state:PitchSpectrumState)->Self {
        let PitchSpectrumState {samples,base,seen,next_rows,sample_limit,ended,halted}=state;
        Self {spectrum,samples,base,seen,next_rows,sample_limit,ended,halted}
    }
    pub(super) fn new(spectrum: &'a PitchAudioSpectrum, sample_limit: usize) -> crate::Result<Self> {
        if sample_limit == 0 || sample_limit > i32::MAX as usize {
            return Err("Pitch spectral stream requires a positive i32 sample limit".into());
        }
        Ok(Self {
            spectrum, samples: VecDeque::new(), base: 0, seen: 0,
            next_rows: [0; 3], sample_limit, ended: false, halted: false,
        })
    }

    pub fn from_mature_histories(
        spectrum: &'a PitchAudioSpectrum,
        histories: [Vec<f32>; 3],
        seen_samples: usize,
        sample_limit: usize,
    ) -> crate::Result<Self> {
        if seen_samples < 1920 || sample_limit > i32::MAX as usize || seen_samples > sample_limit {
            return Err("Mature pitch history requires seen>=1920 within an i32 sample limit".into());
        }
        if spectrum.windows[0].len() != 512 || spectrum.windows[1].len() != 1024
            || spectrum.windows[2].len() != 2048 {
            return Err("Mature pitch history requires the existing512/1024/2048 windows".into());
        }
        let q = seen_samples / 240;
        let remainder = seen_samples % 240;
        let lengths = [272 + remainder, 784 + remainder, 1808 + remainder];
        for family in 0..3 {
            if histories[family].len() != lengths[family]
                || histories[family].iter().any(|value| !value.is_finite()) {
                return Err("Mature pitch history has invalid length or nonfinite samples".into());
            }
        }
        for family in 0..2 {
            let suffix = &histories[2][histories[2].len() - histories[family].len()..];
            if histories[family].iter().zip(suffix).any(|(a, b)| a.to_bits() != b.to_bits()) {
                return Err("Mature pitch histories must be exact bitwise suffixes".into());
            }
        }
        let base = q.checked_mul(240).and_then(|value| value.checked_sub(1808))
            .ok_or("Mature pitch history base overflows")?;
        let [_, _, largest] = histories;
        Ok(Self {
            spectrum, samples: VecDeque::from(largest), base, seen: seen_samples,
            next_rows: [q; 3], sample_limit, ended: false, halted: false,
        })
    }

    pub fn accepted_samples(&self) -> usize { self.seen }
    pub fn emitted_rows(&self) -> [usize; 3] { self.next_rows }
    pub fn retained_samples(&self) -> usize { self.samples.len() }
    pub fn is_ended(&self) -> bool { self.ended }
    pub fn is_halted(&self) -> bool { self.halted }

    fn failure(
        self,
        reason: impl Into<String>,
        first_rows: [usize; 3],
        completed_spectra: [Vec<f32>; 3],
    ) -> PitchSpectrumStreamFailure<'a> {
        PitchSpectrumStreamFailure { reason: reason.into(), first_rows, completed_spectra, advanced: self }
    }

    pub fn push(
        mut self,
        input: &[f32],
        final_input: bool,
    ) -> Result<PitchSpectrumBatch<'a>, PitchSpectrumStreamFailure<'a>> {
        let first_rows = self.next_rows;
        let empty = || std::array::from_fn(|_| Vec::new());
        if self.ended || self.halted {
            return Err(self.failure("Pitch spectral stream is ended or halted", first_rows, empty()));
        }
        if input.iter().any(|v| !v.is_finite()) {
            return Err(self.failure("Pitch spectral stream requires finite input", first_rows, empty()));
        }
        let Some(seen) = self.seen.checked_add(input.len()) else {
            return Err(self.failure("Pitch spectral sample count overflow", first_rows, empty()));
        };
        if seen > self.sample_limit {
            return Err(self.failure("Pitch spectral sample limit exceeded", first_rows, empty()));
        }
        if final_input && seen == 0 {
            return Err(self.failure("Pitch spectral stream requires nonempty final input", first_rows, empty()));
        }
        self.samples.extend(input.iter().copied());
        self.seen = seen;
        self.ended = final_input;
        let mut spectra: [Vec<f32>; 3] = empty();
        for index in 0..3 {
            let n = self.spectrum.windows[index].len();
            let left = n - 240;
            let final_rows = (seen + left).div_ceil(240);
            loop {
                let center = self.next_rows[index] * 240;
                if final_input {
                    if self.next_rows[index] >= final_rows { break; }
                } else if center + 240 > seen {
                    break;
                }
                let mut frame = vec![0.0_f32; n];
                let mut first = n;
                let mut end = 0;
                for (offset, value) in frame.iter_mut().enumerate() {
                    let at = center + offset;
                    if at >= left {
                        let absolute = at - left;
                        if absolute < seen {
                            let Some(relative) = absolute.checked_sub(self.base) else {
                                self.halted = true;
                                return Err(self.failure("Pitch history precedes its retained range", first_rows, spectra));
                            };
                            let Some(sample) = self.samples.get(relative) else {
                                self.halted = true;
                                return Err(self.failure("Pitch history lacks an admitted sample", first_rows, spectra));
                            };
                            *value = *sample;
                            first = first.min(offset);
                            end = offset + 1;
                        }
                    }
                }
                let range = if end == 0 { 0..0 } else { first..end };
                let row = match self.spectrum.process_frame(index, frame, range) {
                    Ok(row) => row,
                    Err(reason) => {
                        self.halted = true;
                        return Err(self.failure(reason, first_rows, spectra));
                    }
                };
                spectra[index].extend(row);
                self.next_rows[index] += 1;
            }
        }
        let keep_from = (0..3).map(|index| {
            let left = self.spectrum.windows[index].len() - 240;
            (self.next_rows[index] * 240).saturating_sub(left)
        }).min().unwrap_or(0).min(seen);
        let discard = keep_from.saturating_sub(self.base).min(self.samples.len());
        drop(self.samples.drain(..discard));
        self.base += discard;
        Ok(PitchSpectrumBatch { first_rows, spectra, final_input, state: self })
    }
}
