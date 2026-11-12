//! User-owned shortcut assignments. Activation is admitted once per raw input batch.
use super::*;
use serde::{Deserialize, Serialize};

pub(super) const ACTIONS: [&str; 5] = ["Go to the Next Waypoint", "Go to the Previous Waypoint",
    "Restart from the First Waypoint", "Shift Pitch Up by an Octave", "Shift Pitch Down by an Octave"];
pub(super) use super::midi_match::MidiBinding;
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Binding { key: Option<String>, midi: Option<MidiBinding> }
#[derive(Clone, Copy, PartialEq, Eq)]
enum Column { Keyboard, Midi }
struct Draft { action: usize, column: Column, key: Option<String>, midi: Option<MidiBinding> }
#[derive(Default)]
pub(super) struct State {
    bindings: [Binding; 5],
    draft: Option<Draft>,
    generation: u64,
    gate: super::midi_match::Gate,
}
impl State {
    pub fn load(&mut self, value: &Value) -> Result<bool> {
        let bindings = if value.is_null() { std::array::from_fn(|_| Binding::default()) }
            else { serde_json::from_value::<[Binding;5]>(value.clone())
                .map_err(|error| format!("Invalid input shortcuts: {error}"))? };
        if bindings.iter().any(|binding|
            binding.key.as_deref().is_some_and(|key| parse_key(key).is_none())
                || binding.midi.is_some_and(|midi| !midi.valid())) {
            return Err("Input shortcuts contain an invalid assignment.".into());
        }
        if bindings == self.bindings { return Ok(false); }
        let next = self.generation.checked_add(1).ok_or("Input shortcut generation is exhausted.")?;
        self.bindings = bindings; self.generation = next; Ok(true)
    }
    pub fn preference(&self) -> Value { json!(self.bindings) }
    pub fn editing(&self) -> bool { self.draft.is_some() }
    pub fn key_editing(&self) -> bool { self.draft.as_ref().is_some_and(|draft| draft.column == Column::Keyboard) }
    pub fn midi_editing(&self) -> bool { self.draft.as_ref().is_some_and(|draft| draft.column == Column::Midi) }
    pub fn cancel_edit(&mut self) { self.draft = None; }
    pub fn claims_key(&self, key: egui::Key) -> bool {
        self.bindings.iter().any(|entry| entry.key.as_deref().and_then(parse_key) == Some(key))
    }
    pub fn claimed_midi(&self, midi: MidiBinding) -> bool { self.midi_count(midi) != 0 }
    fn key_count(&self, key: egui::Key) -> usize {
        self.bindings.iter().filter(|entry| entry.key.as_deref().and_then(parse_key) == Some(key)).count()
    }
    fn midi_count(&self, midi: MidiBinding) -> usize { self.bindings.iter().filter(|entry| entry.midi == Some(midi)).count() }
    pub fn generation(&self) -> u64 { self.generation }
    fn active_key(&self, key: egui::Key) -> Option<usize> {
        let mut matches = self.bindings.iter().enumerate().filter(|(_, entry)| entry.key.as_deref().and_then(parse_key) == Some(key));
        let index = matches.next()?.0;
        matches.next().is_none().then_some(index)
    }
    fn active_midi(&self, midi: MidiBinding) -> Option<usize> {
        let mut matches = self.bindings.iter().enumerate().filter(|(_, entry)| entry.midi == Some(midi));
        let index = matches.next()?.0;
        matches.next().is_none().then_some(index)
    }
    fn input(&mut self, packed: u32) -> Option<MidiBinding> { self.gate.input(packed) }
    fn clear_held(&mut self) { self.gate.clear(); }
}
pub(super) const EXTRA_KEYS: [(egui::Key, &str); 17] = {
    use egui::Key::*;
    [(Backspace, "backspace"), (F1, "F1"), (F2, "F2"), (F3, "F3"), (F4, "F4"),
        (F5, "F5"), (F6, "F6"), (F7, "F7"), (F8, "F8"), (F9, "F9"), (F10, "F10"),
        (F11, "F11"), (F12, "F12"), (ArrowUp, "cursor up"), (ArrowDown, "cursor down"),
        (ArrowLeft, "cursor left"), (ArrowRight, "cursor right")]
};

