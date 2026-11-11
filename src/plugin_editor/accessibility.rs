use std::{cell::RefCell, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration};
use eframe::egui::{self, accesskit};
use windows::Win32::Foundation::{HWND, WPARAM, LPARAM, LRESULT};
use super::input_queue::{InputQueue, RepaintSignal};

struct Actions {
    actions: Mutex<Vec<accesskit::ActionRequest>>,
    activate: AtomicBool,
    repaint: Arc<RepaintSignal>,
}
struct Handler(Arc<Actions>);
impl accesskit::ActionHandler for Handler {
    fn do_action(&mut self, request: accesskit::ActionRequest) {

        self.0.actions.lock().unwrap_or_else(|p| p.into_inner()).push(request);
        self.0.repaint.request(Duration::ZERO);
    }
}
struct Activate(Arc<Actions>);
impl accesskit::ActivationHandler for Activate {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        self.0.activate.store(true, Ordering::Release);
        self.0.repaint.request(Duration::ZERO);

        None
    }
}
pub(super) struct Accessibility {
    adapter: RefCell<accesskit_windows::Adapter>,
    actions: Arc<Actions>,
}
impl Accessibility {
    pub fn new(hwnd: HWND, focused: bool, repaint: Arc<RepaintSignal>) -> Self {
        let actions = Arc::new(Actions { actions: Mutex::new(Vec::new()),
            activate: AtomicBool::new(false), repaint });

        let adapter = accesskit_windows::Adapter::new(hwnd, focused, Handler(actions.clone()));
        Self { adapter: RefCell::new(adapter), actions }
    }
    pub fn get_object(&self, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        let response = {
            let Ok(mut adapter) = self.adapter.try_borrow_mut() else { return None; };
            adapter.handle_wm_getobject(wparam, lparam, &mut Activate(self.actions.clone()))
        };

        response.map(Into::into)
    }
    pub fn before_frame(&self, context: &egui::Context, input: &InputQueue) {
        if self.actions.activate.swap(false, Ordering::AcqRel) { context.enable_accesskit(); }
        let actions = {
            let mut pending = self.actions.actions.lock().unwrap_or_else(|p| p.into_inner());
            std::mem::take(&mut *pending)
        };
        for action in actions { input.push(egui::Event::AccessKitActionRequest(action)); }
    }
    pub fn publish(&self, update: Option<accesskit::TreeUpdate>, focused: bool) {
        let (focus_events, tree_events) = {
            let mut adapter = self.adapter.borrow_mut();
            let focus_events = adapter.update_window_focus_state(focused);
            let tree_events = update.and_then(|tree| adapter.update_if_active(|| tree));
            (focus_events, tree_events)
        };

        if let Some(events) = focus_events { events.raise(); }
        if let Some(events) = tree_events { events.raise(); }
    }
}
