use crate::Result;
use std::{fs::{self, File, OpenOptions}, io::{Read, Write}, path::{Path, PathBuf}};

pub(crate) const MAX_FRAMES: u64 = 48_000 * 60 * 30;

pub(crate) fn input_path(path: &Path) -> Result<PathBuf> {
    resolve_input(path, protected_path)
}

#[cfg(all(windows, feature = "desktop"))]
pub(crate) fn model_path(path: &Path) -> Result<PathBuf> {
    resolve_input(path, protected_model_path)
}

fn resolve_input(path: &Path, protected: fn(&Path) -> bool) -> Result<PathBuf> {
    if !path.is_absolute() { return Err("Offline paths must be absolute.".into()); }
    reject_protected(protected(path))?;
    let resolved = path.canonicalize().map_err(|error| format!("Cannot resolve offline input: {error}"))?;
    reject_protected(protected(&resolved))?;
    Ok(resolved)
}

pub(crate) fn output_path(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() { return Err("Offline paths must be absolute.".into()); }
    reject_protected(protected_path(path))?;
    let parent = path.parent().ok_or("Offline output has no parent directory.")?
        .canonicalize().map_err(|error| format!("Cannot resolve offline output directory: {error}"))?;
    reject_protected(protected_path(&parent))?;
    let output = parent.join(path.file_name().ok_or("Offline output has no filename.")?);
    if output.try_exists().map_err(|error| error.to_string())? {
        return Err("Offline output already exists; choose a new output path.".into());
    }
    Ok(output)
}

fn protected_component_with_license(name: &std::ffi::OsStr, block_license: bool) -> bool {
    let text = name.to_string_lossy().to_ascii_lowercase();
    text.split(|c: char| !c.is_ascii_alphanumeric()).any(|part| {
        part.starts_with("session") || part == "auth" || part.starts_with("authentication")
            || part.starts_with("authorization") || part.starts_with("oauth")
            || (block_license && (part.starts_with("license") || part.starts_with("licence")))
            || part.starts_with("credential") || part.starts_with("token")
            || part.starts_with("cookie") || part.starts_with("password")
            || part.starts_with("secret")
    })
}

pub(crate) fn protected_component(name: &std::ffi::OsStr) -> bool {
    protected_component_with_license(name, true)
}

pub(crate) fn protected_path(path: &Path) -> bool {
    path.components().any(|part| match part {
        std::path::Component::Normal(name) => protected_component(name),
        _ => false,
    })
}

pub(crate) fn protected_model_path(path: &Path) -> bool {
    let named_model = path.file_name().and_then(|name| name.to_str()).is_some_and(|name|
        name.eq_ignore_ascii_case("model.dnni") || name.eq_ignore_ascii_case("userdata.bin"));
    path.components().any(|part| match part {
        std::path::Component::Normal(name) => protected_component_with_license(name, !named_model),
        _ => false,
    })
}

fn reject_protected(protected: bool) -> Result<()> {
    if protected { return Err("Session and authentication files are not offline inputs or outputs.".into()); }
    Ok(())
}

pub(crate) fn read_json(path: &Path, maximum: u64) -> Result<serde_json::Value> {
    let path = input_path(path)?;
    let bytes = crate::model::read_bytes(&path, maximum)?;
    serde_json::from_slice(&bytes).map_err(|error| format!("Invalid offline JSON: {error}"))
}

pub(crate) struct Input { reader: Box<dyn Read>, pub frames: u64, remaining: u64, streaming: bool }
impl Input {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let file = File::open(input_path(path)?).map_err(|error| error.to_string())?;
        let metadata = file.metadata().map_err(|error| error.to_string())?;
        let bytes = metadata.len();
        if !metadata.is_file() || bytes % 4 != 0 || bytes / 4 > MAX_FRAMES {
            return Err("Offline PCM must be float32 mono at 48000 Hz and at most 30 minutes.".into());
        }
        Ok(Self { reader: Box::new(file), frames: bytes / 4, remaining: bytes / 4, streaming: false })
    }

    pub(crate) fn stream(frames: u64) -> Result<Self> {
        if frames > MAX_FRAMES { return Err("Offline PCM cannot exceed 30 minutes.".into()); }
        Ok(Self { reader: Box::new(std::io::stdin()), frames, remaining: frames, streaming: true })
    }

    pub(crate) fn next(&mut self, buffer: &mut Vec<f32>) -> Result<bool> {
        buffer.clear();
        if self.remaining == 0 {
            let mut extra = [0];
            if self.reader.read(&mut extra).map_err(|error| error.to_string())? != 0 {
                return Err("Offline PCM changed length while reading.".into());
            }
            return Ok(false);
        }
        let frames = self.remaining.min(if self.streaming && self.frames - self.remaining < 8192 { 1024 } else { 4096 }) as usize;
        let mut bytes = [0u8; 4096 * 4];
        self.reader.read_exact(&mut bytes[..frames * 4]).map_err(|error| error.to_string())?;
        buffer.try_reserve(frames).map_err(|error| error.to_string())?;
        for word in bytes[..frames * 4].chunks_exact(4) {
            let sample = f32::from_le_bytes(word.try_into().unwrap());
            if !sample.is_finite() { return Err("Offline PCM contains nonfinite audio.".into()); }
            buffer.push(sample);
        }
        self.remaining -= frames as u64;
        Ok(true)
    }
}

pub(crate) struct Output { file: Option<File>, temporary: PathBuf, destination: PathBuf, pub frames: u64 }
impl Output {
    pub(crate) fn create(destination: &Path, progress: Option<&Path>) -> Result<Self> {
        let destination = output_path(destination)?;
        if let Some(progress) = progress {
            let temporary = output_path(progress)?;
            if temporary == destination || temporary.parent() != destination.parent() {
                return Err("The progress file must be a separate path beside the destination.".into());
            }
            let file = OpenOptions::new().write(true).create_new(true).open(&temporary).map_err(|e| e.to_string())?;
            return Ok(Self { file: Some(file), temporary, destination, frames: 0 });
        }
        let parent = destination.parent().ok_or("Offline output has no parent directory.")?;
        for index in 0..1000 {
            let temporary = parent.join(format!(".rocoflex-offline-{}-{index}.tmp", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&temporary) {
                Ok(file) => return Ok(Self { file: Some(file), temporary, destination, frames: 0 }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(format!("Cannot create offline output: {error}")),
            }
        }
        Err("Cannot reserve an offline output file.".into())
    }

    pub(crate) fn append(&mut self, samples: &[f32]) -> Result<()> {
        let file = self.file.as_mut().ok_or("Offline output is closed.")?;
        let mut bytes = [0u8; 4096 * 4];
        for chunk in samples.chunks(4096) {
            for (word, sample) in bytes.chunks_exact_mut(4).zip(chunk) {
                if !sample.is_finite() { return Err("Offline synthesis produced nonfinite audio.".into()); }
                word.copy_from_slice(&sample.to_le_bytes());
            }
            file.write_all(&bytes[..chunk.len() * 4]).map_err(|error| error.to_string())?;
        }
        self.frames = self.frames.checked_add(samples.len() as u64).ok_or("Offline output length overflow.")?;
        Ok(())
    }

    pub(crate) fn commit(mut self) -> Result<()> {
        self.file.take().ok_or("Offline output is closed.")?.sync_all().map_err(|error| error.to_string())?;
        fs::hard_link(&self.temporary, &self.destination)
            .map_err(|error| format!("Cannot publish complete offline output: {error}"))?;
        Ok(())
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        drop(self.file.take());
        let _ = fs::remove_file(&self.temporary);
    }
}
