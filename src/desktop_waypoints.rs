use super::*;
use crate::preset::{Waypoint, waypoints};

pub(super) struct State {

    basis: Option<Value>,
    selected: Option<usize>,
    armed: Option<usize>,
    reroute: Option<Reroute>,
    claimed: bool,
    draft: Option<KeyDraft>,
    batch_allowed: bool,
    batch_used: bool,
    primary_cancelled: bool,
    installation: Option<u64>,
}

impl Default for State {
    fn default() -> Self {
        Self { basis: None, selected: None, armed: None, reroute: None, claimed: false,
            draft: None, batch_allowed: true, batch_used: false, primary_cancelled: false, installation: Some(0) }
    }
}

struct KeyDraft { index: usize, basis: Value, value: Option<String> }

pub(crate) const ALL_KEY_BITS: u64 = ((1u64 << 36) - 1) << 3;

pub(crate) fn key_bit(key: egui::Key) -> u64 {
    use egui::Key::*;
    let keys = [Num0,Num1,Num2,Num3,Num4,Num5,Num6,Num7,Num8,Num9,
        A,B,C,D,E,F,G,H,I,J,K,L,M,N,O,P,Q,R,S,T,U,V,W,X,Y,Z];
    keys.iter().position(|candidate| *candidate == key).map_or(0, |index| 1u64 << (index + 3))
}

fn supported_key(value: &str) -> Option<egui::Key> {
    let bytes = value.as_bytes();
    if bytes.len() != 1 || !(bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit()) { return None; }
    egui::Key::from_name(value).filter(|key| key_bit(*key) != 0)
}

fn key_description(document: &Value, index: usize, route: u64) -> Option<String> {
    let row = raw_array(document)?.as_array()?.get(index)?;
    match row.get("keyBinding") {
        None => Some(route.to_string()),
        Some(Value::String(value)) => Some(value.clone()),
        Some(_) => None,
    }
}

fn effective_keys(document: &Value) -> Vec<(usize, egui::Key)> {
    let Ok(points) = waypoints(document) else { return Vec::new(); };
    points.iter().enumerate().filter_map(|(index, point)| {
        let description = key_description(document, index, point.route)?;
        supported_key(&description).map(|key| (index, key))
    }).collect()
}

fn assigned(document: &Value, index: usize, description: &str) -> Result<Value> {
    let key = supported_key(description).ok_or("Unsupported key shortcut")?;
    waypoints(document)?.get(index).ok_or("No selected waypoint")?;
    if effective_keys(document).iter().any(|(other, candidate)| *other != index && *candidate == key) {
        return Err("This key is already assigned to another waypoint.".into());
    }
    let mut array = raw_array(document).cloned().ok_or("No waypoint list")?;
    array[index].as_object_mut().ok_or("Invalid waypoint")?.insert("keyBinding".into(), json!(description));
    Ok(array)
}

struct Reroute {
    picked: Vec<usize>,
    start: Option<transactions::UndoEntry>,
}

fn raw_array(document: &Value) -> Option<&Value> { voice(document).get("waypoints") }

fn editable(document: &Value) -> Result<(Vec<Waypoint>, Vec<Value>)> {
    let points = waypoints(document)?;
    if points.iter().any(|point| point.route > points.len() as u64) {
        return Err("Waypoint routes must be consecutive before editing.".into());
    }
    if !voice(document).is_object() { return Err("Invalid waypoint voice".into()); }
    let rows = raw_array(document).and_then(Value::as_array).cloned().unwrap_or_default();
    Ok((points, rows))
}

fn added(document: &Value, xy: [f64; 2]) -> Result<Value> {
    let (points, mut rows) = editable(document)?;
    if rows.len() == 256 || xy.iter().any(|v| !v.is_finite()) {
        return Err("The waypoint cannot be added.".into());
    }
    rows.push(json!({"id": points.len() + 1, "midiBinding": "2.veO", "x": xy[0], "y": xy[1]}));
    Ok(Value::Array(rows))
}

