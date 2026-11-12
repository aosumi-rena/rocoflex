use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Publication {
    Committed,
    Inspection,
    Retained,
}

#[derive(Clone)]
pub(super) struct LayoutView {
    pub positions: Vec<Vec<[f32; 2]>>,
    pub scalars: Vec<Vec<[f32; 4]>>,
    pub mix: Option<std::sync::Arc<crate::prototype_controls::ControllerMix>>,
}

pub(super) struct UndoEntry {
    document: Value,
    layout: canvas_state::CanvasState,
    view: Option<LayoutView>,
    offsets: Vec<Point>,
    labels: Vec<Point>,
    probe: Probe,
    status: Option<String>,
    restored_canvas: Option<[f32; 128]>,
}

enum InstallOrigin {
    PresetLoad,
    HostWorkingDocument(ExpectedCondition),
}

enum Change {
    Append,
    Remove(usize),
    Install(InstallOrigin),
}

enum FullContinuation {
    Append,
    Remove(usize),
}

fn full_continuation(
    state: &canvas_state::CanvasState,
    sources: &[layout::Source],
    change: &Change,
) -> Option<FullContinuation> {
    let current = &state.as_ref()?.sources;
    match change {
        Change::Append
            if sources.len() == current.len() + 1
                && layout::same_sources(current, &sources[..current.len()]) =>
        {
            Some(FullContinuation::Append)
        }
        Change::Remove(index)
            if *index < current.len()
                && sources.len() + 1 == current.len()
                && layout::same_sources(&current[..*index], &sources[..*index])
                && layout::same_sources(&current[*index + 1..], &sources[*index..]) =>
        {
            Some(FullContinuation::Remove(*index))
        }
        _ => None,
    }
}

struct DocumentStage {
    preset: Preset,
    offsets: Vec<Point>,
    labels: Vec<Point>,
    probe: Probe,
    change: Change,
}

pub(super) struct PendingLayout {
    #[allow(dead_code)]
    prepared: Option<canvas_state::PreparedCanvas>,
    document: Option<DocumentStage>,
    candidate_document: Option<Value>,
    accepted_source: Option<Value>,
    accepted_later: Vec<Value>,
    issue: String,
    discard_confirm: bool,
    pub loaded_input: Option<(PathBuf, String)>,
}

fn sources(preset: &Preset) -> Result<Vec<layout::Source>> {
    preset
        .sources
        .iter()
        .map(|s| {
            let rows: std::result::Result<Vec<[f32; 4]>, _> = s
                .points
                .iter()
                .map(|p| p.reduced.as_slice().try_into())
                .collect();
            let rows = rows.map_err(|_| "Inspection layout: invalid reduced coordinates.")?;
            Ok(if s.kind == "trajectory" {
                layout::Source::Trajectory {
                    reduced: rows,
                    enabled: s.enabled,
                }
            } else {
                layout::Source::Point {
                    reduced: *rows.first().ok_or("Missing reduced point.")?,
                    enabled: s.enabled,
                }
            })
        })
        .collect()
}

fn view(
    preset: &Preset,
    state: &canvas_state::CanvasState,
    offsets: &[Point],
    labels: &[Point],
) -> Result<LayoutView> {
    let source_list = sources(preset)?;
    if !state
        .sources()
        .is_some_and(|current| layout::same_sources(&source_list, current))
        || offsets.len() != preset.sources.len()
        || labels.len() != preset.sources.len()
        || offsets
            .iter()
            .chain(labels)
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return Err("Document and canvas shape do not agree.".into());
    }
    let checked = state.checked_view()?;
    Ok(LayoutView {
        positions: checked.positions,
        scalars: checked.scalars,
        mix: state.as_ref().and_then(layout::Controller::completed_mix).cloned(),
    })
}

impl DesktopApp {
    pub(super) fn capture_undo(&self) -> UndoEntry {
        UndoEntry {
            document: self.preset.document.clone(),
            layout: self.layout.clone(),
            view: self.layout_view.clone(),
            offsets: self.offsets.clone(),
            labels: self.label_offsets.clone(),
            probe: self.probe,
            status: self.layout_status.clone(),
            restored_canvas: self.live.restored_canvas(),
        }
    }

