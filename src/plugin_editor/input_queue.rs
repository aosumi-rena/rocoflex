use std::{
    cell::{Cell, RefCell},
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    time::{Duration, Instant},
};
use eframe::egui;
use super::navigation::{NavigationEpoch,NavigationTag};
use crate::desktop::canvas_gesture::{PointerCancellation, PointerCancellationCause, UnhandledEscapeDown, POINTER_BUTTONS};

pub(super) struct InputBatch {
    pub raw: egui::RawInput,
    pub pointer_cancellations: Vec<PointerCancellation>,
    pub unhandled_escape_downs: Vec<UnhandledEscapeDown>,
    pub binding_epoch: Option<u64>,
    pub binding_mixed: bool,
    pub navigation_tag: Option<NavigationTag>,
    pub navigation_mixed: bool,
}

#[derive(Default)]
pub(super) struct RepaintSignal {
    immediate: AtomicBool,
    deadline: Mutex<Option<Instant>>,
}
impl RepaintSignal {
    pub fn request(&self, delay: Duration) {
        if delay.is_zero() {
            self.immediate.store(true, Ordering::Release);
        } else if let Some(when) = Instant::now().checked_add(delay) {
            let mut deadline = self.deadline.lock().unwrap_or_else(|p| p.into_inner());
            *deadline = Some(deadline.map_or(when, |old| old.min(when)));
        }
    }
    pub fn take_due(&self, now: Instant) -> bool {
        let immediate = self.immediate.swap(false, Ordering::AcqRel);
        let mut deadline = self.deadline.lock().unwrap_or_else(|p| p.into_inner());
        let timed = deadline.is_some_and(|when| when <= now);
        if timed { *deadline = None; }
        immediate || timed
    }
}

#[derive(Default)]
struct Events {
    events: Vec<egui::Event>,
    pointer_cancellations: Vec<PointerCancellation>,
    unhandled_escape_downs: Vec<UnhandledEscapeDown>,
    hovered: Vec<egui::HoveredFile>,
    dropped: Vec<egui::DroppedFileHandle>,
    binding_epoch: Option<u64>,
    binding_mixed: bool,
    navigation_tag: Option<NavigationTag>,
    navigation_mixed: bool,
}
pub(super) struct InputQueue {
    pending: RefCell<Events>,
    pub repaint: Arc<RepaintSignal>,
    pub cancelled: Arc<AtomicBool>,
    fault: Cell<Option<&'static str>>,
    extent: Cell<[u32; 2]>,
    native_ppp: Cell<f32>,
    content_scale: Cell<Option<f32>>,
    screen_origin: Cell<[i32; 2]>,
    focused: Cell<bool>,
    pub keyboard_wanted: Cell<bool>,
    pub text_wanted: Cell<bool>,
    pub ime_active: Cell<bool>,
    pub shortcut_mask: Cell<u64>,
    binding_epoch: Cell<Option<u64>>,
    binding_owner: Cell<Option<(u64,u64,u8)>>,
    binding_mask: Cell<u64>,
    navigation: Cell<NavigationEpoch>,
    pub paste_requested: Cell<bool>,
}
impl InputQueue {
    pub fn new(extent: [u32; 2], native_ppp: f32) -> Self {
        let repaint = Arc::new(RepaintSignal::default());
        repaint.request(Duration::ZERO);
        Self {
            pending: RefCell::new(Events::default()), repaint,
            cancelled: Arc::new(AtomicBool::new(false)), fault: Cell::new(None),
            extent: Cell::new(extent), native_ppp: Cell::new(native_ppp),
            content_scale: Cell::new(None),
            screen_origin: Cell::new([0, 0]), focused: Cell::new(false),
            keyboard_wanted: Cell::new(false), text_wanted: Cell::new(false),
            ime_active: Cell::new(false),
            shortcut_mask: Cell::new(0), binding_epoch: Cell::new(Some(0)),
            binding_owner: Cell::new(None), binding_mask: Cell::new(0),
            navigation:Cell::new(Default::default()), paste_requested: Cell::new(false),
        }
    }
    pub fn cancel(&self) { self.mute_waypoint_keys(); self.mute_navigation(); self.cancelled.store(true, Ordering::Release); }
    pub fn mute_navigation(&self) {
        let mut navigation=self.navigation.get();navigation.mute();self.navigation.set(navigation);
        self.shortcut_mask.set(self.shortcut_mask.get() & !crate::desktop::EXPERIMENTAL_NAVIGATION_BIT);
    }
    pub fn mute_waypoint_keys(&self) {
        self.shortcut_mask.set(self.shortcut_mask.get() & !crate::desktop::WAYPOINT_KEY_BITS);
    }
    pub fn publish_shortcuts(&self, mask: u64, owner: Option<(u64,u64,u8)>, installed: bool) {
        let keys = mask & crate::desktop::WAYPOINT_KEY_BITS;
        if installed || self.binding_owner.get() != owner || self.binding_mask.get() != keys {
            self.binding_epoch.set(self.binding_epoch.get().and_then(|epoch| epoch.checked_add(1)));
            self.binding_owner.set(owner);
            self.binding_mask.set(keys);
        }
        let enabled = self.binding_epoch.get().is_some() && owner.is_some() && !self.is_cancelled()
            && self.fault().is_none();
        let nav_bit=crate::desktop::EXPERIMENTAL_NAVIGATION_BIT;
        let wanted=mask & nav_bit!=0 && self.focused() && !self.text_wanted.get() && !self.ime_active.get()
            && !self.is_cancelled() && self.fault().is_none();
        let mut owner=self.navigation.get();
        let navigation=owner.publish(wanted,installed);self.navigation.set(owner);
        self.shortcut_mask.set((mask & !(crate::desktop::WAYPOINT_KEY_BITS|nav_bit))
            | if enabled { keys } else { 0 } | if navigation { nav_bit } else { 0 });
    }
    pub fn navigation_batch_allowed(&self,tag:Option<NavigationTag>,mixed:bool)->bool {
        !self.is_cancelled() && self.fault().is_none() && self.navigation.get().accepts(tag,mixed)
    }
    pub fn binding_batch_allowed(&self, epoch: Option<u64>, mixed: bool) -> bool {
        !self.is_cancelled() && self.fault().is_none() && !mixed
            && epoch.is_some() && epoch == self.binding_epoch.get()
    }
    pub fn is_cancelled(&self) -> bool { self.cancelled.load(Ordering::Acquire) }
    pub fn fault(&self) -> Option<&'static str> { self.fault.get() }
    pub fn fail(&self, message: &'static str) {
        self.mute_waypoint_keys(); self.mute_navigation();
        if self.fault.get().is_none() { self.fault.set(Some(message)); }
        self.repaint.request(Duration::ZERO);
    }
    pub fn push(&self, event: egui::Event) -> bool {
        self.push_key(event, false, false)
    }
    pub fn push_key(&self, event: egui::Event, bound_fresh_down: bool, navigation_event: bool) -> bool {
        if self.is_cancelled() { return false; }
        let Ok(mut pending) = self.pending.try_borrow_mut() else {
            self.fail("Native input queue was reentered while borrowed."); return false;
        };
        if pending.events.try_reserve(1).is_err() {
            self.fail("Not enough memory to retain native editor input."); return false;
        }
        if bound_fresh_down && matches!(&event, egui::Event::Key {
            key, pressed: true, repeat: false, modifiers, .. }
                if !modifiers.any() && !self.text_wanted.get() && !self.ime_active.get()
                    && self.shortcut_mask.get() & crate::desktop::waypoint_key_bit(*key) != 0)
            && let Some(epoch) = self.binding_epoch.get()
        {
            if pending.binding_epoch.is_some_and(|previous| previous != epoch) { pending.binding_mixed = true; }
            pending.binding_epoch.get_or_insert(epoch);
        }
        if navigation_event {
            let tag=self.navigation.get().tag();
            if pending.navigation_tag.is_some_and(|prior|prior!=tag) {pending.navigation_mixed=true;}
            pending.navigation_tag.get_or_insert(tag);
        }
        pending.events.push(event);
        self.repaint.request(Duration::ZERO);
        true
    }

