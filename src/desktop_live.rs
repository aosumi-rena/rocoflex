use super::*;
use crate::current_condition::ConditionBus;
#[path = "desktop_condition_cache.rs"]
mod condition_cache;

#[cfg(windows)]
#[derive(Clone, PartialEq, Eq)]
struct LiveModels {
    analysis_model: PathBuf,
    synthesis_model: PathBuf,
    profile: crate::Result<crate::prepare_quality::PrepareProfile>,
    multithread: bool,
}
#[cfg(windows)]
#[derive(Clone, PartialEq, Eq)]
enum LiveRoute { Direct, Target(LiveModels) }
#[cfg(windows)]
#[derive(Clone, PartialEq, Eq)]
struct LiveRequest { selection: audio_devices::Selection, route: LiveRoute }

#[cfg(windows)]
#[derive(Debug, PartialEq, Eq)]
enum PendingLiveError {
    MissingCondition { generation: u64, message: String },
    Other(String),
}

pub(super) struct Controller {
    pub bus: ConditionBus,
    canvas_mix_cache: condition_cache::Cache,

    restored_canvas: Option<[f32; 128]>,
    condition_available: bool,
    condition_empty: bool,

    multithread: bool,
    multithread_edit: Option<bool>,

    multithread_mailbox: Option<std::sync::Arc<std::sync::atomic::AtomicU8>>,
    last_standalone_preference_check: Option<Instant>,
    condition_error: Option<String>,
    #[cfg(windows)]
    request_errors: Vec<PendingLiveError>,

    #[cfg(windows)]
    empty_retired_generation: Option<u64>,
    #[cfg(windows)]
    session: Option<crate::live_audio::Handle>,
    #[cfg(windows)]
    pub snapshot: Option<crate::live_audio::Snapshot>,
    #[cfg(windows)]
    meter:Option<crate::morph_processor::input_meter::Query>,
    #[cfg(windows)]
    desired: Option<LiveRequest>,

    #[cfg(windows)]
    committed_models: Option<LiveModels>,
    #[cfg(windows)]
    generation: u64,
    #[cfg(windows)]
    attempted: Option<u64>,
    #[cfg(windows)]
    active: Option<u64>,
}

impl Default for Controller {
    fn default() -> Self {
        Self { bus: ConditionBus::new(), canvas_mix_cache: Default::default(), restored_canvas: None, condition_available: false,
            condition_empty: false, multithread: true, multithread_edit: None,
            multithread_mailbox: None, last_standalone_preference_check: None,
            condition_error: None,
            #[cfg(windows)] request_errors: Vec::new(),
            #[cfg(windows)] empty_retired_generation: None,
            #[cfg(windows)] session: None,
            #[cfg(windows)] snapshot: None,
            #[cfg(windows)] meter: None,
            #[cfg(windows)] desired: None,
            #[cfg(windows)] committed_models: None,
            #[cfg(windows)] generation: 0,
            #[cfg(windows)] attempted: None,
            #[cfg(windows)] active: None }
    }
}

impl Controller {
    pub(super) fn multithread(&self) -> bool { self.multithread }

    pub(super) fn edit_multithread(&mut self, value: bool, hosted: bool) {
        if self.multithread != value {
            self.multithread = value;
            if hosted {

                self.multithread_edit = Some(value);
            }
        }
    }
    pub(super) fn set_multithread_mailbox(&mut self, mailbox: std::sync::Arc<std::sync::atomic::AtomicU8>) {
        self.multithread_mailbox = Some(mailbox);
    }
    pub(super) fn publish_multithread_edit(&mut self) {
        if let Some(mailbox) = &self.multithread_mailbox {
            if let Some(value) = self.multithread_edit.take() {

                mailbox.store(if value {2} else {1}, std::sync::atomic::Ordering::Release);
            }
        }
    }

    pub(super) fn commit_multithread_edit(&mut self, previous: bool, hosted: bool,
        save: impl FnOnce(bool) -> std::io::Result<()>) -> std::io::Result<()> {
        match save(self.multithread) {
            Ok(()) => { if hosted { self.publish_multithread_edit(); } Ok(()) },
            Err(error) => { self.multithread = previous; self.multithread_edit = None; Err(error) },
        }
    }
    pub(super) fn set_initial_multithread(&mut self, value: bool) { self.multithread = value; }

