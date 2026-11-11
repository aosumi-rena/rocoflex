use super::*;
use std::{
    collections::VecDeque,
    fs::File,
    io::Read,
    sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, TryRecvError}},
};

const MAX_AUDIO_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PENDING_AUDIO: usize = 256;

const MAX_AUDIO_DETAILS_BYTES: usize = 8 * 1024;
pub(super) const AUDIO_IMPORT_TOOLTIP: &str = "Import an audio recording as a voice trajectory.";

#[derive(Default)]
pub(super) struct AudioEpochs {
    request: u64,
    destination: u64,
    model: u64,
    queued: VecDeque<AudioRequest>,
    total: usize,
    finished: usize,
    failures: AudioMessages,
    notices: AudioMessages,
    cancelled_queued: usize,
    pending_error: Option<String>,
    pending_notice: Option<String>,
}

#[derive(Default)]
struct AudioMessages {
    total: usize,
    entries: Vec<String>,
    bytes: usize,
}

impl AudioMessages {
    fn record(&mut self, detail: String) {
        self.total += 1;
        if detail.len() < MAX_AUDIO_DETAILS_BYTES.saturating_sub(self.bytes) {
            self.bytes += detail.len() + 1;
            self.entries.push(detail);
        }
    }

    fn render(&self, label: &str) -> String {
        let omitted = self.total - self.entries.len();
        format!("{label}: {}\n{}\nDetails omitted: {omitted}.",
            self.total, self.entries.join("\n"))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum AudioProgress {
    Analyzing { current: usize, total: usize },
    Waiting { remaining: usize },
    Pending { remaining: usize },
}

#[derive(Clone)]
struct AudioRequest {
    request: u64,
    destination: u64,
    model: u64,
    model_path: String,
    audio_path: PathBuf,
    ffmpeg: Option<PathBuf>,
    cancelled: Arc<AtomicBool>,
    name: String,
    gain_policy: crate::ordinary_audio::MathPolicy,
    resampler_policy: crate::ordinary_audio::ResamplerPolicy,
    indicator: crate::pitch_selected_head::IndicatorAccumulation,
}

pub(super) struct PendingAudioJob {
    request: AudioRequest,
    receiver: Receiver<Result<Option<Value>>>,
    valid: bool,
}

impl PendingAudioJob {
    fn invalidate(&mut self) {
        self.valid = false;
        self.request.cancelled.store(true, Ordering::Relaxed);
    }
}

impl Drop for PendingAudioJob {
    fn drop(&mut self) {
        self.request.cancelled.store(true, Ordering::Relaxed);
    }
}

impl AudioRequest {
    fn check_cancelled(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Relaxed) { Err("Audio import cancelled.".into()) }
        else { Ok(()) }
    }
}

impl DesktopApp {
    pub(super) fn generation_busy(&self) -> bool {
        self.tone_job.is_some() || self.audio_job.is_some() || !self.audio_epochs.queued.is_empty()
    }

    pub(super) fn generation_context(&self) -> (u64, u64) {
        (self.audio_epochs.destination, self.audio_epochs.model)
    }

    pub(super) fn audio_destination_changed(&mut self) {
        self.audio_epochs.destination = self.audio_epochs.destination.wrapping_add(1);

        self.audio_epochs.pending_notice = None;
        self.cancel_queued_audio(false);
        if let Some(job) = &mut self.audio_job {
            job.invalidate();
        }
    }

    pub(super) fn audio_model_changed(&mut self) {
        self.audio_epochs.model = self.audio_epochs.model.wrapping_add(1);
        self.cancel_queued_audio(true);
        if let Some(job) = &mut self.audio_job {
            job.invalidate();
        }
    }

    pub(super) fn cancel_audio_generation(&mut self) {
        self.audio_epochs.request = self.audio_epochs.request.wrapping_add(1);
        self.cancel_queued_audio(true);
        if let Some(job) = &mut self.audio_job {
            job.invalidate();
        }
    }

