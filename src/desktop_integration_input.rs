use super::{wire, egui, Result};
use std::{path::{Path, PathBuf}, sync::Arc};

const BUTTONS: [egui::PointerButton; 5] = [egui::PointerButton::Primary, egui::PointerButton::Secondary,
    egui::PointerButton::Middle, egui::PointerButton::Extra1, egui::PointerButton::Extra2];
const WHEELS: [egui::MouseWheelUnit; 3] = [egui::MouseWheelUnit::Point, egui::MouseWheelUnit::Line, egui::MouseWheelUnit::Page];
const PHASES: [egui::TouchPhase; 4] = [egui::TouchPhase::Start, egui::TouchPhase::Move, egui::TouchPhase::End, egui::TouchPhase::Cancel];

pub(super) fn raw(frame: wire::Input, time: f64, previous_focus: &mut bool) -> Result<egui::RawInput> {
    if !frame.size.into_iter().zip(wire::MIN_SIZE).all(|(v, min)| (min..=4096.0).contains(&v))
        || !(0.2..=8.0).contains(&frame.scale)
    { return Err("Invalid Rocoflex canvas size".into()); }
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::from(frame.size));
    let mut raw = egui::RawInput {
        screen_rect: Some(rect), focused: frame.focused, time: Some(time), max_texture_side: Some(8192),
        ..Default::default()
    };
    if let Some(view) = raw.viewports.get_mut(&egui::ViewportId::ROOT) {
        view.native_pixels_per_point = Some(frame.scale);
        view.focused = Some(frame.focused);
        view.inner_rect = Some(rect);
    }
    if frame.focused != *previous_focus {
        raw.events.push(egui::Event::WindowFocused(frame.focused));
        *previous_focus = frame.focused;
    }
    raw.events.extend(frame.events.into_iter().filter_map(event));
    raw.dropped_files.extend(frame.files.into_iter().filter(|p| p.is_absolute())
        .map(|p| Arc::new(DropPath(p)) as egui::DroppedFileHandle));
    Ok(raw)
}

#[derive(Debug)]
struct DropPath(PathBuf);
impl egui::DroppedFile for DropPath {
    fn path(&self) -> &Path { &self.0 }
    fn bytes(&self) -> Result<Vec<u8>> { std::fs::read(self.path()).map_err(|e| e.to_string()) }
}

fn modifiers(bits: u8) -> egui::Modifiers {
    let bit = |index: u8| bits & (1u8 << index) != 0;
    egui::Modifiers { alt: bit(0), ctrl: bit(1), shift: bit(2), mac_cmd: bit(3), command: bit(4) }
}

fn event(event: wire::Event) -> Option<egui::Event> {
    use egui::{Event as E, ImeEvent};
    use wire::Event::*;
    Some(match event {
        Move(at) => E::PointerMoved(at.into()),
        Motion(delta) => E::MouseMoved(delta.into()),
        Button(at, b, pressed, bits) => E::PointerButton { pos: at.into(), button: *BUTTONS.get(b as usize)?, pressed, modifiers: modifiers(bits) },
        Leave => E::PointerGone,
        Scroll(delta, unit, phase, bits) => E::MouseWheel { delta: delta.into(), unit: *WHEELS.get(unit as usize)?,
            phase: *PHASES.get(phase as usize)?, modifiers: modifiers(bits) },
        Zoom(value) => E::Zoom(value),
        Rotate(value) => E::Rotate(value),
        Key(key, physical, pressed, repeat, bits) => E::Key { key: egui::Key::from_name(&key)?,
            physical_key: physical.as_deref().and_then(egui::Key::from_name), pressed, repeat, modifiers: modifiers(bits) },
        Modifiers(bits) => E::ModifiersChanged(modifiers(bits)),
        Copy => E::Copy,
        Cut => E::Cut,
        Paste(text) => E::Paste(text),
        Text(text) => E::Text(text),
        Preedit(text, range) => E::Ime(ImeEvent::Preedit { text, active_range_chars: range }),
        Commit(text) => E::Ime(ImeEvent::Commit(text)),
        DeleteSurrounding(before_chars, after_chars) => E::Ime(ImeEvent::DeleteSurrounding { before_chars, after_chars }),
        Touch(device, id, phase, at, force) => E::Touch { device_id: egui::TouchDeviceId(device), id: egui::TouchId(id),
            phase: *PHASES.get(phase as usize)?, pos: at.into(), force },
    })
}
