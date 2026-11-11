use crate::{
    Result,
    canvas::{Anchor, Curve, Point, Probe, Ray, Scene},
    preset::Preset,
};
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, TextureHandle, pos2, vec2,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[path = "desktop_audio.rs"]
mod audio;
#[path = "desktop_audio_decode.rs"]
mod audio_decode;
#[cfg(windows)]
#[path = "desktop_audio_devices.rs"]
mod audio_devices;
#[path = "desktop_canvas_state.rs"]
mod canvas_state;
#[path = "desktop_captions.rs"]
mod captions;
#[path = "desktop_controls.rs"]
mod controls;
#[path = "desktop_resources.rs"]
mod resources;
#[path = "desktop_model_inspection.rs"]
mod model_inspection;
#[path = "desktop_empty_background.rs"]
mod empty_background;
#[path = "desktop_canvas_raster.rs"]
mod canvas_raster;
#[path = "desktop_background_sqrt.rs"]
mod background_sqrt;
#[path = "desktop_background_inputs.rs"]
mod background_inputs;
#[path = "desktop_generation.rs"]
mod generation;
#[path = "desktop_geometry.rs"]
#[allow(dead_code)]
mod geometry;
#[path = "desktop_layout.rs"]
mod layout;
#[path = "desktop_library.rs"]
mod library;
#[path = "desktop_localization.rs"]
mod localization;
#[path = "desktop_monitor_controls.rs"]
mod monitor;
#[path = "desktop_waypoints.rs"]
mod waypoints;
pub(crate) use waypoints::{key_bit as waypoint_key_bit, ALL_KEY_BITS as WAYPOINT_KEY_BITS};
#[path = "desktop_live.rs"]
mod live;
#[path = "desktop_host_restore.rs"]
mod host_restore;
#[path = "desktop_editor_session.rs"]
mod editor_session;
pub(crate) use editor_session::CompletedEditorSession;
pub(crate) use host_restore::ExpectedCondition;
#[path = "desktop_icons.rs"]
mod icons;
#[path = "desktop_original_glyphs.rs"]
mod original_glyphs;
#[path = "runtime/ui/glyph_fit.rs"]
mod glyph_fit;
#[path = "desktop_glyph_math.rs"]
mod glyph_math;
#[path = "runtime/ui/glyph_transform.rs"]
mod glyph_transform;
#[path = "desktop_glyph_transform.rs"]
mod glyph_transform_values;
#[path = "runtime/ui/header_glyph_layout.rs"]
mod header_glyph_layout;
#[path = "desktop_ui.rs"]
mod look;
#[path = "desktop_paint.rs"]
mod paint;
#[path = "desktop_frame_cache.rs"]
mod frame_cache;
#[path = "desktop_probe_ring.rs"]
mod probe_ring;
#[allow(dead_code)]
#[path = "runtime/ui/canvas_primitives.rs"]
mod canvas_primitives;
#[allow(dead_code)]
#[path = "runtime/ui/canvas_placement64.rs"]
mod canvas_placement64;
#[allow(dead_code)]
#[path = "runtime/ui/monitor_expansion.rs"]
mod monitor_expansion;
#[allow(dead_code)]
#[path = "runtime/ui/monitor_input.rs"]
mod monitor_input;
#[allow(dead_code)]
#[path = "desktop_monitor_math.rs"]
mod monitor_math;
#[path = "desktop_bitmap_placement.rs"]
mod bitmap_placement;
#[allow(dead_code)]
#[path = "desktop_partial.rs"]
mod partial;
#[path = "desktop_save.rs"]
mod saved;
#[path = "desktop_source_colors.rs"]
mod source_colors;
#[path = "desktop_source_menu.rs"]
mod source_menu;
#[path = "desktop_source_math.rs"]
mod source_math;
#[path = "desktop_source_distance.rs"]
mod source_distance;
#[path = "desktop_source_names.rs"]
mod source_names;
#[path = "desktop_source_radial.rs"]
mod source_radial;
#[path = "desktop_canvas_viewport.rs"]
mod canvas_viewport;
#[path = "desktop_source_camera.rs"]
mod source_camera;
#[path = "desktop_canvas_gesture.rs"]
pub(crate) mod canvas_gesture;
#[path = "desktop_source_parameter_math.rs"]
mod source_parameter_math;
#[path = "desktop_transactions.rs"]
mod transactions;
use transactions::Publication;
#[path = "desktop_tone.rs"]
mod tone;
#[path = "desktop_tone_hex.rs"]
mod tone_hex;
#[path = "desktop_tone_ui.rs"]
mod tone_ui;

#[allow(dead_code)]
#[path = "desktop_tone_surface.rs"]
mod native_tone;
use look::*;

pub fn run(path: Option<PathBuf>) -> Result<()> {
    source_math::initialize_current_process();
    let mut gpu = eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    #[cfg(windows)]
    {
        gpu.instance_descriptor.backends = wgpu::Backends::DX12;
        gpu.instance_descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::Fxc;
    }
    #[cfg(target_os = "linux")]
    {
        gpu.instance_descriptor.backends = wgpu::Backends::VULKAN | wgpu::Backends::GL;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([860.0, 560.0])
            .with_min_inner_size([860.0, 560.0])
            .with_maximize_button(false)
            .with_icon(egui::IconData {
                rgba: include_bytes!("../assets/app-icon.rgba").to_vec(),
                width: 256,
                height: 256,
            }),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(gpu),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "Rocoflex",
        options,
        Box::new(move |cc| Ok(Box::new(DesktopApp::new(cc, path)))),
    )
    .map_err(|e| e.to_string())
}

#[derive(Clone, Copy, PartialEq)]
enum Menu {
    Library,
    Actions,
    Quality,
}
#[derive(Clone, Copy, PartialEq)]
enum Edit {
    WaypointKey(usize),
    SourceName(usize),
    PresetName,
    SaveAs,
    DeletePreset,
}
#[derive(Clone)]
struct LibraryEntry {
    name: String,
    path: PathBuf,
}
#[derive(Clone)]
struct VoiceEntry {
    name: String,
    source: Value,
}
#[derive(Clone, Copy)]
enum Drag {
    Probe,
    ProbeFromSelection,
    Source(usize),
    Label(usize),
}

enum ProbeGesture {
    WindowsWheelLine(f32),
    Magnify(f32),
}

struct PrimaryInput {
    armed_release: bool,
    primary_click: bool,
    completed_click: Option<Pos2>,
    completed_drag: Option<canvas_gesture::CompletedDrag>,
    press_position: Option<Pos2>,
}

pub struct DesktopApp {
    plugin_hosted: bool,
    preset: Preset,
    original: String,
    path: Option<PathBuf>,
    dirty: bool,
    scene: Scene,
    probe: Probe,
    rays: Vec<Ray>,
    shadow: Option<TextureHandle>,
    backdrop: Option<TextureHandle>,
    backdrop_key: Option<[u64; 6]>,
    backdrop_updated: Option<Instant>,
    cache_key: Option<[u64; 6]>,
    revision: u64,
    camera: f64,
    viewport_latched: bool,
    source_camera: Option<source_camera::Cached>,
    monitor_controls: monitor::MonitorControls,
    live: live::Controller,
    settings: bool,
    settings_resources: bool,
    settings_scroll_reset: bool,
    menu: Option<Menu>,
    voices_open: bool,
    editor: Option<Edit>,
    input: String,
    focus_editor: bool,
    error: Option<String>,
    notice: Option<String>,
    model_paths: [String; 3],
    ffmpeg_path: String,
    ffmpeg_detected: Option<PathBuf>,
    #[cfg(windows)]
    audio_devices: audio_devices::State,
    model_info: String,
    model_inspection: model_inspection::Controller,
    userdata_load: resources::UserdataLoader,
    show_tooltips: bool,
    locale: localization::Locale,
    mapping: usize,
    native_layout: bool,
    layout: canvas_state::CanvasState,
    layout_view: Option<transactions::LayoutView>,
    pending_layout: Option<transactions::PendingLayout>,
    layout_status: Option<String>,
    layout_next: Option<Instant>,
    library: Vec<LibraryEntry>,
    library_voices: Vec<VoiceEntry>,
    factory_voices: Vec<VoiceEntry>,
    factory_error: Option<String>,
    library_dir: PathBuf,
    preset_paths: Vec<library::PresetPath>,
    library_scan: library::Scanner,
    selected: Option<usize>,
    selected_at: Pos2,
    source_menu_cache: Option<source_menu::Cached>,
    source_math_failure: Option<source_math::Failure>,
    source_distance_failure: Option<source_distance::Failure>,
    source_pointer_failure: Option<canvas_viewport::Failure>,
    source_pointer_completed: Option<canvas_viewport::Completed>,
    probe_ring: Option<probe_ring::Cached>,
    native_frame_cache: frame_cache::Cache,
    empty_background: empty_background::EmptyBackground,
    background_inputs: background_inputs::Published,
    source_for_curve: Vec<usize>,
    source_for_anchor: Vec<usize>,
    offsets: Vec<Point>,
    label_offsets: Vec<Point>,
    label_rects: Vec<(usize, Rect)>,
    caption_placements: Vec<Option<captions::Cached>>,
    undo: Vec<transactions::UndoEntry>,
    drag: Option<Drag>,
    allow_canvas_editing: bool,
    color_source: Option<usize>,
    color: [u8; 3],
    color_hex: String,
    color_picker: bool,
    tone_hue: f64,
    tone_position: Point,
    tone_texture: Option<TextureHandle>,
    tone_texture_hue: Option<i32>,
    native_tone_texture: Option<TextureHandle>,
    native_tone_texture_key: Option<[u32; 4]>,
    native_tone_controls: Option<[f32; 3]>,
    native_tone_hex: tone_hex::ToneHexEdit,
    tone_epoch: u64,
    tone_job: Option<generation::PendingToneJob>,
    audio_job: Option<audio::PendingAudioJob>,
    audio_epochs: audio::AudioEpochs,
    audio_status: String,
    overlay_rects: Vec<Rect>,
    menu_rects: Vec<Rect>,
    picker_rect: Option<Rect>,
}

