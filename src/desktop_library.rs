use super::*;
use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Write},
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    thread::JoinHandle,
};

const MAX_PRESET_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub(super) struct PresetPath {
    pub path: String,
    #[serde(default)]
    pub recursive: bool,
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

fn protected_component(name: &std::ffi::OsStr) -> bool {
    protected_component_with_license(name, true)
}

pub(super) fn protected_path(path: &Path) -> bool {
    path.components().any(|part| match part {
        std::path::Component::Normal(name) => protected_component(name),
        _ => false,
    })
}

pub(super) fn protected_model_path(path: &Path) -> bool {
    let named_model = path.file_name().and_then(|name| name.to_str()).is_some_and(|name|
        name.eq_ignore_ascii_case("model.dnni") || name.eq_ignore_ascii_case("userdata.bin"));
    path.components().any(|part| match part {
        std::path::Component::Normal(name) => protected_component_with_license(name, !named_model),
        _ => false,
    })
}

pub(super) fn default_preset_paths(own: &Path) -> Vec<PresetPath> {
    let mut paths = vec![PresetPath { path: own.to_string_lossy().into_owned(), recursive: false }];

    if let Some(appdata) = std::env::var_os("APPDATA") {
        let original = PathBuf::from(appdata).join("Dreamtonics/Vocoflex/presets");
        if original.is_dir() && original != own {
            paths.push(PresetPath { path: original.to_string_lossy().into_owned(), recursive: false });
        }
    }
    paths
}

pub(super) fn contains_preset(paths: &[PresetPath], own: &Path, path: &Path) -> bool {
    if protected_path(path) { return false; }
    let Ok(metadata) = fs::symlink_metadata(path) else { return false; };
    if !metadata.is_file() || metadata.file_type().is_symlink() { return false; }
    let Ok(file) = path.canonicalize() else { return false; };
    if protected_path(&file) { return false; }
    let Some(parent) = file.parent() else { return false; };
    let in_root = |directory: &Path, recursive: bool| {
        directory.canonicalize().is_ok_and(|root|
            if recursive { parent.starts_with(root) } else { parent == root })
    };
    in_root(own, false) || paths.iter().any(|item| in_root(Path::new(&item.path), item.recursive))
}

pub(super) fn has_root(paths: &[PresetPath], directory: &Path) -> bool {
    let Ok(root) = directory.canonicalize() else { return false; };
    paths.iter().any(|item| Path::new(&item.path).canonicalize().is_ok_and(|path| path == root))
}

fn scan_preset_paths(paths: &[PresetPath], cancelled: &AtomicBool) -> Vec<(String, PathBuf)> {
    use std::collections::HashSet;
    let mut seen_dirs = HashSet::new();
    let mut seen_files = HashSet::new();
    let mut entries = Vec::new();
    for root in paths {
        if cancelled.load(Ordering::Relaxed) { return Vec::new(); }
        if root.path.is_empty() || protected_path(Path::new(&root.path)) { continue; }
        let mut pending = vec![PathBuf::from(&root.path)];
        while let Some(directory) = pending.pop() {
            if cancelled.load(Ordering::Relaxed) { return Vec::new(); }
            let Ok(canonical) = directory.canonicalize() else { continue; };
            if protected_path(&canonical) { continue; }
            let identity = canonical.to_string_lossy().to_lowercase();

            if !seen_dirs.insert((identity, root.recursive)) { continue; }
            let Ok(children) = fs::read_dir(&directory) else { continue; };
            for child in children.flatten() {
                if cancelled.load(Ordering::Relaxed) { return Vec::new(); }
                if protected_component(&child.file_name()) { continue; }
                let Ok(kind) = child.file_type() else { continue; };
                if kind.is_symlink() { continue; }
                let path = child.path();
                if kind.is_dir() {
                    if root.recursive { pending.push(path); }
                    continue;
                }
                if !kind.is_file() || !path.extension().and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("json")) { continue; }
                let Ok(canonical) = path.canonicalize() else { continue; };
                if !seen_files.insert(canonical.to_string_lossy().to_lowercase()) { continue; }
                let Ok(text) = read_preset_text(&path) else { continue; };
                let Ok(document) = serde_json::from_str::<Value>(&text) else { continue; };
                if validate_preset_kind(&path, &document).is_err() { continue; }
                let name = document["presetName"].as_str().unwrap_or("Untitled").to_owned();
                entries.push((name, path));
            }
        }
    }
    entries.sort_by(|(a, ap), (b, bp)| a.to_lowercase().cmp(&b.to_lowercase())
        .then_with(|| ap.cmp(bp)));
    entries
}

