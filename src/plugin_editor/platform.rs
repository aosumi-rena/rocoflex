use std::{cell::{Cell, RefCell}, collections::BTreeSet, rc::Rc, time::Duration};
use eframe::egui;
use crate::desktop::canvas_gesture::PointerCancellationCause;
use windows::{core::PCWSTR, Win32::{Foundation::{HWND, WPARAM, LPARAM, LRESULT, POINT, RECT},
    Graphics::Gdi::ClientToScreen, UI::{HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::*, WindowsAndMessaging::*}}};
use super::{input_queue::InputQueue, accessibility::Accessibility, ime::Ime,
    drop_target::Registration, clipboard};
type Result<T> = std::result::Result<T, String>;

pub(super) struct Platform {
    hwnd: HWND,
    input: Rc<InputQueue>,
    accessibility: Accessibility,
    ime: Ime,
    _drop: Registration,
    zoom: Cell<f32>,
    position: Cell<[i32; 2]>,
    buttons: Cell<u8>,
    tracking: Cell<bool>,
    modifiers: Cell<egui::Modifiers>,
    keys: RefCell<BTreeSet<egui::Key>>,
    ime_keys: RefCell<BTreeSet<(i16, u16)>>,
    surrogate: Cell<Option<u16>>,
    cursor: Cell<HCURSOR>,
}
impl Platform {
    pub fn new(hwnd: HWND, input: Rc<InputQueue>) -> Result<Self> {
        if unsafe { GetFocus() } == hwnd { input.set_focus(true); }
        let drop = Registration::new(hwnd, input.clone())?;
        let accessibility = Accessibility::new(hwnd, input.focused(), input.repaint.clone());
        let ime = Ime::new(hwnd, input.clone());
        let platform = Self { hwnd, input, accessibility, ime, _drop: drop,
            zoom: Cell::new(1.0), position: Cell::new([0, 0]), buttons: Cell::new(0),
            tracking: Cell::new(false), modifiers: Cell::new(egui::Modifiers::NONE),
            keys: RefCell::new(BTreeSet::new()), ime_keys: RefCell::new(BTreeSet::new()), surrogate: Cell::new(None),
            cursor: Cell::new(HCURSOR::default()) };
        platform.refresh_geometry()?;
        Ok(platform)
    }
    pub fn refresh_geometry(&self) -> Result<()> {
        let mut rectangle = RECT::default();
        unsafe { GetClientRect(self.hwnd, &mut rectangle) }.map_err(|e| e.to_string())?;
        let mut origin = POINT::default();
        if !unsafe { ClientToScreen(self.hwnd, &mut origin) }.as_bool() {
            return Err("Read editor client origin failed.".into());
        }
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        if dpi == 0 { return Err("Read editor DPI failed.".into()); }
        let width = rectangle.right.checked_sub(rectangle.left).ok_or("Client width overflow")?;
        let height = rectangle.bottom.checked_sub(rectangle.top).ok_or("Client height overflow")?;
        if width < 0 || height < 0 { return Err("Invalid native editor client dimensions.".into()); }
        self.input.set_geometry([width as u32, height as u32], dpi as f32 / 96.0, [origin.x, origin.y]);
        Ok(())
    }
    fn set_modifiers(&self, modifiers: egui::Modifiers) {
        if self.modifiers.replace(modifiers) != modifiers {
            self.input.push(egui::Event::ModifiersChanged(modifiers));
        }
    }
    fn native_modifiers(&self) -> egui::Modifiers {
        let held = |key: VIRTUAL_KEY| (unsafe { GetKeyState(key.0 as i32) }) < 0;
        let ctrl = held(VK_CONTROL);
        egui::Modifiers { alt: held(VK_MENU), ctrl, shift: held(VK_SHIFT),
            mac_cmd: false, command: ctrl }
    }
    fn pointer(&self) -> egui::Pos2 { self.input.logical_position(self.position.get(), self.zoom.get()) }
    fn release_buttons(&self, cause: PointerCancellationCause) {
        let buttons = self.buttons.replace(0);
        self.input.cancel_pointer_buttons(cause, buttons, self.pointer(), self.modifiers.get());
    }
    fn focus(&self, focused: bool) {
        if self.input.focused() == focused { return; }
        self.input.set_focus(focused);
        if !focused {
            self.release_buttons(PointerCancellationCause::FocusLost);
            let keys = std::mem::take(&mut *self.keys.borrow_mut());
            for key in keys { self.input.push(egui::Event::Key { key, physical_key: None,
                pressed: false, repeat: false, modifiers: egui::Modifiers::NONE }); }
            self.set_modifiers(egui::Modifiers::NONE);
            self.ime_keys.borrow_mut().clear();
            self.surrogate.set(None);
        }
    }
    #[deny(unreachable_patterns)]
    pub fn message(&self, message: u32, w: WPARAM, l: LPARAM) -> Result<Option<LRESULT>> {
        match message {
            WM_PAINT => { self.input.repaint.request(Duration::ZERO); Ok(Some(LRESULT(0))) }
            WM_ERASEBKGND => Ok(Some(LRESULT(1))),
            WM_SIZE | WM_MOVE | WM_DPICHANGED | WM_DPICHANGED_AFTERPARENT => {
                self.refresh_geometry()?; Ok(None)
            }
            WM_SETFOCUS => { self.focus(true); Ok(None) }
            WM_KILLFOCUS => { self.focus(false); Ok(None) }
            WM_GETOBJECT => Ok(self.accessibility.get_object(w, l)),
            WM_IME_STARTCOMPOSITION => { self.ime.start(); Ok(Some(LRESULT(0))) }
            WM_IME_COMPOSITION => { self.ime.composition(l.0 as u32)?; Ok(Some(LRESULT(0))) }
            WM_IME_ENDCOMPOSITION => { self.ime.end(); Ok(Some(LRESULT(0))) }

            WM_CHAR | WM_SYSCHAR | WM_IME_CHAR => Ok(Some(LRESULT(0))),
            WM_UNICHAR if w.0 == 0xffff => Ok(Some(LRESULT(1))),
            WM_MOUSEMOVE => {
                self.position.set([l.0 as u16 as i16 as i32, ((l.0 >> 16) as u16) as i16 as i32]);
                let modifiers = self.native_modifiers(); self.set_modifiers(modifiers);
                if !self.tracking.replace(true) {
                    let mut tracking = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE, hwndTrack: self.hwnd, dwHoverTime: 0 };
                    unsafe { TrackMouseEvent(&mut tracking) }.map_err(|e| e.to_string())?;
                }
                self.input.push(egui::Event::PointerMoved(self.pointer())); Ok(Some(LRESULT(0)))
            }
            windows::Win32::UI::Controls::WM_MOUSELEAVE => {
                self.tracking.set(false);
                if self.buttons.get() == 0 { self.input.push(egui::Event::PointerGone); }
                Ok(Some(LRESULT(0)))
            }
            WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP
                | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_XBUTTONDOWN | WM_XBUTTONUP =>
            {
                self.position.set([l.0 as u16 as i16 as i32, ((l.0 >> 16) as u16) as i16 as i32]);
                let (bit, button, pressed) = match message {
                    WM_LBUTTONDOWN => (1, egui::PointerButton::Primary, true),
                    WM_LBUTTONUP => (1, egui::PointerButton::Primary, false),
                    WM_RBUTTONDOWN => (2, egui::PointerButton::Secondary, true),
                    WM_RBUTTONUP => (2, egui::PointerButton::Secondary, false),
                    WM_MBUTTONDOWN => (4, egui::PointerButton::Middle, true),
                    WM_MBUTTONUP => (4, egui::PointerButton::Middle, false),
                    _ if (w.0 >> 16) as u16 == 1 => (8, egui::PointerButton::Extra1, message == WM_XBUTTONDOWN),
                    _ => (16, egui::PointerButton::Extra2, message == WM_XBUTTONDOWN),
                };
                self.buttons.set(if pressed { self.buttons.get() | bit } else { self.buttons.get() & !bit });
                let modifiers = self.native_modifiers(); self.set_modifiers(modifiers);
                self.input.push(egui::Event::PointerButton { pos: self.pointer(), button, pressed, modifiers });
                if pressed {
                    unsafe { SetFocus(Some(self.hwnd)); }
                    if self.input.is_cancelled() { return Ok(Some(LRESULT(0))); }
                    unsafe { SetCapture(self.hwnd); }
                }
                else if self.buttons.get() == 0 { let _ = unsafe { ReleaseCapture() }; }
                Ok(Some(LRESULT(if message == WM_XBUTTONDOWN || message == WM_XBUTTONUP { 1 } else { 0 })))
            }
            WM_CAPTURECHANGED => {
                self.release_buttons(PointerCancellationCause::CaptureChanged); Ok(None)
            }
            WM_CANCELMODE => {
                self.release_buttons(PointerCancellationCause::CancelMode); Ok(None)
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                let delta = ((w.0 >> 16) as u16) as i16 as f32 / 120.0;
                let modifiers = self.native_modifiers(); self.set_modifiers(modifiers);
                self.input.push(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line,
                    delta: if message == WM_MOUSEWHEEL { egui::vec2(0.0, delta) } else { egui::vec2(-delta, 0.0) },
                    phase: egui::TouchPhase::Move, modifiers });
                Ok(Some(LRESULT(0)))
            }
            WM_SETCURSOR if (l.0 as u16) == HTCLIENT as u16 => {
                let cursor = self.cursor.get();
                if !cursor.0.is_null() { unsafe { SetCursor(Some(cursor)) }; return Ok(Some(LRESULT(1))); }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    pub fn host_key(&self, unicode: u16, code: i16, mods: i16, down: bool) -> Result<bool> {
        if self.input.is_cancelled() || !self.input.focused() { return Ok(false); }
        let modifiers = egui::Modifiers { shift: mods & 1 != 0, alt: mods & 2 != 0,
            ctrl: mods & 4 != 0, command: mods & 4 != 0, mac_cmd: false };
        self.set_modifiers(modifiers);
        let key = vst_key(unicode, code);
        let identity = if code != 0 { (code, 0) } else { (0, unicode) };
        let suppressed = if down { self.ime_keys.borrow().contains(&identity) }
            else { self.ime_keys.borrow_mut().remove(&identity) };
        if suppressed {
            let balanced = if !down {
                if let Some(key) = key {
                    let delivered = self.keys.borrow_mut().remove(&key);
                    if delivered { self.input.push(egui::Event::Key { key, physical_key: None,
                        pressed: false, repeat: false, modifiers }); }
                    delivered
                } else { false }
            } else { false };
            return Ok(balanced || self.input.text_wanted.get());
        }
        if down && self.input.ime_active.get() {

            self.ime_keys.borrow_mut().insert(identity);
            self.surrogate.set(None);
            return Ok(self.input.text_wanted.get());
        }
        if key.is_none() && unicode < 0x20 && !matches!(code, 54..=56) { return Ok(false); }
        let held = key.is_some_and(|key| self.keys.borrow().contains(&key));
        let shortcuts = self.input.shortcut_mask.get();
        let waypoint_key = !modifiers.any() && !self.input.text_wanted.get()
            && key.is_some_and(|key| shortcuts & crate::desktop::waypoint_key_bit(key) != 0);
        let app_shortcut = key.is_some_and(|key| {
            (shortcuts & 1 != 0 && modifiers.command && matches!(key, egui::Key::O | egui::Key::S | egui::Key::Z))
                || (shortcuts & 2 != 0 && key == egui::Key::Delete)
                || (shortcuts & 4 != 0 && key == egui::Key::Escape && !modifiers.any())
        });
        let handled = if down { self.input.keyboard_wanted.get() || app_shortcut || waypoint_key } else { held };
        if !handled {
            if down && key == Some(egui::Key::Escape) && !modifiers.any() {

                self.input.observe_unhandled_escape_down();
            }
            return Ok(false);
        }
        if let Some(key) = key {
            let repeat = if down { !self.keys.borrow_mut().insert(key) }
                else { self.keys.borrow_mut().remove(&key); false };
            self.input.push_key(egui::Event::Key { key, physical_key: None, pressed: down, repeat, modifiers },
                waypoint_key && down && !repeat);
            if down && modifiers.command && !modifiers.alt {
                match key {
                    egui::Key::C => { self.input.push(egui::Event::Copy); return Ok(true); }
                    egui::Key::X => { self.input.push(egui::Event::Cut); return Ok(true); }
                    egui::Key::V => { self.input.paste_requested.set(true); return Ok(true); }
                    _ => {},
                }
            }
        }
        if down && !self.input.ime_active.get() && (!modifiers.command || modifiers.alt)
            && unicode >= 0x20 && unicode != 0x7f
        {
            if (0xd800..=0xdbff).contains(&unicode) { self.surrogate.set(Some(unicode)); }
            else {
                let mut words = Vec::with_capacity(2);
                if let Some(high) = self.surrogate.take() { words.push(high); }
                words.push(unicode);
                let text = String::from_utf16(&words).map_err(|e| format!("Invalid host Unicode key: {e}"))?;
                self.input.push(egui::Event::Text(text));
            }
        }
        Ok(true)
    }
    pub fn before_frame(&self, context: &egui::Context) -> Result<()> {
        self.refresh_geometry()?;
        self.accessibility.before_frame(context, &self.input);
        if self.input.paste_requested.replace(false) {
            if let Some(text) = clipboard::read_text(self.hwnd)? { self.input.push(egui::Event::Paste(text)); }
        }
        Ok(())
    }
    pub fn after_frame(&self, context: &egui::Context, output: egui::PlatformOutput,
        pixels_per_point: f32) -> Result<()>
    {
        self.zoom.set(context.zoom_factor());
        self.input.keyboard_wanted.set(context.egui_wants_keyboard_input());
        if output.cursor_image.is_some() { return Err("Custom bitmap cursor output is not implemented in this draft.".into()); }
        for command in output.commands {
            if self.input.is_cancelled() { return Ok(()); }
            match command {
                egui::OutputCommand::CopyText(text) => clipboard::write_text(self.hwnd, &text)?,
                egui::OutputCommand::CopyImage(image) => clipboard::write_image(self.hwnd, &image)?,
                egui::OutputCommand::OpenUrl(url) => clipboard::open_url(self.hwnd, &url.url)?,
            }
        }
        if self.input.is_cancelled() { return Ok(()); }
        let identifier: PCWSTR = match output.cursor_icon {
            egui::CursorIcon::None => { self.cursor.set(HCURSOR::default()); unsafe { SetCursor(None) }; PCWSTR::null() }
            egui::CursorIcon::Text | egui::CursorIcon::VerticalText => IDC_IBEAM,
            egui::CursorIcon::PointingHand => IDC_HAND,
            egui::CursorIcon::Crosshair => IDC_CROSS,
            egui::CursorIcon::Wait => IDC_WAIT,
            egui::CursorIcon::Progress => IDC_APPSTARTING,
            egui::CursorIcon::NotAllowed | egui::CursorIcon::NoDrop => IDC_NO,
            egui::CursorIcon::ResizeHorizontal | egui::CursorIcon::ResizeEast | egui::CursorIcon::ResizeWest => IDC_SIZEWE,
            egui::CursorIcon::ResizeVertical | egui::CursorIcon::ResizeNorth | egui::CursorIcon::ResizeSouth => IDC_SIZENS,
            egui::CursorIcon::ResizeNwSe | egui::CursorIcon::ResizeNorthWest | egui::CursorIcon::ResizeSouthEast => IDC_SIZENWSE,
            egui::CursorIcon::ResizeNeSw | egui::CursorIcon::ResizeNorthEast | egui::CursorIcon::ResizeSouthWest => IDC_SIZENESW,
            egui::CursorIcon::Move | egui::CursorIcon::AllScroll | egui::CursorIcon::Grab | egui::CursorIcon::Grabbing => IDC_SIZEALL,
            _ => IDC_ARROW,
        };
        if !identifier.is_null() { self.cursor.set(unsafe { LoadCursorW(None, identifier) }.map_err(|e| e.to_string())?); }
        if self.input.is_cancelled() { return Ok(()); }
        self.ime.output(output.ime, pixels_per_point)?;
        if !self.input.is_cancelled() { self.accessibility.publish(output.accesskit_update, self.input.focused()); }
        Ok(())
    }
}
fn vst_key(unicode: u16, code: i16) -> Option<egui::Key> {
    use egui::Key::*;
    match code {
        1 => Some(Backspace), 2 => Some(Tab), 4 | 19 => Some(Enter), 6 => Some(Escape), 7 => Some(Space),
        9 => Some(End), 10 => Some(Home), 11 => Some(ArrowLeft), 12 => Some(ArrowUp),
        13 => Some(ArrowRight), 14 => Some(ArrowDown), 15 => Some(PageUp), 16 => Some(PageDown),
        21 => Some(Insert), 22 => Some(Delete),
        40..=51 => Some([F1,F2,F3,F4,F5,F6,F7,F8,F9,F10,F11,F12][(code - 40) as usize]),
        65..=76 => Some([F13,F14,F15,F16,F17,F18,F19,F20,F21,F22,F23,F24][(code - 65) as usize]),
        _ => {

            let character = if code >= 128 { char::from_u32((code - 128 + 0x30) as u32) }
                else { char::from_u32(unicode as u32) }?;
            egui::Key::from_name(&character.to_uppercase().to_string())
        }
    }
}