fn deleted(document: &Value, index: usize) -> Result<Value> {
    let (points, mut rows) = editable(document)?;
    let removed = points.get(index).ok_or("No selected waypoint")?.route;
    rows.remove(index);
    for row in &mut rows {
        let id = row["id"].as_u64().expect("validated route");
        if id > removed { row["id"] = json!(id - 1); }
    }
    Ok(Value::Array(rows))
}

fn reordered(document: &Value, order: &[usize]) -> Result<Value> {
    let (points, mut rows) = editable(document)?;
    if order.len() != points.len() { return Err("Incomplete waypoint order".into()); }
    for (route, &index) in order.iter().enumerate() {
        if index >= rows.len() || order[..route].contains(&index) {
            return Err("Invalid waypoint order".into());
        }
        rows[index]["id"] = json!(route + 1);
    }
    Ok(Value::Array(rows))
}

impl DesktopApp {
    pub(super) fn waypoint_cancel(&mut self) {
        let state = &mut self.monitor_controls.waypoint_ui;
        state.armed = None;
        state.reroute = None;
    }

    pub(super) fn waypoint_reset(&mut self) {
        let old = &self.monitor_controls.waypoint_ui;
        self.monitor_controls.waypoint_ui = State { batch_allowed: old.batch_allowed,
            batch_used: old.batch_used, primary_cancelled: old.primary_cancelled,
            installation: old.installation.and_then(|v| v.checked_add(1)),
            ..Default::default() };
    }

    pub(crate) fn set_waypoint_key_batch(&mut self, allowed: bool) {
        self.monitor_controls.waypoint_ui.batch_allowed = allowed;
        self.monitor_controls.waypoint_ui.batch_used = false;
    }

    fn waypoint_keyboard_role(&self, ctx: &egui::Context) -> u8 {
        if self.layout_blocked() || self.settings || self.color_picker || self.error.is_some()
            || self.notice.is_some() || self.menu.is_some() || egui::Popup::is_any_open(ctx)
            || self.drag.is_some() || self.waypoint_active()
            || ctx.input(|i| !i.focused || i.pointer.any_down()) { return 0; }
        if let Some(Edit::WaypointKey(index)) = self.editor {
            return if self.monitor_controls.waypoint_ui.draft.as_ref().is_some_and(|draft|
                draft.index == index && Some(&draft.basis) == raw_array(&self.preset.document)) { 2 } else { 0 };
        }
        if self.editor.is_some() || ctx.egui_wants_keyboard_input() { 0 } else { 1 }
    }

    pub(crate) fn waypoint_key_owner(&self, ctx: &egui::Context) -> Option<(u64,u64,u8)> {
        Some((self.plugin_document_revision(), self.monitor_controls.waypoint_ui.installation?,
            self.waypoint_keyboard_role(ctx)))
    }

    pub(super) fn waypoint_key_mask(&self, ctx: &egui::Context) -> u64 {
        match self.waypoint_keyboard_role(ctx) {
            2 => ALL_KEY_BITS,
            1 => {
                let keys = effective_keys(&self.preset.document);
                keys.iter().filter(|(_, key)| keys.iter().filter(|(_, k)| k == key).count() == 1)
                    .fold(0, |mask, (_, key)| mask | key_bit(*key))
            }
            _ => 0,
        }
    }

