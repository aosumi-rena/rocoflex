use super::*;

pub(super) struct ParentBounds {
    pub pitch: Rect,
    pub waypoints: Rect,
    pub voice_group: Rect,
}

pub(super) fn parent_bounds(full: Rect) -> ParentBounds {

    let values = super::monitor_expansion::parent_layout(
        full.width() as i32, full.height() as i32);
    let mapped = |rect: super::monitor_expansion::ChildRect| Rect::from_min_size(
        full.min + vec2(rect.x as f32, rect.y as f32),
        vec2(rect.width as f32, rect.height as f32));
    ParentBounds {
        pitch: mapped(values.pitch), waypoints: mapped(values.waypoints),
        voice_group: mapped(values.voice_group),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct MonitorPrimaryOwner { widget: egui::Id, layer: egui::LayerId }

#[derive(Default)]
struct MonitorPrimaryInput {
    owner: Option<MonitorPrimaryOwner>,
    last_processed_frame: Option<u64>,
}

impl MonitorPrimaryInput {
    fn begin(&mut self, ctx: &egui::Context, blocked: bool,
        cancellations: &[canvas_gesture::PointerCancellation]) -> bool
    {
        let frame = ctx.cumulative_frame_nr();
        if blocked { self.owner = None; }
        if ctx.current_pass_index() != 0 || self.last_processed_frame == Some(frame) {
            return false;
        }
        let consecutive = self.last_processed_frame.and_then(|n| n.checked_add(1)) == Some(frame);
        self.last_processed_frame = Some(frame);
        let reset = ctx.input(|i| !i.focused || i.raw.events.iter().any(|event| matches!(event,
            egui::Event::WindowFocused(false) |
            egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed: true, .. }))
            || (i.raw.events.iter().any(|event| matches!(event, egui::Event::PointerGone))
                && canvas_gesture::armed_primary_release(&i.raw.events, cancellations).is_none()));
        if !consecutive || reset || cancellations.iter().any(|c| c.affects(egui::PointerButton::Primary)) {
            self.owner = None;
        }
        !blocked
    }

    fn retire(&mut self, response: &egui::Response) {
        if self.owner.is_some_and(|owner| owner.widget == response.id) { self.owner = None; }
    }
}

#[derive(Default)]
pub(super) struct MonitorControls {
    pub(super) waypoint_ui: super::waypoints::State,
    mode: MonitorMode,
    pregain_db: f32,
    input_level: Option<f32>,
    pitch_sample: u64,
    pitch_semitones: f32,
    primary_input: MonitorPrimaryInput,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum MonitorMode {
    #[default]
    Closed,
    Pitch,
    Waypoints,
}

impl MonitorControls {
    pub(super) fn pregain_db(&self) -> f32 { self.pregain_db }
    pub(super) fn pitch_sample(&self) -> u64 { self.pitch_sample }
    pub(super) fn pitch_semitones(&self) -> f32 { self.pitch_semitones }
    pub(super) fn shift_pitch_octave(&mut self, amount: f32) {
        self.pitch_semitones = (self.pitch_semitones + amount).round().clamp(-18.0, 18.0);
    }
    pub(super) fn set_values(&mut self, pregain_db: f32, pitch_semitones: f32) {
        self.primary_input.owner = None;
        self.pregain_db = pregain_db;
        self.pitch_semitones = pitch_semitones;
    }
    pub(super) fn pitch_open(&self) -> bool { self.mode == MonitorMode::Pitch }
    fn waypoints_open(&self) -> bool { self.mode == MonitorMode::Waypoints }

    fn activate(&mut self, mode: MonitorMode) {
        self.primary_input.owner = None;
        self.mode = if self.mode == mode { MonitorMode::Closed } else { mode };
    }
}

struct MonitorBounds { component: Rect, icon: Rect, child_maximum: Rect }
impl MonitorBounds {

    fn new(component: Rect, right_icon: bool) -> Self {
        let side = component.height();
        let (icon, child_maximum) = if right_icon {
            let split = component.right() - side;
            (Rect::from_min_max(pos2(split, component.top()), component.max),
                Rect::from_min_max(component.min, pos2(split, component.bottom())))
        } else {
            let split = component.left() + side;
            (Rect::from_min_max(component.min, pos2(split, component.bottom())),
                Rect::from_min_max(pos2(split, component.top()), component.max))
        };
        Self { component, icon, child_maximum }
    }
}

fn monitor_clicked(response: &egui::Response, bounds: &MonitorBounds, right_icon: bool) -> bool {

    if response.flags.contains(egui::response::Flags::FAKE_PRIMARY_CLICKED) { return true; }
    if !response.clicked_by(egui::PointerButton::Primary) { return false; }
    let Some(at) = response.interact_pointer_pos() else { return false; };

    let values = [bounds.component.width(), bounds.component.height(),
        (at.x - bounds.component.left()).floor(), (at.y - bounds.component.top()).floor()];
    if values.iter().any(|value| !value.is_finite() || *value < -2_147_483_648.0 || *value >= 2_147_483_648.0) {
        return false;
    }
    super::monitor_input::icon_admission(values[0] as i32, values[1] as i32,
        u8::from(right_icon), [values[2] as i32, values[3] as i32])
}

fn monitor_pointer_position_ordinary(ctx:&egui::Context,response:&egui::Response,
    cancellations:&[canvas_gesture::PointerCancellation])->Option<egui::Pos2> {
    let stopped=response.drag_stopped_by(egui::PointerButton::Primary)
        && ctx.input(|input|!input.pointer.primary_pressed());
    let position=if stopped {
        let release=ctx.input(|input|input.events.iter().rev().find_map(|event|match event {
            egui::Event::PointerButton {pos,button:egui::PointerButton::Primary,pressed:false,..}=>Some(*pos),
            _=>None,
        }));

        let transform=ctx.layer_transform_from_global(response.layer_id);
        release.map(|position|transform.map_or(position,|transform|transform*position))
    } else if response.dragged() || response.clicked() {
        response.interact_pointer_pos()
    } else {None};

    let position=position.or_else(|| {
        if !response.enabled() || response.clicked() || response.dragged()
            || response.is_pointer_button_down_on() || egui::Popup::is_any_open(ctx)
            || !ctx.input(|i|i.focused && i.pointer.primary_pressed()
                && i.pointer.primary_released() && !i.pointer.primary_down())
        { return None; }
        let gesture=ctx.input(|i|canvas_gesture::completed_primary_drag(&i.raw.events,cancellations))?;
        let transform=ctx.layer_transform_from_global(response.layer_id);
        let press=transform.map_or(gesture.press,|transform|transform*gesture.press);
        if !response.rect.contains(press)
            || ctx.layer_id_at(gesture.press)!=Some(response.layer_id)
        { return None; }
        Some(transform.map_or(gesture.release,|transform|transform*gesture.release))
    });
    position.filter(|at|at.x.is_finite() && at.y.is_finite())
}

fn monitor_pointer_position(ctx: &egui::Context, response: &egui::Response,
    cancellations: &[canvas_gesture::PointerCancellation], abort_primary: bool,
    observations: &[canvas_gesture::UnhandledEscapeDown],
    primary: &mut MonitorPrimaryInput, eligible: bool) -> Option<Pos2>
{
    let ordinary = monitor_pointer_position_ordinary(ctx, response, cancellations);
    if !eligible { return ordinary; }
    let identity = MonitorPrimaryOwner { widget: response.id, layer: response.layer_id };
    if primary.owner.is_some_and(|owner| owner.widget == response.id && owner != identity) {
        primary.owner = None;
    }
    let (focused, down, released, terminal) = ctx.input(|i|
        (i.focused, i.pointer.primary_down(), i.pointer.primary_released(),
            i.raw.events.iter().any(|event| matches!(event,
                egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed: false, .. }))));
    let passive = ctx.input(|i| canvas_gesture::passive_escape_completion(
        &i.raw.events, cancellations, observations, primary.owner == Some(identity)));
    let abort = (abort_primary && !ctx.input(|i|
        canvas_gesture::primary_release_precedes_escape(&i.raw.events, cancellations)))
        || passive == Some(false);
    if primary.owner == Some(identity) {
        if !response.enabled() || !focused || abort {
            primary.owner = None;
        } else if terminal || released || !down {

            primary.owner = None;
            if ordinary.is_none() && released && !down {
                let release = ctx.input(|i| canvas_gesture::armed_primary_release(
                    &i.raw.events, cancellations));
                let transform = ctx.layer_transform_from_global(response.layer_id);
                return release.map(|at| transform.map_or(at, |t| t * at))
                    .filter(|at| at.x.is_finite() && at.y.is_finite());
            }
        }
    }

    if response.enabled() && focused && down && !released && !abort
        && response.is_pointer_button_down_on()
    {
        let press = ctx.input(|i| {
            let mut press = None;
            for event in &i.raw.events {
                match event {
                    egui::Event::WindowFocused(false) | egui::Event::PointerGone => return None,
                    egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, .. } => {
                        if !*pressed || press.is_some() || !pos.x.is_finite() || !pos.y.is_finite() {
                            return None;
                        }
                        press = Some(*pos);
                    }
                    _ => {}
                }
            }
            press
        });
        if let Some(press) = press {
            let transform = ctx.layer_transform_from_global(response.layer_id);
            let local = transform.map_or(press, |t| t * press);
            if response.rect.contains(local) && ctx.layer_id_at(press) == Some(response.layer_id)
                && !cancellations.iter().any(|c| c.affects(egui::PointerButton::Primary))
            {
                primary.owner = Some(identity);
            }
        }
    }
    ordinary
}

