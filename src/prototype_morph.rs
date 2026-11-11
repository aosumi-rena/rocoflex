use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use crate::{Result, prototype_dsp};

pub const OUTPUT_SAMPLE_RATE: u32 = 48_000;
pub const ENVELOPE_BINS: usize = 1_025;

#[derive(Clone, Debug)]
pub struct MorphAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Debug)]
pub struct MorphTarget {
    pub name: String,
    pub audio: MorphAudio,
    pub weight: f32,
}

#[derive(Clone, Debug)]
pub struct MorphOptions {

    pub strength: f32,
    pub pregain_db: f32,

    pub tone_db: f32,
    pub pitch_semitones: f32,

    pub formant_semitones: f32,
}

impl Default for MorphOptions {
    fn default() -> Self {
        Self { strength: 1.0, pregain_db: 0.0, tone_db: 0.0,
            pitch_semitones: 0.0, formant_semitones: 0.0 }
    }
}

#[derive(Clone, Debug)]
pub struct MorphProfile {
    pub name: String,
    pub log_envelope: Vec<f32>,
    pub weight: f32,
}

#[derive(Clone, Debug)]
pub struct TargetContribution {
    pub name: String,
    pub weight: f32,
}

#[derive(Clone, Debug)]
pub struct MorphDiagnostics {
    pub algorithm: &'static str,
    pub targets: Vec<TargetContribution>,
    pub input_sample_rate: u32,
    pub input_samples: usize,
    pub output_samples: usize,
    pub input_rms: f32,
    pub output_rms: f32,
    pub output_peak: f32,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug)]
pub struct MorphResult {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub diagnostics: MorphDiagnostics,
}

fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) { Err("Morph render cancelled".into()) } else { Ok(()) }
}

fn validate_audio(audio: &MorphAudio, label: &str) -> Result<()> {
    if !(8_000..=384_000).contains(&audio.sample_rate) {
        return Err(format!("{label}: sample rate must be between 8000 and 384000 Hz"));
    }
    if audio.samples.is_empty() { return Err(format!("{label}: audio is empty")); }
    if audio.samples.iter().any(|sample| !sample.is_finite()) {
        return Err(format!("{label}: audio contains non-finite samples"));
    }
    Ok(())
}

fn validate_options(options: &MorphOptions) -> Result<()> {
    let values = [options.strength, options.pregain_db, options.tone_db,
        options.pitch_semitones, options.formant_semitones];
    if values.iter().any(|value| !value.is_finite()) { return Err("Morph controls must be finite".into()); }
    if !(0.0..=1.0).contains(&options.strength) { return Err("Morph strength must be between 0 and 1".into()); }
    if !(-36.0..=24.0).contains(&options.pregain_db) { return Err("Pregain must be between -36 and +24 dB".into()); }
    if !(-12.0..=12.0).contains(&options.tone_db) { return Err("Tone must be between -12 and +12 dB".into()); }
    if !(-12.0..=12.0).contains(&options.pitch_semitones) {
        return Err("Pitch shift must be between -12 and +12 semitones".into());
    }
    if !(-24.0..=24.0).contains(&options.formant_semitones) {
        return Err("Formant shift must be between -24 and +24 semitones".into());
    }
    Ok(())
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    (samples.iter().map(|&sample| (sample as f64).powi(2)).sum::<f64>()
        / samples.len() as f64).sqrt() as f32
}

fn center_envelope(envelope: &mut [f32]) {

    let range = &envelope[4..342];
    let mean = range.iter().map(|&value| value as f64).sum::<f64>() / range.len() as f64;
    for value in envelope { *value -= mean as f32; }
}

pub fn reference_excerpt(audio: &MorphAudio, position: f32, window_seconds: f32) -> Result<MorphAudio> {
    validate_audio(audio, "Reference")?;
    if !position.is_finite() || !(0.0..=1.0).contains(&position)
        || !window_seconds.is_finite() || window_seconds <= 0.0 {
        return Err("Reference position must be 0..1 and excerpt duration must be positive".into());
    }
    let count = ((window_seconds as f64 * audio.sample_rate as f64).round()
        .max(1.0).min(audio.samples.len() as f64)) as usize;
    let center = position as f64 * audio.samples.len().saturating_sub(1) as f64;
    let start = ((center - count as f64 * 0.5).round().max(0.0) as usize)
        .min(audio.samples.len() - count);
    Ok(MorphAudio { samples: audio.samples[start..start + count].to_vec(), sample_rate: audio.sample_rate })
}