    pub(super) fn waypoint_key_dispatch(&mut self, ctx: &egui::Context) {
        if ctx.current_pass_index() != 0 { return; }
        let state = &mut self.monitor_controls.waypoint_ui;
        if state.batch_used { return; }
        state.batch_used = true;
        if !state.batch_allowed { return; }
        let role = self.waypoint_keyboard_role(ctx);
        if role == 0 { return; }
        let key = ctx.input(|input| {
            if !input.raw.dropped_files.is_empty() || input.raw.events.iter().any(|event| matches!(event,
                egui::Event::PointerButton { .. } | egui::Event::WindowFocused(false) | egui::Event::Ime(_)
                | egui::Event::Key { key: egui::Key::Escape, pressed: true, .. })) { return None; }
            let mut downs = input.raw.events.iter().filter_map(|event| match event {
                egui::Event::Key { key, pressed: true, repeat, modifiers, .. } => Some((*key,*repeat,*modifiers)),
                _ => None,
            });
            let (key, repeat, modifiers) = downs.next()?;
            if downs.next().is_some() || repeat || modifiers.any() || key_bit(key) == 0 { return None; }
            input.events.iter().any(|event| matches!(event,
                egui::Event::Key { key: current, pressed: true, repeat: false, modifiers, .. }
                    if *current == key && !modifiers.any())).then_some(key)
        });
        let Some(key) = key else { return; };
        if role == 2 {
            ctx.input_mut(|input| { input.consume_key(egui::Modifiers::NONE, key); });
            let description = key.name().to_owned();
            if let Some(draft) = &mut self.monitor_controls.waypoint_ui.draft { draft.value = Some(description); }
        } else {
            let keys = effective_keys(&self.preset.document);
            let mut matches = keys.iter().filter(|(_, candidate)| *candidate == key);
            if let Some(&(index, _)) = matches.next() && matches.next().is_none() {
                ctx.input_mut(|input| { input.consume_key(egui::Modifiers::NONE, key); });
                if let Err(error) = self.waypoint_recall(index) { self.error = Some(error); }
            }
        }
    }

    pub(super) fn waypoint_begin_key(&mut self, index: usize) {
        if self.layout_blocked() { return; }
        let Ok(points) = waypoints(&self.preset.document) else { return; };
        let Some(point) = points.get(index) else { return; };
        let Some(basis) = raw_array(&self.preset.document).cloned() else { return; };
        let value = key_description(&self.preset.document, index, point.route);
        self.waypoint_cancel();
        let state = &mut self.monitor_controls.waypoint_ui;
        state.installation = state.installation.and_then(|value| value.checked_add(1));
        state.draft = Some(KeyDraft { index, basis, value });
        self.menu = None;
        self.editor = Some(Edit::WaypointKey(index));
    }

    pub(super) fn waypoint_confirm_key(&mut self) -> Result<()> {
        let draft = self.monitor_controls.waypoint_ui.draft.as_ref().ok_or("No shortcut assignment")?;
        if self.editor != Some(Edit::WaypointKey(draft.index))
            || raw_array(&self.preset.document) != Some(&draft.basis) {
            return Err("The selected waypoint has changed.".into());
        }
        let array = assigned(&self.preset.document, draft.index,
            draft.value.as_deref().ok_or("Unsupported key shortcut")?)?;
        if Some(&array) != raw_array(&self.preset.document) {
            self.undo.try_reserve(1).map_err(|_| "Waypoint history is full")?;
            let before = self.capture_undo();
            *voice_mut(&mut self.preset.document).get_mut("waypoints").expect("validated waypoint slot") = array;
            self.changed();
            self.remember_entry(before);
            self.waypoint_reset();
        }
        self.monitor_controls.waypoint_ui.draft = None;
        self.monitor_controls.waypoint_ui.claimed = true;
        self.editor = None;
        Ok(())
    }

