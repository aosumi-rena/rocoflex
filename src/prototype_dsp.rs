use std::{f64::consts::PI, sync::atomic::{AtomicBool, Ordering}};

pub const FFT_SIZE: usize = 2048;
pub const ENVELOPE_BINS: usize = FFT_SIZE / 2 + 1;
const HOP: usize = FFT_SIZE / 4;

#[derive(Clone, Copy, Debug)]
pub struct SpectralMorphOptions {
    pub strength: f32,
    pub tone_db: f32,
    pub formant_semitones: f32,
    pub pitch_semitones: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DspSummary {
    pub non_finite_replaced: usize,
    pub peak_before: f32,
    pub peak_after: f32,
    pub minimum_gain: f32,
}

pub fn hann_window(size: usize) -> Vec<f32> {
    if size == 0 { return Vec::new(); }
    if size == 1 { return vec![1.0]; }
    (0..size).map(|i| (0.5 - 0.5 * (2.0 * PI * i as f64 / size as f64).cos()) as f32).collect()
}

pub fn resample_sinc(input: &[f32], source_rate: u32, target_rate: u32) -> Result<Vec<f32>, String> {
    if source_rate == 0 || target_rate == 0 {
        return Err("Sample rates must be positive".into());
    }
    if input.is_empty() { return Ok(Vec::new()); }
    if source_rate == target_rate {
        return Ok(input.iter().map(|&v| finite(v)).collect());
    }
    let ratio = target_rate as f64 / source_rate as f64;
    let length = (input.len() as f64 * ratio).round().max(1.0);
    if length > (isize::MAX as usize / std::mem::size_of::<f32>()) as f64 {
        return Err("Resampled audio is too long".into());
    }
    resample_ratio(input, ratio, length as usize, None, |_| {})
}

fn resample_ratio(input: &[f32], ratio: f64, length: usize,
    cancel: Option<&AtomicBool>, mut progress: impl FnMut(f32)) -> Result<Vec<f32>, String>
{
    let cutoff = ratio.min(1.0) * 0.94;
    let radius = 24.0 / cutoff;

    if radius > 16_384.0 { return Err("Sample-rate ratio is too extreme".into()); }
    let mut output = Vec::new();
    output.try_reserve_exact(length).map_err(|e| format!("Cannot allocate resampled audio: {e}"))?;
    for i in 0..length {
        if i % 512 == 0 {
            if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) { return Err("Render cancelled".into()); }
            progress(i as f32 / length.max(1) as f32);
        }
        let position = (i as f64 + 0.5) / ratio - 0.5;
        let first = (position - radius).ceil() as i64;
        let last = (position + radius).floor() as i64;
        let mut sum = 0.0;
        let mut weight_sum = 0.0;
        for tap in first..=last {
            let distance = position - tap as f64;
            let phase = PI * cutoff * distance;
            let sinc = if phase.abs() < 1e-12 { 1.0 } else { phase.sin() / phase };
            let window = 0.5 + 0.5 * (PI * distance / radius).cos();
            let weight = cutoff * sinc * window;
            let index = tap.clamp(0, input.len() as i64 - 1) as usize;
            sum += finite(input[index]) as f64 * weight;
            weight_sum += weight;
        }
        output.push(to_finite_f32(if weight_sum.abs() > 1e-12 { sum / weight_sum } else { 0.0 }));
    }
    progress(1.0);
    Ok(output)
}

pub fn spectral_envelope(input: &[f32], sample_rate: u32) -> Result<Vec<f32>, String> {
    validate_audio(input, sample_rate)?;
    if input.is_empty() { return Err("Cannot analyze empty audio".into()); }
    let global_rms = rms(input);
    if global_rms < 1e-7 { return Err("Reference audio is silent or too quiet".into()); }
    let fft = FftPlan::new();
    let window = hann_window(FFT_SIZE);
    let mut frame = vec![Complex::default(); FFT_SIZE];
    let mut profile = vec![0.0_f64; ENVELOPE_BINS];
    let mut weight_sum = 0.0;
    let mut start = -(FFT_SIZE as i64 / 2);
    while start < input.len() as i64 {
        let level = load_frame(input, start, &window, &mut frame);
        if level > global_rms * 0.08 {
            fft.transform(&mut frame, false);
            let weight = (level / global_rms).sqrt().clamp(0.1, 3.0);
            for k in 0..ENVELOPE_BINS {
                let magnitude = frame[k].magnitude() / FFT_SIZE as f64;
                profile[k] += magnitude.max(1e-8).ln() * weight;
            }
            weight_sum += weight;
        }
        start += HOP as i64;
    }
    if weight_sum <= 0.0 { return Err("No active audio frames in reference".into()); }
    for value in &mut profile { *value /= weight_sum; }

    let radius = (300.0 * FFT_SIZE as f64 / sample_rate as f64).round().clamp(3.0, 48.0) as usize;
    for _ in 0..3 { smooth(&mut profile, radius); }
    let low = ((200.0 * FFT_SIZE as f64 / sample_rate as f64) as usize).min(ENVELOPE_BINS - 2);
    let high = ((7000.0 * FFT_SIZE as f64 / sample_rate as f64) as usize)
        .clamp(low + 1, ENVELOPE_BINS - 1);
    let centre = profile[low..high].iter().sum::<f64>() / (high - low) as f64;
    Ok(profile.into_iter().map(|v| (v - centre) as f32).collect())
}