    fn cancel_queued_audio(&mut self, defer_notice: bool) {
        let count = self.audio_epochs.queued.len();
        self.audio_epochs.queued.clear();
        if count > 0 {
            self.audio_epochs.cancelled_queued += count;
            let summary = self.audio_notice_summary();
            if defer_notice {
                self.audio_epochs.pending_notice = Some(summary);
                self.publish_pending_audio_messages();
            } else if self.notice.is_none() {
                self.notice = Some(summary);
            }
        }
    }

    fn audio_notice_summary(&self) -> String {
        let previous = if self.audio_epochs.notices.total == 0 { String::new() }
            else { self.audio_epochs.notices.render("Audio import notices") };
        if self.audio_epochs.cancelled_queued == 0 { previous }
        else {
            format!("{} queued audio recordings cancelled without analysis.\n{previous}",
                self.audio_epochs.cancelled_queued)
        }
    }

    fn publish_pending_audio_messages(&mut self) -> bool {
        let mut published = false;

        if self.error.is_none() && self.audio_epochs.pending_error.is_some() {
            self.error = self.audio_epochs.pending_error.take();
            published = true;
        }
        if self.notice.is_none() && self.audio_epochs.pending_notice.is_some() {
            self.notice = self.audio_epochs.pending_notice.take();
            published = true;
        }
        published
    }

    fn audio_failure(&mut self, path: &Path, message: &str) {
        self.audio_epochs.failures.record(format!("{}: {message}", path.display()));
        self.audio_epochs.pending_error = Some(self.audio_epochs.failures.render("Audio import failures"));
        self.publish_pending_audio_messages();
    }

    fn audio_notice(&mut self, path: &Path, message: &str) {
        self.audio_epochs.notices.record(format!("{}: {message}", path.display()));
        self.audio_epochs.pending_notice = Some(self.audio_notice_summary());
        self.publish_pending_audio_messages();
    }

    pub(super) fn cancel_generation(&mut self) {
        self.cancel_audio_generation();
        self.cancel_tone_generation();
        self.audio_status =
            "Request cancelled. Existing analysis will finish before another can start.".into();
    }

    pub(super) fn choose_audio(&mut self, ctx: &egui::Context) {
        if self.layout_blocked() {
            return;
        }
        if self.tone_job.is_some() {
            self.error = Some("Finish or cancel point generation before importing audio.".into());
            return;
        }
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Import Audio...")
            .add_filter("Audio", &["wav", "bwf", "flac", "mp3", "ogg", "opus", "m4a", "aac", "aiff", "aif", "wma", "alac", "mp4", "mkv", "webm"])
            .add_filter("All files", &["*"])
            .pick_files()
        {
            self.dispatch_import(ctx, paths);
        }
    }