impl DesktopApp {

    pub(crate) fn set_input_pitch(&mut self, word:u64) {
        self.monitor_controls.pitch_sample = if crate::pitch_display::valid(word) {word} else {0};
    }
    pub(crate) fn set_input_meter(&mut self,level:Option<f32>) {
        self.monitor_controls.input_level=level;
    }
    pub(super) fn monitor_controls(&mut self, ctx: &egui::Context, full: Rect,
        cancellations: &[canvas_gesture::PointerCancellation], abort_primary: bool,
        observations: &[canvas_gesture::UnhandledEscapeDown]) {
        let layout_blocked = self.layout_blocked();
        let blocked = self.settings || self.editor.is_some() || self.color_picker
            || self.error.is_some() || self.notice.is_some();
        let primary_pass = self.monitor_controls.primary_input.begin(ctx,
            layout_blocked || blocked || self.menu.is_some() || egui::Popup::is_any_open(ctx),
            cancellations);
        if layout_blocked { return; }
        let mut primary_widgets = [None, None];
        let gain = Rect::from_center_size(
            pos2(full.left() + 28.0, full.center().y), vec2(26.0, 200.0));
        let parent = parent_bounds(full);
        let pitch_bounds = MonitorBounds::new(parent.pitch, false);
        let waypoint_bounds = MonitorBounds::new(parent.waypoints, true);
        let pitch = pitch_bounds.icon;
        let waypoint = waypoint_bounds.icon;
        self.overlay_rects.extend([gain, pitch, waypoint]);
        if self.monitor_controls.pitch_open() {
            self.overlay_rects.push(pitch_bounds.child_maximum);
        }
        if self.monitor_controls.waypoints_open() {
            self.overlay_rects.push(waypoint_bounds.child_maximum);
        }
        egui::Area::new(egui::Id::new("monitor-pre-gain"))
            .order(egui::Order::Foreground).fixed_pos(gain.min).show(ctx, |ui| {
                ui.set_min_size(gain.size());
                if blocked { ui.disable(); }
                let response = ui.interact(gain, ui.id().with("pre-gain"), Sense::click_and_drag());
                primary_widgets[0] = Some(MonitorPrimaryOwner { widget: response.id, layer: response.layer_id });
                if response.double_clicked() {
                    self.monitor_controls.primary_input.retire(&response);
                    self.monitor_controls.pregain_db = 0.0;
                } else if let Some(at) = monitor_pointer_position(ctx, &response, cancellations,
                    abort_primary, observations, &mut self.monitor_controls.primary_input, primary_pass) {

                    let margin = 7.0_f32;
                    let local_y = at.y - gain.top();
                    let numerator = (gain.height() - margin) - local_y;
                    let denominator = gain.height() - (margin + margin);
                    let fraction = numerator / denominator;
                    let mapped = (12.0_f32 - (-12.0_f32)) * fraction + (-12.0_f32);
                    let doubled = mapped + mapped;
                    self.monitor_controls.pregain_db = (doubled.round() * 0.5_f32).clamp(-12.0, 12.0);
                    response.request_focus();
                }
                if response.has_focus() {
                    ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, egui::EventFilter {
                        vertical_arrows: true, ..Default::default()
                    }));
                    let step = ui.input(|input| f32::from(u8::from(input.key_pressed(egui::Key::ArrowUp)))
                        - f32::from(u8::from(input.key_pressed(egui::Key::ArrowDown))));
                    if step != 0.0 {
                        ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
                        self.monitor_controls.pregain_db = (self.monitor_controls.pregain_db + step * 0.5).clamp(-12.0, 12.0);
                    }
                }
                let p = ui.painter();
                let x = gain.center().x;
                p.line_segment([pos2(x, gain.top()+7.0), pos2(x, gain.bottom()-7.0)],
                    Stroke::new(1.0,Color32::from_rgb(81,82,89)));
                if let Some(level)=self.monitor_controls.input_level {

                    let u=if level==0. {0.} else {
                        let db=level.log10()*20.0_f32;
                        let shifted=db+12.0_f32;
                        let bounded=shifted.max(-12.0_f32).min(12.0_f32);
                        (bounded-(-12.0_f32))/(12.0_f32-(-12.0_f32))
                    };
                    let top=((gain.height()-8.0_f32)-(gain.height()-16.0_f32)*u)
                        .trunc().min(gain.height()-7.0_f32);

                    p.rect_filled(Rect::from_min_max(pos2(x-0.5,gain.top()+top),
                        pos2(x+0.5,gain.bottom()-7.0)),0.0,Color32::from_rgb(100,178,113));
                }
                let thumb = pos2(x, gain.center().y - self.monitor_controls.pregain_db / 12.0 * (gain.height() * 0.5 - 7.0));

                p.circle_filled(thumb, 6.6, Color32::from_rgb(57, 58, 70));
                p.circle_stroke(thumb, 6.6, Stroke::new(0.8, TEXT));
                if response.hovered() || response.dragged() || response.is_pointer_button_down_on()
                    || response.has_focus()
                {
                    p.with_clip_rect(full).text(thumb + vec2(13.0, 0.0), Align2::LEFT_CENTER,
                        format!("{:+.1} dB", self.monitor_controls.pregain_db),
                        FontId::proportional(14.0), TEXT);
                }
                optional_tooltip(
                    response,
                    self.show_tooltips,
                    "Adjust the pre-gain.",
                );
            });
        let pitch_frame = if self.monitor_controls.pitch_open() {
            pitch_bounds.component
        } else { pitch };
        egui::Area::new(egui::Id::new("monitor-pitch"))
            .order(egui::Order::Foreground).fixed_pos(pitch_frame.min).show(ctx, |ui| {
                ui.set_min_size(pitch_frame.size());
                if blocked { ui.disable(); }
                let thin = Stroke::new(0.7, Color32::from_gray(117));
                let p = ui.painter();
                let c = pitch.center();
                let stroke = Stroke::new(0.8, TEXT);
                let response = ui.interact(pitch, ui.id().with("pitch-toggle"), Sense::click());

                if response.enabled() && response.hovered() {
                    p.circle_filled(c, pitch.height() * 0.5, Color32::from_rgb(63, 63, 72));
                }
                icon(p, Rect::from_center_size(pitch.center(), vec2(24.0, 24.0)), Icon::TuningFork, TEXT);
                if !blocked && monitor_clicked(&response, &pitch_bounds, false) {
                    self.monitor_controls.activate(MonitorMode::Pitch);
                }
                optional_tooltip(response, self.show_tooltips, "Apply pitch shifting to the voice.");
                if self.monitor_controls.pitch_open() {
                    ui.set_min_size(pitch_bounds.component.size());

                    let y = c.y - 2.0;
                    let child_painter = ui.painter().with_clip_rect(pitch_bounds.child_maximum);
                    let a = pos2(full.left()+64.0,y);
                    let b = pos2(full.left()+219.0,y);
                    let slider = Rect::from_min_max(a - vec2(7.0, 14.0), b + vec2(7.0, 14.0)).intersect(pitch_bounds.child_maximum);
                    let response = ui.interact(slider, ui.id().with("pitch-value"), Sense::click_and_drag());
                    primary_widgets[1] = Some(MonitorPrimaryOwner { widget: response.id, layer: response.layer_id });
                    if response.double_clicked() {
                        self.monitor_controls.primary_input.retire(&response);
                        self.monitor_controls.pitch_semitones = 0.0;
                    } else if let Some(at) = monitor_pointer_position(ctx, &response, cancellations,
                        abort_primary, observations, &mut self.monitor_controls.primary_input, primary_pass) {
                        self.monitor_controls.pitch_semitones = (((at.x - a.x) / (b.x - a.x) * 36.0) - 18.0).round().clamp(-18.0, 18.0);
                        response.request_focus();
                    }
                    if response.has_focus() {
                        ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, egui::EventFilter {
                            horizontal_arrows: true, ..Default::default()
                        }));
                        let step = ui.input(|input| f32::from(u8::from(input.key_pressed(egui::Key::ArrowRight)))
                            - f32::from(u8::from(input.key_pressed(egui::Key::ArrowLeft))));
                        if step != 0.0 {
                            ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
                            self.monitor_controls.pitch_semitones = (self.monitor_controls.pitch_semitones + step).round().clamp(-18.0, 18.0);
                        }
                    }
                    let thumb_x = a.x + (self.monitor_controls.pitch_semitones.clamp(-18.0, 18.0) + 18.0) / 36.0 * (b.x - a.x);

                    let zero_x = (a.x + b.x) * 0.5;
                    let knob_gap = 10.0;
                    for (start, end) in [(a.x, zero_x - 1.0), (zero_x + 1.0, b.x)] {
                        let left_end = end.min(thumb_x - knob_gap);
                        if left_end > start {
                            child_painter.line_segment([pos2(start,y),pos2(left_end,y)],thin);
                        }
                        let right_start = start.max(thumb_x + knob_gap);
                        if end > right_start {
                            child_painter.line_segment([pos2(right_start,y),pos2(end,y)],thin);
                        }
                    }
                    let (amount_start, amount_end) = if thumb_x > zero_x {
                        (zero_x + 1.0, thumb_x - knob_gap)
                    } else {
                        (thumb_x + knob_gap, zero_x - 1.0)
                    };
                    if amount_end > amount_start {
                        child_painter.rect_filled(Rect::from_min_max(
                            pos2(amount_start,y - 0.5), pos2(amount_end,y + 0.5)),
                            1.0, Color32::from_rgb(217,217,218));
                    }
                    child_painter.circle_filled(pos2(thumb_x,y),7.0,Color32::from_rgb(58,59,70));
                    child_painter.circle_stroke(pos2(thumb_x,y),7.0,stroke);
                    child_painter.text(pos2(full.left()+242.0,y),Align2::CENTER_CENTER,
                        if self.monitor_controls.pitch_semitones == 0.0 { "0".to_owned() }
                        else { format!("{:+.0}", self.monitor_controls.pitch_semitones) },FontId::proportional(18.0),TEXT);
                    optional_tooltip(
                        response,
                        self.show_tooltips,
                        "Apply pitch shifting to the voice.",
                    );
                }
            });
        let waypoint_frame = if self.monitor_controls.waypoints_open() {
            waypoint_bounds.component
        } else { waypoint };
        egui::Area::new(egui::Id::new("monitor-waypoints"))
            .order(egui::Order::Foreground).fixed_pos(waypoint_frame.min).show(ctx, |ui| {
                ui.set_min_size(waypoint_frame.size());
                if blocked { ui.disable(); }
                let p = ui.painter();
                let response = ui.interact(waypoint,ui.id().with("waypoint-toggle"),Sense::click());
                if response.enabled() && response.hovered() {
                    p.circle_filled(waypoint.center(), waypoint.height() * 0.5,
                        Color32::from_rgb(63, 63, 72));
                }
                icon(p, Rect::from_center_size(waypoint.center(), vec2(24.0, 24.0)), Icon::Pin, TEXT);
                if !blocked && monitor_clicked(&response, &waypoint_bounds, true) {
                    self.monitor_controls.activate(MonitorMode::Waypoints);
                }
                optional_tooltip(response, self.show_tooltips, "Setup waypoints for quickly switching voices.");
                if self.monitor_controls.waypoints_open() {
                    let child = waypoint_bounds.child_maximum;

                    let rectangles = super::monitor_expansion::waypoint_child_rects(
                        child.width() as i32, child.height() as i32);
                    let [add, order] = rectangles.map(|r| Rect::from_min_size(
                        child.min + vec2(r.x as f32, r.y as f32), vec2(r.width as f32, r.height as f32)));
                    let child_painter = ui.painter().with_clip_rect(child);
                    let add_response = ui.interact(add.intersect(child),ui.id().with("waypoint-add"),Sense::click());
                    let order_response = ui.interact(order.intersect(child),ui.id().with("waypoint-reroute"),Sense::click());

                    let tint = |highlighted| if highlighted { Color32::from_rgb(224, 224, 226) }
                        else { Color32::from_rgb(183, 184, 187) };
                    icon(&child_painter, add.shrink(2.0), Icon::PinAdd, tint(add_response.hovered()));
                    icon(&child_painter, order.shrink(2.0), Icon::Reroute, tint(order_response.hovered()));
                    if !blocked && self.menu.is_none()
                        && self.waypoint_control_clicked(ctx, &add_response, cancellations, abort_primary, observations) {
                        if let Err(error) = self.waypoint_add() { self.error = Some(error); }
                    }
                    if !blocked && self.error.is_none() && self.menu.is_none()
                        && self.waypoint_control_clicked(ctx, &order_response, cancellations, abort_primary, observations) {
                        if let Err(error) = self.waypoint_begin_reroute() { self.notice = Some(error); }
                    }
                    optional_tooltip(
                        add_response,
                        self.show_tooltips,
                        "Add a new waypoint at the cursor position.",
                    );
                    optional_tooltip(
                        order_response,
                        self.show_tooltips,
                        "Reroute the waypoints by selecting them in the desired order.",
                    );
                }
            });

        if self.monitor_controls.primary_input.owner.is_some_and(|owner|
            !primary_widgets.contains(&Some(owner))) {
            self.monitor_controls.primary_input.owner = None;
        }
    }
}
