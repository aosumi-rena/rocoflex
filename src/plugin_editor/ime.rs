use std::rc::Rc;
use eframe::egui;
use windows::Win32::{Foundation::{HWND, POINT, RECT}, UI::Input::Ime::*};
use super::input_queue::InputQueue;
type Result<T> = std::result::Result<T, String>;
struct Context { hwnd: HWND, handle: HIMC }
impl Context {
    fn get(hwnd: HWND) -> Option<Self> {
        let handle = unsafe { ImmGetContext(hwnd) };
        (!handle.0.is_null()).then_some(Self { hwnd, handle })
    }
}
impl Drop for Context {
    fn drop(&mut self) { let _ = unsafe { ImmReleaseContext(self.hwnd, self.handle) }; }
}
pub(super) struct Ime { hwnd: HWND, input: Rc<InputQueue> }
impl Ime {
    pub fn new(hwnd: HWND, input: Rc<InputQueue>) -> Self { Self { hwnd, input } }
    pub fn start(&self) {
        self.input.ime_active.set(true);
        self.input.push(egui::Event::Ime(egui::ImeEvent::Preedit {
            text: String::new(), active_range_chars: None,
        }));
    }
    pub fn composition(&self, flags: u32) -> Result<()> {
        let Some(context) = Context::get(self.hwnd) else {
            return Err("The editor has no native IME composition context.".into());
        };
        if flags & GCS_RESULTSTR.0 != 0 {
            let text = text(context.handle, GCS_RESULTSTR)?;
            self.input.push(egui::Event::Ime(egui::ImeEvent::Commit(text)));
        }
        if flags & GCS_COMPSTR.0 != 0 {
            let text = text(context.handle, GCS_COMPSTR)?;
            let cursor_utf16 = unsafe { ImmGetCompositionStringW(context.handle, GCS_CURSORPOS, None, 0) };
            let active_range_chars = if cursor_utf16 >= 0 {
                let mut units = 0;
                let caret = text.chars().take_while(|character| {
                    units += character.len_utf16(); units <= cursor_utf16 as usize
                }).count();
                Some(caret..caret)
            } else { None };
            self.input.push(egui::Event::Ime(egui::ImeEvent::Preedit { text, active_range_chars }));
        }
        Ok(())
    }
    pub fn end(&self) {
        self.input.ime_active.set(false);
        self.input.push(egui::Event::Ime(egui::ImeEvent::Preedit {
            text: String::new(), active_range_chars: None,
        }));
    }
    pub fn output(&self, output: Option<egui::output::IMEOutput>, pixels_per_point: f32) -> Result<()> {
        if self.input.is_cancelled() { return Ok(()); }
        self.input.text_wanted.set(output.is_some());
        let Some(output) = output else { return Ok(()); };
        let Some(context) = Context::get(self.hwnd) else {
            return Err("Text editing requires a native IME context.".into());
        };
        if output.should_interrupt_composition && self.input.ime_active.get() {
            if !unsafe { ImmNotifyIME(context.handle, NI_COMPOSITIONSTR, CPS_CANCEL, 0) }.as_bool() {
                return Err("Cancel native IME composition failed.".into());
            }
        }
        if self.input.is_cancelled() { return Ok(()); }
        let rectangle = output.cursor_rect * pixels_per_point;
        let point = POINT { x: rectangle.left().round() as i32, y: rectangle.bottom().round() as i32 };
        let composition = COMPOSITIONFORM { dwStyle: CFS_POINT, ptCurrentPos: point,
            rcArea: RECT::default() };
        if !unsafe { ImmSetCompositionWindow(context.handle, &composition) }.as_bool() {
            return Err("Position native IME composition failed.".into());
        }
        if self.input.is_cancelled() { return Ok(()); }
        let candidate = CANDIDATEFORM { dwIndex: 0, dwStyle: CFS_EXCLUDE, ptCurrentPos: point,
            rcArea: RECT { left: rectangle.left().floor() as i32, top: rectangle.top().floor() as i32,
                right: rectangle.right().ceil() as i32, bottom: rectangle.bottom().ceil() as i32 } };
        if !unsafe { ImmSetCandidateWindow(context.handle, &candidate) }.as_bool() {
            return Err("Position native IME candidate window failed.".into());
        }
        Ok(())
    }
}
fn text(context: HIMC, kind: IME_COMPOSITION_STRING) -> Result<String> {
    let length = unsafe { ImmGetCompositionStringW(context, kind, None, 0) };
    if length < 0 || length > 1024 * 1024 || length % 2 != 0 {
        return Err("Native IME composition has invalid bounds.".into());
    }
    let mut words = vec![0u16; length as usize / 2];
    if length != 0 {
        let copied = unsafe { ImmGetCompositionStringW(context, kind,
            Some(words.as_mut_ptr().cast()), length as u32) };
        if copied != length { return Err("Native IME composition changed during capture.".into()); }
    }
    String::from_utf16(&words).map_err(|e| e.to_string())
}
