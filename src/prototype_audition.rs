use std::{fs, io::BufWriter, path::{Path, PathBuf}, sync::atomic::AtomicBool, time::Instant};
use serde::Serialize;
use crate::{Result, prototype_dsp::resample_sinc, prototype_morph::{self, MorphAudio, MorphOptions, MorphTarget}};

pub struct TargetFile { pub path: PathBuf, pub weight: f32 }

#[derive(Debug, Serialize)]
pub struct SignalMetrics {
    pub frames: usize,
    pub non_finite_samples: usize,
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub peak_dbfs: Option<f64>,
    pub rms_dbfs: Option<f64>,
    pub clipping_samples: usize,
}

#[derive(Debug, Serialize)]
pub struct AuditionReport {
    pub source: String,
    pub targets: Vec<serde_json::Value>,
    pub options: serde_json::Value,
    pub sample_rate: u32,
    pub duration_seconds: f64,
    pub render_seconds: f64,
    pub dry: SignalMetrics,
    pub wet: SignalMetrics,
    pub difference: SignalMetrics,
    pub changed_samples: usize,
    pub correlation: Option<f64>,
    pub wet_to_dry_rms: Option<f64>,
    pub dry_wav: PathBuf,
    pub wet_wav: PathBuf,
    pub difference_wav: PathBuf,
    pub metrics_json: PathBuf,
}

pub fn load_audio(path: &Path) -> Result<MorphAudio> {
    let metadata = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if metadata.len() > 256 * 1024 * 1024 { return Err("WAV exceeds the 256 MiB audition limit".into()); }
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoded = crate::wav48k::decode_supported(&bytes)
        .map_err(|e| format!("{}: WAV decode: {e:?}", path.display()))?;
    if decoded.channel0.is_empty() { return Err(format!("{}: audio is empty", path.display())); }
    if decoded.channel0.iter().any(|x| !x.is_finite()) {
        return Err(format!("{}: audio contains nonfinite samples", path.display()));
    }
    Ok(MorphAudio { samples: decoded.channel0, sample_rate: decoded.metadata.sample_rate })
}

pub fn write_audio(path: &Path, sample_rate: u32, samples: &[f32]) -> Result<()> {
    let file = fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut writer = BufWriter::new(file);
    crate::wave_output::write_mono_samples(&mut writer, sample_rate, samples).map_err(|e| {
        use crate::wave_output::Error;
        let reason = match e {
            Error::Io { bytes_written, cause } => format!("after {bytes_written} bytes: {cause}"),
            Error::SampleRate(rate) => format!("invalid sample rate {rate}"),
            Error::Hop => "invalid hop".into(),
            Error::Extent => "WAV exceeds RIFF size limits".into(),
            Error::RowWidth { row, expected, actual } => format!("row {row}: expected {expected}, got {actual}"),
        };
        format!("{}: {reason}", path.display())
    })?;
    Ok(())
}

pub fn signal_metrics(samples: &[f32]) -> SignalMetrics {
    let mut peak = 0.0_f64;
    let mut sum = 0.0;
    let mut squares = 0.0;
    let mut non_finite = 0;
    let mut clipping = 0;
    for &sample in samples {
        if !sample.is_finite() { non_finite += 1; continue; }
        let value = f64::from(sample);
        peak = peak.max(value.abs());
        sum += value;
        squares += value * value;
        if value.abs() >= 1.0 { clipping += 1; }
    }
    let count = samples.len().saturating_sub(non_finite);
    let rms = if count == 0 { 0.0 } else { (squares / count as f64).sqrt() };
    SignalMetrics { frames: samples.len(), non_finite_samples: non_finite, peak, rms,
        dc: if count == 0 { 0.0 } else { sum / count as f64 },
        peak_dbfs: (peak > 0.0).then(|| 20.0 * peak.log10()),
        rms_dbfs: (rms > 0.0).then(|| 20.0 * rms.log10()), clipping_samples: clipping }
}