    pub(super) fn refresh_standalone_multithread(&mut self, now: Instant,
        read: impl FnOnce() -> std::io::Result<bool>) -> bool {
        if self.last_standalone_preference_check.and_then(|last| now.checked_duration_since(last))
            .is_some_and(|age| age < Duration::from_millis(500)) { return false; }
        self.last_standalone_preference_check = Some(now);
        match read() {
            Ok(value) if self.multithread != value => { self.multithread = value; true },
            _ => false,
        }
    }
    pub(super) fn adopt_hosted_multithread(&mut self, value: bool) {

        let unconsumed = self.multithread_edit.is_some() || self.multithread_mailbox.as_ref()
            .is_some_and(|mailbox| mailbox.load(std::sync::atomic::Ordering::Acquire)!=0);
        if !unconsumed { self.multithread = value; }
    }

    #[cfg(windows)]
    fn note_desired(&mut self, desired: Option<LiveRequest>) {
        if self.desired != desired {
            self.desired = desired;
            self.generation = self.generation.wrapping_add(1);
            self.attempted = None;
        }

        if self.condition_empty && matches!(self.desired.as_ref(),
            Some(LiveRequest { route: LiveRoute::Direct, .. }))
        {
            let current = self.generation;
            self.request_errors.retain(|error| !matches!(error,
                PendingLiveError::MissingCondition { generation, .. } if *generation != current));
            if let Some(owner) = self.active.filter(|&generation| generation != current) {
                self.empty_retired_generation = Some(owner);
            }
        }
    }

    #[cfg(windows)]
    fn queue_worker_errors(&mut self, generation: Option<u64>,
        error: Option<(String, Option<crate::live_audio::ProcessorFailure>)>, stop: Option<String>)
    {
        if let Some((message, kind)) = error {
            match (kind, generation) {
                (Some(crate::live_audio::ProcessorFailure::MissingCondition), Some(owner))
                    if self.empty_retired_generation == Some(owner) => {}
                (Some(crate::live_audio::ProcessorFailure::MissingCondition), Some(owner)) =>
                    self.request_errors.push(PendingLiveError::MissingCondition { generation: owner, message }),
                _ => self.request_errors.push(PendingLiveError::Other(message)),
            }
        }
        if let Some(message) = stop { self.request_errors.push(PendingLiveError::Other(message)); }
        if generation.is_some() && self.empty_retired_generation == generation {
            self.empty_retired_generation = None;
        }
    }

    #[cfg(windows)]
    fn take_request_errors(&mut self) -> Option<String> {
        let messages: Vec<String> = std::mem::take(&mut self.request_errors).into_iter()
            .map(|error| match error {
                PendingLiveError::MissingCondition { message, .. } | PendingLiveError::Other(message) => message,
            }).collect();
        if messages.is_empty() { None } else { Some(messages.join("\n")) }
    }

    #[cfg(windows)]
    fn stop_due(&self) -> bool {
        self.desired.is_none() || self.active != Some(self.generation)
    }
    #[cfg(windows)]
    fn start_due(&self) -> bool {
        self.desired.is_some() && self.attempted != Some(self.generation)
    }

    pub(super) fn invalidate_canvas_condition(&mut self) { self.canvas_mix_cache.clear(); }

    pub(super) fn restored_canvas(&self) -> Option<[f32; 128]> { self.restored_canvas }

    pub(super) fn set_restored_canvas(&mut self, embedding: Option<[f32; 128]>) {
        self.restored_canvas = embedding;
        self.canvas_mix_cache.clear();
    }
}

impl DesktopApp {
    pub(super) fn plugin_session_condition_state(&self) -> (bool, Option<String>) {
        (self.live.condition_available, self.live.condition_error.clone())
    }

    pub(super) fn adopt_plugin_session_condition_state(&mut self, available: bool, error: Option<String>) {
        self.live.condition_available = available;
        self.live.condition_error = error;
    }

    pub(super) fn standalone_readiness_status(&self) -> Option<&str> {
        if self.plugin_hosted || self.layout_blocked() {
            return None;
        }
        match (self.model_paths[0].trim().is_empty(), self.model_paths[1].trim().is_empty()) {
            (true, true) => return Some("Choose the analysis model and morphing data in Resources & Workspace."),
            (true, false) => return Some("Choose the analysis model in Resources & Workspace."),
            (false, true) => return Some("Choose the morphing data in Resources & Workspace."),
            (false, false) => {}
        }
        if self.live.condition_available {
            return None;
        }
        let no_voice = "Add a voice to the canvas or open Tone preview.";
        if self.preset.sources.iter().all(|source| source.points.is_empty()) {
            return Some(no_voice);
        }
        self.live.condition_error.as_deref().or(Some(no_voice))
    }