    pub(super) fn dispatch_import(&mut self, ctx: &egui::Context, paths: Vec<PathBuf>) {
        if self.layout_blocked() {
            return;
        }
        if paths.is_empty() {
            return;
        }
        let is_preset = |path: &PathBuf| path.extension().and_then(|s| s.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
        if paths.len() > 1 && paths.iter().any(is_preset) {
            self.error = Some("Import a preset separately from audio recordings.".into());
            return;
        }
        if paths.len() == 1 && is_preset(&paths[0]) {
            if let Err(error) = self.load(paths[0].clone()) {
                self.error = Some(error);
            }
        } else {
            self.enqueue_audio_generation(ctx, paths);
        }
    }

    fn enqueue_audio_generation(&mut self, ctx: &egui::Context, paths: Vec<PathBuf>) {
        if self.layout_blocked() {
            return;
        }
        if self.tone_job.is_some() || self.audio_job.as_ref().is_some_and(|job|
            !job.valid || job.request.request != self.audio_epochs.request
                || job.request.destination != self.audio_epochs.destination
                || job.request.model != self.audio_epochs.model
                || job.request.model_path != self.model_paths[0])
        {
            self.error = Some("Wait for the previous analysis to finish before importing audio.".into());
            return;
        }
        if self.model_paths[0].trim().is_empty() {
            self.error = Some("Choose the analysis model in Resources before importing audio.".into());
            return;
        }
        let pending = self.audio_epochs.queued.len() + usize::from(self.audio_job.is_some());
        if paths.len() > MAX_PENDING_AUDIO.saturating_sub(pending) {
            self.error = Some("At most 256 audio recordings can be pending. No files from this selection were queued.".into());
            return;
        }
        if pending == 0 {
            self.audio_epochs.request = self.audio_epochs.request.wrapping_add(1);
            self.audio_epochs.total = 0;
            self.audio_epochs.finished = 0;

            if self.audio_epochs.pending_error.is_none() {
                self.audio_epochs.failures = AudioMessages::default();
            }
            if self.audio_epochs.pending_notice.is_none() {
                self.audio_epochs.notices = AudioMessages::default();
                self.audio_epochs.cancelled_queued = 0;
            }
        }
        self.audio_epochs.total += paths.len();
        for path in paths {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("Imported Voice")
                .to_owned();
            let request = AudioRequest {
                request: self.audio_epochs.request,
                destination: self.audio_epochs.destination,
                model: self.audio_epochs.model,
                model_path: self.model_paths[0].clone(),
                audio_path: path,
                ffmpeg: if self.ffmpeg_path.trim().is_empty() { self.ffmpeg_detected.clone() }
                    else { Some(PathBuf::from(self.ffmpeg_path.trim())) },
                cancelled: Arc::new(AtomicBool::new(false)),
                name,
                gain_policy: crate::ordinary_audio::MathPolicy::WindowsHostLog10RoundF32AndPowf,
                resampler_policy: crate::ordinary_audio::ResamplerPolicy::WindowsHostF64SourceOrder,
                indicator: crate::pitch_selected_head::IndicatorAccumulation::PerInputSaturatedI16,
            };

            self.audio_epochs.queued.push_back(request);
        }
        self.start_next_audio_generation(ctx);
    }

    fn start_next_audio_generation(&mut self, ctx: &egui::Context) {
        if self.layout_blocked() || self.tone_job.is_some() || self.audio_job.is_some() {
            return;
        }
        let Some(request) = self.audio_epochs.queued.front() else { return; };
        if request.request != self.audio_epochs.request
            || request.destination != self.audio_epochs.destination
            || request.model != self.audio_epochs.model
            || request.model_path != self.model_paths[0]
        {

            self.cancel_queued_audio(true);
            self.audio_status = "Previous analysis finished without changing the preset.".into();
            return;
        }
        let request = self.audio_epochs.queued.pop_front().expect("checked queued request");
        let (sender, receiver) = mpsc::channel();
        let worker_request = request.clone();
        let wake = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("audio-analysis".into())
            .spawn(move || {
                let result = generate_source(&worker_request);
                let _ = sender.send(result);
                wake.request_repaint();
            }).map(|_| ());
        match spawned {
            Ok(()) => {

                self.audio_status = format!("Analyzing {}...", request.name);
                self.audio_job = Some(PendingAudioJob {
                    request,
                    receiver,
                    valid: true,
                });
            }
            Err(error) => {
                self.audio_failure(&request.audio_path,
                    &format!("Could not start audio analysis: {error}"));
                self.audio_epochs.finished += 1;

                ctx.request_repaint();
            }
        }
    }

    pub(super) fn poll_audio_generation(&mut self, ctx: &egui::Context) {
        if self.publish_pending_audio_messages() { ctx.request_repaint(); }
        let Some(mut job) = self.audio_job.take() else {
            self.start_next_audio_generation(ctx);
            return;
        };
        job.valid &= job.request.request == self.audio_epochs.request
            && job.request.destination == self.audio_epochs.destination
            && job.request.model == self.audio_epochs.model
            && job.request.model_path == self.model_paths[0];
        if !job.valid { job.invalidate(); }
        let result = match job.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => {
                self.audio_job = Some(job);
                return;
            }
            Err(TryRecvError::Disconnected) => {
                Err("Audio analysis stopped before returning a result.".into())
            }
        };
        if !job.valid {
            self.cancel_queued_audio(true);
            self.audio_status = "Previous analysis finished without changing the preset.".into();
        } else {
            match result {
                Err(error) => {
                    self.audio_status = error.clone();
                    self.audio_failure(&job.request.audio_path, &error);
                }
                Ok(None) => {
                    self.audio_status =
                        "This recording did not produce a usable voice trajectory.".into();
                    self.audio_notice(&job.request.audio_path,
                        "This recording did not produce a usable voice trajectory.");
                }
                Ok(Some(source)) => match self.append_source(source) {
                    Ok(Publication::Committed | Publication::Inspection) => {
                        self.audio_status = "Voice trajectory added to the preset.".into()
                    }
                    Ok(Publication::Retained) => {
                        self.audio_status =
                            "Analyzed voice retained while the canvas change is unresolved.".into()
                    }
                    Err(_) => {
                        self.audio_failure(&job.request.audio_path,
                            "The analyzed voice could not be added to this preset.")
                    }
                },
            }
        }
        self.audio_epochs.finished += 1;

        self.start_next_audio_generation(ctx);
        ctx.request_repaint();
    }

