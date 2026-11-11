use crate::Result;

const HOP: usize = 240;
const WIDTH: usize = 2 * HOP;
const HARMONICS: usize = 5;
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const DIVISOR: f32 = 0.005_f32;

pub(crate) struct HarmonicRecord<'a> {
    pub scalar0: f32,
    pub scalar4: f32,

    pub projection10: &'a [f32],
}

pub(crate) struct HarmonicExcitation {
    frequencies: [f32; HARMONICS],
    window: [f32; WIDTH],
    overlap: [f32; WIDTH],
    scratch: [f32; WIDTH],
    first: bool,
    ended: bool,
    failure: Option<String>,
}

impl HarmonicExcitation {

    pub(crate) fn new(frequencies: [f32; HARMONICS]) -> Result<Self> {
        if frequencies.iter().any(|x| !x.is_finite()) {
            return Err("Nonfinite learned harmonic frequency".into());
        }
        let window = std::array::from_fn(|sample| {
            let ratio = sample as f32 / WIDTH as f32;
            let angle = ratio * TWO_PI;
            (1.0 - angle.cos()) * 0.5
        });
        Ok(Self { frequencies, window, overlap: [0.; WIDTH], scratch: [0.; WIDTH],
            first: true, ended: false, failure: None })
    }

    pub(crate) fn process(&mut self, record: HarmonicRecord<'_>, output: &mut Vec<f32>) -> Result<()> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        if self.ended { return Err("Harmonic excitation input already ended".into()); }
        let result = self.advance(record, output);
        if let Err(error) = &result { self.failure = Some(error.clone()); }
        result
    }

    fn advance(&mut self, record: HarmonicRecord<'_>, output: &mut Vec<f32>) -> Result<()> {
        if record.projection10.len() != 2 * HARMONICS
            || !record.scalar0.is_finite() || !record.scalar4.is_finite()
            || record.projection10.iter().any(|x| !x.is_finite()) {
            return Err("Harmonic excitation requires the actual finite record and ten projection values".into());
        }
        if !self.first { output.try_reserve(HOP).map_err(|e| e.to_string())?; }
        self.overlap.copy_within(HOP..WIDTH, 0);
        self.overlap[HOP..].fill(0.);
        self.scratch.fill(0.);
        let ratio = HOP as f32 / DIVISOR;
        for harmonic in 0..HARMONICS {
            let frequency = self.frequencies[harmonic];
            let value = record.projection10[harmonic];
            let limited = if value > 0. { 0. } else { value };
            let amplitude = limited.exp();
            let phase0 = record.scalar4 * frequency;
            let phase1 = phase0 * TWO_PI;
            let phase = phase1 + record.projection10[harmonic + HARMONICS];
            for sample in 0..WIDTH {
                let position = (sample as i32 - HOP as i32) as f32;
                let a0 = position * record.scalar0;
                let a1 = a0 * frequency;
                let a2 = a1 / ratio;
                let a3 = a2 * TWO_PI;
                let angle = a3 + phase;
                let product = angle.cos() * amplitude;
                let value = product + self.scratch[sample];
                if !angle.is_finite() || !value.is_finite() {
                    return Err("Nonfinite learned harmonic signal".into());
                }
                self.scratch[sample] = value;
            }
        }
        for sample in 0..WIDTH {
            let product = self.scratch[sample] * self.window[sample];
            self.overlap[sample] = product + self.overlap[sample];
            if !self.overlap[sample].is_finite() {
                return Err("Nonfinite harmonic overlap signal".into());
            }
        }
        if !self.first { output.extend_from_slice(&self.overlap[..HOP]); }
        self.first = false;
        Ok(())
    }

    pub(crate) fn finish(&mut self, output: &mut Vec<f32>) -> Result<()> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        if self.ended { return Ok(()); }
        output.try_reserve(HOP).map_err(|e| e.to_string())?;
        output.extend_from_slice(&self.overlap[HOP..]);
        self.ended = true;
        Ok(())
    }
}
