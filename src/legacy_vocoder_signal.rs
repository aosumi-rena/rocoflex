//! Legacy vocoder signal path.

use std::collections::VecDeque;

use crate::{
    Result,
    continuous_dsp::LtvPair,
    noise::{MinStd, NormalDistribution},
    prototype_synthesis_networks::{
        AnalysisBank8, NcwnNetwork, NcwnPersistentStream, SynthesisBank8, SynthesisPost,
    },
    quantized_matrix::QuantizedWorkspace,
};

use super::model::NeuralFrame;
use super::super::{
    excitation::{HarmonicExcitation, HarmonicRecord},
    generator::SourceGenerator,
};

const HOP: usize = 240;
const WIDTH: usize = 72;
const BANDS: usize = 8;
const ROWS_PER_HOP: usize = HOP / BANDS;
const INITIAL_CONDITION_ROWS: usize = ROWS_PER_HOP / 2;
const ANALYSIS_TRIM_ROWS: usize = 3;
const SYNTHESIS_TRIM_ROWS: usize = 32 / BANDS;
const LTV_STARTUP_HOPS: usize = 3; // ceil(512 / 240) whole-hop publication debt

struct Residual {
    down: AnalysisBank8,
    up: SynthesisBank8,
    network: NcwnPersistentStream,
    post: SynthesisPost,
    post_work: Option<QuantizedWorkspace>,
    features: VecDeque<[f32; 16]>,
    conditions: VecDeque<[f32; WIDTH]>,
    pending_noise: VecDeque<[f32; BANDS]>,
    previous_condition: Option<[f32; WIDTH]>,
    rng: MinStd,
    gaussian: NormalDistribution,
    trim: usize,
    gate_noise: bool,
    lookahead_rows: usize,
}

impl Residual {
    fn new(
        down: AnalysisBank8,
        up: SynthesisBank8,
        network: NcwnNetwork,
        post: SynthesisPost,
        gate_noise: bool,
        seed: u32,
    ) -> Result<Self> {
        if (network.input_width, network.condition_width, network.output_width) != (16, WIDTH, 64) {
            return Err("Legacy residual network must map 16 inputs and 72 conditions to 64 outputs".into());
        }
        let (left, right) = network.streaming_context()?;
        if left != right || right > 65_536 {
            return Err("Legacy residual requires a bounded centered convolution context".into());
        }
        let network = network.into_persistent_stream()?;
        if network.lookahead_rows() != right {
            return Err("Legacy residual streaming lookahead differs from its model".into());
        }
        let post_work = post.quantized_workspace_requirements()
            .map(|(input, output)| QuantizedWorkspace::new(input, output)).transpose()?;
        Ok(Self {
            down, up, network, post, post_work,
            features: VecDeque::with_capacity(ROWS_PER_HOP + 16),
            conditions: VecDeque::with_capacity(ROWS_PER_HOP),
            pending_noise: VecDeque::with_capacity(right.saturating_add(ROWS_PER_HOP)),
            previous_condition: None,
            rng: MinStd::from_state(seed)?,
            gaussian: NormalDistribution::default(),
            trim: ANALYSIS_TRIM_ROWS,
            gate_noise,
            lookahead_rows: right,
        })
    }

