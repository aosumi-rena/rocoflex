//! MIDI 1.0 note and controller edge matching for shortcut gestures.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub(super) enum MidiKind { Note, Controller }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct MidiBinding { pub kind: MidiKind, pub channel: u8, pub number: u8 }
impl MidiBinding {
    pub fn valid(self) -> bool { self.channel < 16 && self.number < 128 }
    pub fn label(self) -> String { format!("{} {} / {}", match self.kind { MidiKind::Note => "Note", MidiKind::Controller => "CC" }, self.number, self.channel + 1) }
}
#[derive(Default)]
pub(super) struct Gate { notes: Vec<bool>, controllers: Vec<bool> }
impl Gate {
    pub fn clear(&mut self) { self.notes.clear(); self.controllers.clear(); }
    pub fn input(&mut self, packed: u32) -> Option<MidiBinding> {
        let status = (packed & 0xff) as u8;
        let a = ((packed >> 8) & 0xff) as u8;
        let b = ((packed >> 16) & 0xff) as u8;
        if a > 127 || b > 127 { return None; }
        if self.notes.len() != 2048 { self.notes = vec![false; 2048]; self.controllers = vec![false; 2048]; }
        let channel = status & 15;
        let slot = usize::from(channel) * 128 + usize::from(a);
        let (kind, active) = match status & 0xf0 {
            0x80 => (MidiKind::Note, false),
            0x90 => (MidiKind::Note, b != 0),
            0xb0 => (MidiKind::Controller, b >= 64),
            _ => return None,
        };
        let state = match kind { MidiKind::Note => &mut self.notes[slot], MidiKind::Controller => &mut self.controllers[slot] };
        let rising = active && !*state;
        *state = active;
        rising.then_some(MidiBinding { kind, channel, number: a })
    }
}