pub fn morph_spectral(input: &[f32], sample_rate: u32, source_envelope: &[f32],
    target_envelope: &[f32], options: SpectralMorphOptions) -> Result<Vec<f32>, String>
{
    morph_spectral_with_progress(input, sample_rate, source_envelope, target_envelope,
        options, &AtomicBool::new(false), |_| {})
}

pub fn morph_spectral_with_progress(input: &[f32], sample_rate: u32,
    source_envelope: &[f32], target_envelope: &[f32], options: SpectralMorphOptions,
    cancel: &AtomicBool, mut progress: impl FnMut(f32)) -> Result<Vec<f32>, String>
{
    validate_audio(input, sample_rate)?;
    if source_envelope.len() != ENVELOPE_BINS || target_envelope.len() != ENVELOPE_BINS
        || source_envelope.iter().chain(target_envelope).any(|v| !v.is_finite()) {
        return Err(format!("Spectral profiles must contain {ENVELOPE_BINS} finite log-magnitude bins"));
    }
    if [options.strength, options.tone_db, options.formant_semitones, options.pitch_semitones]
        .iter().any(|v| !v.is_finite()) { return Err("Morph controls must be finite".into()); }
    if !(0.0..=1.0).contains(&options.strength) || options.tone_db.abs() > 12.0
        || options.formant_semitones.abs() > 24.0 || options.pitch_semitones.abs() > 12.0 {
        return Err("Morph strength must be 0–1, tone within ±12 dB, formants within ±24 and pitch within ±12 semitones".into());
    }
    if cancel.load(Ordering::Relaxed) { return Err("Render cancelled".into()); }
    if input.is_empty() { progress(1.0); return Ok(Vec::new()); }
    let strength = options.strength as f64;
    let formant = 2.0_f64.powf(options.formant_semitones as f64 / 12.0);
    let pitch = 2.0_f64.powf(options.pitch_semitones as f64 / 12.0);
    let pitch_active = options.pitch_semitones.abs() > 0.0001;
    let tone = options.tone_db as f64;
    if strength == 0.0 && formant == 1.0 && tone == 0.0 && !pitch_active {
        progress(1.0);
        return Ok(input.to_vec());
    }
    let mut log_gain = vec![0.0; ENVELOPE_BINS];
    let max_log_gain = 18.0 * std::f64::consts::LN_10 / 20.0;
    for k in 0..ENVELOPE_BINS {
        let position = k as f64 / formant;
        let source = interpolate(source_envelope, position);
        let target = interpolate(target_envelope, position);
        let desired = source + strength * (target - source);
        let frequency = k as f64 * sample_rate as f64 / FFT_SIZE as f64;
        let tilt_db = tone * (frequency.max(80.0) / 1000.0).log2().clamp(-2.0, 2.0) / 2.0;
        let actual_source = if pitch_active { interpolate(source_envelope, k as f64 / pitch) }
            else { source_envelope[k] as f64 };
        log_gain[k] = (desired - actual_source + tilt_db * std::f64::consts::LN_10 / 20.0)
            .clamp(-max_log_gain, max_log_gain);
    }
    smooth(&mut log_gain, 3);
    let pitched = if pitch_active {
        Some(pitch_shift(input, pitch, cancel, |value| progress(value * 0.55))?)
    } else { None };
    let processing_input = pitched.as_deref().unwrap_or(input);
    let progress_base = if pitch_active { 0.55 } else { 0.0 };
    let fft = FftPlan::new();
    let window = hann_window(FFT_SIZE);
    let mut frame = vec![Complex::default(); FFT_SIZE];
    let mut output = vec![0.0_f64; input.len()];
    let mut weights = vec![0.0_f64; input.len()];
    let source_rms = rms(input);
    let mut activity = 0.0_f64;
    let mut start = -(FFT_SIZE as i64 / 2);
    let frame_count = (input.len() + FFT_SIZE / 2).div_ceil(HOP);
    let mut ordinal = 0;
    progress(progress_base);
    while start < input.len() as i64 {
        if cancel.load(Ordering::Relaxed) { return Err("Render cancelled".into()); }
        let frame_rms = load_frame(processing_input, start, &window, &mut frame);
        let reached_activity = (frame_rms / (source_rms * 0.12 + 1e-6)).clamp(0.0, 1.0);
        activity = 0.5 * activity + 0.5 * reached_activity;
        fft.transform(&mut frame, false);
        for k in 0..ENVELOPE_BINS {
            let gain = (log_gain[k] * activity).exp();
            frame[k].scale(gain);
            if k > 0 && k < FFT_SIZE / 2 { frame[FFT_SIZE - k].scale(gain); }
        }
        fft.transform(&mut frame, true);
        for i in 0..FFT_SIZE {
            let index = start + i as i64;
            if index >= 0 && (index as usize) < input.len() {
                let index = index as usize;
                let w = window[i] as f64;
                output[index] += frame[i].re * w;
                weights[index] += w * w;
            }
        }
        ordinal += 1;
        progress(progress_base + (1.0 - progress_base) * (ordinal as f32 / frame_count.max(1) as f32).min(1.0));
        start += HOP as i64;
    }
    for (sample, weight) in output.iter_mut().zip(weights) {
        *sample = if weight > 1e-12 { *sample / weight } else { 0.0 };
    }

    let wet_rms = (output.iter().map(|v| v * v).sum::<f64>() / output.len() as f64).sqrt();
    let loudness_gain = if wet_rms > 1e-12 { (source_rms / wet_rms).clamp(0.25, 4.0) } else { 1.0 };
    let result = output.into_iter().map(|v| to_finite_f32(v * loudness_gain)).collect();
    progress(1.0);
    Ok(result)
}