    pub(super) fn poll_live_audio(&mut self, ctx: &egui::Context) {

        if self.plugin_hosted {return;}
        if !ctx.input(|input| input.viewport().close_requested())
            && self.live.refresh_standalone_multithread(Instant::now(), try_read_multithread_preference)
        { ctx.request_repaint(); }
        #[cfg(windows)]
        {
        let mut level=None;
        let mut pitch=0;
        if let Some(session) = &mut self.live.session {
            if ctx.input(|input| input.viewport().close_requested()) { session.stop(); }
            let finished = session.poll_finished();

            self.live.snapshot = Some(session.snapshot());
            if finished {

                let generation = self.live.active;
                self.live.session = None;
                self.live.active = None;
                self.live.meter = None;
                let (error, stop) = self.live.snapshot.as_ref().map(|snapshot| (
                    snapshot.error.clone().map(|message| (message, snapshot.processor_failure)),
                    snapshot.stop_error.clone(),
                )).unwrap_or_default();
                self.live.queue_worker_errors(generation, error, stop);
                ctx.request_repaint();
            } else {
                let receiving=self.live.snapshot.as_ref().is_some_and(|snapshot|
                    matches!(snapshot.phase,crate::live_audio::Phase::Receiving|crate::live_audio::Phase::Running)
                    && snapshot.error.is_none());
                if receiving && let Some(query)=&mut self.live.meter {
                    if let Some(observation)=query.poll_observation() {level=Some(observation.level);pitch=observation.pitch.word;}
                }
                ctx.request_repaint_after(Duration::from_millis(33));
            }
        }
        self.set_input_meter(level);
        self.set_input_pitch(pitch);
        }
        #[cfg(not(windows))]
        let _ = ctx;
    }

    #[cfg(windows)]
    fn desired_live_request(&mut self) -> Option<LiveRequest> {
        if !self.settings {
            let analysis_model = PathBuf::from(self.model_paths[0].trim());
            let synthesis_model = PathBuf::from(self.model_paths[1].trim());
            self.live.committed_models = if analysis_model.as_os_str().is_empty() || synthesis_model.as_os_str().is_empty() {
                None
            } else {
                Some(LiveModels { analysis_model, synthesis_model,
                    profile: crate::prepare_quality::Preferences::from_document(&self.preset.document)
                        .and_then(|p| p.profile()), multithread: self.live.multithread })
            };
        }

        if self.settings && let Some(models) = &mut self.live.committed_models {
            models.multithread = self.live.multithread;
        }
        let route = if self.live.condition_empty {
            LiveRoute::Direct
        } else if self.live.condition_available {
            LiveRoute::Target(self.live.committed_models.clone()?)
        } else {
            return None;
        };
        Some(LiveRequest { selection: self.audio_devices.selection.clone(), route })
    }

    pub(super) fn reconcile_live_audio(&mut self, ctx: &egui::Context) {
        #[cfg(windows)]
        {
            let closing = ctx.input(|input| input.viewport().close_requested());
            if closing {
                if let Some(session) = &self.live.session { session.stop(); }
                return;
            }

            let desired = self.desired_live_request();
            self.live.note_desired(desired);
            if let Some(session) = &self.live.session {
                if self.live.stop_due() { session.stop(); }
                ctx.request_repaint_after(Duration::from_millis(33));
            } else if self.live.start_due() {
                if let Some(request) = self.live.desired.clone() {

                    self.live.attempted = Some(self.live.generation);
                    self.start_live_audio(request);
                    ctx.request_repaint();
                }
            }
        }
        #[cfg(windows)]
        self.present_live_errors(ctx);
        #[cfg(not(windows))]
        let _ = ctx;
    }

    #[cfg(windows)]
    fn present_live_errors(&mut self, ctx: &egui::Context) {
        if self.error.is_none() && self.notice.is_none() {
            if let Some(error) = self.live.take_request_errors() {
                self.error = Some(error);
                ctx.request_repaint();
            }
        }
    }

    #[cfg(windows)]
    fn queue_live_error(&mut self, error: String) {
        self.live.request_errors.push(PendingLiveError::Other(error));
    }

