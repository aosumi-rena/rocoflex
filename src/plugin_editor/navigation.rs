//! 3D view key ownership, independent of HWNDs and event delivery.
use eframe::egui::{Key, Modifiers};
use std::collections::BTreeSet;

pub(super) fn navigation_candidate(key:Key)->bool {
    matches!(key,Key::W|Key::A|Key::S|Key::D|Key::ArrowUp|Key::ArrowDown|
        Key::ArrowLeft|Key::ArrowRight|Key::Plus|Key::Equals|Key::Minus|Key::Space)
}
pub(super) fn navigation_key(key:Key,modifiers:Modifiers)->bool {
    if modifiers.alt || modifiers.mac_cmd {return false;}
    if !modifiers.ctrl && !modifiers.shift && !modifiers.command {
        matches!(key,Key::W|Key::A|Key::S|Key::D|
            Key::ArrowUp|Key::ArrowDown|Key::ArrowLeft|Key::ArrowRight|Key::Space)
    } else {
        (modifiers.ctrl || modifiers.shift) && matches!(key,Key::Plus|Key::Equals|Key::Minus)
    }
}
/// A host can report the shifted glyph on down and unshifted glyph on up.
/// Canonicalize the zero-code main-keyboard variants; keypad codes stay distinct.
pub(super) fn main_symbol(unicode:u16)->Option<Key> {
    match unicode {0x2b|0x3d=>Some(Key::Equals),0x2d|0x5f=>Some(Key::Minus),_=>None}
}
/// Navigation batches belong to one focus/document owner. Overflow disables
/// further admission instead of reusing an old epoch.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(super) struct NavigationTag {epoch:Option<u64>,owner:bool}
#[derive(Clone,Copy)]
pub(super) struct NavigationEpoch {epoch:Option<u64>,owner:bool}
impl Default for NavigationEpoch {fn default()->Self {Self {epoch:Some(0),owner:false}}}
impl NavigationEpoch {
    pub fn publish(&mut self,wanted:bool,installed:bool)->bool {
        if installed || self.owner!=wanted {self.epoch=self.epoch.and_then(|n|n.checked_add(1));}
        self.owner=wanted && self.epoch.is_some();self.owner
    }
    pub fn mute(&mut self) {if self.owner {self.epoch=self.epoch.and_then(|n|n.checked_add(1));self.owner=false;}}
    pub fn tag(&self)->NavigationTag {NavigationTag {epoch:self.epoch,owner:self.owner}}
    pub fn accepts(&self,tag:Option<NavigationTag>,mixed:bool)->bool {
        self.owner && !mixed && self.epoch.is_some()
            && tag.is_none_or(|tag|tag.owner && tag.epoch==self.epoch)
    }
}
#[derive(Default)]
pub(super) struct NavigationKeys {active:BTreeSet<Key>,retired:BTreeSet<Key>}
impl NavigationKeys {
    pub fn note(&mut self,key:Key,down:bool,repeat:bool,owned:bool) {
        if down && !repeat && owned {self.active.insert(key);}
        if !down {self.active.remove(&key);}
    }
    pub fn retire(&mut self)->Vec<Key> {
        let keys=std::mem::take(&mut self.active);
        self.retired.extend(keys.iter().copied());
        keys.into_iter().collect()
    }
    pub fn consume_retired(&mut self,key:Key,down:bool)->bool {
        if !self.retired.contains(&key) {return false;}
        if !down {self.retired.remove(&key);}
        true
    }
    pub fn clear(&mut self) {self.active.clear();self.retired.clear();}
}