fn pitch_shift(input: &[f32], factor: f64, cancel: &AtomicBool,
    mut progress: impl FnMut(f32)) -> Result<Vec<f32>, String>
{
    let stretched_len = (input.len() as f64 * factor).round().max(1.0);
    if stretched_len > (isize::MAX as usize / std::mem::size_of::<f64>()) as f64 {
        return Err("Pitch-shift workspace is too long".into());
    }
    let stretched_len = stretched_len as usize;
    let fft = FftPlan::new();
    let window = hann_window(FFT_SIZE);
    let hop = FFT_SIZE / 8;
    let mut frame = vec![Complex::default(); FFT_SIZE];
    let mut output = vec![0.0_f64; stretched_len];
    let mut weights = vec![0.0_f64; stretched_len];
    let mut previous_phase = vec![0.0; ENVELOPE_BINS];
    let mut previous_magnitude = vec![0.0; ENVELOPE_BINS];
    let mut synthesis_phase = vec![0.0; ENVELOPE_BINS];
    let mut phase = vec![0.0; ENVELOPE_BINS];
    let mut magnitude = vec![0.0; ENVELOPE_BINS];
    let mut peak_owner = vec![0; ENVELOPE_BINS];
    let mut peaks = Vec::with_capacity(ENVELOPE_BINS);
    let mut previous_start = 0_i64;
    let mut first = true;
    let mut start = -(FFT_SIZE as i64 / 2);
    let frame_count = (input.len() + FFT_SIZE / 2).div_ceil(hop);
    let mut ordinal = 0;
    while start < input.len() as i64 {
        if cancel.load(Ordering::Relaxed) { return Err("Render cancelled".into()); }
        load_frame(input, start, &window, &mut frame);
        fft.transform(&mut frame, false);
        let synthesis_start = ((start + FFT_SIZE as i64 / 2) as f64 * factor).round() as i64 - FFT_SIZE as i64 / 2;
        let synthesis_hop = (synthesis_start - previous_start) as f64;
        let mut flux = 0.0;
        let mut magnitude_sum = 0.0;
        for k in 0..ENVELOPE_BINS {
            phase[k] = frame[k].im.atan2(frame[k].re);
            magnitude[k] = frame[k].magnitude();
            flux += (magnitude[k] - previous_magnitude[k]).max(0.0);
            magnitude_sum += magnitude[k];
        }
        let transient = first || flux > magnitude_sum * 0.55;
        for k in 0..ENVELOPE_BINS {
            if transient {
                synthesis_phase[k] = phase[k];
            } else {
                let omega = 2.0 * PI * k as f64 / FFT_SIZE as f64;
                let residual = wrap_phase(phase[k] - previous_phase[k] - omega * hop as f64);
                synthesis_phase[k] += (omega + residual / hop as f64) * synthesis_hop;
                synthesis_phase[k] = wrap_phase(synthesis_phase[k]);
            }
        }
        peaks.clear();
        for k in 1..ENVELOPE_BINS - 1 {
            if magnitude[k] >= magnitude[k - 1] && magnitude[k] > magnitude[k + 1] { peaks.push(k); }
        }
        if peaks.is_empty() { peaks.push(0); }
        let mut nearest = 0;
        for (k, owner) in peak_owner.iter_mut().enumerate() {
            while nearest + 1 < peaks.len() && k * 2 > peaks[nearest] + peaks[nearest + 1] { nearest += 1; }
            *owner = peaks[nearest];
        }
        for k in 0..ENVELOPE_BINS {
            let owner = peak_owner[k];
            let locked_phase = if transient { phase[k] }
                else { synthesis_phase[owner] + wrap_phase(phase[k] - phase[owner]) };
            let bin = Complex { re: magnitude[k] * locked_phase.cos(), im: magnitude[k] * locked_phase.sin() };
            frame[k] = bin;
            if k > 0 && k < FFT_SIZE / 2 { frame[FFT_SIZE - k] = Complex { re: bin.re, im: -bin.im }; }
            previous_phase[k] = phase[k];
            previous_magnitude[k] = magnitude[k];
        }

        frame[0].im = 0.0;
        frame[FFT_SIZE / 2].im = 0.0;
        fft.transform(&mut frame, true);
        for i in 0..FFT_SIZE {
            let index = synthesis_start + i as i64;
            if index >= 0 && (index as usize) < stretched_len {
                let index = index as usize;
                let w = window[i] as f64;
                output[index] += frame[i].re * w;
                weights[index] += w * w;
            }
        }
        previous_start = synthesis_start;
        first = false;
        start += hop as i64;
        ordinal += 1;
        progress(0.65 * (ordinal as f32 / frame_count.max(1) as f32).min(1.0));
    }
    let stretched: Vec<f32> = output.into_iter().zip(weights).map(|(value, weight)|
        to_finite_f32(if weight > 1e-12 { value / weight } else { 0.0 })).collect();
    resample_ratio(&stretched, input.len() as f64 / stretched.len() as f64,
        input.len(), Some(cancel), |value| progress(0.65 + value * 0.35))
}