    pub(super) fn remember_entry(&mut self, entry: UndoEntry) {
        self.undo.push(entry);
        if self.undo.len() > 40 {
            self.undo.remove(0);
        }
    }

    pub(super) fn undo_committed_layout(&mut self) {
        if self.layout_blocked() {
            return;
        }
        let Some(entry) = self.undo.last() else { return; };

        let checked = (|| -> Result<Preset> {
            let preset = Preset::parse(entry.document.clone())?;
            if entry.offsets.len() != preset.sources.len()
                || entry.labels.len() != preset.sources.len()
                || entry.offsets.iter().chain(&entry.labels)
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
                || !entry.probe.position.x.is_finite() || !entry.probe.position.y.is_finite()
                || !entry.probe.radius.is_finite() || entry.probe.radius <= 0.0
            {
                return Err("Document and canvas history do not agree.".into());
            }
            match (&entry.layout, &entry.view) {
                (canvas_state::CanvasState::Inspection, None) => {},
                (canvas_state::CanvasState::Inspection, Some(_)) | (_, None) =>
                    return Err("Canvas history has no matching display view.".into()),
                (_, Some(view)) => {
                    let source_list = sources(&preset)?;
                    if !entry.layout.sources()
                        .is_some_and(|current| layout::same_sources(&source_list, current))
                        || view.positions.len() != source_list.len()
                        || view.scalars.len() != source_list.len()
                        || source_list.iter().zip(&view.positions).zip(&view.scalars)
                            .any(|((source, positions), scalars)| {
                                let count = match source {
                                    layout::Source::Point { .. } => 1,
                                    layout::Source::Trajectory { reduced, .. } => reduced.len(),
                                };
                                positions.len() != count || scalars.len() != count
                            })
                        || view.positions.iter().flatten().flatten()
                            .chain(view.scalars.iter().flatten().flatten())
                            .any(|value| !value.is_finite())
                    {
                        return Err("Document and canvas history do not agree.".into());
                    }
                }
            }
            if let Some(embedding) = entry.restored_canvas {
                ExpectedCondition::Canvas { embedding, parameters: crate::prototype_controls::Parameters {
                    pregain_db: self.monitor_controls.pregain_db(),
                    pitch_semitones: self.monitor_controls.pitch_semitones(), tone_db: 0.0,
                } }.verify(&preset, entry.view.as_ref())?;
            }
            Ok(preset)
        })();
        let preset = match checked {
            Ok(checked) => checked,
            Err(error) => { self.error = Some(error); return; }
        };
        let UndoEntry { layout, view, offsets, labels, probe, status, restored_canvas, .. } =
            self.undo.pop().expect("validated retained undo entry");

        self.background_inputs.begin_document();
        self.audio_destination_changed();
        self.waypoint_reset();
        self.preset = preset;
        self.offsets = offsets;
        self.label_offsets = labels;
        self.probe = probe;
        self.experimental_restore_document();
        self.background_inputs.observe(&layout, &self.preset);
        self.layout = layout;
        self.layout_view = view;
        self.layout_status = status;
        self.live.set_restored_canvas(restored_canvas);
        self.layout_next = self.layout.active()
            .then(|| Instant::now() + Duration::from_millis(16));
        self.selected = None;
        self.drag = None;
        self.camera = 1.0;
        self.viewport_latched = false;
        self.source_camera = None;
        self.menu = None;
        self.voices_open = false;
        self.rebuild_scene();
        self.dirty = true;
    }

    pub(super) fn initialize_actual_no_file_startup(&mut self) {
        match canvas_state::CanvasState::actual_no_file_startup(partial::ProbeControls {
            xy: [self.probe.position.x as f32, self.probe.position.y as f32],
            radius: self.probe.radius,
        }) {
            Ok(state) => match view(&self.preset, &state, &self.offsets, &self.label_offsets) {
                Ok(checked) => {
                    self.layout = state;
                    self.layout_view = Some(checked);
                    self.rebuild_scene();
                }
                Err(error) => self.layout_status = Some(error),
            },
            Err(error) => self.layout_status = Some(error),
        }
    }

