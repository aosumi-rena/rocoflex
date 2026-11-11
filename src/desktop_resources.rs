use super::*;

struct UserdataRequest {
    epoch: u64,
    path: String,
}

struct UserdataJob {
    request: UserdataRequest,
    handle: std::thread::JoinHandle<crate::Result<model_inspection::Summary>>,
}

enum UserdataOutcome {
    Loaded { request: UserdataRequest, summary: model_inspection::Summary },
    ReadFailure { request: UserdataRequest, error: String },
    Spawn { request: UserdataRequest, error: std::io::Error },
    Panic { request: UserdataRequest, payload: Box<dyn std::any::Any + Send + 'static> },
}

impl UserdataOutcome {
    fn request(&self) -> &UserdataRequest {
        match self {
            Self::Loaded { request, .. } | Self::ReadFailure { request, .. }
            | Self::Spawn { request, .. } | Self::Panic { request, .. } => request,
        }
    }
}

#[derive(Default)]
pub(super) struct UserdataLoader {
    epoch: u64,
    path: Option<String>,
    job: Option<UserdataJob>,
    last: Option<UserdataOutcome>,
}

impl UserdataLoader {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    fn current(&self, request: &UserdataRequest, path: &str) -> bool {
        request.epoch == self.epoch && request.path == path
            && self.path.as_deref() == Some(path)
    }

    pub fn invalidate(&mut self, path: &str) {
        self.epoch = self.epoch.wrapping_add(1);
        self.path = Some(path.to_owned());
    }

    fn observe_path(&mut self, path: &str) {
        if self.path.as_deref() != Some(path) {
            self.invalidate(path);
        }
    }

    pub fn start(&mut self, ctx: &egui::Context, path: &str, maximum_bytes: u64) {
        self.observe_path(path);
        if self.busy() { return; }
        self.epoch = self.epoch.wrapping_add(1);
        let request = UserdataRequest { epoch: self.epoch, path: path.to_owned() };
        let worker_path = request.path.clone();
        let repaint = ctx.clone();
        let handle = std::thread::Builder::new().name("userdata-load".into()).spawn(move || {
            let path = std::path::Path::new(&worker_path);
            let result = if library::protected_model_path(path)
                || path.canonicalize().is_ok_and(|resolved| library::protected_model_path(&resolved))
            {
                Err("Session and authentication files are not model inputs.".into())
            } else { crate::userdata::Loaded::read(path, maximum_bytes)
                .and_then(|loaded| {
                    #[cfg(windows)]
                    crate::morph_processor::inspect_morphing_model(loaded.bytes())?;
                    #[cfg(not(windows))]
                    drop(crate::prototype_synthesis_networks::SynthesisNetworks::from_userdata(loaded.bytes())?);
                    model_inspection::Summary::from_bytes(loaded.bytes())
                }) };
            repaint.request_repaint();
            result
        });
        match handle {
            Ok(handle) => {
                self.job = Some(UserdataJob { request, handle });
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
            Err(error) => {
                self.last = Some(UserdataOutcome::Spawn { request, error });
                ctx.request_repaint();
            }
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context, path: &str) {
        self.observe_path(path);
        let Some(job) = &self.job else { return; };
        if !job.handle.is_finished() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            return;
        }
        let Some(UserdataJob { request, handle }) = self.job.take() else { return; };

        self.last = Some(match handle.join() {
            Ok(Ok(summary)) => UserdataOutcome::Loaded { request, summary },
            Ok(Err(error)) => UserdataOutcome::ReadFailure { request, error },
            Err(payload) => UserdataOutcome::Panic { request, payload },
        });
        ctx.request_repaint();
    }

    pub fn inspection_message(&self, path: &str) -> Option<String> {
        if self.busy() { return None; }
        match self.last.as_ref().filter(|o| self.current(o.request(), path)) {
            Some(UserdataOutcome::Loaded { summary, .. }) => Some(summary.message("Valid morphing model")),
            Some(UserdataOutcome::ReadFailure { error, .. }) => Some(format!("Invalid morphing model: {error}")),
            Some(UserdataOutcome::Spawn { .. }) => Some("Could not start model inspection.".into()),
            Some(UserdataOutcome::Panic { .. }) => Some("Model inspection stopped unexpectedly.".into()),
            None => None,
        }
    }

}

fn heading(ui: &mut egui::Ui, title: &str, detail: &str) {
    let keep_english = title == "Canvas";
    controls::settings_section(ui, ui.available_width(), title);
    if !detail.is_empty() {
        let detail = if keep_english { detail } else { localization::tr(ui.ctx(), detail) };
        ui.label(egui::RichText::new(detail).size(12.0).color(MUTED));
    }
}

fn divider(ui: &mut egui::Ui) { ui.add_space(7.0); }

fn resource_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let translated = localization::tr(ui.ctx(), text).to_owned();
    resource_button_raw(ui, &translated, enabled)
}

fn resource_button_raw(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let width = ui.painter().layout_no_wrap(text.into(), FontId::proportional(13.0), TEXT)
        .size().x + 24.0;
    let (id, rect) = ui.allocate_space(vec2(width.max(100.0).min(ui.max_rect().width().max(0.0)), 24.0));
    let clip = ui.clip_rect();
    ui.set_clip_rect(clip.intersect(rect));
    let response = controls::settings_button_raw(ui, rect, id, text, enabled);
    ui.set_clip_rect(clip);
    response
}

impl DesktopApp {
    fn resources_available(&self) -> bool {
        self.settings && self.settings_resources && self.editor.is_none()
            && !self.layout_blocked() && self.error.is_none() && self.notice.is_none()
    }

