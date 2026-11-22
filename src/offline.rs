use crate::{Result, prototype_controls::Parameters};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[path = "offline_io.rs"]
pub(crate) mod io;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub preset: PathBuf,
    pub input: Option<PathBuf>,
    pub input_frames: Option<u64>,
    pub progress: Option<PathBuf>,
    pub output: PathBuf,
    pub pregain_db: Option<f32>,
    pub pitch_semitones: Option<f32>,
}

#[derive(Serialize)]
pub struct Response {
    pub version: u32,
    pub sample_rate: u32,
    pub channels: u32,
    pub input_frames: u64,
    pub output_frames: u64,
    pub quality: &'static str,
    pub pregain_db: f32,
    pub pitch_semitones: f32,
    pub appended_zero_frames: u64,
    pub discarded_padding_frames: u64,
    pub trimmed_latency_frames: u64,
}

pub fn integration_info() -> serde_json::Value {
    serde_json::json!({"version":2,"offline_render":cfg!(all(windows,feature="desktop")),
        "editor":cfg!(all(windows,feature="desktop")),
        "streaming":cfg!(all(windows,feature="desktop")), "render_check":cfg!(all(windows,feature="desktop")),
        "sample_rate":48000,"channels":1,"pcm":"f32le","max_frames":io::MAX_FRAMES})
}

pub fn run_request(path: &Path) -> Result<Response> {
    let request: Request = serde_json::from_value(io::read_json(path, 64 * 1024)?)
        .map_err(|error| format!("Invalid offline request: {error}"))?;
    render(request)
}

pub fn check_models() -> Result<()> {
    #[cfg(all(windows, feature = "desktop"))]
    {
        let (_, synthesis, legacy) = model_paths()?;
        if legacy {
            let bytes = crate::model::read_bytes(&synthesis, crate::morph_processor::LEGACY_VOCODER_MAX_BYTES)?;
            crate::morph_processor::inspect_legacy_vocoder(&bytes)?;
        }
        Ok(())
    }
    #[cfg(not(all(windows, feature = "desktop")))]
    { Err("Offline synthesis requires the Windows desktop provider build.".into()) }
}

#[cfg(all(windows, feature = "desktop"))]
fn model_paths() -> Result<(PathBuf, PathBuf, bool)> {
    let (analysis, synthesis, legacy) = crate::desktop::offline_model_selection()?;
    let required = |path: &Path, label: &str| {
        let invalid = || format!("Choose a valid {label} file in Rocoflex Settings.");
        let resolved = io::model_path(path).map_err(|error| format!("{} {error}", invalid()))?;
        if !resolved.metadata().is_ok_and(|meta| meta.is_file() && meta.len() > 0) {
            return Err(invalid());
        }
        Ok(resolved)
    };
    let name = if legacy { "custom vocoder" } else { "userdata" };
    Ok((required(&analysis, "analysis model")?, required(&synthesis, name)?, legacy))
}

#[cfg(all(windows, feature = "desktop"))]
pub fn render(request: Request) -> Result<Response> {
    if request.version != 2 { return Err("Unsupported offline protocol version.".into()); }
    let parameters = parameters(&request)?;
    let (analysis, synthesis, legacy) = model_paths()?;
    let _fp = crate::morph_processor::offline::Environment::enter();
    let preset = crate::preset::Preset::parse(io::read_json(&request.preset, 16 * 1024 * 1024)?)?;
    let snapshot = crate::desktop::offline_snapshot(preset, parameters)?
        .ok_or("The preset has no completed enabled canvas voice target.")?;
    let input = match (request.input.as_deref(), request.input_frames) {
        (Some(path), None) => io::Input::open(path)?,
        (None, Some(frames)) => io::Input::stream(frames)?,
        _ => return Err("Provide a PCM file or an exact frame count for stdin.".into()),
    };
    let output = io::Output::create(&request.output, request.progress.as_deref())?;
    crate::morph_processor::offline::render(&analysis, &synthesis, legacy, snapshot, input, output)
}

#[cfg(not(all(windows, feature = "desktop")))]
pub fn render(_request: Request) -> Result<Response> {
    Err("Offline synthesis requires the Windows desktop provider build.".into())
}

fn parameters(request: &Request) -> Result<Parameters> {
    let pregain_db = request.pregain_db.unwrap_or(0.0);
    let pitch_semitones = request.pitch_semitones.unwrap_or(0.0);
    if !pregain_db.is_finite() || !(-36.0..=36.0).contains(&pregain_db)
        || !pitch_semitones.is_finite() || !(-24.0..=24.0).contains(&pitch_semitones)
    {
        return Err("Offline pregain or pitch is outside the supported range.".into());
    }
    Ok(Parameters { pregain_db, pitch_semitones, tone_db: 0.0 })
}
