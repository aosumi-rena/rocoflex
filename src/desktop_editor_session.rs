use super::*;

pub(crate) struct CompletedEditorSession {
    serialized: String,
    paths: [String; 2],
    accepted: ExpectedCondition,
    preset: Preset,
    original: String,
    path: Option<PathBuf>,
    dirty: bool,
    probe: Probe,
    layout: canvas_state::CanvasState,
    view: Option<transactions::LayoutView>,
    status: Option<String>,
    offsets: Vec<Point>,
    labels: Vec<Point>,
    undo: Vec<transactions::UndoEntry>,
    scene: Scene,
    rays: Vec<Ray>,
    source_for_curve: Vec<usize>,
    source_for_anchor: Vec<usize>,
    revision: u64,
    mapping: usize,
    native_layout: bool,
    camera: f64,
    viewport_latched: bool,
    experimental: experimental_canvas::State,
    selected: Option<usize>,
    selected_at: Pos2,
    source_menu_cache: Option<source_menu::Cached>,
    allow_canvas_editing: bool,
    condition_available: bool,
    condition_error: Option<String>,
}

impl ExpectedCondition {

    pub(crate) fn same_editor_target(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Canvas { embedding: a, .. }, Self::Canvas { embedding: b, .. }) =>
                a.map(f32::to_bits) == b.map(f32::to_bits),
            (Self::Tone { controls: a, .. }, Self::Tone { controls: b, .. }) =>
                a.map(f32::to_bits) == b.map(f32::to_bits),
            _ => false,
        }
    }
}

impl CompletedEditorSession {
    pub(crate) fn matches(&self, document: &str, paths: &[String; 2], expected: &ExpectedCondition) -> bool {
        self.serialized == document && self.paths == *paths
            && self.accepted.same_editor_target(expected)
    }
}

impl DesktopApp {

    pub(crate) fn merge_plugin_session_controls(&mut self, document: &str,
        expected: &ExpectedCondition) -> Result<bool>
    {
        expected.validate()?;
        if !self.plugin_hosted || self.layout_blocked() { return Ok(false); }
        if self.plugin_document().ok().as_deref() != Some(document) { return Ok(false); }
        if expected.verify_current(self).is_err() { return Ok(false); }
        expected.apply_controls(self);
        Ok(true)
    }

    pub(crate) fn detach_plugin_session(&mut self, accepted: &ExpectedCondition)
        -> Result<CompletedEditorSession>
    {
        if !self.plugin_hosted || self.pending_layout.is_some() {
            return Err("Only a completed hosted canvas can be retained.".into());
        }
        accepted.verify_current(self)?;
        let serialized = self.plugin_document()?;
        let paths = self.plugin_model_paths().map(str::to_owned);
        let (condition_available, condition_error) = self.plugin_session_condition_state();
        self.experimental.retire_input();
        Ok(CompletedEditorSession {
            serialized, paths, accepted: accepted.clone(),
            preset: std::mem::replace(&mut self.preset, blank_preset()),
            original: std::mem::take(&mut self.original), path: self.path.take(), dirty: self.dirty,
            probe: self.probe,
            layout: std::mem::replace(&mut self.layout, canvas_state::CanvasState::Inspection),
            view: self.layout_view.take(), status: self.layout_status.take(),
            offsets: std::mem::take(&mut self.offsets), labels: std::mem::take(&mut self.label_offsets),
            undo: std::mem::take(&mut self.undo), scene: std::mem::take(&mut self.scene),
            rays: std::mem::take(&mut self.rays),
            source_for_curve: std::mem::take(&mut self.source_for_curve),
            source_for_anchor: std::mem::take(&mut self.source_for_anchor),
            revision: self.revision, mapping: self.mapping, native_layout: self.native_layout,
            camera: self.camera, viewport_latched: self.viewport_latched,
            experimental: std::mem::take(&mut self.experimental),
            selected: self.selected, selected_at: self.selected_at,
            source_menu_cache: self.source_menu_cache,
            allow_canvas_editing: self.allow_canvas_editing,
            condition_available, condition_error,
        })
    }

    pub(crate) fn adopt_plugin_session(&mut self, value: CompletedEditorSession,
        expected: &ExpectedCondition) -> Result<()>
    {
        if !self.plugin_hosted || self.layout_blocked() {
            return Err("The new editor cannot accept a retained canvas yet.".into());
        }
        expected.validate()?;
        if !value.accepted.same_editor_target(expected) {
            return Err("The host replaced the retained canvas voice target.".into());
        }

        expected.verify(&value.preset, value.view.as_ref())?;
        self.preset = value.preset; self.original = value.original; self.path = value.path;
        self.dirty = value.dirty; self.probe = value.probe; self.layout = value.layout;
        self.layout_view = value.view; self.layout_status = value.status;
        self.offsets = value.offsets; self.label_offsets = value.labels; self.undo = value.undo;
        self.scene = value.scene; self.rays = value.rays;
        self.source_for_curve = value.source_for_curve; self.source_for_anchor = value.source_for_anchor;
        self.revision = value.revision; self.mapping = value.mapping; self.native_layout = value.native_layout;
        self.camera = value.camera; self.viewport_latched = value.viewport_latched;
        self.experimental = value.experimental;
        self.selected = value.selected; self.selected_at = value.selected_at;
        self.source_menu_cache = value.source_menu_cache;
        self.allow_canvas_editing = value.allow_canvas_editing;
        self.adopt_plugin_session_condition_state(value.condition_available, value.condition_error);
        self.layout_next = self.layout.active().then(|| Instant::now() + Duration::from_millis(16));
        self.background_inputs.observe(&self.layout, &self.preset);

        expected.apply_controls(self);
        Ok(())
    }
}

#[allow(dead_code)]
fn require_plain_transfer_types() {
    fn require<T: Send + Sync>() {}
    require::<CompletedEditorSession>();
    require::<canvas_state::CanvasState>();
}