fn wrap_phase(value: f64) -> f64 { (value + PI).rem_euclid(2.0 * PI) - PI }

pub fn finish_audio(samples: &mut [f32], sample_rate: u32, ceiling: f32) -> DspSummary {
    let mut summary = DspSummary { minimum_gain: 1.0, ..DspSummary::default() };
    if samples.is_empty() { return summary; }
    let rate = sample_rate.max(1) as f64;
    let pole = (-2.0 * PI * 15.0 / rate).exp();
    let mut previous_input = 0.0;
    let mut previous_output = 0.0;
    let fade = ((rate * 0.005).round() as usize).min(samples.len() / 2);
    let length = samples.len();
    let mut cleaned = Vec::with_capacity(length);
    let mut peak = 0.0_f64;
    for (i, sample) in samples.iter().enumerate() {
        let input = if sample.is_finite() { *sample as f64 } else { summary.non_finite_replaced += 1; 0.0 };
        summary.peak_before = summary.peak_before.max(input.abs().min(f32::MAX as f64) as f32);
        let highpass = input - previous_input + pole * previous_output;
        previous_input = input;
        previous_output = highpass;
        let mut gain = 1.0;
        if fade > 0 {
            let edge = i.min(length - 1 - i);
            if edge < fade { gain = 0.5 - 0.5 * (PI * edge as f64 / fade as f64).cos(); }
        }
        let value = highpass * gain;
        peak = peak.max(value.abs());
        cleaned.push(value);
    }
    let ceiling = if ceiling.is_finite() { ceiling.clamp(0.01, 1.0) as f64 } else { 0.95 };
    let gain = if peak > ceiling { ceiling / peak } else { 1.0 };
    summary.minimum_gain = gain as f32;
    for (sample, clean) in samples.iter_mut().zip(cleaned) {
        *sample = to_finite_f32(clean * gain);
        summary.peak_after = summary.peak_after.max(sample.abs());
    }
    summary
}