fn parse_key(text: &str) -> Option<egui::Key> {
    egui::Key::from_name(text).filter(|key| key.name() == text && super::waypoints::key_bit(*key) != 0)
}

fn key_label(value: &str) -> &str {
    parse_key(value).and_then(|key| EXTRA_KEYS.iter().find(|(candidate, _)| *candidate == key))
        .map_or(value, |(_, label)| *label)
}
impl DesktopApp {
    pub(super) fn shortcut_key_mask(&self, ctx: &egui::Context) -> u64 {
        self.shortcuts.bindings.iter().filter_map(|entry| entry.key.as_deref().and_then(parse_key))
            .filter(|key| !self.shortcut_key_reserved_for_view(ctx, *key))
            .fold(0, |mask, key| mask | super::waypoints::key_bit(key))
    }
    pub(super) fn shortcut_key_reserved_for_view(&self, ctx: &egui::Context, key: egui::Key) -> bool {
        self.experimental_canvas_navigation_active(ctx) && matches!(key,
            egui::Key::W | egui::Key::A | egui::Key::S | egui::Key::D |
            egui::Key::ArrowUp | egui::Key::ArrowDown | egui::Key::ArrowLeft | egui::Key::ArrowRight | egui::Key::Space)
    }
    pub(super) fn shortcut_key_capture_or_execute(&mut self, ctx: &egui::Context, key: egui::Key, role: u8) -> bool {
        if role == 3 {
            if let Some(draft) = &mut self.shortcuts.draft { draft.key = Some(key.name().to_owned()); }
            return true;
        }
        if role != 1 || self.shortcut_key_reserved_for_view(ctx, key) { return false; }
        let waypoint_count = super::waypoints::key_match_count(&self.preset.document, key);
        if self.shortcuts.key_count(key) + waypoint_count > 1 {
            self.notice = Some("This key is assigned more than once in this preset. Change an assignment in Settings or on the waypoint.".into());
            return true;
        }
        if waypoint_count != 0 { return false; }
        let Some(action) = self.shortcuts.active_key(key) else { return false; };
        self.shortcut_action(ctx, action); true
    }
    fn shortcut_action(&mut self, ctx: &egui::Context, action: usize) {
        match action {
            0..=2 => if let Err(error) = self.waypoint_step(action) { self.error = Some(error); },
            3 => self.monitor_controls.shift_pitch_octave(12.0),
            4 => self.monitor_controls.shift_pitch_octave(-12.0),
            _ => return,
        }
        self.publish_live_condition();
        ctx.request_repaint();
    }
    fn shortcut_midi_role(&self, ctx: &egui::Context) -> u8 {
        if self.layout_blocked() || self.color_picker || self.error.is_some() || self.notice.is_some()
            || (self.editor.is_some() && !matches!(self.editor, Some(Edit::WaypointMidi(_))))
            || self.menu.is_some() || self.drag.is_some() || self.waypoint_active()
            || egui::Popup::is_any_open(ctx) || ctx.egui_wants_keyboard_input()
            || ctx.input(|i| !i.focused || i.pointer.any_down() || !i.raw.dropped_files.is_empty()
                || i.raw.events.iter().any(|e| matches!(e, egui::Event::WindowFocused(false)
                    | egui::Event::Ime(_) | egui::Event::PointerButton { .. }
                    | egui::Event::Key { key: egui::Key::Escape, pressed: true, .. })))
        { return 0; }
        if matches!(self.editor, Some(Edit::WaypointMidi(_))) {
            if self.waypoint_midi_draft_valid() { 3 } else { 0 }
        } else if self.settings { if self.shortcuts.midi_editing() { 2 } else { 0 } } else { 1 }
    }
    #[cfg(windows)]
    pub(super) fn shortcut_midi_frame(&mut self, ctx: &egui::Context) {
        if ctx.current_pass_index() != 0 { return; }
        let role = self.shortcut_midi_role(ctx);
        let scope = (self.plugin_document_revision(), self.shortcuts.generation(), role);
        let events = self.midi_input.reconcile(ctx, self.plugin_hosted, scope);
        if self.midi_input.take_reset() { self.shortcuts.clear_held(); }
        if self.error.is_none() && self.notice.is_none()
            && let Some(error) = self.midi_input.take_error()
        {
            self.error = Some(error);
            self.shortcuts.clear_held();
            return;
        }
        if role == 0 { self.shortcuts.clear_held(); return; }
        let mut binding = None;
        let mut ambiguous = false;
        for event in events {
            if let Some(value) = event.trigger {
                if binding.replace(value).is_some() { ambiguous = true; }
            }
        }
        // Still apply all release edges, but never guess between multiple downs.
        if ambiguous { return; }
        let Some(binding) = binding else { return; };
        if role == 2 {
            if let Some(draft) = &mut self.shortcuts.draft { draft.midi = Some(binding); }
        } else if role == 3 {
            self.waypoint_learn_midi(binding);
        } else {
            if self.shortcuts.midi_count(binding) + super::waypoints::midi_match_count(&self.preset.document, binding) > 1 {
                self.notice = Some("This MIDI event is assigned more than once in this preset. Change an assignment in Settings or on the waypoint.".into());
                return;
            }
            let point = super::waypoints::midi_target(&self.preset.document, binding);
            let action = self.shortcuts.active_midi(binding);
            match (point, action, self.shortcuts.claimed_midi(binding)) {
                (Some(index), None, false) => if let Err(error) = self.waypoint_recall(index) { self.error = Some(error); },
                (None, Some(action), _) if !super::waypoints::midi_in_use(&self.preset.document, binding) => self.shortcut_action(ctx, action),
                _ => {},
            }
        }
    }
    pub(super) fn shortcut_cell(&mut self, ui: &mut egui::Ui, rect: Rect, action: usize, column: usize) {
        let entry = &self.shortcuts.bindings[action];
        let value = if column == 0 { entry.key.as_deref().map(key_label).unwrap_or("Not Assigned Yet").to_owned() }
            else { entry.midi.map(MidiBinding::label).unwrap_or_else(|| "Not Assigned Yet".into()) };
        let response = ui.interact(rect, ui.id().with(("shortcut-binding", action, column)), Sense::click());
        ui.painter().rect_filled(rect, 12, Color32::from_black_alpha(if response.hovered() { 65 } else { 38 }));
        ui.painter().with_clip_rect(rect.shrink(3.0)).text(rect.center(), Align2::CENTER_CENTER,
            localization::tr(ui.ctx(), &value), FontId::proportional(12.0), Color32::from_rgb(189,189,189));
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(),
            format!("{}, {}: {}", ACTIONS[action], if column == 0 { "Keyboard" } else { "MIDI" }, value)));
        if response.clicked() && ui.ctx().current_pass_index() == 0 {
            response.surrender_focus();
            self.shortcuts.draft = Some(Draft { action, column: if column == 0 { Column::Keyboard } else { Column::Midi },
                key: entry.key.clone(), midi: entry.midi });
            self.waypoint_key_owner_changed();
        }
    }
    pub(super) fn shortcut_assignment_dialog(&mut self, ctx: &egui::Context, full: Rect) {
        let Some(draft) = self.shortcuts.draft.as_ref() else { return; };
        let (title, description, placeholder, value) = match draft.column {
            Column::Keyboard => ("Assign key shortcut", "Assign a key shortcut for the action.",
                "Press a key to assign a shortcut.", draft.key.as_deref().map(key_label).map(str::to_owned)),
            Column::Midi => ("Assign MIDI shortcut", "Assign a MIDI event for the action.",
                "Send a MIDI event to assign a shortcut.", draft.midi.map(MidiBinding::label)),
        };
        let rect = Rect::from_center_size(full.center(), vec2(320.0, 196.0));
        let field = Rect::from_min_size(rect.min + vec2(24.0, 80.0), vec2(272.0, 48.0));
        let lights = self.ambient_lights(full);
        let message_open = self.error.is_some() || self.notice.is_some() || self.layout_blocked();
        let mut confirm = false;
        let mut cancel = false;
        self.overlay_rects.push(full);
        controls::modal_panel(ctx, "shortcut-assignment", rect, Color32::from_black_alpha(25), |ui| {
            ui.set_min_size(rect.size());
            glass(ui.painter(), rect, &lights, false);
            ui.painter().rect_filled(rect, 10.0, Color32::from_white_alpha(40));
            ui.add_enabled_ui(!message_open, |ui| {
                ui.painter().text(rect.min + vec2(24.0, 27.0), Align2::LEFT_CENTER,
                    localization::tr(ctx, title), FontId::proportional(14.0), TEXT);
                ui.painter().text(rect.min + vec2(24.0, 68.0), Align2::LEFT_CENTER,
                    localization::tr(ctx, description), FontId::proportional(12.0), MUTED);
                ui.painter().rect_filled(field, 2.0, Color32::from_black_alpha(38));
                let painter = ui.painter().with_clip_rect(field.shrink2(vec2(10.0, 0.0)));
                if let Some(value) = value.as_deref() {
                    let assigned = localization::tr(ctx, "Currently assigned to: _")
                        .replacen('_', value, 1);
                    painter.text(field.min + vec2(10.0, 15.0), Align2::LEFT_CENTER,
                        assigned, FontId::proportional(12.0), TEXT);
                    let hint = match draft.column {
                        Column::Keyboard => "Press a key to change the assignment.",
                        Column::Midi => "Send a MIDI event to change the assignment.",
                    };
                    painter.text(field.min + vec2(10.0, 33.0), Align2::LEFT_CENTER,
                        localization::tr(ctx, hint), FontId::proportional(12.0), MUTED);
                } else {
                    painter.text(field.left_center() + vec2(10.0, 0.0), Align2::LEFT_CENTER,
                        localization::tr(ctx, placeholder), FontId::proportional(12.0), MUTED);
                }
                let ok_rect = Rect::from_min_size(rect.min + vec2(24.0, 148.0), vec2(132.0, 24.0));
                let cancel_rect = ok_rect.translate(vec2(140.0, 0.0));
                for button in [ok_rect, cancel_rect] {
                    ui.painter().rect_filled(button, 12.0, Color32::from_black_alpha(52));
                }
                let ok = controls::settings_button(ui,ok_rect,"shortcut-assignment-ok", "OK", true);
                let no = controls::settings_button(ui,cancel_rect,"shortcut-assignment-cancel", "Cancel", true);
                confirm = ok.clicked();
                cancel = no.clicked();
                if confirm { ok.surrender_focus(); }
                if cancel { no.surrender_focus(); }
            });
        });
        if !message_open && ctx.current_pass_index() == 0 {
            if ctx.input(|i| i.focused && i.key_pressed(egui::Key::Escape)) {
                ctx.input_mut(|i| { i.consume_key(egui::Modifiers::NONE, egui::Key::Escape); });
                cancel = true;
            }
            if cancel {
                self.shortcuts.draft = None;
                ctx.request_repaint();
            } else if confirm {
                if let Err(error) = self.shortcut_commit() { self.error = Some(error); }
                ctx.request_repaint();
            }
        }
    }
    fn shortcut_commit(&mut self) -> Result<()> {
        let draft = self.shortcuts.draft.as_ref().ok_or("No shortcut assignment")?;
        let action = draft.action; let column = draft.column;
        let key = draft.key.clone(); let midi = draft.midi;
        let document = &self.preset.document;
        let value = input_preferences::edit(&preferences_dir(), |value| {
            let mut latest = State::default();
            latest.load(&value["shortcuts"])?;
            match column {
                Column::Keyboard => if let Some(candidate) = key.as_deref().and_then(parse_key) {
                    if super::waypoints::key_in_use(document, candidate)
                        || latest.bindings.iter().enumerate().any(|(i,b)| i != action && b.key.as_deref().and_then(parse_key) == Some(candidate)) {
                        return Err("This key is already assigned to another shortcut or waypoint.".into());
                    }
                },
                Column::Midi => if let Some(candidate) = midi {
                    if super::waypoints::midi_in_use(document, candidate)
                        || latest.bindings.iter().enumerate().any(|(i,b)| i != action && b.midi == Some(candidate)) {
                        return Err("This MIDI event is already assigned to another shortcut or waypoint.".into());
                    }
                },
            }
            match column { Column::Keyboard => latest.bindings[action].key = key,
                Column::Midi => latest.bindings[action].midi = midi }
            value["shortcuts"] = latest.preference();
            Ok(())
        })?;
        if self.shortcuts.load(&value["shortcuts"])? { self.waypoint_key_owner_changed(); }
        self.shortcuts.draft = None;
        Ok(())
    }
}