    pub(super) fn publish_probe_event(&mut self) -> bool {
        if let Err(error) = self.layout.probe_changed(partial::ProbeControls {
            xy: [self.probe.position.x as f32, self.probe.position.y as f32],
            radius: self.probe.radius,
        }) {
            self.layout_status = Some(error);
            self.layout_next = None;
            return false;
        }
        if self.layout.active() {
            self.layout_next
                .get_or_insert_with(|| Instant::now() + Duration::from_millis(16));
        }
        true
    }

    pub(super) fn layout_blocked(&self) -> bool {
        self.pending_layout.is_some()
    }

    pub(super) fn current_layout_matches_document(&self) -> bool {
        !self.layout_blocked()
            && view(&self.preset, &self.layout, &self.offsets, &self.label_offsets).is_ok()
    }

    fn retain_layout(
        &mut self,
        prepared: Option<canvas_state::PreparedCanvas>,
        document: Option<DocumentStage>,
        candidate_document: Option<Value>,
        accepted_source: Option<Value>,
        issue: String,
    ) -> Publication {
        self.layout_status = Some(issue.clone());
        self.layout_next = None;
        self.drag = None;
        self.pending_layout = Some(PendingLayout {
            prepared,
            document,
            candidate_document,
            accepted_source,
            accepted_later: vec![],
            issue,
            discard_confirm: false,
            loaded_input: None,
        });
        Publication::Retained
    }

    fn commit_stage(&mut self, stage: DocumentStage) {
        let installing = matches!(stage.change, Change::Install(_));
        if !installing {
            self.remember();
        }
        self.waypoint_reset();
        self.live.set_restored_canvas(None);
        self.preset = stage.preset;
        if installing {
            self.background_inputs.begin_document();
            self.audio_destination_changed();
        }
        self.offsets = stage.offsets;
        self.label_offsets = stage.labels;
        self.probe = stage.probe;
        if installing { self.experimental_restore_document(); }
        self.selected = None;
        self.drag = None;
        if installing {
            self.camera = 1.0;
            self.viewport_latched = false;
            self.source_camera = None;
        }
        self.menu = None;
        self.voices_open = false;
        if let Change::Install(InstallOrigin::HostWorkingDocument(expected)) = stage.change {
            expected.apply_controls(self);
        }
    }

    fn finish_publication(
        &mut self,
        prepared: canvas_state::PreparedCanvas,
        stage: Option<DocumentStage>,
        accepted: Option<Value>,
        reconstruction: bool,
    ) -> Publication {
        let prepared = match prepared.finish_probe() {
            Ok(prepared) => prepared,
            Err((prepared, error)) => {
                return self.retain_layout(Some(prepared), stage, None, accepted, error);
            }
        };
        let ready = prepared.ready().expect("completed canvas stage");
        let checked = match &stage {
            Some(s) => view(&s.preset, ready, &s.offsets, &s.labels),
            None => view(&self.preset, ready, &self.offsets, &self.label_offsets),
        };
        let checked = match checked {
            Ok(checked) => checked,
            Err(error) => return self.retain_layout(Some(prepared), stage, None, accepted, error),
        };
        if let Some(DocumentStage { preset, change: Change::Install(InstallOrigin::HostWorkingDocument(expected)), .. }) = &stage {
            if let Err(error) = expected.verify(preset, Some(&checked)) {
                return self.retain_layout(Some(prepared), stage, None, accepted, error);
            }
        }
        let moved = prepared.geometry_changed();
        let document_change = stage.is_some();
        let installing = stage
            .as_ref()
            .is_some_and(|s| matches!(s.change, Change::Install(_)));
        let state = prepared
            .into_ready()
            .ok()
            .expect("completed checked layout");
        if let Some(stage) = stage {
            self.commit_stage(stage);
        } else if reconstruction {
            self.background_inputs.begin_document();
        }
        self.background_inputs.observe(&state, &self.preset);
        self.layout = state;
        self.layout_view = Some(checked);
        self.layout_status = None;
        self.layout_next = self
            .layout
            .active()
            .then(|| Instant::now() + Duration::from_millis(16));
        if document_change || moved {
            self.rebuild_scene();
        }
        if document_change && !installing {
            self.changed();
        }
        Publication::Committed
    }

