use eframe::egui::{Event, Key, PointerButton, Pos2};

pub(crate) const POINTER_BUTTONS: [(u8, PointerButton); 5] = [
    (1, PointerButton::Primary), (2, PointerButton::Secondary),
    (4, PointerButton::Middle), (8, PointerButton::Extra1), (16, PointerButton::Extra2),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerCancellationCause {
    CaptureChanged,
    CancelMode,
    FocusLost,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PointerCancellation {
    pub cause: PointerCancellationCause,
    pub held_buttons: u8,

    pub event_offset: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct UnhandledEscapeDown {
    pub event_boundary: usize,
}

impl PointerCancellation {
    pub(crate) fn affects(&self, button: PointerButton) -> bool {
        POINTER_BUTTONS.iter().any(|&(bit, candidate)| {
            candidate == button && self.held_buttons & bit != 0
        })
    }

    fn is_release_at(&self, button: PointerButton, event_index: usize) -> bool {
        let mut offset = self.event_offset;
        for (bit, candidate) in POINTER_BUTTONS {
            if self.held_buttons & bit != 0 {
                if candidate == button { return offset == event_index; }
                let Some(next) = offset.checked_add(1) else { return false; };
                offset = next;
            }
        }
        false
    }
}

fn cancelled_release(cancellations: &[PointerCancellation], button: PointerButton,
    event_index: usize) -> bool
{
    cancellations.iter().any(|c| c.is_release_at(button, event_index))
}

pub(super) fn primary_release_precedes_escape(events: &[Event],
    cancellations: &[PointerCancellation]) -> bool
{
    let Some(escape_index) = events.iter().position(|event| matches!(event,
        Event::Key { key: Key::Escape, pressed: true, .. })) else { return false; };
    primary_release_precedes_boundary(events, cancellations, escape_index)
}

pub(super) fn passive_escape_completion(events: &[Event],
    cancellations: &[PointerCancellation], observations: &[UnhandledEscapeDown],
    existing_owner: bool) -> Option<bool>
{
    let mut previous = 0;
    for observation in observations {
        if observation.event_boundary > events.len() || observation.event_boundary < previous {
            return None;
        }
        previous = observation.event_boundary;
    }
    let first_press = events.iter().position(|event| matches!(event,
        Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, .. }
            if pos.x.is_finite() && pos.y.is_finite()));
    let boundary = observations.iter().find_map(|observation| {
        (existing_owner || first_press.is_some_and(|press| press < observation.event_boundary))
            .then_some(observation.event_boundary)
    })?;
    Some(primary_release_precedes_boundary(events, cancellations, boundary))
}

fn primary_release_precedes_boundary(events: &[Event],
    cancellations: &[PointerCancellation], boundary: usize) -> bool
{
    if cancellations.iter().any(|c| c.affects(PointerButton::Primary)) { return false; }
    let mut press_seen = false;
    let mut release_index = None;
    for (event_index, event) in events.iter().enumerate() {
        match event {
            Event::WindowFocused(false) => return false,
            Event::PointerGone if release_index.is_none() => return false,
            Event::PointerButton { pos, button: PointerButton::Primary, pressed, .. } => {
                if !pos.x.is_finite() || !pos.y.is_finite() { return false; }
                if *pressed {
                    if press_seen || release_index.is_some() { return false; }
                    press_seen = true;
                } else {
                    if release_index.is_some()
                        || cancelled_release(cancellations, PointerButton::Primary, event_index)
                    { return false; }
                    release_index = Some(event_index);
                }
            }
            _ => {}
        }
    }
    release_index.is_some_and(|release| release < boundary)
}

#[derive(Clone, Copy)]
pub(super) struct CompletedDrag {
    pub press: Pos2,
    pub release: Pos2,
}

pub(super) fn primary_click_release(events: &[Event], cancellations: &[PointerCancellation])
    -> Option<Pos2>
{
    let mut press = None;
    let mut release = None;
    for (event_index, event) in events.iter().enumerate() {
        match event {
            Event::WindowFocused(false) => return None,
            Event::PointerGone if release.is_none() => return None,
            Event::PointerButton { pos, button: PointerButton::Primary, pressed, .. } => {
                if !*pressed && cancelled_release(cancellations, PointerButton::Primary, event_index) {
                    return None;
                }
                if !pos.x.is_finite() || !pos.y.is_finite() { return None; }
                if *pressed {
                    if press.is_some() || release.is_some() { return None; }
                    press = Some(*pos);
                } else {
                    press?;
                    if release.is_some() { return None; }
                    release = Some(*pos);
                }
            }
            _ => {}
        }
    }
    release
}

pub(super) fn secondary_click_release(events: &[Event], cancellations: &[PointerCancellation])
    -> Option<Pos2>
{
    let mut release = None;
    for (event_index, event) in events.iter().enumerate() {
        match event {
            Event::WindowFocused(false) => return None,
            Event::PointerGone if release.is_none() => return None,
            Event::PointerButton { pos, button: PointerButton::Secondary, pressed, .. } => {
                if !*pressed && cancelled_release(cancellations, PointerButton::Secondary, event_index) {
                    return None;
                }
                if release.is_some() || !pos.x.is_finite() || !pos.y.is_finite() {
                    return None;
                }
                if !*pressed { release = Some(*pos); }
            }
            _ => {}
        }
    }
    release
}

pub(super) fn armed_primary_release(events: &[Event], cancellations: &[PointerCancellation])
    -> Option<Pos2>
{
    let mut release = None;
    for (event_index, event) in events.iter().enumerate() {
        match event {
            Event::WindowFocused(false) => return None,
            Event::PointerGone if release.is_none() => return None,
            Event::PointerButton { pos, button: PointerButton::Primary, pressed, .. } => {
                if !*pressed && cancelled_release(cancellations, PointerButton::Primary, event_index) {
                    return None;
                }
                if *pressed || release.is_some()
                    || !pos.x.is_finite() || !pos.y.is_finite()
                {
                    return None;
                }
                release = Some(*pos);
            }
            _ => {}
        }
    }
    release
}

pub(super) fn completed_primary_drag(events: &[Event], cancellations: &[PointerCancellation])
    -> Option<CompletedDrag>
{
    let mut press = None;
    let mut release = None;
    let mut moved = false;
    for (event_index, event) in events.iter().enumerate() {
        match event {
            Event::WindowFocused(false) => return None,

            Event::PointerGone if release.is_none() => return None,
            Event::PointerButton { pos, button: PointerButton::Primary, pressed, .. } => {
                if !*pressed && cancelled_release(cancellations, PointerButton::Primary, event_index) {
                    return None;
                }
                if !pos.x.is_finite() || !pos.y.is_finite() { return None; }
                if *pressed {

                    if press.is_some() || release.is_some() { return None; }
                    press = Some(*pos);
                } else {
                    let start = press?;
                    if release.is_some() { return None; }
                    moved |= *pos != start;
                    release = Some(*pos);
                }
            }
            Event::PointerMoved(pos) if press.is_some() && release.is_none() => {
                if !pos.x.is_finite() || !pos.y.is_finite() { return None; }
                moved |= Some(*pos) != press;
            }
            _ => {}
        }
    }
    if !moved { return None; }
    Some(CompletedDrag { press: press?, release: release? })
}