pub(super) struct ScanResult {
    pub entries: Vec<LibraryEntry>,
    pub voices: Vec<VoiceEntry>,
}

fn scan(paths: &[PresetPath], cancelled: &AtomicBool) -> ScanResult {
    let entries: Vec<_> = scan_preset_paths(paths, cancelled).into_iter()
        .map(|(name, path)| LibraryEntry { name, path }).collect();
    let mut voices: Vec<VoiceEntry> = Vec::new();
    for entry in &entries {
        if cancelled.load(Ordering::Relaxed) { break; }
        if let Ok(text) = read_preset_text(&entry.path)
            && let Ok(document) = serde_json::from_str::<Value>(&text)
            && validate_preset_kind(&entry.path, &document).is_ok()
            && let Some(sources) = voice(&document)["sources"].as_array()
        {
            for source in sources.iter().filter(|source| source["type"] == "trajectory") {
                let name = source["name"].as_str().filter(|name| !name.is_empty())
                    .unwrap_or(&entry.name).to_owned();
                if !voices.iter().any(|item| item.name == name && item.source["data"] == source["data"]) {
                    voices.push(VoiceEntry { name, source: source.clone() });
                }
            }
        }
    }
    voices.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    ScanResult { entries, voices }
}

struct ScanJob {
    epoch: u64,
    cancelled: Arc<AtomicBool>,
    handle: JoinHandle<ScanResult>,
}

#[derive(Default)]
pub(super) struct Scanner {
    epoch: u64,
    job: Option<ScanJob>,
    queued: Option<(u64, Vec<PresetPath>)>,
    repaint: Option<egui::Context>,
}

impl Scanner {
    pub fn request(&mut self, paths: &[PresetPath]) -> Result<()> {
        self.epoch = self.epoch.checked_add(1).ok_or("Preset scan generation exhausted")?;
        if let Some(job) = &self.job { job.cancelled.store(true, Ordering::Relaxed); }
        self.queued = Some((self.epoch, paths.to_vec()));
        self.launch_queued()
    }

    fn launch_queued(&mut self) -> Result<()> {
        if self.job.is_some() { return Ok(()); }
        let Some((epoch, paths)) = self.queued.take() else { return Ok(()); };
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = cancelled.clone();
        let repaint = self.repaint.clone();
        let handle = std::thread::Builder::new().name("preset-library-scan".into())
            .spawn(move || {
                let result = scan(&paths, &worker_cancelled);
                if let Some(ctx) = repaint { ctx.request_repaint(); }
                result
            })
            .map_err(|error| format!("Could not start preset scan: {error}"))?;
        self.job = Some(ScanJob { epoch, cancelled, handle });
        Ok(())
    }

    pub fn busy(&self) -> bool { self.job.is_some() || self.queued.is_some() }

    pub fn poll(&mut self, ctx: &egui::Context) -> Option<Result<ScanResult>> {
        self.repaint = Some(ctx.clone());
        let mut result = None;
        if self.job.as_ref().is_some_and(|job| job.handle.is_finished()) {
            let job = self.job.take().unwrap();
            let current = job.epoch == self.epoch && !job.cancelled.load(Ordering::Relaxed);
            let joined = job.handle.join();
            if current {
                result = Some(joined.map_err(|_| "Preset scan stopped unexpectedly.".to_owned()));
            }
        }
        if let Err(error) = self.launch_queued() { result = Some(Err(error)); }
        if self.busy() { ctx.request_repaint_after(std::time::Duration::from_millis(50)); }
        result
    }
}

impl Drop for Scanner {
    fn drop(&mut self) {
        if let Some(job) = &self.job { job.cancelled.store(true, Ordering::Relaxed); }
    }
}

pub(super) fn preset_candidate_name(name: &str, suffix: usize) -> String {
    let stem = if name.trim().is_empty() { "Preset" } else { name };
    if suffix == 0 {
        return stem.to_owned();
    }
    let ending = format!(" ({suffix})");
    let base: String = stem
        .chars()
        .take(100usize.saturating_sub(ending.chars().count()))
        .collect();
    format!("{base}{ending}")
}

pub(super) fn publish_new_preset<T: AsRef<[u8]>>(
    directory: &Path,
    name: &str,
    experimental: bool,
    mut prepare: impl FnMut(&str) -> Result<T>,
) -> Result<(PathBuf, T)> {
    let mut suffix = 0usize;
    loop {
        let candidate = preset_candidate_name(name, suffix);
        let path = directory.join(preset_filename(&candidate, experimental));

        let prepared = prepare(&candidate)?;
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                suffix = suffix.checked_add(1).ok_or_else(|| {
                    "A new preset filename could not be reserved.".to_owned()
                })?;
                continue;
            }
            Err(_) => return Err("The new preset file could not be created.".into()),
        };
        file.write_all(prepared.as_ref())
            .and_then(|_| file.flush())
            .map_err(|_| {
                "The new preset file could not be completed. A partial file may remain; the current preset has not been changed.".to_owned()
            })?;
        return Ok((path, prepared));
    }
}