impl DesktopApp {
    fn configure_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "rocoflex".into(),
            egui::FontData::from_static(include_bytes!(
                "../assets/fonts/NotoSansCJKjp-Regular.otf"
            ))
            .into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("rocoflex".into());
        ctx.set_fonts(fonts);
    }

    pub fn new(cc: &eframe::CreationContext<'_>, path: Option<PathBuf>) -> Self {
        Self::new_with_context(&cc.egui_ctx, path)
    }

    fn new_with_context(ctx: &egui::Context, path: Option<PathBuf>) -> Self {
        Self::configure_fonts(ctx);
        ctx.set_theme(egui::Theme::Dark);
        let mut style = (*ctx.global_style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = Color32::from_rgb(42, 43, 53);
        style.visuals.window_fill = Color32::from_rgb(83, 83, 91);
        style.visuals.extreme_bg_color = Color32::from_rgb(63, 63, 71);
        style.visuals.selection.bg_fill = Color32::from_rgb(99, 113, 122);
        style.visuals.selection.stroke = Stroke::new(1.0, TEXT);
        style.visuals.widgets.inactive.bg_fill = Color32::from_black_alpha(45);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        style.visuals.widgets.hovered.bg_fill = Color32::from_black_alpha(65);
        style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        style.visuals.widgets.active.bg_stroke = Stroke::NONE;
        style.visuals.override_text_color = Some(TEXT);
        style.spacing.item_spacing = vec2(10.0, 10.0);
        style.spacing.interact_size = vec2(36.0, 30.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(15.0));
        ctx.set_global_style(style);
        let library_dir = preferences_dir().join("presets");
        let preset_paths = library::default_preset_paths(&library_dir);
        let mut app = Self {
            plugin_hosted: false,
            preset: blank_preset(),
            original: String::new(),
            path: None,
            dirty: false,
            scene: Scene::default(),
            probe: Probe {
                position: Point::default(),
                radius: 0.1,
            },
            rays: Vec::new(),
            shadow: None,
            backdrop: None,
            backdrop_key: None,
            backdrop_updated: None,
            cache_key: None,
            revision: 0,
            camera: 1.0,
            viewport_latched: false,
            source_camera: None,
            monitor_controls: monitor::MonitorControls::default(),
            live: live::Controller::default(),
            settings: false,
            settings_resources: false,
            settings_scroll_reset: true,
            menu: None,
            voices_open: false,
            editor: None,
            input: String::new(),
            focus_editor: false,
            error: None,
            notice: None,
            model_paths: Default::default(),
            ffmpeg_path: String::new(),
            ffmpeg_detected: audio_decode::detect_ffmpeg(),
            #[cfg(windows)]
            audio_devices: audio_devices::State::default(),
            model_info: String::new(),
            model_inspection: Default::default(),
            userdata_load: Default::default(),
            show_tooltips: true,
            locale: localization::Locale::default(),
            mapping: 0,
            native_layout: false,
            layout: canvas_state::CanvasState::Inspection,
            layout_view: None,
            pending_layout: None,
            layout_status: None,
            layout_next: None,
            library: Vec::new(),
            library_voices: Vec::new(),
            factory_voices: Vec::new(),
            factory_error: None,
            library_dir,
            preset_paths,
            library_scan: library::Scanner::default(),
            selected: None,
            selected_at: Pos2::ZERO,
            source_menu_cache: None,
            source_math_failure: None,
            source_distance_failure: None,
            source_pointer_failure: None,
            source_pointer_completed: None,
            probe_ring: None,
            native_frame_cache: Default::default(),
            empty_background: empty_background::EmptyBackground::default(),
            background_inputs: background_inputs::Published::default(),
            source_for_curve: Vec::new(),
            source_for_anchor: Vec::new(),
            offsets: Vec::new(),
            label_offsets: Vec::new(),
            label_rects: Vec::new(),
            caption_placements: Vec::new(),
            undo: Vec::new(),
            drag: None,
            allow_canvas_editing: false,
            color_source: None,
            color: [197, 71, 71],
            color_hex: "C54747".into(),
            color_picker: false,
            tone_hue: 0.0,
            tone_position: Point::new(0.5, 0.5),
            tone_texture: None,
            tone_texture_hue: None,
            native_tone_texture: None,
            native_tone_texture_key: None,
            native_tone_controls: None,
            native_tone_hex: tone_hex::ToneHexEdit::default(),
            tone_epoch: 0,
            tone_job: None,
            audio_job: None,
            audio_epochs: audio::AudioEpochs::default(),
            audio_status: String::new(),
            overlay_rects: Vec::new(),
            menu_rects: Vec::new(),
            picker_rect: None,
        };
        if path.is_none() {
            app.initialize_actual_no_file_startup();
        }
        app.read_preferences();
        app.load_factory_voices();
        app.scan_library();
        if let Some(path) = path {
            if let Err(e) = app.load(path) {
                app.error = Some(e);
            }
        }
        app
    }

    pub(crate) fn new_plugin(ctx: &egui::Context, bus: crate::current_condition::ConditionBus) -> Self {
        source_math::initialize_current_process();
        let mut app = Self::new_with_context(ctx, None);
        app.plugin_hosted = true;
        app.live.bus = bus;
        app
    }

    pub(crate) fn plugin_document_revision(&self) -> u64 { self.revision }

    pub(crate) fn adopt_plugin_multithread(&mut self, value: bool) {
        self.live.adopt_hosted_multithread(value);
    }
    pub(crate) fn set_plugin_multithread_mailbox(&mut self,
        mailbox: std::sync::Arc<std::sync::atomic::AtomicU8>) {
        self.live.set_multithread_mailbox(mailbox);
    }

    pub(crate) fn plugin_layout_pending(&self) -> bool { self.layout_blocked() }

    pub(crate) fn plugin_shortcut_mask(&self, ctx: &egui::Context) -> u64 {
        let global = !self.layout_blocked() && self.editor.is_none()
            && !self.settings && !self.color_picker && self.error.is_none() && self.notice.is_none();
        let escape = self.error.is_some() || self.notice.is_some()
            || (self.editor.is_none() && !self.settings && !egui::Popup::is_any_open(ctx)
                && (self.color_picker || self.menu.is_some() || self.drag.is_some() || self.waypoint_active()))
            || matches!(self.editor, Some(Edit::WaypointKey(_)));
        u64::from(global) | (u64::from(global && self.selected.is_some()) << 1)
            | (u64::from(escape) << 2) | self.waypoint_key_mask(ctx)
    }

    pub(crate) fn try_report_plugin_error(&mut self, message: &str) -> bool {
        if !self.plugin_hosted || self.layout_blocked()
            || self.error.is_some() || self.notice.is_some() {
            return false;
        }
        self.error = Some(message.to_owned());
        true
    }

    pub(crate) fn restore_plugin_condition(&mut self, condition: &crate::current_condition::CurrentCondition) -> bool {
        if self.layout_blocked() { return false; }
        self.monitor_controls.set_values(condition.pregain_db, condition.pitch_semitones);
        match &condition.target {
            crate::current_condition::ConditionTarget::TonePreview { controls } => {
                if !self.color_picker || self.color_source.is_some() { self.begin_color(None); }
                self.native_tone_controls = Some(*controls);
                self.tone_position = Point::new(f64::from(controls[1]), 1.0 - f64::from(controls[0]));
                self.tone_hue = f64::from(controls[2]) * 360.0;
                self.sync_native_color_label();
            }
            crate::current_condition::ConditionTarget::Canvas128 { .. } => {
                if self.color_picker && self.color_source.is_none() { self.color_picker = false; }
            }
        }
        true
    }

    pub(crate) fn plugin_document(&self) -> Result<String> {
        Ok(self.prepare_saved_snapshot(None)?.serialized)
    }

    pub(crate) fn plugin_model_paths(&self) -> [&str; 2] {
        [&self.model_paths[0], &self.model_paths[1]]
    }

    pub(crate) fn set_plugin_model_paths(&mut self, analysis: String, synthesis: String) {
        if self.model_paths[0] != analysis {
            self.model_paths[0] = analysis;
            self.model_info.clear();
            self.model_inspection.invalidate(&self.model_paths[0]);
            self.audio_model_changed();
        }
        if self.model_paths[1] != synthesis {
            self.model_paths[1] = synthesis;
            self.userdata_load.invalidate(&self.model_paths[1]);
        }
    }

    pub(crate) fn restore_plugin_document(&mut self, text: &str, expected: &ExpectedCondition) -> Result<()> {
        expected.validate()?;
        if self.layout_blocked() { return Err("A canvas transaction is still pending.".into()); }
        if text.is_empty() {
            expected.verify(&blank_preset(), None)?;
            if !self.install(blank_preset()) { return Err("Empty preset reset is pending canvas completion.".into()); }
            self.original.clear();
            self.path = None;
            self.dirty = false;
            self.undo.clear();
            expected.apply_controls(self);
            return Ok(());
        }
        let document = serde_json::from_str(text).map_err(|e| format!("Invalid preset state: {e}"))?;
        let preset = Preset::parse(document)?;
        if !self.publish_host_install(preset, expected.clone()) {
            return Err("Preset state is retained: the saved canvas and voice target could not be restored together.".into());
        }
        self.original = text.to_owned();
        self.path = None;
        self.dirty = false;
        self.undo.clear();
        Ok(())
    }

    pub fn load(&mut self, path: PathBuf) -> Result<()> {
        if self.layout_blocked() {
            return Ok(());
        }
        let text = library::read_preset_text(&path)?;
        self.load_owned_preset(path, text)
    }
    fn load_owned_preset(&mut self, path: PathBuf, text: String) -> Result<()> {
        if self.layout_blocked() {
            return Ok(());
        }
        let doc = serde_json::from_str(&text).map_err(|e| format!("Invalid preset: {e}"))?;
        let preset = Preset::parse(doc)?;
        if !self.install(preset) {
            if let Some(pending) = &mut self.pending_layout {
                pending.loaded_input = Some((path, text));
            }
            return Ok(());
        }
        self.original = text;
        self.path = Some(path);
        self.dirty = false;
        self.undo.clear();
        self.close_overlays();
        Ok(())
    }
    fn install(&mut self, preset: Preset) -> bool {
        self.publish_install(preset)
    }
    fn close_overlays(&mut self) {
        self.waypoint_cancel();
        self.menu = None;
        self.voices_open = false;
        self.settings = false;
        self.color_picker = false;
        self.editor = None;
        self.drag = None;
    }
    fn clear(&mut self) {
        if self.layout_blocked() {
            return;
        }
        let previous = self.capture_undo();
        if !self.install(blank_preset()) {
            return;
        }
        self.remember_entry(previous);
        self.original.clear();
        self.path = None;
        self.dirty = false;
        self.notice = None;
        self.close_overlays();
    }
    fn open(&mut self) {
        if self.layout_blocked() {
            return;
        }
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Preset JSON", &["json"])
            .pick_file()
        {
            if let Err(e) = self.load(p) {
                self.error = Some(e);
            }
        }
    }
    fn remember(&mut self) {
        let entry = self.capture_undo();
        self.remember_entry(entry);
    }
    fn undo(&mut self) {
        self.undo_committed_layout();
    }
    fn changed(&mut self) {
        self.waypoint_cancel();
        self.tone_epoch = self.tone_epoch.wrapping_add(1);
        self.dirty = true;
        self.revision = self.revision.wrapping_add(1);
        self.cache_key = None;
    }
    fn reparse(&mut self) {
        if self.layout_blocked() {
            return;
        }
        match Preset::parse(self.preset.document.clone()) {
            Ok(p) => {
                self.waypoint_reset();
                let target_changed = !host_restore::same_canvas_sources(&self.preset, &p);
                self.preset = p;
                self.selected = None;
                self.drag = None;
                self.rebuild();
                if target_changed && self.current_layout_matches_document() {
                    self.live.set_restored_canvas(None);
                }
                self.changed();
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn probe_changed(&mut self, proposed: Probe) -> bool {
        if self.layout_blocked()
            || !proposed.position.x.is_finite()
            || !proposed.position.y.is_finite()
            || !proposed.radius.is_finite()
            || proposed.radius <= 0.0
        {
            return false;
        }
        let previous = self.probe;
        self.probe = proposed;
        if !self.publish_probe_event() {
            self.probe = previous;
            return false;
        }
        if previous.position.x.to_bits() != proposed.position.x.to_bits()
            || previous.position.y.to_bits() != proposed.position.y.to_bits()
            || previous.radius.to_bits() != proposed.radius.to_bits()
        {
            self.live.set_restored_canvas(None);
        }
        let v = voice_mut(&mut self.preset.document);
        v["probeLocationX"] = json!(self.probe.position.x);
        v["probeLocationY"] = json!(self.probe.position.y);
        v["probeRadius"] = json!(self.probe.radius);
        self.changed();
        true
    }
    fn latch_viewport(&mut self) -> bool {

        if let Some(camera) = source_camera::Cached::update(&mut self.source_camera,
            [self.probe.position.x as f32, self.probe.position.y as f32])
        {
            self.camera = camera;
            self.viewport_latched = true;
            true
        } else {
            false
        }
    }
    fn probe_at(&mut self, at: Pos2, rect: Rect) -> std::result::Result<Point, canvas_viewport::Failure> {
        let completed = canvas_viewport::project(at, rect, self.camera)?;
        let position = completed.position();
        self.source_pointer_completed = Some(completed);
        Ok(Point::new(f64::from(position[0]), f64::from(position[1])))
    }
    fn persist_layout(&mut self) {
        if self.layout_blocked() {
            return;
        }
        self.preset.document["rocoflexCanvas"] =
            json!({"sourceOffsets":self.offsets,"labelOffsets":self.label_offsets});
        self.changed();
    }
    fn realtime(&self) -> bool {
        crate::prepare_quality::Preferences::from_document(&self.preset.document)
            .map(|p|p.realtime).unwrap_or(true)
    }
    fn quality(&self) -> usize {
        crate::prepare_quality::Preferences::from_document(&self.preset.document)
            .map(|p|p.label.index()).unwrap_or(0)
    }
    fn set_quality(&mut self, index: usize) {
        if self.layout_blocked() { return; }
        if index>=quality_names().len() {return;}
        self.remember();
        self.preset.document["realtimeLatencyOption"] = json!(quality_names()[index]);
        if voice(&self.preset.document).get("quality").is_some() {
            voice_mut(&mut self.preset.document)["quality"] =
                json!(["highest", "higher", "lower", "lowest"][index]);
        }
        self.changed();
        self.menu = None;
    }

    fn rebuild(&mut self) {
        if self.layout_blocked() {
            return;
        }
        self.sync_layout();
        self.rebuild_scene();
    }

    fn sync_layout(&mut self) {
        self.sync_committed_layout();
    }

    fn advance_layout(&mut self, ctx: &egui::Context) {
        self.step_committed_layout(ctx);
    }

    fn rebuild_scene(&mut self) {
        self.cache_key = None;
        self.revision = self.revision.wrapping_add(1);
        let native = if self.mapping == 0 {
            self.layout_view.as_ref().map(|view| view.positions.clone())
        } else {
            None
        };
        self.native_layout = native.is_some();
        let mut raw = Vec::new();
        for (s, source) in self.preset.sources.iter().enumerate() {
            for (i, p) in source.points.iter().enumerate() {
                let xy = if let Some(nodes) = &native {
                    nodes[s][i]
                } else if self.mapping == 2 {
                    self.preset.basis_points[s][i]
                } else {
                    [p.reduced[0], p.reduced[1]]
                };
                raw.push(Point::new(xy[0] as f64, xy[1] as f64));
            }
        }
        let min = Point::new(
            raw.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
            raw.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
        );
        let max = Point::new(
            raw.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
            raw.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
        );
        let center = if raw.is_empty() {
            Point::default()
        } else {
            Point::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5)
        };
        let scale = if raw.is_empty() {
            1.0
        } else {
            (2.5 / (max.x - min.x).max(0.01)).min(1.7 / (max.y - min.y).max(0.01))
        };
        let mut curves = Vec::new();
        let mut points = Vec::new();
        let mut curve_sources = Vec::new();
        let mut point_sources = Vec::new();
        let mut cursor = 0;
        for (index, source) in self.preset.sources.iter().enumerate() {
            let color = parse_color(&source.color);
            let offset = self.offsets.get(index).copied().unwrap_or_default();
            let mut nodes = Vec::new();
            for point in &source.points {
                let p = raw[cursor];
                cursor += 1;
                if source.enabled && point.enabled {
                    nodes.push(if self.native_layout {
                        Point::new(p.x + offset.x, p.y + offset.y)
                    } else {
                        Point::new(
                            (p.x - center.x) * scale + offset.x,
                            (p.y - center.y) * scale + offset.y,
                        )
                    });
                }
            }
            if !source.enabled {
                continue;
            }
            if source.kind == "trajectory" {
                curves.push(Curve {
                    path: if self.native_layout {
                        nodes.clone()
                    } else {
                        smooth_path(&nodes)
                    },
                    nodes,
                    color,
                });
                curve_sources.push(index);
            } else if let Some(position) = nodes.first() {
                points.push(Anchor {
                    position: *position,
                    color,
                    curve: None,
                    index: points.len(),
                });
                point_sources.push(index);
            }
        }
        self.scene = Scene::new(curves, points);
        self.source_for_curve = curve_sources;
        self.source_for_anchor = self
            .scene
            .anchors
            .iter()
            .map(|a| {
                if let Some(c) = a.curve {
                    self.source_for_curve[c]
                } else {
                    point_sources[a.index]
                }
            })
            .collect();
    }

    fn scan_library(&mut self) {
        self.library.clear();
        self.library_voices.clear();
        if let Err(error) = self.library_scan.request(&self.preset_paths) {
            self.error = Some(error);
        }
    }
    fn poll_library(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.library_scan.poll(ctx) {
            match result {
                Ok(result) => { self.library = result.entries; self.library_voices = result.voices; }
                Err(error) => self.error = Some(error),
            }
        }
    }
    fn append_voice(&mut self, entry: &VoiceEntry) {
        if let Err(error) = self.append_source(entry.source.clone()) {
            self.error = Some(error);
        }
    }
    fn append_source(&mut self, mut source: Value) -> Result<Publication> {
        if source["type"] == "trajectory" {
            source["color"] = json!(source_colors::next_color(
                self.preset
                    .sources
                    .iter()
                    .map(|source| source.color.as_str()),
            ));
        }
        Ok(self.publish_append(source))
    }
    fn export(&mut self) {
        let name = format!("{}.json", safe_filename(&self.preset.name));
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Preset JSON", &["json"])
            .set_file_name(name);
        if let Some(p) = self.path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(p);
        }
        if let Some(path) = dialog.save_file() {
            match self.prepare_saved_snapshot(None).and_then(|snapshot| {
                fs::write(&path, &snapshot.serialized)
                    .map(|_| snapshot)
                    .map_err(|e| e.to_string())
            }) {
                Ok(snapshot) => {
                    self.commit_saved_snapshot(snapshot);
                    self.waypoint_reset();
                }
                Err(e) => self.error = Some(e),
            }
        }
        self.menu = None;
    }
    #[allow(dead_code)]
    fn serialize(&self) -> Result<String> {
        self.prepare_saved_snapshot(None)
            .map(|snapshot| snapshot.serialized)
    }
    fn preset_needs_name(&self) -> bool {
        self.path.is_none() && self.preset.name == "My Preset"
    }
    fn save_library(&mut self, name: String, rename: bool) -> Result<()> {
        if self.layout_blocked() && name != self.preset.name {
            return Err("Discard the retained canvas change before renaming this preset.".into());
        }
        let preserves_loaded_name = rename && self.path.is_some() && name == self.preset.name;
        if name.is_empty() || (name == "My Preset" && !preserves_loaded_name) {
            return Err("Give this preset a name first.".into());
        }
        let renamed = name != self.preset.name;
        let snapshot = self.prepare_saved_snapshot(Some(&name))?;
        let same = self.path.as_ref().filter(|path|
            library::contains_preset(&self.preset_paths, &self.library_dir, path)).cloned();
        if rename && self.path.is_some() && same.is_none() {
            return Err("The preset is outside the configured preset folders or is no longer available.".into());
        }
        fs::create_dir_all(&self.library_dir).map_err(|e| e.to_string())?;
        let path = if rename && let Some(path) = same {
            fs::write(&path, &snapshot.serialized).map_err(|e| e.to_string())?;
            path
        } else {
            library::publish_new_preset(&self.library_dir, &name, |_| Ok(&snapshot))?.0
        };
        if renamed {
            self.audio_destination_changed();
        }
        self.commit_saved_snapshot(snapshot);
        self.waypoint_reset();
        self.path = Some(path);
        if !library::has_root(&self.preset_paths, &self.library_dir) {
            self.preset_paths.push(library::PresetPath {
                path: self.library_dir.to_string_lossy().into_owned(), recursive: false,
            });
            self.save_preferences();
        }
        self.editor = None;
        self.menu = None;
        self.scan_library();
        Ok(())
    }
    fn save_current(&mut self) {
        if self.preset_needs_name() {
            self.begin_edit(Edit::SaveAs);
            return;
        }
        let name = self.preset.name.clone();
        if let Err(e) = self.save_library(name, true) {
            self.error = Some(e);
        }
    }
    fn import_library(&mut self) {
        if self.layout_blocked() {
            return;
        }
        self.menu = None;
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Preset JSON", &["json"])
            .pick_files()
        {
            if let Err(e) = fs::create_dir_all(&self.library_dir) {
                self.error = Some(e.to_string());
                return;
            }
            let mut first = None;
            let mut failures = 0;
            for path in paths {
                let result = (|| -> Result<(PathBuf, String)> {
                    let text = library::read_preset_text(&path)?;
                    let mut p =
                        Preset::parse(serde_json::from_str(&text).map_err(|e| e.to_string())?)?;
                    let original_name = p.name.clone();
                    let published = library::publish_new_preset(
                        &self.library_dir,
                        &original_name,
                        |unique| {
                            if unique == original_name {
                                Ok(text.clone())
                            } else {
                                p.document["presetName"] = json!(unique);
                                serde_json::to_string_pretty(&p.document)
                                    .map_err(|e| e.to_string())
                            }
                        },
                    )?;
                    Ok(published)
                })();
                match result {
                    Ok(published) => {
                        if first.is_none() {
                            first = Some(published)
                        }
                    }
                    Err(_) => failures += 1,
                }
            }
            self.scan_library();
            if let Some((path, text)) = first {
                if let Err(e) = self.load_owned_preset(path, text) {
                    self.error = Some(e);
                }
            }
            if failures > 0 {
                self.notice = Some(format!(
                    "Could not import {failures} preset file(s). Existing preset files were not replaced."
                ));
            }
        }
    }
    fn export_library(&mut self) {
        self.menu = None;
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            let result = (|| -> Result<()> {
                for entry in &self.library {
                    let text = library::read_preset_text(&entry.path)?;
                    library::publish_new_preset(&folder, &entry.name, |_| Ok(text.as_str()))?;
                }
                Ok(())
            })();
            if let Err(e) = result {
                self.error = Some(e);
            }
        }
    }
    fn begin_edit(&mut self, edit: Edit) {
        if let Edit::WaypointKey(index) = edit { self.waypoint_begin_key(index); return; }
        if self.layout_blocked() {
            return;
        }
        self.menu = None;
        self.input = match edit {
            Edit::SourceName(i) => self
                .preset
                .sources
                .get(i)
                .map(|s| s.name.clone())
                .unwrap_or_default(),
            Edit::PresetName => {
                if self.preset_needs_name() {
                    String::new()
                } else {
                    self.preset.name.clone()
                }
            }
            Edit::SaveAs => {
                if self.preset.name == "My Preset" {
                    String::new()
                } else {
                    unique_library_name(&self.library_dir, &self.preset.name)
                }
            }
            Edit::DeletePreset => String::new(),
            Edit::WaypointKey(_) => unreachable!("dispatched before name editing"),
        };
        self.editor = Some(edit);
        self.focus_editor = true;
    }
    fn confirm_edit(&mut self) {
        if self.layout_blocked() || self.error.is_some() || self.notice.is_some() {
            return;
        }
        let Some(edit) = self.editor else {
            return;
        };
        let name = if matches!(edit, Edit::SourceName(_)) {
            self.input.clone()
        } else {
            self.input.trim().to_owned()
        };
        match edit {
            Edit::WaypointKey(_) => {
                if let Err(error) = self.waypoint_confirm_key() { self.error = Some(error); }
            }
            Edit::SourceName(i) => {
                if !name.as_bytes().first().is_some_and(|byte| *byte != 0) {
                    return;
                }
                let Some(current) = self.preset.sources.get(i) else {
                    return;
                };

                if source_names::equal(&current.name, &name) {
                    self.editor = None;
                    return;
                }
                self.remember();
                if let Some(s) = voice_mut(&mut self.preset.document)["sources"]
                    .as_array_mut()
                    .and_then(|a| a.get_mut(i))
                {
                    s["name"] = json!(name);
                }
                self.reparse();
                self.editor = None;
            }
            Edit::SaveAs | Edit::PresetName => {
                if let Err(e) = self.save_library(name, edit == Edit::PresetName) {
                    self.error = Some(e);
                }
            }
            Edit::DeletePreset => {
                if let Some(p) = &self.path {
                    if !library::contains_preset(&self.preset_paths, &self.library_dir, p) {
                        self.error = Some("The preset is outside the configured preset folders or is no longer available.".into());
                        return;
                    }
                    if let Err(e) = fs::remove_file(p) {
                        self.error = Some(e.to_string());
                        return;
                    }
                }
                self.clear();
                self.scan_library();
            }
        }
    }
    fn delete_source(&mut self, index: usize) {
        self.publish_remove(index);
    }
    fn begin_color(&mut self, index: Option<usize>) {
        if self.layout_blocked() {
            return;
        }
        self.tone_epoch = self.tone_epoch.wrapping_add(1);
        self.native_tone_controls = None;
        self.color_source = index;
        if index.is_none() {
            self.native_tone_controls = Some([0.5, 0.5, 0.0]);
            self.tone_position = Point::new(0.5, 0.5);
            self.tone_hue = 0.0;
            self.sync_native_color_label();
            self.color_picker = true;
            self.menu = None;
            return;
        }
        self.color = index
            .and_then(|i| self.preset.sources.get(i))
            .map(|s| parse_color(&s.color))
            .unwrap_or([197, 71, 71]);
        let fitted = tone::fit(self.color, self.tone_hue);
        self.color = fitted.rgb;
        self.tone_hue = fitted.hue;
        self.tone_position = Point::new(fitted.x, fitted.y);
        self.color_hex = format!(
            "{:02X}{:02X}{:02X}",
            self.color[0], self.color[1], self.color[2]
        );
        if index.is_none() {
            self.sync_native_color_label();
        }
        self.color_picker = true;
        self.menu = None;
    }
    fn apply_color(&mut self) {
        if self.layout_blocked() {
            return;
        }
        if let Some(index) = self.color_source {
            self.remember();
            if let Some(source) = voice_mut(&mut self.preset.document)["sources"]
                .as_array_mut()
                .and_then(|a| a.get_mut(index))
            {
                source["color"] = json!(format!(
                    "ff{:02x}{:02x}{:02x}",
                    self.color[0], self.color[1], self.color[2]
                ));
            }
            self.reparse();
            self.color_picker = false;
        }
    }
    fn read_preferences(&mut self) { self.read_preferences_at(&preferences_dir()); }

    fn read_preferences_at(&mut self, dir: &Path) {

        if let Ok(value) = try_read_multithread_preference_at(&dir.join(MULTITHREAD_PREFERENCE_FILE)) {
            self.live.set_initial_multithread(value);
        }
        if let Ok(text) = fs::read_to_string(dir.join("settings.json")) {
            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                for (i, key) in ["model", "userdata", "license"].iter().enumerate() {
                    self.model_paths[i] = value[key].as_str().unwrap_or_default().into();
                }
                self.show_tooltips = value["showTooltips"].as_bool().unwrap_or(true);
                self.locale = localization::Locale::from_id(value["interfaceLanguage"].as_str().unwrap_or("en-us"));
                if let Some(paths) = value.get("presetPaths")
                    && let Ok(paths) = serde_json::from_value::<Vec<library::PresetPath>>(paths.clone())
                { self.preset_paths = paths; }
                self.ffmpeg_path = value["ffmpeg"].as_str().unwrap_or_default().into();
                #[cfg(windows)]
                {
                    self.audio_devices.selection = audio_devices::Selection::from_preferences(&value["audioDevices"]);
                }
            }
        }
    }
    fn save_preferences(&mut self) { self.save_preferences_at(&preferences_dir()); }

    fn save_multithread_edit(&mut self, previous: bool) {
        let dir = preferences_dir();
        if let Err(error) = self.live.commit_multithread_edit(previous, self.plugin_hosted,
            |value| write_multithread_preference_at(&dir, value)) {
            self.preferences_error(error);
        }
    }

    fn preferences_error(&mut self, error: std::io::Error) {
        let message = format!("Could not save preferences: {error}");
        if let Some(existing) = &mut self.error {
            existing.push_str("\n\n");
            existing.push_str(&message);
        } else {
            self.error = Some(message);
        }
    }

    fn save_preferences_at(&mut self, dir: &Path) {
        #[allow(unused_mut)]
        let mut data = json!({"model":self.model_paths[0],"userdata":self.model_paths[1],"license":self.model_paths[2],"ffmpeg":self.ffmpeg_path,"showTooltips":self.show_tooltips});
        data["presetPaths"] = json!(self.preset_paths);
        data["interfaceLanguage"] = json!(self.locale.id());
        #[cfg(windows)]
        {
            data["audioDevices"] = self.audio_devices.selection.to_preferences();
        }
        if let Err(e) = fs::create_dir_all(&dir).and_then(|_| {
            fs::write(
                dir.join("settings.json"),
                serde_json::to_string_pretty(&data).unwrap_or_default(),
            )
        }) {
            self.preferences_error(e);
        }
    }

    fn native_paint(&mut self, rect: Rect) -> Option<std::sync::Arc<paint::Frame>> {
        if !self.native_layout {
            return None;
        }
        let Some(view) = self.layout_view.as_ref() else {
            return Some(std::sync::Arc::new(paint::Frame::default()));
        };
        let positions = &view.positions;
        if positions.len() != self.preset.sources.len()
            || view.scalars.len() != self.preset.sources.len()
        {
            return Some(std::sync::Arc::new(paint::Frame::default()));
        }
        let mut sources = Vec::new();
        for (index, source) in self.preset.sources.iter().enumerate() {
            if positions[index].len() != source.points.len()
                || view.scalars[index].len() != source.points.len()
            {
                return Some(std::sync::Arc::new(paint::Frame::default()));
            }
            if !source.enabled {
                continue;
            }
            let offset = self.offsets.get(index).copied().unwrap_or_default();
            let points = source
                .points
                .iter()
                .enumerate()
                .filter_map(|(i, point)| {
                    if !point.enabled {
                        return None;
                    }
                    let xy = positions[index][i];
                    let at = world_to_screen(
                        Point::new(f64::from(xy[0]) + offset.x, f64::from(xy[1]) + offset.y),
                        rect,
                        self.camera,
                    );
                    Some(paint::PointInput {
                        index: i,
                        position: [at.x, at.y],
                        fields: view.scalars[index][i],
                    })
                })
                .collect();
            sources.push(paint::SourceInput {
                index,
                argb: paint::parse_argb(&source.color),
                trajectory: source.kind == "trajectory",
                selected: self.selected == Some(index),
                points,
            });
        }
        let probe = world_to_screen(self.probe.position, rect, self.camera);
        Some(self.native_frame_cache.get_or_build(sources, [probe.x, probe.y], self.camera))
    }

    fn paint_canvas(&mut self, ui: &egui::Ui, rect: Rect) {
        let native_frame = self.native_paint(rect);
        let painter = ui.painter().with_clip_rect(rect);
        let transform = |p| world_to_screen(p, rect, self.camera);

        let native_background = self.native_layout && self.layout_view.is_some();
        let empty_fallback = self.preset.sources.is_empty() && !native_background;
        let source_empty = empty_fallback || (native_background && self.preset.sources.is_empty()
            && self.layout_view.as_ref().is_some_and(|view| view.positions.is_empty() && view.scalars.is_empty()));
        let source_background = native_background || empty_fallback;

        let geometry = if empty_fallback { None } else { self.background_inputs.display_geometry() };
        self.empty_background.update(ui.ctx(), rect, self.probe, self.camera,
            source_background, source_empty, geometry, self.background_inputs.lineage());
        if source_background {
            painter.rect_filled(rect, 0.0, Color32::BLACK);
            if let Some(background) = self.empty_background.view().image {
                background.paint(&painter, rect);
            }
            if let Some(status) = self.empty_background.failure_status() {
                painter.text(rect.left_bottom() + vec2(16.0, -66.0), Align2::LEFT_BOTTOM,
                    status, FontId::proportional(12.0), Color32::from_rgb(232, 166, 166));
            }
        } else {
        quad(
            &painter,
            [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            if self.preset.sources.is_empty() {
                [Color32::from_rgb(42, 42, 52); 4]
            } else { [
                Color32::from_rgb(52, 55, 68),
                Color32::from_rgb(46, 49, 60),
                Color32::from_rgb(38, 41, 52),
                Color32::from_rgb(46, 49, 60),
            ] },
        );
        }
        if !source_background {
        for curve in &self.scene.curves {
            if !curve.nodes.is_empty() {
                let c = curve.nodes.iter().fold(Point::default(), |a, b| {
                    Point::new(
                        a.x + b.x / curve.nodes.len() as f64,
                        a.y + b.y / curve.nodes.len() as f64,
                    )
                });
                glow(
                    &painter,
                    transform(c),
                    rect.width() * 0.24,
                    curve.color,
                    0.12,
                );
            }
        }
        for anchor in self.scene.anchors.iter().filter(|a| a.curve.is_none()) {
            glow(
                &painter,
                transform(anchor.position),
                rect.width() * 0.16,
                anchor.color,
                0.16,
            );
        }
        }
        let key = [
            self.probe.position.x.to_bits(),
            self.probe.position.y.to_bits(),
            self.probe.radius.to_bits(),
            self.camera.to_bits(),
            ((rect.width().to_bits() as u64) << 32) | rect.height().to_bits() as u64,
            self.revision,
        ];
        if !source_background && self.cache_key != Some(key) {
            self.rays = self.scene.rays(self.probe);
            let polygons = self.scene.shadow_fan(self.probe, self.camera);
            let image = shadow_image(&polygons, rect, self.camera);
            if let Some(texture) = &mut self.shadow {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.shadow = Some(ui.ctx().load_texture(
                    "canvas-shadow-union",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
            self.cache_key = Some(key);
        }
        if !source_background && let Some(texture) = &self.shadow {
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        let center = transform(self.probe.position);
        let radius =
            self.probe.radius as f32 * rect.width().min(rect.height()) * 0.5 * self.camera as f32;
        if self.preset.sources.is_empty() {
            self.probe_ring = None;
        } else if let Some(frame) = native_frame.as_ref() {
            probe_ring::Cached::update(&mut self.probe_ring, ui.ctx(), frame, center, radius);
        } else {
            self.probe_ring = None;
        }
        if self.menu.is_some()
            || self.settings
            || self.editor.is_some()
            || self.color_picker
            || self.error.is_some()
            || self.notice.is_some()
        {
            let mut backdrop_key = key;
            backdrop_key[5] = backdrop_key[5].wrapping_add(self.empty_background.view().fingerprint);
            if let Some(frame) = &native_frame {
                backdrop_key[5] = backdrop_key[5].wrapping_add(frame.cache_key());

                if let Some(ring) = self.probe_ring.as_ref() {
                    backdrop_key[5] = backdrop_key[5].wrapping_add(ring.publication_fingerprint());
                }
            }
            if self.backdrop_key != Some(backdrop_key) {
                let polygons = if source_background { Vec::new() }
                    else { self.scene.shadow_fan(self.probe, self.camera) };
                let image = frosted_canvas(
                    &self.scene,
                    self.probe,
                    &self.rays,
                    &polygons,
                    rect,
                    self.camera,
                    native_frame.as_deref(),
                    self.probe_ring.as_ref().and_then(|cache| cache.rendered()),
                    self.empty_background.view(),
                );
                if let Some(texture) = &mut self.backdrop {
                    if texture.size() == image.size {
                        texture.set_partial([0, 0], image, egui::TextureOptions::LINEAR);
                    } else { texture.set(image, egui::TextureOptions::LINEAR); }
                } else {
                    self.backdrop = Some(ui.ctx().load_texture(
                        "canvas-frosted-backdrop",
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                self.backdrop_key = Some(backdrop_key);
                self.backdrop_updated = Some(Instant::now());
            }
            if let Some(texture) = &self.backdrop {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new("rocoflex-glass-backdrop"),
                        Backdrop {
                            texture: texture.clone(),
                            rect,
                        },
                    )
                });
            }
        } else { self.backdrop_updated = None; }
        if !source_background { glow(
            &painter,
            center,
            radius * 1.48 + 10.0,
            [244, 247, 255],
            0.026,
        ); }
        if let Some(frame) = &native_frame {
            frame.paint(&painter);
        } else if !source_background {
            for ray in self.rays.iter().rev() {
                let source = &self.scene.anchors[ray.source];
                let p = source.position;
                let d = p.distance(self.probe.position);
                if d < 1e-6 {
                    continue;
                }
                if ray.shadow_strength > 0.01 {
                    let reach = crate::canvas::connection_reach(self.probe.radius);
                    let extension = (0.16 + d * 0.36 + reach * 0.12).clamp(0.16, 0.74);
                    let end = Point::new(
                        p.x + (p.x - self.probe.position.x) / d * extension,
                        p.y + (p.y - self.probe.position.y) / d * extension,
                    );
                    fading_line(
                        &painter,
                        transform(p),
                        transform(end),
                        source.color,
                        0.082 * if source.curve.is_some() { 1.0 } else { 0.72 },
                        if source.curve.is_some() { 0.72 } else { 0.58 },
                        1.0,
                        true,
                    );
                } else if !ray.blocked && ray.strength > 0.002 && ray.edge_distance > 1e-5 {
                    let length = 0.5_f64.min(ray.edge_distance);
                    let end = Point::new(
                        p.x + (self.probe.position.x - p.x) / d * length,
                        p.y + (self.probe.position.y - p.y) / d * length,
                    );
                    fading_line(
                        &painter,
                        transform(p),
                        transform(end),
                        source.color,
                        0.34 * if source.curve.is_some() { 1.0 } else { 1.12 },
                        if source.curve.is_some() { 0.92 } else { 1.06 },
                        length / 0.5,
                        false,
                    );
                }
            }
            for curve in &self.scene.curves {
                painter.add(egui::Shape::line(
                    curve.path.iter().copied().map(transform).collect(),
                    Stroke::new(1.8, rgba(curve.color, 0.56)),
                ));
            }
            for (index, source) in self.scene.anchors.iter().enumerate() {
                let at = transform(source.position);
                let chosen = self.selected == self.source_for_anchor.get(index).copied();
                if source.curve.is_none() {
                    polygon(&painter, at, 14.0, rgba(source.color, 0.82), 1.4);
                }
                painter.circle_filled(at, 6.2, rgba(source.color, 0.96));
                if chosen {
                    painter.circle_filled(at, 6.2, WASH);
                    painter.circle_stroke(
                        at,
                        if source.curve.is_some() { 7.45 } else { 9.15 },
                        Stroke::new(1.8, rgba([244, 247, 252], 0.96)),
                    );
                }
            }
        }

        self.paint_source_labels(&painter, rect);
        painter.circle_filled(center, (radius - 1.0).max(0.0),
            Color32::from_rgba_unmultiplied(53, 54, 64, 128));
        if self.preset.sources.is_empty() {
            painter.circle_stroke(center, (radius - 1.5).max(0.0),
                Stroke::new(3.0, Color32::WHITE));
            painter.circle_filled(center, 6.0, Color32::from_gray(202));
            return;
        }
        if native_frame.is_some() {
            painter.circle_filled(center, 6.0, Color32::from_gray(202));
            if let Some(ring) = self.probe_ring.as_ref().and_then(|cache| cache.rendered()) {
                ring.paint(&painter);
            }
            return;
        }
        if source_background {

            painter.circle_filled(center, 6.0, Color32::from_gray(202));
            return;
        }

        painter.circle_stroke(center, (radius - 1.5).max(0.0),
            Stroke::new(3.0, Color32::WHITE));
        painter.circle_filled(center, 6.0, Color32::from_gray(202));
    }

    fn paint_inspection_source_labels(&mut self, painter: &egui::Painter, rect: Rect) {
        let transform = |p| world_to_screen(p, rect, self.camera);
        self.label_rects.clear();
        let mut occupied = Vec::new();
        for (source_index, source) in self.preset.sources.iter().enumerate() {

            if source.name.as_bytes().first().copied().unwrap_or(0) == 0 {
                continue;
            }
            if !source.enabled { continue; }
            let nodes: Vec<_> = self.scene.anchors.iter().enumerate()
                .filter(|(i, _)| self.source_for_anchor.get(*i) == Some(&source_index))
                .map(|(_, a)| transform(a.position)).collect();
            if nodes.is_empty() || nodes.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                continue;
            }
            let bottom = *nodes.iter().max_by(|a, b| a.y.total_cmp(&b.y)).unwrap();
            let delta = self
                .label_offsets
                .get(source_index)
                .copied()
                .unwrap_or_default();
            let display_delta = vec2(delta.x as f32, delta.y as f32);
            if !display_delta.x.is_finite() || !display_delta.y.is_finite() {
                continue;
            }

            let label = clipped_title(&source.name, 4096);

            let caption_color = Color32::from_rgb(183, 184, 187);
            let galley = painter.layout(
                label,
                FontId::proportional(13.5),
                caption_color,
                180.0,
            );
            let size = galley.size();
            let base = bottom + vec2(-size.x / 2.0, 14.0);
            let mut r = Rect::from_min_size(base + display_delta, size);
            if delta == Point::default() {
                for _ in 0..8 {
                    if !occupied.iter().any(|v: &Rect| v.expand(4.0).intersects(r))
                        && !nodes.iter().any(|p| r.expand(7.0).contains(*p))
                    {
                        break;
                    }
                    r = r.translate(vec2(0.0, 16.0));
                }
            }
            r = r.translate(vec2(
                (rect.left() + 12.0 - r.left()).max(0.0)
                    - (r.right() - rect.right() + 12.0).max(0.0),
                (rect.top() + 75.0 - r.top()).max(0.0)
                    - (r.bottom() - rect.bottom() + 16.0).max(0.0),
            ));
            if !r.min.x.is_finite() || !r.min.y.is_finite()
                || !r.max.x.is_finite() || !r.max.y.is_finite()
            {
                continue;
            }
            painter.galley(r.min, galley, caption_color);
            occupied.push(r);
            self.label_rects.push((source_index, r));
        }
    }

    fn primary_input(&self, ctx: &egui::Context, rect: Rect, response: &egui::Response,
        cancellations: &[canvas_gesture::PointerCancellation], pointer: Option<Pos2>, over: bool)
        -> PrimaryInput
    {
        let armed_release = (self.drag.is_some() || self.waypoint_armed())
            && ctx.input(|i| i.pointer.primary_released());
        let primary_click = response.clicked_by(egui::PointerButton::Primary)
            && ctx.input(|i| !i.pointer.primary_down());
        let completed_click = if !armed_release && primary_click
            && ctx.input(|i| i.pointer.primary_pressed())
        {
            ctx.input(|i| canvas_gesture::primary_click_release(&i.raw.events, cancellations))
                .filter(|at| rect.contains(*at)
                    && !self.overlay_rects.iter().any(|r| r.contains(*at))
                    && ctx.layer_id_at(*at).is_none_or(|layer| layer == response.layer_id))
        } else {
            None
        };

        let completed_drag = if !self.allow_canvas_editing
            && self.drag.is_none()
            && !ctx.input(|i| i.pointer.primary_down())
            && !response.is_pointer_button_down_on()
            && !response.clicked_by(egui::PointerButton::Primary)
            && !egui::Popup::is_any_open(ctx)
        {
            ctx.input(|i| canvas_gesture::completed_primary_drag(&i.raw.events, cancellations))
                .filter(|gesture| rect.contains(gesture.press)
                    && !self.overlay_rects.iter().any(|r| r.contains(gesture.press))
                    && ctx.layer_id_at(gesture.press)
                        .is_none_or(|layer| layer == response.layer_id))
        } else {
            None
        };

        let press_position = if let Some(gesture) = completed_drag {
            Some(gesture.press)
        } else if primary_click {
            completed_click
        } else if !over && response.is_pointer_button_down_on()
            && ctx.input(|i| i.pointer.primary_pressed())
        {
            pointer
        } else {
            None
        };
        PrimaryInput { armed_release, primary_click, completed_click, completed_drag, press_position }
    }

    fn passive_primary_escape(&self, ctx: &egui::Context, rect: Rect, response: &egui::Response,
        cancellations: &[canvas_gesture::PointerCancellation],
        observations: &[canvas_gesture::UnhandledEscapeDown])
        -> std::result::Result<Option<bool>, source_distance::Failure>
    {
        if observations.is_empty() || self.layout_blocked()
            || self.settings || self.editor.is_some() || self.error.is_some() || self.notice.is_some()
            || egui::Popup::is_any_open(ctx)
            || !rect.is_finite() || !rect.width().is_finite() || !rect.height().is_finite()
            || rect.width() <= 0.0 || rect.height() <= 0.0
            || !self.camera.is_finite() || self.camera <= 0.0
            || cancellations.iter().any(|c| c.affects(egui::PointerButton::Primary))
        { return Ok(None); }
        let pointer = ctx.pointer_hover_pos().filter(|at| at.x.is_finite() && at.y.is_finite());
        let over = pointer.is_some_and(|at| self.overlay_rects.iter().any(|r| r.contains(at)));
        let input = self.primary_input(ctx, rect, response, cancellations, pointer, over);

        let existing_owner = (self.drag.is_some() || self.waypoint_armed())
            && (self.allow_canvas_editing
                || !matches!(self.drag, Some(Drag::Source(_) | Drag::Label(_))))
            && (response.is_pointer_button_down_on() || response.dragged()
                || response.drag_stopped_by(egui::PointerButton::Primary) || input.armed_release)
            && ctx.input(|i| i.pointer.primary_down() || i.pointer.primary_released());
        let completion = ctx.input(|i| canvas_gesture::passive_escape_completion(
            &i.raw.events, cancellations, observations, existing_owner));
        let Some(completion) = completion else { return Ok(None); };
        if !existing_owner {
            let Some(at) = input.press_position.filter(|_| !input.armed_release) else { return Ok(None); };

            let press = ctx.input(|i| i.raw.events.iter().find_map(|event| match event {
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary,
                    pressed: true, .. } => Some(*pos),
                _ => None,
            }));
            if !press.is_some_and(|press| press.x.is_finite() && press.y.is_finite()
                && rect.contains(press) && !self.overlay_rects.iter().any(|r| r.contains(press))
                && ctx.layer_id_at(press).is_none_or(|layer| layer == response.layer_id))
            { return Ok(None); }
            if !(self.allow_canvas_editing
                && self.label_rects.iter().any(|(_, r)| r.expand(4.0).contains(at)))
            {

                if self.waypoint_press(ctx, rect, response, cancellations).is_none() {
                    self.hit_source(at, rect)?;
                }
            }
        }
        Ok(Some(completion))
    }

    fn canvas_input(&mut self, ctx: &egui::Context, rect: Rect, response: &egui::Response,
        cancellations: &[canvas_gesture::PointerCancellation], escape_aborts_primary: bool,
        passive_hit_failure: Option<source_distance::Failure>)
    {
        if self.layout_blocked()
            || !self.latch_viewport()
            || !rect.min.x.is_finite()
            || !rect.min.y.is_finite()
            || !rect.max.x.is_finite()
            || !rect.max.y.is_finite()
            || !rect.width().is_finite()
            || !rect.height().is_finite()
            || rect.width() <= 0.0
            || rect.height() <= 0.0
            || !self.camera.is_finite()
            || self.camera <= 0.0
        {
            self.drag = None;
            return;
        }
        let pointer = ctx
            .pointer_hover_pos()
            .filter(|at| at.x.is_finite() && at.y.is_finite());
        let over = pointer.is_some_and(|p| self.overlay_rects.iter().any(|r| r.contains(p)));
        let modal =
            self.settings || self.editor.is_some() || self.error.is_some() || self.notice.is_some();
        if modal {
            self.drag = None;
            self.waypoint_cancel();
            return;
        }
        if !over && response.hovered() {

            let gestures: Vec<ProbeGesture> = ctx.input(|i| i.events.iter().filter_map(|event| {
                match event {
                    egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta, .. }
                        if cfg!(target_os = "windows") => Some(ProbeGesture::WindowsWheelLine(delta.y)),
                    egui::Event::Zoom(factor) => Some(ProbeGesture::Magnify(*factor)),
                    _ => None,
                }
            }).collect());
            let mut before_gestures = None;
            for gesture in gestures {
                let factor = match gesture {
                    ProbeGesture::WindowsWheelLine(line) => {
                        if !line.is_finite() || line == 0.0 {
                            continue;
                        }

                        let raw = (line * 120.0).round();
                        if !(-32768.0..=32767.0).contains(&raw) || raw / 120.0 != line {
                            continue;
                        }

                        let vertical = (raw * 0.5).clamp(-1000.0, 1000.0) * (1.0 / 256.0);
                        let scaled = f64::from(vertical) * 4.0;
                        let exponent = scaled * -0.1;
                        match source_math::wheel_exp(exponent) {
                            Ok(factor) => factor,
                            Err(error) => {
                                self.error = Some(error.message().into());
                                self.source_math_failure = Some(error);
                                break;
                            }
                        }
                    }
                    ProbeGesture::Magnify(input) => {

                        if !input.is_finite() || input <= 0.0 {
                            continue;
                        }
                        let factor = f64::from(input);
                        let difference = -1.0 + factor;
                        if difference.abs() < f64::from_bits(0x3f50624dd2f1a9fc) {
                            continue;
                        }

                        factor
                    }
                };
                let previous = before_gestures.is_none().then(|| self.capture_undo());
                let proposed = Probe {
                    radius: (self.probe.radius * factor).clamp(0.04, 0.15),
                    ..self.probe
                };
                if self.probe_changed(proposed) && before_gestures.is_none() {
                    before_gestures = previous;
                }
            }
            if let Some(previous) = before_gestures {
                self.remember_entry(previous);
            }
        }

        if self.layout_blocked() || self.settings || self.editor.is_some()
            || self.error.is_some() || self.notice.is_some()
        {
            self.drag = None;
            return;
        }

        if !self.allow_canvas_editing && self.drag.is_none()
            && response.clicked_by(egui::PointerButton::Secondary)
            && !ctx.input(|i| i.pointer.primary_down())
            && !egui::Popup::is_any_open(ctx)
            && let Some(at) = ctx.input(|i| canvas_gesture::secondary_click_release(&i.raw.events, cancellations))
                .filter(|at| rect.contains(*at)
                    && !self.overlay_rects.iter().any(|r| r.contains(*at))
                    && ctx.layer_id_at(*at).is_none_or(|layer| layer == response.layer_id))
        {
            let position = match self.probe_at(at, rect) {
                Ok(position) => position,
                Err(failure) => { self.retain_source_pointer_failure(failure); return; }
            };
            self.selected = None;
            let previous = self.capture_undo();
            if self.probe_changed(Probe { position, ..self.probe }) {
                self.remember_entry(previous);
            }
            return;
        }

        if escape_aborts_primary
            || cancellations.iter().any(|c| c.affects(egui::PointerButton::Primary))
        {
            self.drag = None;
            self.waypoint_cancel();
            return;
        }
        if self.waypoint_primary(ctx, rect, response, cancellations) { return; }
        let PrimaryInput { armed_release, primary_click, completed_click, completed_drag,
            press_position } = self.primary_input(ctx, rect, response, cancellations, pointer, over);
        if !armed_release && let Some(at) = press_position
        {
            self.waypoint_cancel();
            if self.allow_canvas_editing && let Some((index, _)) = self
                .label_rects
                .iter()
                .find(|(_, r)| r.expand(4.0).contains(at))
            {
                let i = *index;
                self.remember();
                self.selected = Some(i);
                self.selected_at = at;
                self.drag = Some(Drag::Label(i));
            } else {

                let hit = match passive_hit_failure {
                    Some(failure) => Err(failure),
                    None => self.hit_source(at, rect),
                };
                let hit = match hit {
                    Ok(hit) => hit,
                    Err(failure) => { self.retain_source_distance_failure(failure); return; }
                };
                if let Some((index, nearest)) = hit {
                    let already_selected = self.selected == Some(index);
                    if !already_selected {
                        if let Err(failure) = self.cache_source_actions(index, nearest, rect) {
                            self.retain_source_distance_failure(failure); return;
                        }
                    }
                    self.selected = Some(index);
                    self.selected_at = nearest;
                    self.drag = if self.allow_canvas_editing {
                        self.remember();
                        Some(Drag::Source(index))
                    } else {

                        Some(Drag::ProbeFromSelection)
                    };
                } else {
                    let position = match self.probe_at(at, rect) {
                        Ok(position) => position,
                        Err(failure) => { self.retain_source_pointer_failure(failure); return; }
                    };
                    self.selected = None;
                    let previous = self.capture_undo();
                    let proposed = Probe {
                        position,
                        ..self.probe
                    };
                    if self.probe_changed(proposed) {
                        self.remember_entry(previous);
                        self.drag = Some(Drag::Probe);
                    } else {
                        self.drag = None;
                    }
                }
            }
        }
        if !self.allow_canvas_editing
            && matches!(self.drag, Some(Drag::Source(_) | Drag::Label(_)))
        {
            self.drag = None;
        }

        let drag_pointer = if let Some(gesture) = completed_drag {
            Some(gesture.release)
        } else if armed_release {
            ctx.input(|i| canvas_gesture::armed_primary_release(&i.raw.events, cancellations))
        } else if primary_click {
            completed_click
        } else {
            pointer
        };
        if ctx.input(|i| i.pointer.primary_down() || i.pointer.primary_released())
            && let Some(drag) = self.drag
            && let Some(at) = drag_pointer
        {
            match drag {
                Drag::ProbeFromSelection => {

                    if completed_drag.is_some() || ((response.dragged()
                        || response.drag_stopped_by(egui::PointerButton::Primary))
                        && !ctx.input(|i| i.pointer.primary_pressed()))
                    {
                        let position = match self.probe_at(at, rect) {
                            Ok(position) => position,
                            Err(failure) => { self.retain_source_pointer_failure(failure); return; }
                        };
                        let previous = self.capture_undo();
                        let proposed = Probe {
                            position,
                            ..self.probe
                        };
                        if self.probe_changed(proposed) {
                            self.remember_entry(previous);
                            self.drag = Some(Drag::Probe);
                        } else {
                            self.drag = None;
                        }
                    }
                }
                Drag::Probe => {
                    let target = match self.probe_at(at, rect) {
                        Ok(position) => position,
                        Err(failure) => { self.retain_source_pointer_failure(failure); return; }
                    };
                    if self.probe.position != target {
                        let proposed = Probe { position: target, ..self.probe };
                        if !self.probe_changed(proposed) {
                            self.drag = None;
                        }
                    }
                }
                Drag::Source(index) => {
                    let delta = ctx.input(|i| i.pointer.delta());
                    if response.dragged()
                        && !ctx.input(|i| i.pointer.primary_pressed())
                        && delta.x.is_finite() && delta.y.is_finite()
                        && delta.length_sq() > 0.0
                    {
                        let half = rect.width().min(rect.height()) as f64 * 0.5;
                        let v = Point::new(
                            delta.x as f64 / half / self.camera,
                            -delta.y as f64 / half / self.camera,
                        );
                        let proposed = Point::new(
                            self.offsets[index].x + v.x,
                            self.offsets[index].y + v.y,
                        );
                        let selected_at = self.selected_at + delta;
                        if !proposed.x.is_finite() || !proposed.y.is_finite()
                            || !selected_at.x.is_finite() || !selected_at.y.is_finite()
                        {
                            self.drag = None;
                            return;
                        }
                        self.offsets[index] = proposed;
                        self.selected_at = selected_at;
                        self.rebuild();
                        self.persist_layout();
                    }
                }
                Drag::Label(index) => {
                    let delta = ctx.input(|i| i.pointer.delta());
                    if response.dragged()
                        && !ctx.input(|i| i.pointer.primary_pressed())
                        && delta.x.is_finite() && delta.y.is_finite()
                        && delta.length_sq() > 0.0
                    {
                        let proposed = Point::new(
                            self.label_offsets[index].x + delta.x as f64,
                            self.label_offsets[index].y + delta.y as f64,
                        );
                        let selected_at = self.selected_at + delta;
                        if !proposed.x.is_finite() || !proposed.y.is_finite()
                            || !selected_at.x.is_finite() || !selected_at.y.is_finite()
                        {
                            self.drag = None;
                            return;
                        }
                        self.label_offsets[index] = proposed;
                        self.selected_at = selected_at;
                        self.persist_layout();
                    }
                }
            }
        }
        if ctx.input(|i| i.pointer.primary_released()) {
            self.drag = None;
        }
        if self.allow_canvas_editing && !over
            && response.double_clicked()
            && let Some(index) = self.selected
        {
            self.begin_edit(Edit::SourceName(index));
        }
    }
    fn retain_source_pointer_failure(&mut self, failure: canvas_viewport::Failure) {
        self.error = Some(failure.message().into());
        self.source_pointer_failure = Some(failure);
        self.drag = None;
    }

    fn retain_source_distance_failure(&mut self, failure: source_distance::Failure) {
        self.error = Some(failure.message().into());
        self.source_distance_failure = Some(failure);
        self.drag = None;
    }

    fn hit_source(&self, at: Pos2, rect: Rect)
        -> std::result::Result<Option<(usize, Pos2)>, source_distance::Failure> {
        if !self.allow_canvas_editing {

            let mut norm = source_distance::Norm::bind()?;
            let hits = |norm: &mut source_distance::Norm, sample: Pos2| {
                if !sample.x.is_finite() || !sample.y.is_finite() { return Ok(false); }
                norm.distance([sample.x, sample.y], [at.x, at.y]).map(|distance| distance < 7.0)
            };
            if self.native_layout {
                let Some(view) = self.layout_view.as_ref() else { return Ok(None); };
                if view.positions.len() != self.preset.sources.len()
                    || view.positions.iter().zip(&self.preset.sources)
                        .any(|(positions, source)| positions.len() != source.points.len())
                {
                    return Ok(None);
                }
                for (index, positions) in view.positions.iter().enumerate() {
                    let offset = self.offsets.get(index).copied().unwrap_or_default();

                    for xy in positions {
                        let sample = world_to_screen(
                            Point::new(f64::from(xy[0]) + offset.x, f64::from(xy[1]) + offset.y),
                            rect,
                            self.camera,
                        );
                        match hits(&mut norm, sample) {
                            Ok(true) => return Ok(Some((index, sample))),
                            Ok(false) => {},
                            Err(()) => return Err(norm.into_failure()),
                        }
                    }
                }
            } else {

                for index in 0..self.preset.sources.len() {
                    for (anchor, owner) in self.scene.anchors.iter().zip(&self.source_for_anchor) {
                        if *owner == index {
                            let sample = world_to_screen(anchor.position, rect, self.camera);
                            match hits(&mut norm, sample) {
                                Ok(true) => return Ok(Some((index, sample))),
                                Ok(false) => {},
                                Err(()) => return Err(norm.into_failure()),
                            }
                        }
                    }
                }
            }
            return Ok(None);
        }
        let mut best: Option<(usize, Pos2, f32)> = None;
        for (i, anchor) in self.scene.anchors.iter().enumerate() {
            let p = world_to_screen(anchor.position, rect, self.camera);
            let d = p.distance(at);
            let threshold = if anchor.curve.is_some() { 13.0 } else { 18.0 };
            if d < threshold && best.as_ref().is_none_or(|v| d < v.2) {
                best = Some((self.source_for_anchor[i], p, d));
            }
        }
        for (i, curve) in self.scene.curves.iter().enumerate() {
            for pair in curve.path.windows(2) {
                let a = world_to_screen(pair[0], rect, self.camera);
                let b = world_to_screen(pair[1], rect, self.camera);
                let line = b - a;
                let t = ((at - a).dot(line) / line.length_sq().max(1e-6)).clamp(0.0, 1.0);
                let p = a + line * t;
                let d = at.distance(p);
                if d < 9.0 && best.as_ref().is_none_or(|v| d < v.2) {
                    best = Some((self.source_for_curve[i], p, d));
                }
            }
        }
        Ok(best.map(|(i, p, _)| (i, p)))
    }
    fn ambient_lights(&self, full: Rect) -> Vec<(Pos2, [u8; 3])> {
        self.scene
            .anchors
            .iter()
            .filter(|a| a.curve.is_none() || a.index == 0)
            .take(12)
            .map(|a| (world_to_screen(a.position, full, self.camera), a.color))
            .collect()
    }
}

impl eframe::App for DesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

impl DesktopApp {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        self.show_with_pointer_cancellations(ui, &[], &[]);
    }

    pub(crate) fn show_with_pointer_cancellations(&mut self, ui: &mut egui::Ui,
        cancellations: &[canvas_gesture::PointerCancellation],
        observations: &[canvas_gesture::UnhandledEscapeDown])
    {
        localization::set(ui.ctx(), self.locale);
        self.poll_library(ui.ctx());

        let mut retire_primary_on_return = false;
        self.waypoint_frame_start(ui.ctx(), cancellations);
        self.show_pointer_frame(ui, cancellations, observations, &mut retire_primary_on_return);
        if retire_primary_on_return { self.drag = None; self.waypoint_cancel(); }
        if self.layout_blocked() || self.settings || self.editor.is_some() || self.color_picker
            || self.error.is_some() || self.notice.is_some() || egui::Popup::is_any_open(ui.ctx())
        { self.waypoint_cancel(); }
    }

    fn show_pointer_frame(&mut self, ui: &mut egui::Ui,
        cancellations: &[canvas_gesture::PointerCancellation],
        observations: &[canvas_gesture::UnhandledEscapeDown], retire_primary_on_return: &mut bool)
    {
        let ctx = ui.ctx().clone();

        let focus_aborts_primary = ctx.input(|input| !input.focused
            || input.raw.events.iter().any(|event|
                matches!(event, egui::Event::WindowFocused(false))));

        if focus_aborts_primary
            || cancellations.iter().any(|c| c.affects(egui::PointerButton::Primary))
        {
            self.drag = None;
        }
        #[cfg(windows)]
        if !self.plugin_hosted { self.audio_devices.poll(&ctx); }
        if !self.plugin_hosted { self.poll_live_audio(&ctx); }
        if let Some(info) = self.model_inspection.poll(&ctx, &self.model_paths[0]) {
            self.model_info = info;
        }
        self.userdata_load.poll(&ctx, &self.model_paths[1]);
        let full = ui.max_rect();
        self.advance_layout(&ctx);
        self.publish_live_condition();
        if !self.plugin_hosted { self.reconcile_live_audio(&ctx); }
        if self.layout_blocked() {
            self.paint_canvas(ui, full);
            let input_blocked = self.pending_layout_ui(&ctx, full);
            self.generation_frame(&ctx, full, input_blocked);
            return;
        }
        let message_at_frame_start = self.error.is_some() || self.notice.is_some();
        let key_editor_at_frame_start = matches!(self.editor, Some(Edit::WaypointKey(_)));
        let dismiss_error = self.error.is_some();

        let app_escape = ctx.input(|i| i.key_pressed(egui::Key::Escape))
            && (message_at_frame_start || (self.editor.is_none() && !self.settings
                && !egui::Popup::is_any_open(&ctx)));

        let escape_aborts_primary = focus_aborts_primary || (app_escape
            && !ctx.input(|i| canvas_gesture::primary_release_precedes_escape(
                &i.raw.events, cancellations)));
        let dismiss_message = message_at_frame_start
            && app_escape;
        if message_at_frame_start {
            self.drag = None;
        }
        if app_escape {
            if self.error.is_some() || self.notice.is_some() {

                ctx.input_mut(|input| {
                    input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
                });
            } else if self.color_picker
                && self.color_source.is_none()
                && self.native_tone_hex.cancel()
            {
                ctx.input_mut(|input| {
                    input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
                });
            } else if self.color_picker {
                self.color_picker = false;
            } else if self.menu.is_some() {
                self.menu = None;
                self.voices_open = false;
            }
            if escape_aborts_primary {
                self.drag = None;
                self.waypoint_cancel();
            } else {

                *retire_primary_on_return = true;
            }
        }
        self.waypoint_key_dispatch(&ctx);
        let shortcuts_allowed = |app: &Self| {
            !app.layout_blocked()
                && app.editor.is_none()
                && !app.settings
                && !app.color_picker
                && app.error.is_none()
                && app.notice.is_none()
        };
        if shortcuts_allowed(self) {
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
                self.open();
            }
            if shortcuts_allowed(self)
                && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S))
            {
                if ctx.input(|i| i.modifiers.shift) {
                    self.begin_edit(Edit::SaveAs);
                } else {
                    self.save_current();
                }
            }
            if shortcuts_allowed(self)
                && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z))
            {
                self.undo();
            }
            if shortcuts_allowed(self)
                && ctx.input(|i| i.key_pressed(egui::Key::Delete))
                && let Some(index) = self.selected
            {
                self.delete_source(index);
            }
        }
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect::<Vec<_>>()
        });
        if self.error.is_none() && self.notice.is_none()
            && !matches!(self.editor, Some(Edit::WaypointKey(_))) {
            self.dispatch_import(&ctx, dropped);
        }
        if !self.latch_viewport() {

            self.drag = None;
            ui.painter().rect_filled(full, 0.0, Color32::BLACK);
            self.error.get_or_insert_with(|| "Canvas zoom could not be calculated.".into());
            self.message_ui(&ctx, full);
            self.analysis_ui_with_barrier(&ctx, full, true);
            if dismiss_message {
                if dismiss_error { self.error = None; } else { self.notice = None; }
                ctx.request_repaint();
            }
            self.poll_generation(&ctx);
            if self.layout_blocked() && ctx.input(|i| i.viewport().close_requested()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            return;
        }
        if self.layout_blocked() {
            self.paint_canvas(ui, full);
            let input_blocked = self.pending_layout_ui(&ctx, full);
            self.generation_frame(&ctx, full, input_blocked);
            return;
        }
        let response = ui.interact(
            full,
            ui.id().with("canvas-interaction"),
            Sense::click_and_drag(),
        );
        self.overlay_rects.clear();
        self.menu_rects.clear();
        self.picker_rect = None;
        let had_menu = self.menu;
        let had_picker = self.color_picker;
        let had_settings = self.settings;
        self.header(&ctx, full);
        if self.layout_blocked() {
            self.paint_canvas(ui, full);
            let input_blocked = self.pending_layout_ui(&ctx, full);
            self.generation_frame(&ctx, full, input_blocked);
            return;
        }
        self.rail(&ctx, full);
        self.monitor_controls(&ctx, full, cancellations, escape_aborts_primary, observations);
        self.waypoint_toolbar(&ctx, full, cancellations, escape_aborts_primary, observations);
        if !self.settings && self.editor.is_none() && !self.color_picker {
            self.source_actions(&ctx, full);
        }
        if self.layout_blocked() {
            self.paint_canvas(ui, full);
            let input_blocked = self.pending_layout_ui(&ctx, full);
            self.generation_frame(&ctx, full, input_blocked);
            return;
        }

        let settings_panel = self.settings;
        if settings_panel && self.color_picker && self.color_source.is_none() {
            self.color_ui(&ctx, full);
        }
        if self.settings {
            self.settings_ui(&ctx, full);
        }
        if self.layout_blocked() {
            self.paint_canvas(ui, full);
            let input_blocked = self.pending_layout_ui(&ctx, full);
            self.generation_frame(&ctx, full, input_blocked);
            return;
        }
        if self.color_picker && !settings_panel {
            self.color_ui(&ctx, full);
        }
        if self.editor.is_some() {
            self.editor_ui(&ctx, full);
        }
        let message_in_frame = message_at_frame_start
            || self.error.is_some()
            || self.notice.is_some();
        self.message_ui(&ctx, full);
        self.analysis_ui_with_barrier(&ctx, full, message_in_frame);
        let binding_barrier = key_editor_at_frame_start || matches!(self.editor, Some(Edit::WaypointKey(_)));
        let mut dismissed = message_in_frame || binding_barrier;
        let mut clear_selection = false;
        if !message_in_frame && !binding_barrier && ctx.input(|i| i.pointer.primary_pressed())
            && let Some(at) = ctx.pointer_hover_pos()
        {
            if had_menu.is_some()
                && self.menu.is_some()
                && !self.menu_rects.iter().any(|r| r.contains(at))
            {
                self.menu = None;
                self.voices_open = false;
                dismissed = true;
            }
            if had_picker && !had_settings && !self.settings && self.color_picker
                && !self.picker_rect.is_some_and(|r| r.contains(at))
            {
                self.color_picker = false;
                dismissed = true;
            }
            if !self.overlay_rects.iter().any(|r| r.contains(at)) {
                if !focus_aborts_primary && self.selected.is_some() && !self.waypoint_armed()
                    && !self.waypoint_claimed_pass(&ctx)
                    && self.waypoint_press(&ctx, full, &response, cancellations).is_none() {
                    match self.hit_source(at, full) {
                        Ok(None) => clear_selection = true,
                        Ok(Some(_)) => {},
                        Err(failure) => {
                            self.retain_source_distance_failure(failure);
                            dismissed = true;
                        }
                    }
                }
            }
        }
        let mut passive_aborts_primary = false;
        let mut passive_hit_failure = None;
        if !dismissed {

            match self.passive_primary_escape(&ctx, full, &response, cancellations, observations) {
                Ok(Some(completed_before_escape)) => {
                    *retire_primary_on_return = true;
                    passive_aborts_primary = !completed_before_escape;
                }
                Ok(None) => {},
                Err(failure) => {

                    passive_hit_failure = Some(failure);
                }
            }
        }
        if clear_selection && !passive_aborts_primary { self.selected = None; }
        if !dismissed {
            self.canvas_input(&ctx, full, &response, cancellations,
                escape_aborts_primary || passive_aborts_primary, passive_hit_failure);
        } else if message_in_frame {
            self.drag = None;
        }
        if dismiss_message {
            if dismiss_error {
                self.error = None;
            } else {
                self.notice = None;
            }
            ctx.request_repaint();
        }
        self.poll_generation(&ctx);

        if self.latch_viewport() {
            self.paint_canvas(ui, full);
            self.paint_waypoints(ui, full);
        } else {
            ui.painter().rect_filled(full, 0.0, Color32::BLACK);
            self.error.get_or_insert_with(|| "Canvas zoom could not be calculated.".into());
            ctx.request_repaint();
        }
        if self.layout_blocked() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if self.mapping == 0
            && let Some(due) = self.layout_next
        {
            ctx.request_repaint_after(due.saturating_duration_since(Instant::now()));
        }
        if self.drag.is_some() {
            ctx.request_repaint();
        }
        self.publish_live_condition();
        if !self.plugin_hosted { self.reconcile_live_audio(&ctx); }
    }
}