pub fn render_files(source_path: &Path, targets: &[TargetFile], options: &MorphOptions,
    output_directory: &Path) -> Result<AuditionReport>
{
    if targets.is_empty() { return Err("at least one target reference WAV is required".into()); }
    let source = load_audio(source_path)?;
    let references = targets.iter().map(|target| {
        Ok(MorphTarget { name: target.path.display().to_string(),
            audio: load_audio(&target.path)?, weight: target.weight })
    }).collect::<Result<Vec<_>>>()?;
    let started = Instant::now();
    let rendered = prototype_morph::render(&source, &references, options, &AtomicBool::new(false), |_| {})?;
    let render_seconds = started.elapsed().as_secs_f64();
    let dry = resample_sinc(&source.samples, source.sample_rate, rendered.sample_rate)?;
    if dry.len() != rendered.samples.len() {
        return Err(format!("render duration changed: dry {} frames, wet {} frames", dry.len(), rendered.samples.len()));
    }
    let dry_metrics = signal_metrics(&dry);
    let wet_metrics = signal_metrics(&rendered.samples);
    if wet_metrics.non_finite_samples != 0 { return Err("render produced nonfinite samples".into()); }
    if dry_metrics.rms > 1.0e-8 && wet_metrics.rms == 0.0 { return Err("render is silent for a non-silent input".into()); }
    let difference: Vec<f32> = rendered.samples.iter().zip(&dry).map(|(&wet, &dry)| wet - dry).collect();
    let mut covariance = 0.0;
    let mut dry_variance = 0.0;
    let mut wet_variance = 0.0;
    for (&a, &b) in dry.iter().zip(&rendered.samples) {
        let a = f64::from(a) - dry_metrics.dc;
        let b = f64::from(b) - wet_metrics.dc;
        covariance += a * b;
        dry_variance += a * a;
        wet_variance += b * b;
    }
    let denominator = (dry_variance * wet_variance).sqrt();
    fs::create_dir_all(output_directory).map_err(|e| format!("{}: {e}", output_directory.display()))?;
    let directory = fs::canonicalize(output_directory).map_err(|e| e.to_string())?;
    let dry_wav = directory.join("dry.wav");
    let wet_wav = directory.join("wet.wav");
    let difference_wav = directory.join("difference.wav");
    let metrics_json = directory.join("metrics.json");

    for input in std::iter::once(source_path).chain(targets.iter().map(|t| t.path.as_path())) {
        let input = fs::canonicalize(input).map_err(|e| e.to_string())?;
        for output in [&dry_wav, &wet_wav, &difference_wav, &metrics_json] {
            if output.exists() && fs::canonicalize(output).map_err(|e| e.to_string())? == input {
                return Err(format!("audition output would overwrite input {}", input.display()));
            }
        }
    }
    let report = AuditionReport {
        source: source_path.display().to_string(),
        targets: targets.iter().map(|t| serde_json::json!({"path": t.path, "weight": t.weight})).collect(),
        options: serde_json::json!({"strength": options.strength, "pregain_db": options.pregain_db,
            "tone_db": options.tone_db, "pitch_semitones": options.pitch_semitones,
            "formant_semitones": options.formant_semitones}),
        sample_rate: rendered.sample_rate, duration_seconds: dry.len() as f64 / rendered.sample_rate as f64,
        render_seconds, changed_samples: difference.iter().filter(|&&x| x != 0.0).count(),
        correlation: (denominator > 0.0).then(|| (covariance / denominator).clamp(-1.0, 1.0)),
        wet_to_dry_rms: (dry_metrics.rms > 0.0).then(|| wet_metrics.rms / dry_metrics.rms),
        difference: signal_metrics(&difference), dry: dry_metrics, wet: wet_metrics,
        dry_wav, wet_wav, difference_wav, metrics_json,
    };
    write_audio(&report.dry_wav, report.sample_rate, &dry)?;
    write_audio(&report.wet_wav, report.sample_rate, &rendered.samples)?;
    write_audio(&report.difference_wav, report.sample_rate, &difference)?;
    fs::write(&report.metrics_json, serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| format!("{}: {e}", report.metrics_json.display()))?;
    Ok(report)
}