    fn publish_stage(&mut self, stage: DocumentStage, accepted: Option<Value>) -> Publication {
        let source_list = match sources(&stage.preset) {
            Ok(sources) => sources,
            Err(error) => return self.retain_layout(None, Some(stage), None, accepted, error),
        };
        let mut first_trajectory = false;
        let continuation = full_continuation(&self.layout, &source_list, &stage.change);
        if matches!(stage.change, Change::Append)
            && let canvas_state::CanvasState::Partial {
                controller: state,
                ..
            } = &self.layout
            && source_list.len() == state.sources().len() + 1
            && layout::same_sources(state.sources(), &source_list[..state.sources().len()])
        {
            let new_source = source_list.last().expect("source append").clone();
            let route = if state.sources().is_empty() {
                first_trajectory = matches!(&new_source, layout::Source::Trajectory { .. });
                layout::context::partial_gate::Route::FirstTrajectory
            } else {
                layout::context::partial_gate::Route::Promotion(
                    partial::GeometryHook::ExactTwoStandaloneFirstProductInstalled,
                )
            };
            let eligible = (state.sources().is_empty()
                && matches!(&new_source, layout::Source::Point { .. }))
                || layout::context::partial_gate::contextual_admission(state, &new_source, route)
                    .is_ok();
            if eligible {
                return match self.layout.prepare_append(new_source) {
                    Ok(prepared) => self.finish_publication(prepared, Some(stage), accepted, false),
                    Err(error) => self.retain_layout(None, Some(stage), None, accepted, error),
                };
            }
        }
        let inspection = if first_trajectory {
            Some(layout::InspectionDomain::FirstTrajectory)
        } else if continuation.is_some() {
            layout::context::inspection_domain(&source_list)
        } else {
            layout::inspection_domain(&source_list)
        }
        .or_else(|| {
            if (self.layout.as_ref().is_none() || matches!(stage.change, Change::Install(_)))
                && (stage.preset.transformation[..4].iter().all(|x| *x == 0.0)
                    || stage.preset.transformation[4..].iter().all(|x| *x == 0.0))
            {
                Some(layout::InspectionDomain::IncomingProjection)
            } else {
                None
            }
        });
        if let Some(reason) = inspection {
            let empty = stage.preset.sources.is_empty();
            if let Change::Install(InstallOrigin::HostWorkingDocument(expected)) = &stage.change {
                let checked = if empty { expected.verify(&stage.preset, None) }
                    else { Err("The saved canvas cannot be reconstructed in the supported display domain.".into()) };
                if let Err(error) = checked {
                    return self.retain_layout(None, Some(stage), None, accepted, error);
                }
            }
            let installing = matches!(stage.change, Change::Install(_));
            self.commit_stage(stage);
            self.layout = canvas_state::CanvasState::Inspection;
            self.layout_view = None;
            self.layout_status = if empty {
                None
            } else {
                Some(reason.message().into())
            };
            self.layout_next = None;
            self.rebuild_scene();
            if !installing {
                self.changed();
            }
            return Publication::Inspection;
        }
        let prepared = match continuation {
            Some(FullContinuation::Append) => {
                self.layout
                    .prepare_append(source_list.last().expect("source append").clone())
            }
            Some(FullContinuation::Remove(index)) => self.layout.prepare_remove(index),
            _ => Ok(canvas_state::PreparedCanvas::document_display(
                source_list,
                stage.preset.transformation,
                [stage.probe.position.x as f32, stage.probe.position.y as f32],
                stage.probe.radius,
                self.layout.cache_binding(),
                match &stage.change {
                    Change::Install(InstallOrigin::HostWorkingDocument(_)) => layout::context::display_gate::Origin::HostWorkingDocument,
                    _ => layout::context::display_gate::Origin::ExistingInspectionDisplay,
                },
            )),
        };
        match prepared {
            Ok(prepared) => self.finish_publication(prepared, Some(stage), accepted, false),
            Err(error) => self.retain_layout(None, Some(stage), None, accepted, error),
        }
    }

    pub(super) fn publish_install(&mut self, preset: Preset) -> bool {
        self.publish_install_origin(preset, InstallOrigin::PresetLoad)
    }