    fn process(&mut self, signal: &[f32], condition: &[f32; WIDTH], out: &mut Vec<f32>) -> Result<()> {
        if signal.len() != HOP || signal.iter().chain(condition).any(|x| !x.is_finite()) {
            return Err("Legacy residual requires a finite 240-sample source and 72 conditions".into());
        }
        let mut bands = Vec::with_capacity(ROWS_PER_HOP);
        self.down.process(signal, &mut bands)?;
        for row in bands {
            if self.trim != 0 { self.trim -= 1; continue; }
            if row.len() != BANDS { return Err("Legacy analysis bank produced a partial subband row".into()); }
            let mut feature = [0.0; 16];
            feature[..BANDS].copy_from_slice(&row);
            for value in &mut feature[BANDS..] {
                *value = self.gaussian.sample(&mut self.rng, 0.0, 1.0, true)?;
            }
            self.features.push_back(feature);
        }

        match &self.previous_condition {
            None => for _ in 0..INITIAL_CONDITION_ROWS { self.conditions.push_back(*condition); },
            Some(previous) => for step in 0..ROWS_PER_HOP {
                let fraction = (step as f32 + 0.5) / ROWS_PER_HOP as f32;
                self.conditions.push_back(std::array::from_fn(|i|
                    (condition[i] - previous[i]) * fraction + previous[i]));
            },
        }
        self.previous_condition = Some(*condition);

        let count = self.features.len().min(self.conditions.len());
        if count == 0 { return Ok(()); }
        let mut input = Vec::with_capacity(count);
        let mut conditions = Vec::with_capacity(count);
        for _ in 0..count {
            let row = self.features.pop_front().ok_or("Legacy residual feature queue underflow")?;
            let condition = self.conditions.pop_front().ok_or("Legacy residual condition queue underflow")?;
            if self.gate_noise {
                self.pending_noise.push_back(row[BANDS..].try_into().map_err(|_| "Legacy noise row width")?);
            }
            input.push(row.to_vec());
            conditions.push(condition.to_vec());
        }
        let mut hidden = Vec::new();
        self.network.push(&input, &conditions, false, &mut hidden)?;
        let mut projected = Vec::with_capacity(hidden.len());
        for row in hidden {
            let mut values = [0.0; BANDS];
            self.post.forward_row_using(&row, &mut values, self.post_work.as_mut())?;
            if self.gate_noise {
                let noise = self.pending_noise.pop_front().ok_or("Legacy residual noise queue underflow")?;
                for (value, noise) in values.iter_mut().zip(noise) {
                    *value = noise / (1.0 + (-*value).exp());
                }
            }
            if values.iter().any(|x| !x.is_finite()) {
                return Err("Legacy residual projection produced a nonfinite value".into());
            }
            projected.push(values.to_vec());
        }
        self.up.process(&projected, out)
    }
}

pub(crate) struct Signal {
    source: SourceGenerator,
    harmonic: HarmonicExcitation,
    residual: Residual,
    paired_filters: LtvPair,
    noise_filter: LtvPair,
    modulation_bias: f32,
    phase: f64,
    normalized: VecDeque<[f32; WIDTH]>,
    spectra: [VecDeque<Vec<f32>>; 3],
    branches: [VecDeque<f32>; 3],
    residual_samples: VecDeque<f32>,
    delay: usize,
    accepted: usize,
    published: usize,
    failure: Option<String>,
}