    pub(super) fn resources_ui(&mut self, ui: &mut egui::Ui, message_open: bool) {
        ui.spacing_mut().item_spacing = vec2(8.0, 7.0);
        heading(ui, "Analysis model", "");
        let inspecting = self.model_inspection.busy();
        let inspect_label = if inspecting { "Inspecting..." } else { "Inspect model" };
        if self.resource_path(ui, 0, "model.dnni", "Choose model.dnni", "dnni", message_open,
            inspect_label, !inspecting)
        {
            self.model_info.clear();
            if let Err(error) = self.model_inspection.start(ui.ctx(), &self.model_paths[0]) {
                self.model_info = error;
            }
        }
        if !self.model_info.is_empty() {
            ui.label(egui::RichText::new(localization::status(ui.ctx(), &self.model_info)).size(12.0).color(MUTED));
        }
        divider(ui);
        heading(ui, "Morphing model", "");
        let inspecting = self.userdata_load.busy();
        let inspect_label = if inspecting { "Inspecting..." } else { "Inspect model" };
        if self.resource_path(ui, 1, "userdata.bin", "Choose userdata.bin", "bin", message_open,
            inspect_label, !inspecting)
        {
            self.userdata_load.start(ui.ctx(), &self.model_paths[1], 256 * 1024 * 1024);
        }
        if let Some(status) = self.userdata_load.inspection_message(&self.model_paths[1]) {
            ui.label(egui::RichText::new(localization::status(ui.ctx(), &status)).size(12.0).color(MUTED));
        }
        divider(ui);
        heading(ui, "Audio import", "");
        let label = ui.label(egui::RichText::new(localization::tr(ui.ctx(), "FFmpeg executable")).size(12.0).color(TEXT));
        let editable = ui.is_enabled() && !message_open && self.resources_available();
        ui.add_sized([ui.available_width(), 24.0],
            egui::TextEdit::singleline(&mut self.ffmpeg_path).interactive(editable)
                .hint_text(localization::tr(ui.ctx(), "Auto-detect")).font(FontId::proportional(13.0))
                .margin(vec2(9.0, 3.0))).labelled_by(label.id);
        ui.horizontal_wrapped(|ui| {
            if resource_button(ui, "Choose FFmpeg…", self.resources_available()).clicked()
                && let Some(path) = rfd::FileDialog::new().set_title("Choose FFmpeg executable").pick_file()
            {
                if let Some(path) = path.to_str() { self.ffmpeg_path = path.into(); }
                else { self.error = Some("The selected path cannot be represented as UTF-8.".into()); }
            }
            if resource_button(ui, "Auto-detect", self.resources_available()).clicked() {
                self.ffmpeg_detected = audio_decode::detect_ffmpeg();
                self.ffmpeg_path.clear();
            }
        });
        if self.ffmpeg_path.trim().is_empty() {
            let status = self.ffmpeg_detected.as_ref().map(|path|
                localization::tr(ui.ctx(), "Detected: _").replacen('_', &path.display().to_string(), 1))
                .unwrap_or_else(|| localization::tr(ui.ctx(), "FFmpeg not found. Choose its executable to import additional formats.").into());
            ui.label(egui::RichText::new(status).size(12.0).color(MUTED));
        }
        divider(ui);
        heading(ui, "Preset files", "");
        self.preset_paths_ui(ui);
        if !self.resources_available() { return; }
        divider(ui);
        heading(ui, "Canvas", "");
        let selected = self.selected.filter(|index| *index < self.preset.sources.len());
        ui.horizontal(|ui| {
            ui.label("Selected source");
            let response = resource_button_raw(ui, "Display color…",
                selected.is_some() && self.editor.is_none() && !self.color_picker);
            if response.clicked()
                && let Some(selected) = selected
            {
                response.surrender_focus();
                self.settings = false;
                self.begin_color(Some(selected));
            }
        });
        if !self.resources_available() { return; }
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
        ui.painter().text(row.left_center(), Align2::LEFT_CENTER, "Allow dragging sources and labels",
            FontId::proportional(13.0), TEXT);
        let check = Rect::from_center_size(pos2(row.right() - 12.0, row.center().y), vec2(24.0, 24.0));
        let enabled = self.resources_available();
        controls::settings_check_raw(ui, check, "allow-canvas-editing", "Allow dragging sources and labels",
            &mut self.allow_canvas_editing, enabled);
        let before = self.mapping;
        ui.horizontal(|ui| {
            let label = ui.label("Layout mapping");

            let mapping_available = ui.is_enabled() && !message_open && self.resources_available();
            let response = egui::ComboBox::from_id_salt("inspection-mapping")
                .icon(controls::settings_combo_icon)
                .selected_text(match self.mapping { 0 => "Automatic", 1 => "Reduced dimensions", _ => "Stored basis" })
                .show_ui(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add_enabled_ui(mapping_available, |ui| {
                        for (value, label) in [(0, "Automatic"), (1, "Reduced dimensions"), (2, "Stored basis")] {
                            if controls::settings_option_raw(ui, label, self.mapping == value).clicked() { self.mapping = value; }
                        }
                    });
                });
            let _ = response.response.labelled_by(label.id);
        });
        if before != self.mapping && self.resources_available() { self.rebuild(); }
        ui.label(egui::RichText::new("Canvas options are not saved between sessions.").size(12.0).color(MUTED));
        if !self.native_layout && let Some(status) = &self.layout_status {
            ui.label(egui::RichText::new(status).size(12.0).color(MUTED));
        }
    }