pub fn prepare_targets(targets: &[MorphTarget], cancel: &AtomicBool,
    mut progress: impl FnMut(f32)) -> Result<Vec<MorphProfile>>
{
    if targets.is_empty() { return Err("Choose at least one target reference recording".into()); }
    if targets.iter().any(|target| !target.weight.is_finite() || target.weight < 0.0) {
        return Err("Target weights must be finite and nonnegative".into());
    }
    let active = targets.iter().filter(|target| target.weight > 0.0).count();
    if active == 0 { return Err("At least one target must have positive weight".into()); }
    let mut profiles = Vec::with_capacity(active);
    for target in targets.iter().filter(|target| target.weight > 0.0) {
        cancelled(cancel)?;
        validate_audio(&target.audio, &target.name)?;
        if rms(&target.audio.samples) < 1.0e-7 { return Err(format!("{}: reference is silent", target.name)); }
        let samples = prototype_dsp::resample_sinc(&target.audio.samples,
            target.audio.sample_rate, OUTPUT_SAMPLE_RATE)?;
        cancelled(cancel)?;
        let mut envelope = prototype_dsp::spectral_envelope(&samples, OUTPUT_SAMPLE_RATE)?;
        if envelope.len() != ENVELOPE_BINS || envelope.iter().any(|value| !value.is_finite()) {
            return Err(format!("{}: invalid reference spectral profile", target.name));
        }
        center_envelope(&mut envelope);
        profiles.push(MorphProfile { name: target.name.clone(), log_envelope: envelope, weight: target.weight });
        progress(profiles.len() as f32 / active as f32);
    }
    Ok(profiles)
}

pub fn render(source: &MorphAudio, targets: &[MorphTarget], options: &MorphOptions,
    cancel: &AtomicBool, mut progress: impl FnMut(f32)) -> Result<MorphResult>
{
    let started = Instant::now();
    validate_audio(source, "Source")?;
    validate_options(options)?;
    cancelled(cancel)?;
    progress(0.0);
    let profiles = prepare_targets(targets, cancel, |value| progress(value * 0.25))?;
    let mut result = render_with_profiles(source, &profiles, options, cancel,
        |value| progress(0.25 + value * 0.75))?;
    result.diagnostics.elapsed_ms = started.elapsed().as_millis();
    Ok(result)
}

pub fn render_with_profiles(source: &MorphAudio, targets: &[MorphProfile], options: &MorphOptions,
    cancel: &AtomicBool, mut progress: impl FnMut(f32)) -> Result<MorphResult>
{
    let started = Instant::now();
    validate_audio(source, "Source")?;
    validate_options(options)?;
    cancelled(cancel)?;
    if targets.is_empty() { return Err("Choose at least one target reference profile".into()); }
    let mut weight_sum = 0.0_f64;
    for target in targets {
        if !target.weight.is_finite() || target.weight < 0.0
            || target.log_envelope.len() != ENVELOPE_BINS
            || target.log_envelope.iter().any(|value| !value.is_finite()) {
            return Err(format!("{}: invalid target profile or weight", target.name));
        }
        weight_sum += target.weight as f64;
    }
    if weight_sum <= 0.0 { return Err("At least one target must have positive weight".into()); }
    let mut target_envelope = vec![0.0_f32; ENVELOPE_BINS];
    let mut contributions = Vec::new();
    for target in targets.iter().filter(|target| target.weight > 0.0) {
        let weight = (target.weight as f64 / weight_sum) as f32;
        for (mixed, &value) in target_envelope.iter_mut().zip(&target.log_envelope) { *mixed += weight * value; }
        contributions.push(TargetContribution { name: target.name.clone(), weight });
    }
    center_envelope(&mut target_envelope);
    progress(0.02);
    let mut input = prototype_dsp::resample_sinc(&source.samples, source.sample_rate, OUTPUT_SAMPLE_RATE)?;
    cancelled(cancel)?;
    let input_rms = rms(&input);
    let pregain = 10.0_f32.powf(options.pregain_db / 20.0);
    for sample in &mut input { *sample *= pregain; }
    progress(0.08);
    let mut source_envelope = prototype_dsp::spectral_envelope(&input, OUTPUT_SAMPLE_RATE)?;
    if source_envelope.len() != ENVELOPE_BINS { return Err("Invalid source spectral profile".into()); }
    center_envelope(&mut source_envelope);
    cancelled(cancel)?;
    progress(0.16);
    let mut samples = prototype_dsp::morph_spectral_with_progress(&input, OUTPUT_SAMPLE_RATE,
        &source_envelope, &target_envelope, prototype_dsp::SpectralMorphOptions {
            strength: options.strength, tone_db: options.tone_db,
            formant_semitones: options.formant_semitones, pitch_semitones: options.pitch_semitones,
        }, cancel, |value| progress(0.16 + value * 0.80))?;
    cancelled(cancel)?;
    prototype_dsp::finish_audio(&mut samples, OUTPUT_SAMPLE_RATE, 0.97);
    let output_rms = rms(&samples);
    let output_peak = samples.iter().map(|value| value.abs()).fold(0.0_f32, f32::max);
    let diagnostics = MorphDiagnostics {
        algorithm: "Reference spectral-envelope conversion (STFT)", targets: contributions,
        input_sample_rate: source.sample_rate, input_samples: source.samples.len(),
        output_samples: samples.len(), input_rms, output_rms, output_peak,
        elapsed_ms: started.elapsed().as_millis(),
    };
    progress(1.0);
    Ok(MorphResult { samples, sample_rate: OUTPUT_SAMPLE_RATE, diagnostics })
}