fn quality_names() -> [&'static str; 4] {
    [
        "Highest Quality",
        "Higher Quality",
        "Lower Latency",
        "Lowest Latency",
    ]
}
fn blank_preset() -> Preset {
    Preset::parse(json!({"presetName":"My Preset","realtimeLatencyOption":"Highest Quality","useRealtimeLatency":true,"voice":{"probeLocationX":0.0,"probeLocationY":0.0,"probeRadius":0.1,"transformation":[1.0,0.0,0.0,0.0,0.0,1.0,0.0,0.0],"sources":[],"waypoints":[]}})).expect("valid empty preset")
}
fn voice(doc: &Value) -> &Value {
    doc.get("voice").unwrap_or(doc)
}
fn voice_mut(doc: &mut Value) -> &mut Value {
    if doc.get("voice").is_some() {
        &mut doc["voice"]
    } else {
        doc
    }
}

const MULTITHREAD_PREFERENCE_FILE: &str = "threading.json";

fn try_read_multithread_preference_at(path: &Path) -> std::io::Result<bool> {
    use std::io::{Error, ErrorKind};
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error),
    };
    serde_json::from_str::<bool>(&text).map_err(|error| Error::new(ErrorKind::InvalidData, error))
}
fn read_multithread_preference_at(path: &Path) -> bool {
    try_read_multithread_preference_at(path).unwrap_or(true)
}
pub(crate) fn read_multithread_preference() -> bool {
    read_multithread_preference_at(&preferences_dir().join(MULTITHREAD_PREFERENCE_FILE))
}
pub(crate) fn try_read_multithread_preference() -> std::io::Result<bool> {
    try_read_multithread_preference_at(&preferences_dir().join(MULTITHREAD_PREFERENCE_FILE))
}