    pub(super) fn publish_host_install(&mut self, preset: Preset, expected: ExpectedCondition) -> bool {
        self.publish_install_origin(preset, InstallOrigin::HostWorkingDocument(expected))
    }

    fn publish_install_origin(&mut self, preset: Preset, origin: InstallOrigin) -> bool {
        if self.layout_blocked() {
            return false;
        }
        let probe = Probe {
            position: Point::new(
                preset.probe[0].clamp(-2.15, 2.15),
                preset.probe[1].clamp(-1.4, 1.4),
            ),
            radius: preset.probe[2].clamp(0.04, 0.15),
        };
        let offsets = read_offsets(&preset.document, "sourceOffsets", preset.sources.len());
        let labels = read_offsets(&preset.document, "labelOffsets", preset.sources.len());
        self.publish_stage(
            DocumentStage {
                preset,
                offsets,
                labels,
                probe,
                change: Change::Install(origin),
            },
            None,
        ) != Publication::Retained
    }

    pub(super) fn publish_append(&mut self, source: Value) -> Publication {
        if let Some(pending) = &mut self.pending_layout {
            pending.accepted_later.push(source);
            return Publication::Retained;
        }
        let mut document = self.preset.document.clone();
        let Some(array) = voice_mut(&mut document)["sources"].as_array_mut() else {
            return self.retain_layout(
                None,
                None,
                Some(document),
                Some(source),
                "Preset has no source list.".into(),
            );
        };
        array.push(source.clone());
        let preset = match Preset::parse(document.clone()) {
            Ok(preset) => preset,
            Err(error) => {
                return self.retain_layout(None, None, Some(document), Some(source), error);
            }
        };
        let mut offsets = self.offsets.clone();
        offsets.push(Point::default());
        let mut labels = self.label_offsets.clone();
        labels.push(Point::default());
        self.publish_stage(
            DocumentStage {
                preset,
                offsets,
                labels,
                probe: self.probe,
                change: Change::Append,
            },
            Some(source),
        )
    }

    pub(super) fn publish_remove(&mut self, index: usize) {
        if self.layout_blocked() || index >= self.preset.sources.len() {
            return;
        }
        let mut document = self.preset.document.clone();
        let Some(array) = voice_mut(&mut document)["sources"].as_array_mut() else {
            return;
        };
        array.remove(index);
        let mut offsets = self.offsets.clone();
        offsets.remove(index);
        let mut labels = self.label_offsets.clone();
        labels.remove(index);
        document["rocoflexCanvas"] = json!({"sourceOffsets":offsets,"labelOffsets":labels});
        let preset = match Preset::parse(document.clone()) {
            Ok(preset) => preset,
            Err(error) => {
                self.retain_layout(None, None, Some(document), None, error);
                return;
            }
        };
        self.publish_stage(
            DocumentStage {
                preset,
                offsets,
                labels,
                probe: self.probe,
                change: Change::Remove(index),
            },
            None,
        );
    }

    pub(super) fn sync_committed_layout(&mut self) {
        if self.layout_blocked() {
            return;
        }
        let source_list = match sources(&self.preset) {
            Ok(sources) => sources,
            Err(error) => {
                self.layout_status = Some(error);
                return;
            }
        };

        if self
            .layout
            .sources()
            .is_some_and(|current| layout::same_sources(current, &source_list))
        {
            let updated = self.layout.matching_metadata(&source_list);
            match updated.and_then(|state| {
                view(&self.preset, &state, &self.offsets, &self.label_offsets)
                    .map(|checked| (state, checked))
            }) {
                Ok((state, checked)) => {
                    self.background_inputs.observe(&state, &self.preset);
                    self.layout = state;
                    self.layout_view = Some(checked);
                }
                Err(error) => {
                    self.layout_status = Some(error);
                    self.layout_next = None;
                }
            }
            return;
        }
        if let Some(reason) = layout::inspection_domain(&source_list) {
            self.background_inputs.begin_document();
            self.layout = canvas_state::CanvasState::Inspection;
            self.layout_view = None;
            self.layout_next = None;
            self.layout_status = if source_list.is_empty() {
                None
            } else {
                Some(reason.message().into())
            };
            return;
        }
        let prepared = canvas_state::PreparedCanvas::inspection_display(
            source_list,
            self.preset.transformation,
            [self.probe.position.x as f32, self.probe.position.y as f32],
            self.probe.radius,
            self.layout.cache_binding(),
        );
        self.finish_publication(prepared, None, None, true);
    }