    pub(super) fn analysis_ui(&mut self, ctx: &egui::Context, full: Rect) {
        self.analysis_ui_with_barrier(ctx, full, false);
    }

    pub(super) fn audio_progress(&self) -> Option<AudioProgress> {
        if self.audio_job.is_some() {
            let total = self.audio_epochs.total.max(1);
            Some(AudioProgress::Analyzing {
                current: (self.audio_epochs.finished + 1).min(total), total,
            })
        } else if self.audio_epochs.queued.is_empty() {
            None
        } else if self.layout_blocked() {
            Some(AudioProgress::Waiting { remaining: self.audio_epochs.queued.len() })
        } else {
            Some(AudioProgress::Pending { remaining: self.audio_epochs.queued.len() })
        }
    }

    pub(super) fn analysis_ui_with_barrier(
        &mut self,
        ctx: &egui::Context,
        full: Rect,
        _input_blocked: bool,
    ) {

        let Some(progress) = self.audio_progress() else { return; };
        let (text, spinning) = match progress {
            AudioProgress::Analyzing { current, total } => (
                localization::tr(ctx, "Analyzing Audio _/_")
                    .replacen('_', &current.to_string(), 1).replacen('_', &total.to_string(), 1), true),
            AudioProgress::Waiting { remaining } => (
                localization::tr(ctx, "Audio import waiting: Canvas change retained (_ queued)")
                    .replacen('_', &remaining.to_string(), 1), false),
            AudioProgress::Pending { remaining } => (
                localization::tr(ctx, "Audio import pending (_ remaining)")
                    .replacen('_', &remaining.to_string(), 1), false),
        };
        let font = FontId::proportional(12.0);
        let text_width = ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text.clone(), font.clone(), TEXT).size().x);
        let rect = Rect::from_center_size(
            pos2(full.center().x, full.bottom() - 29.0), vec2(text_width + 48.0, 32.0),
        );
        egui::Area::new(egui::Id::new("analysis-status"))
            .order(egui::Order::Foreground)
            .interactable(false)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                ui.painter().rect_filled(rect, 16.0, Color32::from_rgb(79, 79, 87));
                if spinning {
                    let spinner = Rect::from_center_size(
                        pos2(rect.left() + 16.0, rect.center().y), vec2(18.0, 18.0));
                    egui::Spinner::new().color(TEXT).paint_at(ui, spinner);
                }
                ui.painter().text(pos2(rect.left() + if spinning { 32.0 } else { 16.0 }, rect.center().y),
                    Align2::LEFT_CENTER, text, font, TEXT);
            });
    }

    pub(super) fn generation_frame(
        &mut self,
        ctx: &egui::Context,
        full: Rect,
        input_blocked: bool,
    ) {
        if input_blocked {
            self.analysis_ui_with_barrier(ctx, full, true);
        } else {
            self.analysis_ui(ctx, full);
        }
        self.poll_generation(ctx);
    }

    pub(super) fn poll_generation(&mut self, ctx: &egui::Context) {
        if !self.layout_blocked() && ctx.input(|i| i.viewport().close_requested()) {
            self.cancel_tone_generation();
            self.cancel_audio_generation();
        }
        self.poll_tone_generation(ctx);
        self.poll_audio_generation(ctx);
        if self.layout_blocked() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }
}

