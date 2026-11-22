use std::f64::consts::PI;

pub(super) struct StreamingResampler {
    input: Vec<f32>,
    position: usize,
    phase: usize,
    step: usize,
    phases: usize,
    radius: usize,
    taps: Vec<f32>,
}

impl StreamingResampler {
    pub(super) fn new(source_rate: u32, target_rate: u32) -> Result<Self, String> {
        if source_rate == 0 || target_rate == 0 {
            return Err("Sample rates must be positive".into());
        }
        let divisor = gcd(source_rate, target_rate);
        let step = (source_rate / divisor) as usize;
        let phases = (target_rate / divisor) as usize;
        let cutoff = (f64::from(target_rate) / f64::from(source_rate)).min(1.0) * 0.94;
        let radius = (32.0 / cutoff).ceil() as usize;
        let width = radius.checked_mul(2).ok_or("Resampler filter extent overflow")?;
        let count = phases.checked_mul(width).ok_or("Resampler coefficient extent overflow")?;
        let mut taps = Vec::new();
        taps.try_reserve_exact(count).map_err(|e| format!("Cannot allocate resampler coefficients: {e}"))?;
        for phase in 0..phases {
            let fraction = phase as f64 / phases as f64;
            let mut coefficients = Vec::new();
            coefficients.try_reserve_exact(width).map_err(|e| format!("Cannot allocate resampler phase: {e}"))?;
            for tap in 0..width {
                let offset = tap as f64 + 1.0 - radius as f64;
                let distance = fraction - offset;
                let coefficient = if distance.abs() >= radius as f64 { 0.0 } else {
                    let angle = PI * cutoff * distance;
                    let sinc = if angle.abs() < 1e-12 { 1.0 } else { angle.sin() / angle };
                    cutoff * sinc * (0.5 + 0.5 * (PI * distance / radius as f64).cos())
                };
                coefficients.push(coefficient);
            }
            let total: f64 = coefficients.iter().sum();
            if !total.is_finite() || total.abs() < 1e-12 {
                return Err("Cannot normalize resampler filter".into());
            }
            taps.extend(coefficients.into_iter().map(|coefficient| (coefficient / total) as f32));
        }
        let mut input = Vec::new();
        input.try_reserve_exact(radius).map_err(|e| format!("Cannot allocate resampler history: {e}"))?;
        input.resize(radius, 0.0);
        Ok(Self { input, position: radius, phase: 0, step, phases, radius, taps })
    }

    pub(super) fn process(&mut self, samples: &[f32], output: &mut Vec<f32>) -> Result<(), String> {
        self.input.try_reserve(samples.len()).map_err(|e| format!("Cannot buffer resampler input: {e}"))?;
        self.input.extend_from_slice(samples);
        let lookahead = self.position.checked_add(self.radius).ok_or("Resampler position overflow")?;
        let available = self.input.len().saturating_sub(lookahead);
        let estimate = available.checked_mul(self.phases).ok_or("Resampler output extent overflow")?
            .div_ceil(self.step).checked_add(1).ok_or("Resampler output extent overflow")?;
        output.try_reserve(estimate).map_err(|e| format!("Cannot buffer resampler output: {e}"))?;
        let width = self.radius * 2;
        while self.position.checked_add(self.radius).ok_or("Resampler position overflow")? < self.input.len() {
            let first = self.position + 1 - self.radius;
            let coefficients = &self.taps[self.phase * width..(self.phase + 1) * width];
            let value = self.input[first..first + width].iter().zip(coefficients)
                .fold(0.0_f64, |sum, (&sample, &coefficient)| sum + f64::from(sample) * f64::from(coefficient));
            let value = value as f32;
            if !value.is_finite() { return Err("Resampler produced nonfinite audio".into()); }
            output.push(value);
            let phase = self.phase.checked_add(self.step).ok_or("Resampler phase overflow")?;
            self.position = self.position.checked_add(phase / self.phases).ok_or("Resampler position overflow")?;
            self.phase = phase % self.phases;
        }
        let consumed = self.position.saturating_sub(self.radius);
        if consumed != 0 {
            self.input.copy_within(consumed.., 0);
            self.input.truncate(self.input.len() - consumed);
            self.position -= consumed;
        }
        Ok(())
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 { let remainder = a % b; a = b; b = remainder; }
    a
}
