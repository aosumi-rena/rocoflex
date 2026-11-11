use crate::{
    Result,
    pitch_selected_head::IndicatorAccumulation,
    preset::Preset,
    voice_generation::{GeneratedVoice, VoiceGenerator},
};
use std::{collections::VecDeque, ops::Range, path::{Path, PathBuf}, sync::Arc, time::SystemTime};

pub const MAX_REFERENCE_ANALYSIS_FRAMES: usize = 30 * 48_000;
const MAX_ANALYSIS_MODEL_BYTES: u64 = 128 * 1024 * 1024;

pub struct ReferenceAnalyzer {
    generator: VoiceGenerator,
    model_stamp: Option<FileStamp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileStamp {
    path: PathBuf,
    modified: SystemTime,
    bytes: u64,
}

impl FileStamp {
    fn read(path: &Path) -> Result<Self> {
        let path = path.canonicalize().map_err(|error| format!("{}: {error}", path.display()))?;
        let metadata = std::fs::metadata(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        if !metadata.is_file() { return Err(format!("{} is not a file.", path.display())); }
        let modified = metadata.modified().map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(Self { path, modified, bytes: metadata.len() })
    }

    fn is_current(&self) -> Result<bool> { Ok(Self::read(&self.path)? == *self) }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceAnalysisKey {
    audio: FileStamp,
    model: FileStamp,
    range: Range<usize>,
}

impl ReferenceAnalysisKey {
    pub fn from_paths(audio: &Path, model: &Path, range: Range<usize>) -> Result<Self> {
        let frames = range.end.checked_sub(range.start).ok_or("Invalid reference excerpt range.")?;
        if frames == 0 || frames > MAX_REFERENCE_ANALYSIS_FRAMES {
            return Err("Select a nonempty reference excerpt of at most 30 seconds at 48 kHz.".into());
        }
        Ok(Self { audio: FileStamp::read(audio)?, model: FileStamp::read(model)?, range })
    }

    pub fn audio_path(&self) -> &Path { &self.audio.path }
    pub fn model_path(&self) -> &Path { &self.model.path }
    pub fn excerpt_range(&self) -> Range<usize> { self.range.clone() }
    pub fn is_current(&self) -> Result<bool> { Ok(self.audio.is_current()? && self.model.is_current()?) }
}

pub struct CachedReferenceAnalysis {
    pub voice: GeneratedVoice,

    pub excerpt_range: Range<usize>,
    pub total_frames: usize,
}

pub struct ReferenceAnalysisCache {
    max_entries: usize,
    entries: VecDeque<(ReferenceAnalysisKey, Arc<CachedReferenceAnalysis>)>,
}

impl ReferenceAnalysisCache {

    pub fn new(max_entries: usize) -> Self {
        Self { max_entries: max_entries.max(1), entries: VecDeque::new() }
    }

    pub fn get(&self, key: &ReferenceAnalysisKey) -> Option<Arc<CachedReferenceAnalysis>> {
        self.entries.iter().find(|(stored, _)| stored == key).map(|(_, value)| Arc::clone(value))
    }

    pub fn analyze_once(
        &mut self,
        key: ReferenceAnalysisKey,
        analyzer: &ReferenceAnalyzer,
        samples: &[f32],
    ) -> Result<Arc<CachedReferenceAnalysis>> {
        if !key.is_current()? { return Err("Reference audio or model changed; start analysis again.".into()); }
        if let Some(cached) = self.get(&key) { return Ok(cached); }
        if analyzer.model_stamp.as_ref() != Some(&key.model) {
            return Err("Cached analysis requires the same model loaded with ReferenceAnalyzer::from_path.".into());
        }
        let excerpt = samples.get(key.range.clone()).ok_or("Reference excerpt exceeds prepared audio.")?;
        let voice = analyzer.analyze_samples48k(excerpt)?;
        if !key.is_current()? { return Err("Reference audio or model changed during analysis; result discarded.".into()); }
        let result = Arc::new(CachedReferenceAnalysis {
            voice, excerpt_range: key.range.clone(), total_frames: samples.len(),
        });

        self.entries.retain(|(stored, _)| !(stored.audio.path == key.audio.path
            && stored.model.path == key.model.path
            && (stored.audio != key.audio || stored.model != key.model)));
        while self.entries.len() >= self.max_entries { self.entries.pop_front(); }
        self.entries.push_back((key, Arc::clone(&result)));
        Ok(result)
    }
}

pub struct ReferenceVoice {
    pub name: String,

    pub samples: Vec<f32>,
    pub voice: GeneratedVoice,
}

#[derive(Clone, Copy, Debug)]
pub struct ReferenceMatch {
    pub index: usize,
    pub mean_squared_distance: f64,
}

impl ReferenceAnalyzer {

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        Ok(Self {
            generator: VoiceGenerator::from_dnni(
                model,
                IndicatorAccumulation::PerInputSaturatedI16,
            )?,
            model_stamp: None,
        })
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let stamp = FileStamp::read(path)?;
        let bytes = crate::model::read_bytes(&stamp.path, MAX_ANALYSIS_MODEL_BYTES)?;
        let mut analyzer = Self::from_dnni(&bytes)?;
        if !stamp.is_current()? { return Err("Analysis model changed while loading.".into()); }
        analyzer.model_stamp = Some(stamp);
        Ok(analyzer)
    }

    pub fn analyze_normalized48k(
        &self,
        name: impl Into<String>,
        samples: Vec<f32>,
    ) -> Result<ReferenceVoice> {
        let voice = self.analyze_samples48k(&samples)?;
        Ok(ReferenceVoice { name: name.into(), samples, voice })
    }

    pub fn analyze_samples48k(&self, samples: &[f32]) -> Result<GeneratedVoice> {
        if samples.is_empty() || samples.iter().any(|sample| !sample.is_finite()) {
            return Err("Reference audio must contain finite mono 48 kHz samples.".into());
        }
        let voice = self
            .generator
            .generate_final_normalized48k(samples)?
            .ok_or("Reference audio did not produce enough usable voice frames.")?;
        validate_embedding(&voice.data)?;
        Ok(voice)
    }
}

pub fn target_from_preset(preset: &Preset, source_index: usize) -> Result<[f32; 128]> {
    let source = preset.sources.get(source_index).ok_or("Target source index is outside the preset.")?;
    if !source.enabled {
        return Err("The selected target source is disabled.".into());
    }
    let target: [f32; 128] = source.data.as_slice().try_into()
        .map_err(|_| "The target source must contain 128 voice values.")?;
    validate_embedding(&target)?;
    Ok(target)
}

pub fn closest_reference(
    target: &[f32; 128],
    references: &[ReferenceVoice],
) -> Result<ReferenceMatch> {
    for (index, reference) in references.iter().enumerate() {
        if reference.samples.is_empty() || reference.samples.iter().any(|x| !x.is_finite()) {
            return Err(format!("Reference {index} has no usable audio."));
        }
    }
    closest_embedding(target, references.iter().map(|reference| &reference.voice.data))
}

pub fn closest_embedding<'a>(
    target: &[f32; 128],
    embeddings: impl IntoIterator<Item = &'a [f32; 128]>,
) -> Result<ReferenceMatch> {
    validate_embedding(target)?;
    let mut best: Option<ReferenceMatch> = None;
    for (index, embedding) in embeddings.into_iter().enumerate() {
        validate_embedding(embedding)?;
        let distance = target.iter().zip(embedding)
            .map(|(&a, &b)| {
                let difference = f64::from(a) - f64::from(b);
                difference * difference
            })
            .sum::<f64>() / 128.0;
        if best.is_none_or(|current| distance < current.mean_squared_distance) {
            best = Some(ReferenceMatch { index, mean_squared_distance: distance });
        }
    }
    best.ok_or_else(|| "Provide target reference audio; a preset embedding alone is not a waveform decoder.".into())
}

pub fn select_reference_for_preset(
    preset: &Preset,
    source_index: usize,
    references: &[ReferenceVoice],
) -> Result<ReferenceMatch> {
    closest_reference(&target_from_preset(preset, source_index)?, references)
}

fn validate_embedding(values: &[f32; 128]) -> Result<()> {
    if values.iter().any(|value| !value.is_finite()) {
        Err("Voice embedding contains nonfinite values.".into())
    } else {
        Ok(())
    }
}
