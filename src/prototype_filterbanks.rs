use super::{ModelNode, Result, vector};

const TAPS: usize = 65;
const RING: usize = 512;
const GROUP: usize = 8;
const HALF: usize = TAPS / 2;

fn coefficients(node: &ModelNode<'_>) -> Result<[[f32; TAPS]; GROUP]> {
    if !node.data.is_empty() || node.children.len() != GROUP {
        return Err("Expected eight stored filterbank kernels".into());
    }
    let mut result = [[0.; TAPS]; GROUP];
    for (destination, child) in result.iter_mut().zip(&node.children) {
        destination.copy_from_slice(&vector(child, TAPS)?);
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AnalysisBankProgress {
    pub input_samples: usize,
    pub padding_samples: usize,
    pub output_rows: usize,
    pub ring_position: usize,
    pub counter: usize,
    pub finished: bool,
}

pub struct AnalysisBank8 {
    coefficients: [[f32; TAPS]; GROUP],
    ring: [f32; RING],
    progress: AnalysisBankProgress,
}

impl AnalysisBank8 {
    pub fn from_node(node: &ModelNode<'_>) -> Result<Self> {
        Ok(Self { coefficients: coefficients(node)?, ring: [0.; RING],
            progress: AnalysisBankProgress { counter: HALF, ..Default::default() } })
    }

    pub fn reset(&mut self) {
        self.ring.fill(0.);

        self.progress = AnalysisBankProgress {
            ring_position: self.progress.ring_position, counter: HALF, ..Default::default()
        };
    }

    pub fn progress(&self) -> AnalysisBankProgress { self.progress }

    pub fn process(&mut self, input: &[f32], output: &mut Vec<Vec<f32>>) -> Result<()> {
        if self.progress.finished { return Err("Analysis filterbank already finished".into()); }
        if input.iter().any(|x| !x.is_finite()) { return Err("Nonfinite filterbank audio".into()); }
        output.reserve(input.len() / GROUP + 1);
        for &sample in input {
            self.progress.input_samples += 1;
            self.sample(sample, output)?;
        }
        Ok(())
    }

    pub fn finish(&mut self, output: &mut Vec<Vec<f32>>) -> Result<()> {
        if self.progress.finished { return Ok(()); }
        self.progress.finished = true;
        output.reserve(HALF / GROUP + 1);
        for _ in 0..HALF {
            self.progress.padding_samples += 1;
            self.sample(0., output)?;
        }
        Ok(())
    }

    fn sample(&mut self, sample: f32, output: &mut Vec<Vec<f32>>) -> Result<()> {
        let position = self.progress.ring_position;
        self.ring[(position + TAPS) % RING] = sample;
        self.progress.ring_position = (position + 1) % RING;
        self.progress.counter += 1;
        if self.progress.counter >= HALF && self.progress.counter % GROUP == 0 {
            let position = self.progress.ring_position;
            let mut row = Vec::with_capacity(GROUP);
            for weights in &self.coefficients {
                let mut sum = 0.;
                for (tap, &weight) in weights.iter().enumerate() {
                    sum += weight * self.ring[(position + tap) % RING];
                }
                if !sum.is_finite() { return Err("Analysis filterbank overflow".into()); }
                row.push(sum);
            }
            output.push(row);
            self.progress.output_rows += 1;
            self.progress.counter -= GROUP;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SynthesisBankProgress {
    pub input_rows: usize,
    pub suppressed_ticks: usize,
    pub output_samples: usize,
    pub eos_samples: usize,
    pub ring_position: usize,
    pub counter: usize,
    pub finished: bool,
}

pub struct SynthesisBank8 {
    coefficients: [[f32; TAPS]; GROUP],
    ring: [f32; RING],
    progress: SynthesisBankProgress,
}

impl SynthesisBank8 {
    pub fn from_node(node: &ModelNode<'_>) -> Result<Self> {
        Ok(Self { coefficients: coefficients(node)?, ring: [0.; RING],
            progress: SynthesisBankProgress::default() })
    }

    pub fn reset(&mut self) {
        self.ring.fill(0.);
        self.progress = SynthesisBankProgress {
            ring_position: self.progress.ring_position, ..Default::default()
        };
    }

    pub fn progress(&self) -> SynthesisBankProgress { self.progress }

    pub fn process(&mut self, input: &[Vec<f32>], output: &mut Vec<f32>) -> Result<()> {
        if self.progress.finished { return Err("Synthesis filterbank already finished".into()); }
        if input.iter().any(|row| row.len() != GROUP || row.iter().any(|x| !x.is_finite())) {
            return Err("Synthesis filterbank requires finite eight-channel rows".into());
        }
        output.reserve(input.len().checked_mul(GROUP).ok_or("Filterbank output extent overflow")?);
        for row in input {
            let position = self.progress.ring_position;
            for (&sample, weights) in row.iter().zip(&self.coefficients) {
                for (tap, &weight) in weights.iter().enumerate() {
                    self.ring[(position + tap) % RING] += weight * sample;
                }
            }
            self.progress.input_rows += 1;
            for _ in 0..GROUP {
                if self.progress.counter < HALF {

                    self.progress.counter += 1;
                    self.progress.suppressed_ticks += 1;
                } else {
                    self.publish(output)?;
                }
            }
        }
        Ok(())
    }

    pub fn finish(&mut self, output: &mut Vec<f32>) -> Result<()> {
        if self.progress.finished { return Ok(()); }
        self.progress.finished = true;
        output.reserve(self.progress.counter);
        while self.progress.counter > 0 {
            self.publish(output)?;
            self.progress.counter -= 1;
            self.progress.eos_samples += 1;
        }
        Ok(())
    }

    fn publish(&mut self, output: &mut Vec<f32>) -> Result<()> {
        let position = self.progress.ring_position;
        let sample = self.ring[position];
        if !sample.is_finite() { return Err("Synthesis filterbank overflow".into()); }
        output.push(sample);
        self.ring[position] = 0.;
        self.progress.ring_position = (position + 1) % RING;
        self.progress.output_samples += 1;
        Ok(())
    }
}