    pub(super) fn step_committed_layout(&mut self, ctx: &egui::Context) {
        if self.layout_blocked() || self.mapping != 0 {
            return;
        }
        if self.layout_next.is_some_and(|due| Instant::now() >= due)
            && let Some(prepared) = self.layout.prepare_callback()
        {
            self.finish_publication(prepared, None, None, false);
        }
        if let Some(due) = self.layout_next {
            ctx.request_repaint_after(due.saturating_duration_since(Instant::now()));
        }
    }

    pub(super) fn abandon_pending_layout(&mut self) {
        if self.pending_layout.take().is_none() {
            return;
        }
        self.layout_next = None;
        self.layout_status = Some("Retained changes discarded. Canvas updates are paused.".into());
        self.color_picker = false;
        self.tone_epoch = self.tone_epoch.wrapping_add(1);
        self.cancel_audio_generation();
        self.drag = None;
    }

    pub(super) fn pending_layout_ui(&mut self, ctx: &egui::Context, full: Rect) -> bool {
        if self.pending_layout.is_some() {
            self.empty_background.submit_current(ctx);
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        let message_open = self.error.is_some() || self.notice.is_some();
        let dismiss_error = self.error.is_some();
        let dismiss_message = message_open
            && ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if dismiss_message {
            ctx.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            });
        }
        let Some(pending) = &mut self.pending_layout else {
            return message_open;
        };
        let count = usize::from(pending.accepted_source.is_some()) + pending.accepted_later.len();
        let change =
            usize::from(pending.document.is_some() || pending.candidate_document.is_some());
        let mut discard = false;
        let mut save = false;
        let mut export = false;
        egui::Window::new("Canvas change retained").id(egui::Id::new("pending-layout"))
            .collapsible(false).resizable(false).fixed_size(vec2(500.0,230.0))
            .fixed_pos(full.center()-vec2(250.0,115.0)).show(ctx,|ui| {
                ui.add_enabled_ui(!message_open, |ui| {
                ui.label("The last committed preset is still displayed. Canvas updates are paused.");
                ui.label(format!("Retained: {change} preset change, {count} accepted source(s)."));
                egui::ScrollArea::vertical().id_salt("pending-layout-issue")
                    .max_height(64.0).auto_shrink([false, true]).show(ui, |ui| {
                        ui.label(egui::RichText::new(clipped_title(&pending.issue, 16 * 1024))
                            .color(MUTED).size(12.0));
                    });
                ui.horizontal(|ui| {save=ui.button("Save Committed Preset").clicked();export=ui.button("Export Committed Preset").clicked();});
                if pending.discard_confirm {
                    ui.label(format!("Discard the retained change and all {count} accepted source(s)? This also cancels any running analysis request."));
                    ui.horizontal(|ui| {discard=ui.button("Discard Retained Changes").clicked();if ui.button("Keep Retained Changes").clicked(){pending.discard_confirm=false;}});
                } else if ui.button("Discard Retained Changes...").clicked() { pending.discard_confirm=true; }
                });
            });
        let actions_available = |app: &Self| {
            !message_open && app.pending_layout.is_some()
                && app.error.is_none() && app.notice.is_none()
        };
        if save && actions_available(self) {
            if self.preset_needs_name() || self.experimental_save_requires_copy() {
                self.export();
            } else {
                self.save_current();
            }
        }
        if export && actions_available(self) {
            self.export();
        }
        if discard && actions_available(self) {
            self.abandon_pending_layout();
            ctx.request_repaint();
        }
        let message_in_frame = message_open || self.error.is_some() || self.notice.is_some();
        self.message_ui(ctx, full);
        if dismiss_message {
            if dismiss_error {
                self.error = None;
            } else {
                self.notice = None;
            }
            ctx.request_repaint();
        }
        message_in_frame
    }
}