    #[cfg(windows)]
    fn start_live_audio(&mut self, request: LiveRequest) {
        let profile = match &request.route {
            LiveRoute::Direct => None,
            LiveRoute::Target(models) => match models.profile.clone() {
                Ok(profile) => Some(profile), Err(error) => { self.queue_live_error(error); return; }
            },
        };
        let configuration = match request.selection.duplex_configuration(crate::live_audio::interactive_policy()) {
            Ok(configuration) => configuration,
            Err(error) => { self.queue_live_error(error.to_string()); return; }
        };
        let sample_rate = configuration.capture.sample_rate;
        let input_channel = request.selection.input_channel;
        self.live.snapshot = None;
        let started = match request.route {
            LiveRoute::Direct => {
                let (query, writer) = crate::morph_processor::input_meter::channel();
                self.live.meter = Some(query);
                crate::live_audio::start_direct(configuration, crate::live_audio::DirectProcessor::new(input_channel, writer))
            }
            LiveRoute::Target(models) => {

                let reader = match self.live.bus.reader_retaining_target_on_empty() {
                    Ok(reader) => reader, Err(error) => { self.queue_live_error(error); return; }
                };
                let (query, writer) = crate::morph_processor::input_meter::channel();
                self.live.meter = Some(query);
                crate::live_audio::start(configuration, move || {
                    crate::morph_processor::build_processor_with_instance_policy(
                        &models.analysis_model, &models.synthesis_model, reader, sample_rate,
                        input_channel, writer, profile.expect("target profile checked"), models.multithread)
                })
            }
        };
        match started {
            Ok(session) => {
                self.live.snapshot = Some(session.snapshot());
                self.live.session = Some(session);
                self.live.active = Some(self.live.generation);
            }
            Err(error) => {self.live.meter=None;self.queue_live_error(error);},
        }
    }

    pub(super) fn publish_live_condition(&mut self) {
        if self.layout_blocked() {

            self.live.condition_error = Some("Canvas change is pending; the last committed voice condition is retained.".into());
            return;
        }
        let parameters = crate::prototype_controls::Parameters {
            pregain_db: self.monitor_controls.pregain_db(),
            pitch_semitones: self.monitor_controls.pitch_semitones(),
            tone_db: 0.0,
        };
        let tone_preview = self.color_picker && self.color_source.is_none();
        let enabled_target = self.preset.sources.iter().any(|source|
            source.enabled && source.points.iter().any(|point| point.enabled));
        if !tone_preview && !enabled_target {

            self.live.condition_available = false;
            let result = self.live.bus.publish_empty();
            self.live.condition_empty = result.is_ok();
            if result.is_ok() { self.live.set_restored_canvas(None); }
            self.live.condition_error = result.err();
            return;
        }
        let result = if tone_preview {
            self.live.bus.publish_tone_preview(self.tone_controls(), parameters)
        } else {
            let selection = if self.experimental.enabled {
                self.experimental_mix().and_then(|mix| self.live.canvas_mix_cache.get_or_build(
                    self.revision, &mix, parameters,
                    || crate::prototype_controls::from_controller_mix(&self.preset, &mix, parameters)))
            } else {
                match &self.layout_view {
                    Some(view) => view.mix.as_ref().ok_or_else(||
                        "The current canvas has no completed pre-display controller mix.".to_owned())
                        .and_then(|mix| self.live.canvas_mix_cache.get_or_build(self.revision, mix, parameters,
                            || crate::prototype_controls::from_controller_mix(&self.preset, mix, parameters))),
                    None => Err("Live conditioning requires the original controller mix, not an inspection-mode distance blend.".into()),
                }
            };
            selection.map(|mut snapshot| {
                if let Some(embedding) = self.live.restored_canvas() {
                    snapshot.embedding = embedding;
                    snapshot.targets.clear();
                }
                snapshot
            }).and_then(|snapshot| self.live.bus.publish_canvas(snapshot))
        };
        self.live.condition_empty = false;
        match result {
            Ok(_) => {
                if tone_preview { self.live.set_restored_canvas(None); }
                self.live.condition_available = true;
                self.live.condition_error = None;
            }
            Err(error) => {
                let _ = self.live.bus.publish_unavailable();
                self.live.condition_available = false;
                self.live.condition_error = Some(error);
            }
        }
    }
}