fn write_multithread_preference_at(dir: &Path, enabled: bool) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind, Write};
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = dir.join(MULTITHREAD_PREFERENCE_FILE);
    let bytes = serde_json::to_vec(&enabled).map_err(Error::other)?;
    fs::create_dir_all(dir)?;
    let mut created = None;
    for _ in 0..32 {
        let ordinal = NEXT.fetch_add(1, Ordering::Relaxed);
        let temporary = dir.join(format!(".threading-{}-{ordinal}.tmp", std::process::id()));
        match fs::OpenOptions::new().write(true).create_new(true).open(&temporary) {
            Ok(file) => { created = Some((temporary,file)); break; },
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let (temporary, mut file) = created.ok_or_else(|| Error::new(ErrorKind::AlreadyExists, "Cannot reserve a preference temporary"))?;
    let written = file.write_all(&bytes).and_then(|_| file.sync_all());
    drop(file);
    let result = written.and_then(|_| fs::rename(&temporary, &path));
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    result
}
fn preferences_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Rocoflex")
}
fn safe_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(100)
        .collect();
    let cleaned = cleaned.trim_matches([' ', '.']);
    if cleaned.is_empty() {
        "Preset".into()
    } else {
        let base = cleaned
            .split('.')
            .next()
            .unwrap_or_default()
            .trim_end_matches(' ')
            .to_ascii_uppercase();
        let device = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || base
                .strip_prefix("COM")
                .or_else(|| base.strip_prefix("LPT"))
                .is_some_and(|number| {
                    matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                });
        if device {
            format!("_{}", cleaned.chars().take(99).collect::<String>())
        } else {
            cleaned.into()
        }
    }
}
fn unique_library_name(dir: &Path, name: &str) -> String {
    let mut suffix = 0usize;
    let mut value = library::preset_candidate_name(name, suffix);
    while dir.join(format!("{}.json", safe_filename(&value))).exists() {
        let Some(next) = suffix.checked_add(1) else { break };
        suffix = next;
        value = library::preset_candidate_name(name, suffix);
    }
    value
}
fn read_offsets(doc: &Value, key: &str, len: usize) -> Vec<Point> {
    (0..len)
        .map(|i| {
            let value = &doc["rocoflexCanvas"][key][i];
            Point::new(
                value["x"].as_f64().filter(|n| n.is_finite()).unwrap_or(0.0),
                value["y"].as_f64().filter(|n| n.is_finite()).unwrap_or(0.0),
            )
        })
        .collect()
}