    pub fn observe_unhandled_escape_down(&self) -> bool {
        if self.is_cancelled() { return false; }
        let Ok(mut pending) = self.pending.try_borrow_mut() else {
            self.fail("Unhandled Escape observation queue was reentered."); return false;
        };
        if pending.unhandled_escape_downs.try_reserve(1).is_err() {
            self.fail("Not enough memory to retain an unhandled Escape observation."); return false;
        }
        let event_boundary = pending.events.len();
        pending.unhandled_escape_downs.push(UnhandledEscapeDown { event_boundary });
        self.repaint.request(Duration::ZERO);
        true
    }
    pub fn cancel_pointer_buttons(&self, cause: PointerCancellationCause, held_buttons: u8,
        pos: egui::Pos2, modifiers: egui::Modifiers) -> bool
    {
        if self.is_cancelled() { return false; }

        if held_buttons & !31 != 0 {
            self.fail("Native pointer cancellation had an invalid button mask."); return false;
        }
        if held_buttons == 0 { return true; }
        let Ok(mut pending) = self.pending.try_borrow_mut() else {
            self.fail("Native pointer cancellation queue was reentered."); return false;
        };
        let count = held_buttons.count_ones() as usize;

        if pending.events.try_reserve(count).is_err()
            || pending.pointer_cancellations.try_reserve(1).is_err()
        {
            self.fail("Not enough memory to retain native pointer cancellation."); return false;
        }
        let event_offset = pending.events.len();
        pending.pointer_cancellations.push(PointerCancellation { cause, held_buttons, event_offset });
        for (bit, button) in POINTER_BUTTONS {
            if held_buttons & bit != 0 {
                pending.events.push(egui::Event::PointerButton { pos, button, pressed: false, modifiers });
            }
        }
        self.repaint.request(Duration::ZERO);
        true
    }
    pub fn set_hovered(&self, hovered: Vec<egui::HoveredFile>) -> bool {
        let Ok(mut pending) = self.pending.try_borrow_mut() else {
            self.fail("Native file hover queue was reentered."); return false;
        };
        pending.hovered = hovered;
        self.repaint.request(Duration::ZERO);
        true
    }
    pub fn drop_files(&self, files: Vec<egui::DroppedFileHandle>) -> bool {
        if self.is_cancelled() { return false; }
        let Ok(mut pending) = self.pending.try_borrow_mut() else {
            self.fail("Native file drop queue was reentered."); return false;
        };
        if pending.dropped.try_reserve(files.len()).is_err() {
            self.fail("Not enough memory to retain dropped files."); return false;
        }
        pending.dropped.extend(files);
        pending.hovered.clear();
        self.repaint.request(Duration::ZERO);
        true
    }
    pub fn set_geometry(&self, extent: [u32; 2], native_ppp: f32, screen_origin: [i32; 2]) {
        if !native_ppp.is_finite() || native_ppp <= 0.0 {
            self.fail("The editor received an invalid native DPI scale."); return;
        }
        let previous_ppp = self.effective_ppp();

        self.native_ppp.set(native_ppp);
        let changed = (self.extent.replace(extent) != extent)
            | (previous_ppp.to_bits() != self.effective_ppp().to_bits())
            | (self.screen_origin.replace(screen_origin) != screen_origin);
        if changed { self.repaint.request(Duration::ZERO); }
    }
    pub fn set_content_scale(&self, factor: f32) -> Result<(), &'static str> {
        let scale = if factor == 0.0 { None }
            else if factor.is_finite() && factor > 0.0 && factor <= 8.0 { Some(factor) }
            else { return Err("Invalid editor content scale."); };
        let previous_ppp = self.effective_ppp();