impl Signal {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        down: AnalysisBank8,
        up: SynthesisBank8,
        network: NcwnNetwork,
        post: SynthesisPost,
        log_gain: f32,
        modulation_bias: f32,
        gate_noise: bool,
        ratios: [f32; 5],
        seed: u32,
    ) -> Result<Self> {
        if !log_gain.is_finite() || !modulation_bias.is_finite() {
            return Err("Legacy vocoder source controls must be finite".into());
        }
        let residual = Residual::new(down, up, network, post, gate_noise, seed)?;
        let subband_debt = residual.lookahead_rows
            .checked_add(INITIAL_CONDITION_ROWS + SYNTHESIS_TRIM_ROWS)
            .and_then(|n| n.checked_add(ROWS_PER_HOP - 1))
            .ok_or("Legacy signal publication count overflow")? / ROWS_PER_HOP;
        let delay = 1usize.checked_add(subband_debt)
            .and_then(|n| n.checked_add(LTV_STARTUP_HOPS))
            .ok_or("Legacy signal publication delay overflow")?;
        Ok(Self {
            source: SourceGenerator::legacy(log_gain)?,
            harmonic: HarmonicExcitation::legacy(ratios)?,
            residual,
            paired_filters: LtvPair::new()?,
            noise_filter: LtvPair::new()?,
            modulation_bias, phase: 0.0,
            normalized: VecDeque::new(),
            spectra: std::array::from_fn(|_| VecDeque::new()),
            branches: std::array::from_fn(|_| VecDeque::new()),
            residual_samples: VecDeque::new(),
            delay, accepted: 0, published: 0, failure: None,
        })
    }

    /// Startup delay in hops.
    pub(crate) fn delay_rows(&self) -> usize { self.delay }

    pub(crate) fn process(&mut self, frame: &NeuralFrame, out: &mut Vec<f32>) -> Result<()> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let result = self.advance(frame, out);
        if let Err(error) = &result { self.failure = Some(error.clone()); }
        result
    }

    fn advance(&mut self, frame: &NeuralFrame, out: &mut Vec<f32>) -> Result<()> {
        if frame.raw.iter().chain(&frame.normalized).any(|v| !v.is_finite())
            || frame.raw[1] <= 0.0 || !frame.rd.is_finite()
            || !(0.1..=3.0).contains(&frame.rd)
            || frame.modulation.iter().any(|v| !v.is_finite())
            || frame.spectra.iter().any(|row| row.len() != 512 || row.iter().any(|v| !v.is_finite()))
        {
            return Err("Invalid legacy vocoder acoustic frame".into());
        }
        self.accepted = self.accepted.checked_add(1).ok_or("Legacy signal frame count overflow")?;
        self.normalized.push_back(frame.normalized);
        for (queue, row) in self.spectra.iter_mut().zip(&frame.spectra) { queue.push_back(row.clone()); }

        let mut raw = frame.raw;
        raw[2] = frame.rd;
        self.phase += f64::from(0.005_f32 * raw[1]);
        let phase = self.phase as f32;
        if !phase.is_finite() { return Err("Legacy source phase overflow".into()); }
        let mut source = Vec::with_capacity(HOP);
        let mut harmonic = Vec::with_capacity(HOP);
        self.source.process(&raw, &mut source)?;
        self.harmonic.process(HarmonicRecord {
            scalar0: raw[1], scalar4: phase, projection10: &frame.modulation,
        }, &mut harmonic)?;
        if source.len() != harmonic.len() || !matches!(source.len(), 0 | HOP) {
            return Err("Legacy source and harmonic publication are misaligned".into());
        }
        if !source.is_empty() {
            let condition = self.normalized.pop_front().ok_or("Legacy source condition queue underflow")?;
            let mut residual = Vec::new();
            self.residual.process(&source, &condition, &mut residual)?;
            self.residual_samples.extend(residual);

            let spectral0 = self.spectra[0].pop_front().ok_or("Missing legacy harmonic spectrum")?;
            let spectral2 = self.spectra[2].pop_front().ok_or("Missing legacy modulated spectrum")?;
            let mut first = Vec::new();
            let mut third = Vec::new();
            let modulated: Vec<f32> = source.iter().zip(&harmonic)
                .map(|(sample, gain)| sample * (self.modulation_bias + gain)).collect();
            self.paired_filters.process_first(&source, &spectral0, &mut first)?;
            self.paired_filters.process_second(&modulated, &spectral2, &mut third)?;
            self.branches[0].extend(first);
            self.branches[2].extend(third);
        }

        while self.residual_samples.len() >= HOP {
            let spectral1 = self.spectra[1].pop_front().ok_or("Missing legacy residual spectrum")?;
            let signal: Vec<f32> = self.residual_samples.drain(..HOP).collect();
            let mut noise = Vec::new();
            self.noise_filter.process_first(&signal, &spectral1, &mut noise)?;
            self.branches[1].extend(noise);
        }
        let ready = self.branches.iter().map(VecDeque::len).min().unwrap_or(0);
        if ready % HOP != 0 { return Err("Legacy branch publication did not contain whole hops".into()); }
        out.try_reserve(ready).map_err(|e| e.to_string())?;
        for _ in 0..ready {
            let a = self.branches[0].pop_front().ok_or("Legacy first branch underflow")?;
            let b = self.branches[1].pop_front().ok_or("Legacy noise branch underflow")?;
            let c = self.branches[2].pop_front().ok_or("Legacy third branch underflow")?;
            let sample = (a + c) + b;
            if !sample.is_finite() { return Err("Legacy vocoder produced nonfinite PCM".into()); }
            out.push(sample);
        }
        self.published = self.published.checked_add(ready).ok_or("Legacy output count overflow")?;
        let predicted = self.accepted.saturating_sub(self.delay)
            .checked_mul(HOP).ok_or("Legacy predicted output count overflow")?;
        if self.published != predicted {
            return Err("Legacy vocoder publication count differs from its prepared delay".into());
        }
        Ok(())
    }
}