fn validate_audio(input: &[f32], sample_rate: u32) -> Result<(), String> {
    if !(8000..=384000).contains(&sample_rate) { return Err("DSP sample rate must be between 8 and 384 kHz".into()); }
    if input.iter().any(|v| !v.is_finite()) { return Err("Audio contains nonfinite samples".into()); }
    Ok(())
}
fn finite(v: f32) -> f32 { if v.is_finite() { v } else { 0.0 } }
fn to_finite_f32(v: f64) -> f32 {
    if v.is_finite() { v.clamp(-(f32::MAX as f64), f32::MAX as f64) as f32 } else { 0.0 }
}
fn rms(input: &[f32]) -> f64 {
    if input.is_empty() { 0.0 } else { (input.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / input.len() as f64).sqrt() }
}
fn interpolate(input: &[f32], position: f64) -> f64 {
    let position = position.clamp(0.0, (input.len() - 1) as f64);
    let left = position as usize;
    let right = (left + 1).min(input.len() - 1);
    let fraction = position - left as f64;
    input[left] as f64 * (1.0 - fraction) + input[right] as f64 * fraction
}
fn smooth(values: &mut [f64], radius: usize) {
    let mut prefix = Vec::with_capacity(values.len() + 1);
    prefix.push(0.0);
    for &value in values.iter() { prefix.push(prefix.last().copied().unwrap() + value); }
    for (i, value) in values.iter_mut().enumerate() {
        let first = i.saturating_sub(radius);
        let last = (i + radius + 1).min(prefix.len() - 1);
        *value = (prefix[last] - prefix[first]) / (last - first) as f64;
    }
}
fn load_frame(input: &[f32], start: i64, window: &[f32], frame: &mut [Complex]) -> f64 {
    let mut sum = 0.0;
    for i in 0..FFT_SIZE {
        let index = start + i as i64;
        let value = if index >= 0 && (index as usize) < input.len() { input[index as usize] as f64 } else { 0.0 };
        sum += value * value;
        frame[i] = Complex { re: value * window[i] as f64, im: 0.0 };
    }
    (sum / FFT_SIZE as f64).sqrt()
}

#[derive(Clone, Copy, Default)]
struct Complex { re: f64, im: f64 }
impl Complex {
    fn magnitude(self) -> f64 { self.re.hypot(self.im) }
    fn scale(&mut self, gain: f64) { self.re *= gain; self.im *= gain; }
}
struct FftPlan { reverse: Vec<usize>, roots: Vec<Complex> }
impl FftPlan {
    fn new() -> Self {
        let bits = FFT_SIZE.trailing_zeros();
        let reverse = (0..FFT_SIZE).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect();
        let roots = (0..FFT_SIZE / 2).map(|k| {
            let angle = -2.0 * PI * k as f64 / FFT_SIZE as f64;
            Complex { re: angle.cos(), im: angle.sin() }
        }).collect();
        Self { reverse, roots }
    }
    fn transform(&self, values: &mut [Complex], inverse: bool) {
        for i in 0..FFT_SIZE {
            if self.reverse[i] > i { values.swap(i, self.reverse[i]); }
        }
        let mut width = 2;
        while width <= FFT_SIZE {
            let half = width / 2;
            let step = FFT_SIZE / width;
            for start in (0..FFT_SIZE).step_by(width) {
                for j in 0..half {
                    let root = self.roots[j * step];
                    let im = if inverse { -root.im } else { root.im };
                    let b = values[start + j + half];
                    let product = Complex { re: b.re * root.re - b.im * im, im: b.re * im + b.im * root.re };
                    let a = values[start + j];
                    values[start + j] = Complex { re: a.re + product.re, im: a.im + product.im };
                    values[start + j + half] = Complex { re: a.re - product.re, im: a.im - product.im };
                }
            }
            width *= 2;
        }
        if inverse { for value in values { value.scale(1.0 / FFT_SIZE as f64); } }
    }
}