pub(super) fn preset_filename(name: &str, experimental: bool) -> String {
    let mut stem = safe_filename(name);
    if experimental {
        return format!("{stem}.exp.json");
    }
    if stem.to_ascii_lowercase().ends_with(".exp") {
        // Keep an ordinary preset's display name while avoiding the reserved suffix.
        stem.push('_');
    }
    format!("{stem}.json")
}

pub(super) fn is_experimental_preset_path(path: &Path) -> bool {
    path.file_stem().is_some_and(|stem| {
        stem.as_encoded_bytes().len() >= 4
            && stem.as_encoded_bytes()[stem.as_encoded_bytes().len() - 4..].eq_ignore_ascii_case(b".exp")
    }) && is_json_preset_path(path)
}

fn is_json_preset_path(path: &Path) -> bool {
    #[cfg(windows)]
    if path.file_name().is_some_and(|name| name.as_encoded_bytes().contains(&b':')) {
        return false;
    }
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

pub(super) fn validate_preset_kind(path: &Path, document: &Value) -> Result<()> {
    if !is_json_preset_path(path) {
        return Err("Choose a preset filename ending in .json.".into());
    }
    let metadata = crate::preset::experimental_canvas_metadata(document)?;
    match (is_experimental_preset_path(path), metadata) {
        (true, None) => Err("The experimental preset is missing canvas settings.".into()),
        (false, Some(_)) => Err("Experimental presets must use the .exp.json extension.".into()),
        (true, Some(_)) => crate::preset::waypoints(document).map(|_| ()),
        _ => Ok(()),
    }
}

pub(super) fn write_exported_preset(path: &Path, bytes: &[u8], create_new: bool) -> Result<()> {
    if protected_path(path) || path.canonicalize().is_ok_and(|target| protected_path(&target))
        || path.parent().and_then(|parent| parent.canonicalize().ok())
            .is_some_and(|parent| protected_path(&parent))
    {
        return Err("Choose a different preset location.".into());
    }
    if !create_new {
        return fs::write(path, bytes).map_err(|error| error.to_string());
    }
    let mut file = OpenOptions::new().write(true).create_new(true).open(path).map_err(|error| {
        if error.kind() == ErrorKind::AlreadyExists {
            "Choose a new filename for this preset copy.".to_owned()
        } else {
            "The new preset file could not be created.".to_owned()
        }
    })?;
    file.write_all(bytes).and_then(|_| file.flush()).map_err(|error| error.to_string())
}

pub(super) fn read_preset_text(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(|_| "The preset file could not be opened.".to_owned())?;
    let metadata = file
        .metadata()
        .map_err(|_| "The preset file could not be inspected.".to_owned())?;
    if !metadata.is_file() {
        return Err("Choose a regular preset JSON file.".into());
    }
    if metadata.len() > MAX_PRESET_BYTES {
        return Err("The preset exceeds the 16 MiB import limit.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_PRESET_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "The preset file could not be read.".to_owned())?;
    if bytes.len() as u64 > MAX_PRESET_BYTES {
        return Err("The preset exceeds the 16 MiB import limit.".into());
    }
    String::from_utf8(bytes).map_err(|_| "The preset file is not valid UTF-8.".into())
}

fn load_factory() -> Result<Vec<VoiceEntry>> {
    let catalog = crate::voice_library::FactoryLibrary::bundled()?;
    let voices = catalog
        .voices
        .into_iter()
        .map(|voice| {
            let source = voice.source_document()?;
            Ok(VoiceEntry {
                name: voice.name,
                source,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if voices.is_empty() {
        return Err("The included factory catalog is empty.".into());
    }
    Ok(voices)
}

impl DesktopApp {
    pub(super) fn load_factory_voices(&mut self) {
        self.factory_voices.clear();
        self.factory_error = None;
        match load_factory() {
            Ok(voices) => self.factory_voices = voices,
            Err(error) => self.factory_error = Some(error),
        }
    }

    pub(super) fn factory_status(&self) -> String {
        if let Some(error) = &self.factory_error {
            format!("Factory voices unavailable: {error}")
        } else {
            format!("{} factory voices included.", self.factory_voices.len())
        }
    }
}