fn read_audio(path: &Path) -> Result<Vec<u8>> {
    let file =
        File::open(path).map_err(|_| "The selected audio file could not be opened.".to_owned())?;
    let metadata = file
        .metadata()
        .map_err(|_| "The audio file could not be inspected.".to_owned())?;
    if !metadata.is_file() {
        return Err("Choose a regular RIFF/WAVE audio file.".into());
    }
    if metadata.len() > MAX_AUDIO_BYTES {
        return Err("The audio file exceeds the 128 MiB import limit.".into());
    }
    read_audio_bytes(file, MAX_AUDIO_BYTES)
}

fn read_audio_bytes(reader: impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "The audio file could not be read.".to_owned())?;
    if bytes.len() as u64 > limit {
        return Err("The audio file exceeds the 128 MiB import limit.".into());
    }
    Ok(bytes)
}

fn audio_error(error: crate::ordinary_audio::Error) -> String {
    use crate::{
        ordinary_audio::{Error as E, ResamplerError as R},
        wav48k::Error as W,
    };
    match error {
        E::UnsupportedShortInput(_) => "Choose a recording longer than 40 ms.",
        E::NonFiniteInput => "The recording contains invalid sample values.",
        E::Decode(W::UnsupportedSampleRate(_)) => {
            "This sample rate requires FFmpeg. Set its path in Resources & Workspace."
        }
        E::Decode(W::TooLong(_)) => "Choose a recording no longer than 300 seconds.",
        E::Decode(W::Allocation) => "There is not enough memory to import this recording.",
        E::Decode(_) => {
            "This WAV file could not be decoded. Set up FFmpeg in Resources & Workspace for additional encodings."
        }
        E::NumericDomain(_) => "This recording could not be prepared for analysis.",
        E::Resample(R::UnsupportedPlatform) => {
            "Sample-rate conversion is not supported on this platform."
        }
        E::Resample(R::UnsupportedFloatEnvironment { .. }) => {
            "The current numeric settings do not support sample-rate conversion."
        }
        E::Resample(R::NumericDomain(_)) => {
            "This recording could not be converted for analysis."
        }
    }
    .into()
}

fn generate_source(request: &AudioRequest) -> Result<Option<Value>> {
    request.check_cancelled()?;
    let wave = request.audio_path.extension().and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav") || extension.eq_ignore_ascii_case("bwf"));
    let samples = if let Some(executable) = request.ffmpeg.as_ref() {
        let decoded = audio_decode::decode(&request.audio_path, executable, &request.cancelled)?;
        request.check_cancelled()?;
        crate::ordinary_audio::normalize_mono48k(&decoded, request.gain_policy).map_err(audio_error)?
    } else if wave {
        let bytes = read_audio(&request.audio_path)?;
        request.check_cancelled()?;
        crate::ordinary_audio::normalize_supported_wav(&bytes, request.gain_policy, request.resampler_policy)
            .map_err(audio_error)?.samples
    } else {
        return Err("FFmpeg was not found. Set its path in Settings → Resources & Workspace to import this format.".into());
    };
    request.check_cancelled()?;
    let model = crate::model::read_bytes(Path::new(&request.model_path), generation::MAX_MODEL_BYTES)?;
    request.check_cancelled()?;
    let generator =
        crate::voice_generation::VoiceGenerator::from_dnni(&model, request.indicator)
            .map_err(|_| "The selected model could not be loaded for audio analysis.".to_owned())?;
    drop(model);
    request.check_cancelled()?;
    let generated = generator
        .generate_final_normalized48k(&samples)
        .map_err(|_| "This recording could not be analyzed with the selected model.".to_owned())?;
    drop(generator);
    drop(samples);
    request.check_cancelled()?;
    generated
        .map(|voice| {
            voice.source_document(request.name.clone()).map_err(|_| {
                "The generated trajectory could not be prepared for this preset.".to_owned()
            })
        })
        .transpose()
}