    pub(super) fn waypoint_key_dialog(&mut self, ctx: &egui::Context, full: Rect) {
        let Some(Edit::WaypointKey(index)) = self.editor else { return; };
        let valid = self.monitor_controls.waypoint_ui.draft.as_ref().is_some_and(|draft|
            draft.index == index && Some(&draft.basis) == raw_array(&self.preset.document));
        if !valid || ctx.input(|i| !i.focused || i.raw.events.iter()
            .any(|event| matches!(event, egui::Event::WindowFocused(false)))) {
            self.monitor_controls.waypoint_ui.draft = None;
            self.monitor_controls.waypoint_ui.claimed = true; self.editor = None; return;
        }
        self.overlay_rects.push(full);
        let rect = Rect::from_center_size(full.center(), vec2(360.0, 160.0));
        let mut cancel = false; let mut confirm = false;
        let message_open = self.error.is_some() || self.notice.is_some() || self.layout_blocked();
        let lights = self.ambient_lights(full);
        controls::modal_panel(ctx, "waypoint-key-editor", rect, Color32::TRANSPARENT, |ui| {
            glass(ui.painter(), rect, &lights, false);
            ui.add_enabled_ui(!message_open, |ui| {
                let current = self.monitor_controls.waypoint_ui.draft.as_ref()
                    .and_then(|draft| draft.value.as_deref()).unwrap_or("Unsupported");
                let current = localization::tr(ctx, current);
                ui.painter().text(rect.min + vec2(20.0, 24.0), Align2::LEFT_CENTER,
                    localization::tr(ctx, "Assign key shortcut"), FontId::proportional(14.0), TEXT);
                ui.painter().text(rect.min + vec2(20.0, 58.0), Align2::LEFT_CENTER,
                    localization::tr(ctx, "Currently assigned to: _").replacen('_', &clipped_title(current, 24), 1), FontId::proportional(13.0), TEXT);
                ui.painter().text(rect.min + vec2(20.0, 82.0), Align2::LEFT_CENTER,
                    localization::tr(ctx, "Press a key to change the assignment."), FontId::proportional(12.0), MUTED);
                let ok = pill_button(ui, Rect::from_min_size(rect.min + vec2(20.0,112.0),vec2(150.0,28.0)),
                    "waypoint-key-ok", "OK", true);
                let no = pill_button(ui, Rect::from_min_size(rect.min + vec2(190.0,112.0),vec2(150.0,28.0)),
                    "waypoint-key-cancel", "Cancel", true);
                let genuine_click = |response: &egui::Response| {
                    response.clicked_by(egui::PointerButton::Primary)
                        && !self.monitor_controls.waypoint_ui.primary_cancelled
                        && ctx.input(|i| {
                            let press = i.raw.events.iter().any(|event| matches!(event,
                                egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed:true, .. }));
                            let at = if press { canvas_gesture::primary_click_release(&i.raw.events, &[]) }
                                else { canvas_gesture::armed_primary_release(&i.raw.events, &[]) };
                            at.is_some_and(|at| response.rect.contains(at))
                        })
                };
                confirm = genuine_click(&ok); cancel = genuine_click(&no);
                if confirm { ok.surrender_focus(); }
                if cancel { no.surrender_focus(); }
            });
        });
        if !message_open && ctx.current_pass_index() == 0 {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                ctx.input_mut(|i| { i.consume_key(egui::Modifiers::NONE, egui::Key::Escape); }); cancel = true;
            }
            if cancel {
                self.monitor_controls.waypoint_ui.draft = None;
                self.monitor_controls.waypoint_ui.claimed = true; self.editor = None;
            }
            else if confirm && let Err(error) = self.waypoint_confirm_key() { self.error = Some(error); }
        }
    }

    pub(super) fn waypoint_frame_start(&mut self, ctx: &egui::Context,
        cancellations: &[canvas_gesture::PointerCancellation])
    {
        if ctx.current_pass_index() == 0 {
            self.monitor_controls.waypoint_ui.claimed = false;
            self.monitor_controls.waypoint_ui.batch_used = false;
        }
        self.monitor_controls.waypoint_ui.primary_cancelled = cancellations.iter()
            .any(|c| c.affects(egui::PointerButton::Primary));
        if self.monitor_controls.waypoint_ui.basis.as_ref()
            .is_some_and(|basis| Some(basis) != raw_array(&self.preset.document))
        { self.waypoint_reset(); }
        if self.layout_blocked() || self.settings || self.editor.is_some() || self.color_picker
            || self.error.is_some() || self.notice.is_some() || egui::Popup::is_any_open(ctx)
            || ctx.input(|i| !i.focused || i.raw.events.iter().any(|event|
                matches!(event, egui::Event::WindowFocused(false))))
            || cancellations.iter().any(|c| c.affects(egui::PointerButton::Primary))
        { self.waypoint_cancel(); }
    }

    pub(super) fn waypoint_active(&self) -> bool {
        let state = &self.monitor_controls.waypoint_ui;
        state.armed.is_some() || state.reroute.is_some()
    }

    pub(super) fn waypoint_armed(&self) -> bool {
        self.monitor_controls.waypoint_ui.armed.is_some()
    }

    pub(super) fn waypoint_claimed_pass(&self, ctx: &egui::Context) -> bool {
        ctx.current_pass_index() != 0 && self.monitor_controls.waypoint_ui.claimed
    }

    pub(super) fn waypoint_hit(&self, at: Pos2, rect: Rect) -> Option<usize> {
        if !rect.contains(at) || !at.is_finite() { return None; }
        let points = waypoints(&self.preset.document).ok()?;
        points.iter().enumerate().rev().find_map(|(index, point)| {
            let p = world_to_screen(Point::new(point.xy[0], point.xy[1]), rect, self.camera);
            (p.is_finite() && p.distance(at) <= 11.0).then_some(index)
        })
    }

    pub(super) fn waypoint_add(&mut self) -> Result<()> {
        let array = added(&self.preset.document, [self.probe.position.x, self.probe.position.y])?;
        self.undo.try_reserve(1).map_err(|_| "Waypoint history is full")?;
        let before = self.capture_undo();
        voice_mut(&mut self.preset.document).as_object_mut().expect("editable voice")
            .insert("waypoints".into(), array);
        self.waypoint_reset();
        self.changed();
        self.remember_entry(before);
        self.monitor_controls.waypoint_ui.claimed = true;
        Ok(())
    }

    pub(super) fn waypoint_delete(&mut self) -> Result<()> {
        let index = self.monitor_controls.waypoint_ui.selected.ok_or("No selected waypoint")?;
        if self.monitor_controls.waypoint_ui.basis.as_ref() != raw_array(&self.preset.document) {
            return Err("The selected waypoint has changed.".into());
        }
        let array = deleted(&self.preset.document, index)?;
        self.undo.try_reserve(1).map_err(|_| "Waypoint history is full")?;
        let before = self.capture_undo();
        *voice_mut(&mut self.preset.document).get_mut("waypoints").expect("validated existing array") = array;
        self.waypoint_reset();
        self.changed();
        self.remember_entry(before);
        self.monitor_controls.waypoint_ui.claimed = true;
        Ok(())
    }

    pub(super) fn waypoint_begin_reroute(&mut self) -> Result<()> {
        let (points, _) = editable(&self.preset.document)?;
        if points.len() < 3 { return Err("At least three waypoints are required".into()); }
        let mut picked = Vec::new();
        picked.try_reserve_exact(points.len()).map_err(|_| "Waypoint order is too large")?;
        self.undo.try_reserve(1).map_err(|_| "Waypoint history is full")?;
        let start = self.capture_undo();
        let basis = raw_array(&self.preset.document).cloned();
        let state = &mut self.monitor_controls.waypoint_ui;
        state.basis = basis; state.selected = None; state.armed = None; state.draft = None;
        state.reroute = Some(Reroute { picked, start: Some(start) }); state.claimed = true;
        Ok(())
    }

    pub(super) fn waypoint_recall(&mut self, index: usize) -> Result<bool> {
        let points = waypoints(&self.preset.document)?;
        let point = *points.get(index).ok_or("No selected waypoint")?;
        let mut state = std::mem::take(&mut self.monitor_controls.waypoint_ui);
        let prepare = (|| -> Result<(Option<Value>, Option<Value>, Option<transactions::UndoEntry>)> {
            if state.basis.as_ref().is_some_and(|basis| Some(basis) != raw_array(&self.preset.document)) {
                return Err("The waypoint array has changed.".into());
            }
            if let Some(route) = &state.reroute {
                if route.picked.contains(&index) { return Ok((None, None, None)); }
            }
            self.undo.try_reserve(1).map_err(|_| "Waypoint history is full")?;
            let array = if let Some(route) = &state.reroute {
                if route.picked.len() + 1 == points.len() {
                    let mut order = route.picked.clone();
                    order.push(index);
                    Some(reordered(&self.preset.document, &order)?)
                } else { None }
            } else { None };
            let basis = array.as_ref().or_else(|| raw_array(&self.preset.document)).cloned();
            let before = state.reroute.is_none().then(|| self.capture_undo());
            Ok((array, basis, before))
        })();
        let (array, basis, before) = match prepare {
            Ok(prepared) => prepared,
            Err(error) => { self.monitor_controls.waypoint_ui = state; return Err(error); }
        };
        if state.reroute.as_ref().is_some_and(|r| r.picked.contains(&index)) {
            self.monitor_controls.waypoint_ui = state;
            return Ok(false);
        }
        if !self.probe_changed(Probe { position: Point::new(point.xy[0], point.xy[1]), ..self.probe }) {
            self.monitor_controls.waypoint_ui = state;
            return Ok(false);
        }

        if let Some(array) = array {
            *voice_mut(&mut self.preset.document).get_mut("waypoints")
                .expect("prepared existing waypoint slot survives probe_changed") = array;
        }
        if let Some(route) = &mut state.reroute {
            if let Some(start) = route.start.take() { self.remember_entry(start); }
            route.picked.push(index);
            if route.picked.len() == points.len() { state.reroute = None; }
        } else if let Some(before) = before { self.remember_entry(before); }
        state.basis = basis;
        state.selected = Some(index);
        state.armed = None;
        self.selected = None;
        self.drag = None;
        self.monitor_controls.waypoint_ui = state;
        Ok(true)
    }

    pub(super) fn waypoint_control_clicked(&self, ctx: &egui::Context,
        response: &egui::Response, cancellations: &[canvas_gesture::PointerCancellation],
        abort: bool, observations: &[canvas_gesture::UnhandledEscapeDown]) -> bool
    {
        if abort || ctx.current_pass_index() != 0 || egui::Popup::is_any_open(ctx)
            || !response.clicked_by(egui::PointerButton::Primary)
            || ctx.input(|i| !i.focused || i.pointer.primary_down()
                || i.raw.events.iter().any(|e| matches!(e, egui::Event::WindowFocused(false))))
            || cancellations.iter().any(|c| c.affects(egui::PointerButton::Primary))
        { return false; }
        ctx.input(|i| {
            let press = i.raw.events.iter().any(|e| matches!(e,
                egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed: true, .. }));
            let up = if press { canvas_gesture::primary_click_release(&i.raw.events, cancellations) }
                else { canvas_gesture::armed_primary_release(&i.raw.events, cancellations) };
            up.is_some_and(|at| response.rect.contains(at))
                && canvas_gesture::passive_escape_completion(&i.raw.events, cancellations, observations, !press)
                    != Some(false)
        })
    }

    pub(super) fn waypoint_press(&self, ctx: &egui::Context, rect: Rect,
        response: &egui::Response, cancellations: &[canvas_gesture::PointerCancellation]) -> Option<(usize, Pos2)>
    {
        if self.drag.is_some() || self.settings || self.editor.is_some() || self.color_picker
            || self.error.is_some() || self.notice.is_some() || self.menu.is_some()
            || egui::Popup::is_any_open(ctx) { return None; }
        let pointer = ctx.pointer_hover_pos();
        let over = pointer.is_some_and(|at| self.overlay_rects.iter().any(|r| r.contains(at)));
        self.primary_input(ctx, rect, response, cancellations, pointer, over).press_position?;
        let press = ctx.input(|i| {
            let mut presses = i.raw.events.iter().filter_map(|e| match e {
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, .. } => Some(*pos),
                _ => None,
            });
            let press = presses.next()?;
            presses.next().is_none().then_some(press)
        })?;
        if !rect.contains(press) || self.overlay_rects.iter().any(|r| r.contains(press))
            || ctx.layer_id_at(press).is_some_and(|layer| layer != response.layer_id)
        { return None; }
        self.waypoint_hit(press, rect).map(|index| (index, press))
    }

    pub(super) fn waypoint_primary(&mut self, ctx: &egui::Context, rect: Rect,
        response: &egui::Response, cancellations: &[canvas_gesture::PointerCancellation]) -> bool
    {
        if ctx.current_pass_index() != 0 { return self.monitor_controls.waypoint_ui.claimed; }
        if self.color_picker || egui::Popup::is_any_open(ctx) { self.waypoint_cancel(); return false; }
        let allowed = |app: &Self, at| rect.contains(at)
            && !app.overlay_rects.iter().any(|r| r.contains(at))
            && ctx.layer_id_at(at).is_none_or(|layer| layer == response.layer_id);
        if let Some(index) = self.monitor_controls.waypoint_ui.armed {
            self.monitor_controls.waypoint_ui.claimed = true;
            if ctx.input(|i| i.pointer.primary_released()) {
                let up = ctx.input(|i| canvas_gesture::armed_primary_release(&i.raw.events, cancellations));
                self.monitor_controls.waypoint_ui.armed = None;
                if up.is_some_and(|at| allowed(self, at) && self.waypoint_hit(at, rect) == Some(index)) {
                    if let Err(error) = self.waypoint_recall(index) { self.error = Some(error); }
                }
            } else if ctx.input(|i| !i.pointer.primary_down() || i.raw.events.iter()
                .any(|e| matches!(e, egui::Event::PointerGone))) {
                self.monitor_controls.waypoint_ui.armed = None;
            }
            return true;
        }
        let Some((index, _)) = self.waypoint_press(ctx, rect, response, cancellations) else { return false; };
        let basis = raw_array(&self.preset.document).cloned();
        self.monitor_controls.waypoint_ui.basis = basis;
        self.monitor_controls.waypoint_ui.claimed = true;
        if ctx.input(|i| i.pointer.primary_down()) {
            self.monitor_controls.waypoint_ui.armed = Some(index);
        } else {
            let up = ctx.input(|i| canvas_gesture::primary_click_release(&i.raw.events, cancellations));
            if up.is_some_and(|at| allowed(self, at) && self.waypoint_hit(at, rect) == Some(index)) {
                if let Err(error) = self.waypoint_recall(index) { self.error = Some(error); }
            }
        }
        true
    }

    pub(super) fn waypoint_toolbar(&mut self, ctx: &egui::Context, full: Rect,
        cancellations: &[canvas_gesture::PointerCancellation], abort: bool,
        observations: &[canvas_gesture::UnhandledEscapeDown])
    {
        if self.settings || self.editor.is_some() || self.color_picker || self.menu.is_some()
            || self.error.is_some() || self.notice.is_some() || self.layout_blocked()
            || self.monitor_controls.waypoint_ui.reroute.is_some()
        { return; }
        let Some(index) = self.monitor_controls.waypoint_ui.selected else { return; };
        let Ok(points) = waypoints(&self.preset.document) else { return; };
        let Some(point) = points.get(index) else { return; };
        let at = world_to_screen(Point::new(point.xy[0], point.xy[1]), full, self.camera);
        if !at.is_finite() || !full.contains(at) { return; }
        let remove = Rect::from_min_size(at + vec2(16.0, -16.0), vec2(30.0, 30.0));
        if !full.contains_rect(remove) { return; }
        let key_rect = Rect::from_min_size(remove.min + vec2(32.0,0.0),vec2(30.0,30.0));
        let key_fits = full.contains_rect(key_rect);
        let rect = if key_fits { remove.union(key_rect) } else { remove };
        self.overlay_rects.push(rect);
        egui::Area::new(egui::Id::new("waypoint-actions")).order(egui::Order::Foreground)
            .fixed_pos(rect.min).show(ctx, |ui| {
                ui.set_min_size(rect.size());
                let response = ui.interact(remove, ui.id().with("delete"), Sense::click());
                icon(ui.painter(), remove.shrink(5.0), Icon::Trash, TEXT);
                if self.waypoint_control_clicked(ctx, &response, cancellations, abort, observations) {
                    if let Err(error) = self.waypoint_delete() { self.error = Some(error); }
                }
                optional_tooltip(response, self.show_tooltips, "Delete waypoint");
                if key_fits {
                    let key_response = ui.interact(key_rect, ui.id().with("key"), Sense::click());
                    ui.painter().text(key_rect.center(), Align2::CENTER_CENTER, "K", FontId::proportional(14.0), TEXT);
                    if self.waypoint_control_clicked(ctx, &key_response, cancellations, abort, observations) {
                        self.waypoint_begin_key(index);
                    }
                    optional_tooltip(key_response, self.show_tooltips, "Assign a key shortcut for the selected waypoint.");
                }
            });
    }

    pub(super) fn paint_waypoints(&self, ui: &egui::Ui, full: Rect) {
        let Ok(points) = waypoints(&self.preset.document) else { return; };
        let p = ui.painter().with_clip_rect(full);
        let screen = |point: &Waypoint| world_to_screen(Point::new(point.xy[0], point.xy[1]), full, self.camera);
        let reroute = self.monitor_controls.waypoint_ui.reroute.as_ref();
        if reroute.is_none() {
            let mut order: Vec<_> = points.iter().collect();
            order.sort_by_key(|point| point.route);

            let mut remaining_dots = 4096usize;
            for pair in order.windows(2) {
                let a = screen(pair[0]); let b = screen(pair[1]);
                let length = a.distance(b);
                if !a.is_finite() || !b.is_finite() || !length.is_finite() || length == 0.0 { continue; }
                let steps = (length / 6.0).ceil().clamp(1.0, 1024.0) as usize;
                let dots = (steps + 1).min(remaining_dots);
                for step in 0..dots {
                    p.circle_filled(a.lerp(b, step as f32 / steps as f32), 1.0, Color32::from_gray(180));
                }
                remaining_dots -= dots;
                let d = (b - a) / length;
                let tip = b - d * 13.0;
                let side = vec2(-d.y, d.x) * 3.0;
                p.line_segment([tip, tip - d * 6.0 + side], Stroke::new(1.0, TEXT));
                p.line_segment([tip, tip - d * 6.0 - side], Stroke::new(1.0, TEXT));
            }
        }
        for (index, point) in points.iter().enumerate() {
            let at = screen(point);
            if !at.is_finite() || !full.contains(at) { continue; }
            let picked = reroute.is_some_and(|r| r.picked.contains(&index));
            let color = if picked || self.monitor_controls.waypoint_ui.selected == Some(index) { TEXT }
                else { Color32::from_gray(180) };
            p.circle_filled(at, 10.0, Color32::from_black_alpha(185));
            p.circle_stroke(at, 10.0, Stroke::new(1.0, color));
            if reroute.is_none() {
                let label = key_description(&self.preset.document, index, point.route)
                    .unwrap_or_else(|| "?".into());
                p.text(at, Align2::CENTER_CENTER, clipped_title(&label, 8), FontId::proportional(12.0), color);
            }
        }
    }
}