        self.content_scale.set(scale);
        if previous_ppp.to_bits() != self.effective_ppp().to_bits() {
            self.repaint.request(Duration::ZERO);
        }
        Ok(())
    }
    pub fn extent(&self) -> [u32; 2] { self.extent.get() }
    pub fn effective_ppp(&self) -> f32 { self.content_scale.get().unwrap_or(self.native_ppp.get()) }
    pub fn focused(&self) -> bool { self.focused.get() }
    pub fn set_focus(&self, focused: bool) {
        self.focused.set(focused);
        self.push(egui::Event::WindowFocused(focused));
        if !focused {
            self.mute_waypoint_keys(); self.mute_navigation();
            self.binding_epoch.set(self.binding_epoch.get().and_then(|epoch| epoch.checked_add(1)));
            self.push(egui::Event::PointerGone);
            self.ime_active.set(false);
        }
    }
    pub fn logical_position(&self, physical: [i32; 2], zoom: f32) -> egui::Pos2 {
        let ppp = self.effective_ppp() * zoom;
        egui::pos2(physical[0] as f32 / ppp, physical[1] as f32 / ppp)
    }
    pub fn take_input(&self, context: &egui::Context, elapsed: f64, max_texture_side: usize)
        -> Result<InputBatch, &'static str>
    {
        if let Some(fault) = self.fault.get() { return Err(fault); }
        if self.is_cancelled() { return Err("The native editor window has been retired."); }
        let mut pending = self.pending.try_borrow_mut().map_err(|_| "The input queue is busy.")?;

        let events = std::mem::take(&mut pending.events);
        let pointer_cancellations = std::mem::take(&mut pending.pointer_cancellations);
        let unhandled_escape_downs = std::mem::take(&mut pending.unhandled_escape_downs);
        let binding_epoch = pending.binding_epoch.take();
        let binding_mixed = std::mem::take(&mut pending.binding_mixed);
        let navigation_tag=pending.navigation_tag.take();
        let navigation_mixed=std::mem::take(&mut pending.navigation_mixed);
        let dropped_files = std::mem::take(&mut pending.dropped);
        let hovered_files = pending.hovered.clone();
        drop(pending);
        let ppp = self.effective_ppp() * context.zoom_factor();
        if !ppp.is_finite() || ppp <= 0.0 { return Err("The editor UI scale is invalid."); }
        let [width, height] = self.extent.get();
        let size = egui::vec2(width as f32 / ppp, height as f32 / ppp);
        let [x, y] = self.screen_origin.get();
        let screen_origin = egui::pos2(x as f32 / ppp, y as f32 / ppp);
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            time: Some(elapsed), max_texture_side: Some(max_texture_side),
            focused: self.focused.get(), events, hovered_files, dropped_files,
            ..Default::default()
        };
        input.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo {
            title: Some("Rocoflex".into()), native_pixels_per_point: Some(self.effective_ppp()),
            inner_rect: Some(egui::Rect::from_min_size(screen_origin, size)),
            focused: Some(self.focused.get()), minimized: Some(width == 0 || height == 0),
            ..Default::default()
        });
        Ok(InputBatch { raw: input, pointer_cancellations, unhandled_escape_downs, binding_epoch, binding_mixed,
            navigation_tag,navigation_mixed })
    }
}

