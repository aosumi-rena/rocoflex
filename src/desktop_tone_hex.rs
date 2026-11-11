use eframe::egui::{self, Rect, TextBuffer, text::CharIndex};
use std::ops::Range;

#[derive(Default)]
pub(super) struct ToneHexEdit {
    committed: String,
    draft: Option<String>,
    reset_focus: bool,
    suspended: bool,
}

pub(super) struct HexResponse {
    pub began: bool,
    pub rgb: Option<[u8; 3]>,
}

struct HexBuffer<'a>(&'a mut String);

impl TextBuffer for HexBuffer<'_> {
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<HexBuffer<'static>>()
    }

    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.0
    }

    fn insert_text(&mut self, text: &str, index: CharIndex) -> usize {

        let room = 6usize.saturating_sub(self.0.len());
        let inserted: String = text
            .chars()
            .filter(char::is_ascii_hexdigit)
            .take(room)
            .collect();
        let count = inserted.len();
        self.0.insert_str(index.0.min(self.0.len()), &inserted);
        count
    }

    fn delete_char_range(&mut self, range: Range<CharIndex>) {
        self.0.replace_range(range.start.0..range.end.0, "");
    }
}

impl ToneHexEdit {
    fn id() -> egui::Id {
        egui::Id::new("native-tone-hex")
    }

    pub fn is_editing(&self) -> bool {
        self.draft.is_some()
    }

    pub fn sync_from_controls(&mut self, rgb: [u8; 3]) {
        self.committed = format!("{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
        self.draft = None;
        self.reset_focus = true;
    }

    fn begin(&mut self) -> bool {
        if self.is_editing() {
            return false;
        }
        self.draft = Some(self.committed.clone());
        true
    }

    pub fn cancel(&mut self) -> bool {
        let cancelled = self.draft.take().is_some();
        self.reset_focus |= cancelled;
        cancelled
    }

    pub fn commit(&mut self) -> Option<[u8; 3]> {
        let text = self.draft.take()?;
        self.reset_focus = true;
        if text == self.committed {
            return None;
        }
        let value = text.bytes().fold(0u32, |value, digit| {
            let digit = match digit {
                b'0'..=b'9' => digit - b'0',
                b'A'..=b'F' => digit - b'A' + 10,
                b'a'..=b'f' => digit - b'a' + 10,
                _ => unreachable!("hex input is filtered"),
            };
            (value << 4) | u32::from(digit)
        });
        self.committed = text;
        Some([(value >> 16) as u8, (value >> 8) as u8, value as u8])
    }

    pub fn show(&mut self, ui: &mut egui::Ui, rect: Rect) -> HexResponse {
        if !ui.is_enabled() {
            self.suspended = true;
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
                self.draft.as_deref().unwrap_or(&self.committed),
                egui::FontId::proportional(13.0), super::look::MUTED);
            return HexResponse { began: false, rgb: None };
        }
        let resumed = std::mem::take(&mut self.suspended);
        let id = Self::id();
        if self.reset_focus {
            ui.memory_mut(|memory| memory.surrender_focus(id));
            self.reset_focus = false;
        }
        if self.is_editing()
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel();
            ui.memory_mut(|memory| memory.surrender_focus(id));
        }

        let layer = ui.layer_id();
        ui.memory_mut(|memory| memory.interested_in_focus(id, layer));
        let pressed_at = ui.input(|input| {
            input.pointer.primary_pressed().then(|| input.pointer.interact_pos()).flatten()
        });

        let pressed = pressed_at.is_some_and(|at| rect.contains(at)
            && ui.is_rect_visible(rect)
            && ui.ctx().layer_id_at(at) == Some(ui.layer_id())
            && ui.memory(|memory| memory.allows_interaction(ui.layer_id())));
        let mut began = if pressed || ui.memory(|memory| memory.has_focus(id)) {
            self.begin()
        } else {
            false
        };
        if began || !self.is_editing() {
            let mut state = egui::text_edit::TextEditState::default();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(self.draft.as_ref().unwrap_or(&self.committed).len()),
                )));
            state.store(ui.ctx(), id);
            if began && !ui.memory(|memory| memory.has_focus(id)) {
                ui.memory_mut(|memory| memory.request_focus(id));
            }
        }
        let mut label = self.committed.clone();
        let text = self.draft.as_mut().unwrap_or(&mut label);
        let mut buffer = HexBuffer(text);
        let focused_before_edit = ui.memory(|memory| memory.has_focus(id));
        let response = ui.put(
            rect,
            egui::TextEdit::singleline(&mut buffer)
                .id(id)
                .font(egui::FontId::proportional(13.0))
                .horizontal_align(egui::Align::Center)
                .vertical_align(egui::Align::Center)
                .min_size(rect.size())
                .frame(egui::Frame::NONE)
                .margin(egui::vec2(6.0, 0.0)),
        );

        let focus_ended_in_edit = focused_before_edit
            && !ui.memory(|memory| memory.has_focus(id));
        if began && pressed && !response.changed() {
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(self.draft.as_ref().map_or(0, String::len)),
                )));
            state.store(ui.ctx(), id);
        }
        let rgb = if self.is_editing()
            && (response.lost_focus() || focus_ended_in_edit) && !resumed {
            self.commit()
        } else {
            None
        };

        if response.gained_focus() && !self.is_editing() && !response.lost_focus() {
            began |= self.begin();
            if response.changed() {

                self.draft = Some(label);
            } else {
                let mut state = egui::text_edit::TextEditState::default();
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.draft.as_ref().map_or(0, String::len)),
                    )));
                state.store(ui.ctx(), id);
            }
        }
        HexResponse { began, rgb }
    }
}