    fn preset_paths_ui(&mut self, ui: &mut egui::Ui) {
        let enabled = self.resources_available();
        let mut removed = None;
        let mut changed = false;
        for index in 0..self.preset_paths.len() {
            ui.push_id(("preset-folder-row", index), |ui| {
                ui.horizontal(|ui| {
                    let measure = |text: &str| ui.painter().layout_no_wrap(
                        localization::tr(ui.ctx(), text).to_owned(), FontId::proportional(13.0), TEXT).size().x;
                    let change_width = (measure("Change") + 24.0).max(64.0);
                    let delete_width = (measure("Delete") + 24.0).max(64.0);
                    let recursive_width = measure("Recursive") + 28.0;
                    let width = (ui.available_width() - change_width - delete_width - recursive_width
                        - ui.spacing().item_spacing.x * 3.0).max(20.0);
                    ui.add_sized([width, 24.0], egui::Label::new(&self.preset_paths[index].path).truncate());
                    let (rect, _) = ui.allocate_exact_size(vec2(change_width, 24.0), Sense::hover());
                    if controls::settings_button(ui, rect, "change", "Change", enabled).clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_folder()
                    {
                        if let Some(path) = path.to_str() {
                            self.preset_paths[index].path = path.into(); changed = true;
                        } else { self.error = Some("The selected path cannot be represented as UTF-8.".into()); }
                    }
                    let (rect, _) = ui.allocate_exact_size(vec2(delete_width, 24.0), Sense::hover());
                    if controls::settings_button(ui, rect, "delete", "Delete", enabled).clicked() { removed = Some(index); }
                    let (recursive, _) = ui.allocate_exact_size(vec2(recursive_width, 24.0), Sense::hover());
                    ui.painter().text(recursive.left_center(), Align2::LEFT_CENTER, localization::tr(ui.ctx(), "Recursive"),
                        FontId::proportional(13.0), TEXT);
                    let check = Rect::from_center_size(pos2(recursive.right() - 12.0, recursive.center().y), vec2(24.0, 24.0));
                    if controls::settings_check(ui, check, "recursive", "Recursive",
                        &mut self.preset_paths[index].recursive, enabled).changed() { changed = true; }
                });
            });
        }
        if let Some(index) = removed { self.preset_paths.remove(index); changed = true; }
        let (rect, _) = ui.allocate_exact_size(vec2(32.0, 24.0), Sense::hover());
        if controls::settings_button(ui, rect, "add-preset-folder", "+", enabled).clicked()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            if let Some(path) = path.to_str() {
                self.preset_paths.push(library::PresetPath { path: path.into(), recursive: false }); changed = true;
            } else { self.error = Some("The selected path cannot be represented as UTF-8.".into()); }
        }
        if changed { self.scan_library(); self.save_preferences(); }
    }

    fn resource_path(&mut self, ui: &mut egui::Ui, index: usize, label: &str,
        browse: &str, extension: &str, message_open: bool,
        inspect_label: &str, inspect_available: bool) -> bool {
        let label_response = ui.label(egui::RichText::new(label).size(12.0).color(TEXT));
        let editable = ui.is_enabled() && !message_open && self.resources_available();
        let response = ui.add_sized([ui.available_width(), 24.0],
            egui::TextEdit::singleline(&mut self.model_paths[index])
                .interactive(editable).hint_text(localization::tr(ui.ctx(), "No file selected"))
                .font(FontId::proportional(13.0)).margin(vec2(9.0, 3.0)))
            .labelled_by(label_response.id);
        let mut changed = response.changed();
        let mut inspect_clicked = false;
        ui.horizontal(|ui| {
            if resource_button(ui, browse, self.resources_available()).clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter(label, &[extension]).pick_file()
            {
                if let Some(path) = path.to_str() {
                    if self.model_paths[index] != path {
                        self.model_paths[index] = path.to_owned(); changed = true;
                    }
                } else {
                    self.error = Some("The selected path cannot be represented as UTF-8. The current path was kept.".into());
                }
            }
            inspect_clicked = resource_button(ui, inspect_label,
                self.resources_available() && inspect_available && !self.model_paths[index].is_empty()).clicked();
        });
        if index == 0 && changed {
            self.model_info.clear();
            self.model_inspection.invalidate(&self.model_paths[index]);
            self.audio_model_changed();
        }
        if index == 1 && changed {
            self.userdata_load.invalidate(&self.model_paths[index]);
        }
        inspect_clicked
    }
}

