mod input_queue;
mod clipboard;
mod ime;
mod drop_target;
mod accessibility;
mod navigation;
mod platform;
mod gpu;
use std::{cell::{Cell, RefCell}, ffi::{c_char, c_void, CStr}, panic::{catch_unwind, AssertUnwindSafe},
    ptr, rc::Rc, time::{Duration, Instant}, thread::{self, ThreadId}};
use eframe::egui;
use windows::Win32::{Foundation::{HWND, WPARAM, LPARAM}, System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsWindow, WM_DESTROY, WM_NCDESTROY,
        WM_PAINT, WM_SETFOCUS, WM_KILLFOCUS}};
use crate::{desktop::DesktopApp, plugin_bridge::{self, RocoPlugin}, Result};
use input_queue::InputQueue;
use gpu::{Gpu, Present, SkipReason};
const MAX_STATE_BYTES: usize = 16 * 1024 * 1024;
const MAX_ERROR_BYTES: usize = 16 * 1024;
thread_local! { static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) }; }
struct FrameFp { saved: u32 }
impl FrameFp {
    fn enter() -> Self {
        let mut saved = 0u32;
        let normal = 0x1f80u32;
        unsafe {
            std::arch::asm!("stmxcsr [{p}]", p = in(reg) &mut saved, options(nostack, preserves_flags));
            std::arch::asm!("ldmxcsr [{p}]", p = in(reg) &normal, options(nostack, preserves_flags));
        }
        Self { saved }
    }
}
impl Drop for FrameFp {
    fn drop(&mut self) {
        unsafe { std::arch::asm!("ldmxcsr [{p}]", p = in(reg) &self.saved, options(nostack, preserves_flags)); }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase { Pending, Creating, Ready, Failed, Closing, Closed }

struct RestoreRequest {
    document: String,
    expected: crate::desktop::ExpectedCondition,
}

fn expected_condition(value: plugin_bridge::RocoCondition) -> Result<crate::desktop::ExpectedCondition> {
    use crate::desktop::ExpectedCondition;
    if !value.pregain_db.is_finite() || !value.pitch_semitones.is_finite()
        || value.tone.iter().chain(value.canvas.iter()).any(|v| !v.is_finite()) {
        return Err("Invalid host condition snapshot.".into());
    }
    let parameters = crate::prototype_controls::Parameters {
        pregain_db: value.pregain_db, pitch_semitones: value.pitch_semitones, tone_db: 0.0,
    };
    let result = match value.mode {
        0 => {

            if value.pregain_db.to_bits()!=0 || value.pitch_semitones.to_bits()!=0 {
                return Err("An empty saved selection with nonzero audio controls cannot be restored by this transport.".into());
            }
            ExpectedCondition::Empty
        },
        1 => ExpectedCondition::Canvas { embedding: value.canvas, parameters },
        2 => ExpectedCondition::Tone { controls: value.tone, parameters },
        3 => ExpectedCondition::Unavailable,
        _ => return Err("Unknown host condition mode.".into()),
    };
    result.validate()?;
    Ok(result)
}

struct EditorData {
    app: Option<DesktopApp>, bus: crate::current_condition::ConditionBus,
    condition: crate::current_condition::ConditionReader, condition_revision: Option<u64>,
    staged_restore: Option<RestoreRequest>, document_revision: u64, serialized_document: Option<String>,
    paths: [String; 2], multithread: bool,
    multithread_mailbox: std::sync::Arc<std::sync::atomic::AtomicU8>,
    revision: u64, accepted_error_event: u64,
}
impl EditorData {
    fn changed(&mut self) { self.revision = self.revision.wrapping_add(1).max(1); }
    fn sync_condition(&mut self) -> bool {
        let condition = self.condition.poll_shared();
        let revision = self.condition.revision();
        if self.condition_revision == Some(revision) { return false; }
        if let Some(condition) = condition {
            let Some(app) = &mut self.app else { return false; };
            if !app.restore_plugin_condition(&condition) { return false; }
        }
        self.condition_revision = Some(revision);
        true
    }
    fn accept_own_publication(&mut self) {
        if self.app.as_ref().is_some_and(DesktopApp::plugin_layout_pending) { return; }
        self.condition.poll(); self.condition_revision = Some(self.condition.revision());
    }
    fn retained_expected(&mut self) -> Result<crate::desktop::ExpectedCondition> {
        use crate::{desktop::ExpectedCondition, current_condition::{ConditionTarget, ConditionAbsence}};
        let current = self.condition.poll_shared();
        let expected = if let Some(current) = current {
            let parameters = crate::prototype_controls::Parameters {
                pregain_db: current.pregain_db, pitch_semitones: current.pitch_semitones, tone_db: 0.0,
            };
            match &current.target {
                ConditionTarget::Canvas128 { embedding, .. } => ExpectedCondition::Canvas { embedding: *embedding, parameters },
                ConditionTarget::TonePreview { controls } => ExpectedCondition::Tone { controls: *controls, parameters },
            }
        } else {
            match self.condition.absence() {
                Some(ConditionAbsence::Empty) => ExpectedCondition::Empty,
                _ => return Err("An unavailable condition is not a completed editor session.".into()),
            }
        };
        expected.validate()?;
        Ok(expected)
    }
    fn refresh(&mut self) {
        let Some(app) = &self.app else { return; };
        let paths = app.plugin_model_paths();
        let document_changed = self.document_revision != app.plugin_document_revision();
        if document_changed || self.paths[0] != paths[0] || self.paths[1] != paths[1] {
            self.document_revision = app.plugin_document_revision();
            if document_changed { self.serialized_document = None; }
            self.paths = paths.map(str::to_owned); self.changed();
        }
    }
}
struct SharedEditor {
    hwnd: HWND, owner: ThreadId, phase: Cell<Phase>, busy: Cell<bool>,
    input: Rc<InputQueue>, context: egui::Context, started: Instant,
    data: RefCell<EditorData>, error: RefCell<Option<String>>,
    session: RefCell<Option<crate::editor_session::Lease<crate::desktop::CompletedEditorSession>>>,
    platform: RefCell<Option<Rc<platform::Platform>>>, gpu: RefCell<Option<Gpu>>,
    input_meter: Cell<Option<u32>>,
    input_pitch: Cell<u64>,
}
impl SharedEditor {
    fn publish_key_owner(&self, app: &DesktopApp, installed: bool) {
        if self.cancelled() || self.input.fault().is_some()
            || matches!(self.phase.get(), Phase::Failed | Phase::Closing | Phase::Closed) {
            self.input.mute_waypoint_keys(); return;
        }
        let previous=self.input.shortcut_mask.get();
        self.input.publish_shortcuts(app.plugin_shortcut_mask(&self.context),
            app.waypoint_key_owner(&self.context), installed);
        let navigation=crate::desktop::EXPERIMENTAL_NAVIGATION_BIT;
        if previous & navigation != 0 && (installed || self.input.shortcut_mask.get() & navigation == 0) {
            match self.platform.try_borrow() {
                Ok(platform)=>{if let Some(platform)=platform.as_ref(){platform.retire_navigation_keys();}},
                Err(_)=>self.input.fail("Navigation ownership could not be retired."),
            }
        }
    }
    fn cancelled(&self) -> bool { self.input.is_cancelled() }
    fn cancel(&self) { self.input.cancel(); if self.phase.get() != Phase::Closed { self.phase.set(Phase::Closing); } }
    fn initialize(&self) -> Result<()> {
        self.phase.set(Phase::Creating);
        if !unsafe { IsWindow(Some(self.hwnd)) }.as_bool() { return Err("The host editor window no longer exists.".into()); }
        let platform = platform::Platform::new(self.hwnd, self.input.clone());
        let platform = Rc::new(platform?);
        if self.cancelled() { return Ok(()); }
        *self.platform.borrow_mut() = Some(platform);
        let gpu = Gpu::new(self.hwnd, self.input.extent(), self.input.cancelled.clone());
        let gpu = gpu?;
        if self.cancelled() { return Ok(()); }
        *self.gpu.borrow_mut() = Some(gpu);
        self.input.mute_waypoint_keys();
        let mut data = self.data.borrow_mut();
        let mut app = DesktopApp::new_plugin(&self.context, data.bus.clone());

        let configured = app.plugin_model_paths();
        let paths: [String; 2] = std::array::from_fn(|i| {
            if data.paths[i].is_empty() { configured[i].to_owned() }
            else { data.paths[i].clone() }
        });
        app.set_plugin_model_paths(paths[0].clone(), paths[1].clone());
        let restore = data.staged_restore.as_ref().ok_or("The host has not supplied a complete editor snapshot.")?;
        let retained = self.session.borrow_mut().as_mut().ok_or("Missing editor session lease.")?.take()?;
        if let Some(retained) = retained.filter(|value|value.matches(&restore.document, &paths, &restore.expected)) {
            app.adopt_plugin_session(retained, &restore.expected)?;
        } else {
            app.restore_plugin_document(&restore.document, &restore.expected)?;
        }
        app.set_plugin_multithread_mailbox(data.multithread_mailbox.clone());
        app.adopt_plugin_multithread(data.multithread);
        if self.cancelled() { return Ok(()); }
        data.app = Some(app); data.refresh();
        if let Some(app) = &data.app { self.publish_key_owner(app, true); }
        Ok(())
    }
    fn advance(&self) -> Result<i32> {
        if self.busy.replace(true) { return Ok(1); }
        struct Busy<'a>(&'a Cell<bool>);
        impl Drop for Busy<'_> { fn drop(&mut self) { self.0.set(false); } }
        let _busy = Busy(&self.busy);
        if self.phase.get() == Phase::Pending { self.initialize()?; }
        if self.cancelled() { return Ok(1); }
        if let Some(fault) = self.input.fault() { return Err(fault.into()); }
        self.input.mute_waypoint_keys();
        let condition_changed = {
            let mut data = self.data.borrow_mut();
            let changed = data.sync_condition();
            if let Some(app) = &data.app { self.publish_key_owner(app, false); }
            changed
        };
        if condition_changed { self.input.repaint.request(Duration::ZERO); }
        self.gpu.borrow().as_ref().ok_or("Editor GPU was not initialized")?.check_health()?;
        if !self.input.repaint.take_due(Instant::now()) {
            return Ok(if self.phase.get() == Phase::Ready { 0 } else { 1 });
        }
        let platform = self.platform.borrow().as_ref().cloned().ok_or("Native editor services were not initialized")?;
        platform.before_frame(&self.context)?;
        if self.cancelled() { return Ok(1); }
        let max_texture = self.gpu.borrow().as_ref().ok_or("Editor GPU was not initialized")?.max_texture_side();
        let input_queue::InputBatch { raw, pointer_cancellations, unhandled_escape_downs,
            binding_epoch, binding_mixed, navigation_tag, navigation_mixed } =
            self.input.take_input(&self.context, self.started.elapsed().as_secs_f64(), max_texture)?;
        let binding_allowed = self.input.binding_batch_allowed(binding_epoch, binding_mixed);
        let navigation_allowed=self.input.navigation_batch_allowed(navigation_tag,navigation_mixed);
        self.input.mute_waypoint_keys();
        let output = {
            let mut data = self.data.borrow_mut();
            if let Some(app)=&mut data.app {
                app.set_waypoint_key_batch(binding_allowed);
                app.set_experimental_navigation_batch(navigation_allowed);
                app.set_input_meter(self.input_meter.get().map(f32::from_bits));
                app.set_input_pitch(self.input_pitch.get());
            }
            let output = self.context.run_ui(raw, |ui| {
                if let Some(app) = &mut data.app {

                    let observations = if ui.ctx().current_pass_index() == 0 {
                        unhandled_escape_downs.as_slice()
                    } else { &[] };
                    app.show_with_pointer_cancellations(ui, &pointer_cancellations, observations);
                }
            });
            data.accept_own_publication(); data.refresh();
            if let Some(app) = &data.app { self.publish_key_owner(app, false); }
            output
        };
        if self.cancelled() { return Ok(1); }
        let egui::FullOutput { platform_output, mut textures_delta, shapes, pixels_per_point,
            viewport_output, .. } = output;
        platform.after_frame(&self.context, platform_output, pixels_per_point)?;
        if self.cancelled() { return Ok(1); }
        for (id, viewport) in &viewport_output {
            if *id != egui::ViewportId::ROOT { return Err("Plugin UI requested an unsupported additional native viewport.".into()); }
            for command in &viewport.commands {
                match command {
                    egui::ViewportCommand::SetTheme(_) => {

                    }
                    egui::ViewportCommand::Close | egui::ViewportCommand::CancelClose => {},
                    egui::ViewportCommand::RequestCut => { self.input.push(egui::Event::Cut); }
                    egui::ViewportCommand::RequestCopy => { self.input.push(egui::Event::Copy); }
                    egui::ViewportCommand::RequestPaste => { self.input.paste_requested.set(true); }
                    other => return Err(format!("Unsupported plugin viewport request: {other:?}")),
                }
            }
            self.input.repaint.request(viewport.repaint_delay);
        }
        if self.cancelled() { return Ok(1); }
        let primitives = self.context.tessellate(shapes, pixels_per_point);
        let mut gpu = self.gpu.borrow_mut();
        let present = gpu.as_mut().ok_or("Editor GPU was not initialized")?.render(&self.context,
            self.input.extent(), pixels_per_point, &primitives, &mut textures_delta)?;
        if self.cancelled() { return Ok(1); }
        match present {
            Present::Presented => self.phase.set(Phase::Ready),
            Present::Skipped(SkipReason::Timeout | SkipReason::Reconfigured) => self.input.repaint.request(Duration::from_millis(16)),
            Present::Skipped(SkipReason::Occluded | SkipReason::ZeroExtent) => {},
        }
        Ok(if self.phase.get() == Phase::Ready { 0 } else { 1 })
    }
    fn retire(&self) {

        let completed = self.error.borrow().is_none();
        self.cancel();

        self.platform.borrow_mut().take();
        self.gpu.borrow_mut().take();
        let retained = {
            let mut data = self.data.borrow_mut();
            let expected = if completed { data.retained_expected().ok() } else { None };
            let retained = expected.and_then(|expected|
                data.app.as_mut().and_then(|app|app.detach_plugin_session(&expected).ok()));
            data.app.take();
            retained
        };
        if let Some(lease) = self.session.borrow_mut().take() { lease.finish(retained); }
        self.phase.set(Phase::Closed);
    }
    fn fail(&self, error: String) {
        if self.cancelled() { return; }
        *self.error.borrow_mut() = Some(error);
        self.input.cancel();
        self.phase.set(Phase::Failed);

    }
}
pub struct RocoEditor { shared: Rc<SharedEditor> }
unsafe fn editor<'a>(handle: *const RocoEditor) -> Result<&'a RocoEditor> {
    let editor = unsafe { handle.as_ref() }.ok_or("Missing editor handle")?;
    if editor.shared.owner != thread::current().id() { return Err("Use the editor on its attaching UI thread.".into()); }
    Ok(editor)
}
fn available(editor: &RocoEditor) -> Result<()> {
    if matches!(editor.shared.phase.get(), Phase::Failed | Phase::Closing | Phase::Closed) {
        Err(editor.shared.error.borrow().clone().unwrap_or_else(|| "The editor is closed.".into()))
    } else { Ok(()) }
}
fn call<T>(failure: T, body: impl FnOnce() -> Result<T>) -> T {
    let _fp = FrameFp::enter();
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => { LAST_ERROR.with(|last| *last.borrow_mut() = error); failure }
        Err(_) => { LAST_ERROR.with(|last| *last.borrow_mut() = "Plugin editor callback panicked.".into()); failure }
    }
}
unsafe fn copy_text(text: &str, buffer: *mut c_char, capacity: usize) -> usize {
    let needed = text.len().saturating_add(1);
    if !buffer.is_null() && capacity >= needed {
        unsafe { ptr::copy_nonoverlapping(text.as_ptr(), buffer.cast::<u8>(), text.len()); *buffer.add(text.len()) = 0; }
    }
    needed
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_open(plugin: *mut RocoPlugin, parent: *mut c_void, width: u32, height: u32) -> *mut RocoEditor {
    call(ptr::null_mut(), || {
        if width == 0 || height == 0 || width > 8192 || height > 8192 { return Err("Invalid editor dimensions.".into()); }
        let hwnd = HWND(parent);
        if parent.is_null() || !unsafe { IsWindow(Some(hwnd)) }.as_bool() { return Err("A live host child window is required.".into()); }
        let ui_thread = unsafe { GetCurrentThreadId() };
        if unsafe { GetWindowThreadProcessId(hwnd, None) } != ui_thread {
            return Err("Open the editor on the host child's UI thread.".into());
        }
        let bus = unsafe { plugin_bridge::condition_bus(plugin) }?;
        let store = unsafe { plugin_bridge::editor_session_store(plugin) }?;
        let multithread_mailbox = unsafe { plugin_bridge::multithread_mailbox(plugin) }?;
        let lease = store.claim()?;
        let input = Rc::new(InputQueue::new([width, height], 1.0));
        let context = egui::Context::default();
        let repaint = input.repaint.clone();
        context.set_request_repaint_callback(move |request| repaint.request(request.delay));
        let shared = Rc::new(SharedEditor { hwnd, owner: thread::current().id(), phase: Cell::new(Phase::Pending),
            busy: Cell::new(false), input, context, started: Instant::now(),
            session: RefCell::new(Some(lease)),
            data: RefCell::new(EditorData { app: None, condition: bus.reader(), bus, condition_revision: None,
                staged_restore: None, document_revision: 0, serialized_document: None,
                paths: Default::default(), multithread: true,
                multithread_mailbox, revision: 1, accepted_error_event: 0 }),
            error: RefCell::new(None), platform: RefCell::new(None), gpu: RefCell::new(None), input_meter: Cell::new(None), input_pitch: Cell::new(0) });
        Ok(Box::into_raw(Box::new(RocoEditor { shared })))
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_idle(handle: *mut RocoEditor) -> i32 {
    call(-2, || {
        let editor = unsafe { editor(handle) }?;
        available(editor)?;
        match catch_unwind(AssertUnwindSafe(|| editor.shared.advance())) {
            Ok(Ok(_)) if editor.shared.phase.get() == Phase::Failed => available(editor).map(|_| 1),
            Ok(Ok(result)) => Ok(result),
            Ok(Err(error)) if !editor.shared.cancelled() => {
                editor.shared.fail(error);
                available(editor).map(|_| 1)
            }
            Err(_) if !editor.shared.cancelled() => {
                let error = "The embedded editor frame panicked.".to_owned();
                editor.shared.fail(error); available(editor).map(|_| 1)
            }
            _ => Ok(1),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_set_input_pitch(handle:*mut RocoEditor,word:u64)->i32 {
    call(-2,|| {
        let editor=unsafe {editor(handle)}?;
        if editor.shared.cancelled() {return Ok(0);}
        if !crate::pitch_display::valid(word) {return Err("Invalid pitch observation.".into());}
        if editor.shared.input_pitch.replace(word)!=word {editor.shared.input.repaint.request(Duration::ZERO);}
        Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_set_input_meter(handle:*mut RocoEditor,present:u32,envelope:f32)->i32 {
    call(-2,|| {
        let editor=unsafe {editor(handle)}?;
        if editor.shared.cancelled() || matches!(editor.shared.phase.get(),Phase::Failed|Phase::Closing|Phase::Closed) {

            return Ok(0);
        }
        if present>1 || !envelope.is_finite() || envelope<0.
            || (present==0 && envelope.to_bits()!=0) {
            return Err("Invalid input meter value.".into());
        }
        let next=if present==1 {Some(envelope.to_bits())} else {None};
        if editor.shared.input_meter.replace(next)!=next {
            editor.shared.input.repaint.request(Duration::ZERO);
        }
        Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_window_message(handle: *mut RocoEditor, message: u32,
    wparam: usize, lparam: isize, result: *mut isize) -> i32
{
    call(-2, || {
        let editor = unsafe { editor(handle) }?;
        if result.is_null() { return Err("Missing native result output.".into()); }
        unsafe { *result = 0; }
        if message == WM_DESTROY || message == WM_NCDESTROY { editor.shared.cancel(); return Ok(1); }
        if editor.shared.cancelled() { return Ok(0); }
        if message == WM_PAINT { editor.shared.input.repaint.request(Duration::ZERO); return Ok(1); }
        let platform = editor.shared.platform.try_borrow().ok().and_then(|platform| platform.as_ref().cloned());
        if let Some(platform) = platform {
            match platform.message(message, WPARAM(wparam), LPARAM(lparam)) {
                Ok(Some(value)) => { unsafe { *result = value.0; } Ok(1) }
                Ok(None) => Ok(0),
                Err(error) => { editor.shared.input.fail("Native editor input failed; see plugin error.");
                    editor.shared.fail(error.clone()); Err(error) }
            }
        } else {
            if message == WM_SETFOCUS { editor.shared.input.set_focus(true); }
            else if message == WM_KILLFOCUS { editor.shared.input.set_focus(false); }
            Ok(0)
        }
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_host_key(handle: *mut RocoEditor, unicode: u16,
    code: i16, modifiers: i16, down: u32) -> i32
{
    call(-2, || {
        let editor = unsafe { editor(handle) }?;
        if down > 1 { return Err("Invalid host key direction.".into()); }
        if editor.shared.cancelled() { return Ok(0); }
        let platform = editor.shared.platform.try_borrow().ok().and_then(|platform| platform.as_ref().cloned());
        match platform { Some(platform) => platform.host_key(unicode, code, modifiers, down != 0).map(i32::from), None => Ok(0) }
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_set_content_scale(handle: *mut RocoEditor, factor: f32) -> i32 {
    call(-2, || {
        let editor = unsafe { editor(handle) }?;
        if editor.shared.cancelled()
            || matches!(editor.shared.phase.get(), Phase::Failed | Phase::Closing | Phase::Closed)
        { return Err("The editor cannot accept a content scale after retirement.".into()); }

        editor.shared.input.set_content_scale(factor)?;
        Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_resize(handle: *mut RocoEditor, width: u32, height: u32) -> i32 {
    call(-2, || {
        let editor = unsafe { editor(handle) }?; available(editor)?;
        if width == 0 || height == 0 || width > 8192 || height > 8192 { return Err("Invalid editor dimensions.".into()); }
        editor.shared.input.repaint.request(Duration::ZERO);

        Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_cancel(handle: *mut RocoEditor) -> i32 {
    let Some(editor) = (unsafe { handle.as_ref() }) else { return -2; };
    if editor.shared.owner != thread::current().id() { return -2; }
    editor.shared.cancel(); 0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_close(handle: *mut RocoEditor) {
    call((), || {
        let editor = unsafe { editor(handle) }?;
        editor.shared.cancel();
        if editor.shared.busy.get() { return Err("Close must wait for the active editor call to return.".into()); }
        editor.shared.retire();
        unsafe { drop(Box::from_raw(handle)); }
        Ok(())
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_state_revision(handle: *const RocoEditor) -> u64 {
    call(0, || { let editor = unsafe { editor(handle) }?; available(editor)?;
        let Ok(mut data) = editor.shared.data.try_borrow_mut() else { return Ok(0); };
        data.refresh(); Ok(data.revision) })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_get_state(handle: *const RocoEditor, buffer: *mut c_char, capacity: usize) -> usize {
    call(0, || {
        let editor = unsafe { editor(handle) }?; available(editor)?;
        let mut data = editor.shared.data.try_borrow_mut().map_err(|_| "Editor state is busy")?;
        data.refresh();
        if data.serialized_document.is_none() { data.serialized_document = Some(match &data.app {
            Some(app) => app.plugin_document()?, None => data.staged_restore.as_ref().map(|r| r.document.clone()).unwrap_or_default() }); }
        Ok(unsafe { copy_text(data.serialized_document.as_deref().expect("serialized state"), buffer, capacity) })
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_set_multithread(handle:*mut RocoEditor, value:u32)->i32 {
    call(-2, || {
        let editor=unsafe {editor(handle)}?; available(editor)?;
        if value>1 {return Err("Invalid multithread preference observation.".into());}
        let Ok(mut data)=editor.shared.data.try_borrow_mut() else {return Ok(2);};
        let next=value!=0; let changed=data.multithread!=next;
        data.multithread=next;
        if let Some(app)=&mut data.app {app.adopt_plugin_multithread(next);}
        if changed {editor.shared.input.repaint.request(Duration::ZERO);}
        Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_get_model_path(handle: *const RocoEditor, which: u32, buffer: *mut c_char, capacity: usize) -> usize {
    call(0, || { let editor = unsafe { editor(handle) }?; available(editor)?;
        let data = editor.shared.data.try_borrow().map_err(|_| "Editor state is busy")?;
        Ok(unsafe { copy_text(data.paths.get(which as usize).ok_or("Unknown model path index")?, buffer, capacity) }) })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_set_state(handle: *mut RocoEditor, state: *const c_char, length: usize,
    condition: *const plugin_bridge::RocoCondition) -> i32 {
    call(-2, || { let editor = unsafe { editor(handle) }?; available(editor)?;
        if (state.is_null() && length != 0) || length > MAX_STATE_BYTES { return Err("Invalid preset state buffer.".into()); }
        let bytes = if length == 0 { &[] } else { unsafe { std::slice::from_raw_parts(state.cast::<u8>(), length) } };
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;

        let value = *unsafe { condition.as_ref() }.ok_or("Missing host condition snapshot.")?;
        let expected = expected_condition(value)?;
        let generation = editor.shared.session.borrow().as_ref()
            .ok_or("Missing editor session lease.")?.current_generation()?;
        let same_generation = editor.shared.session.borrow().as_ref()
            .ok_or("Missing editor session lease.")?.bound_generation() == generation;
        let Ok(mut data) = editor.shared.data.try_borrow_mut() else { return Ok(2); };
        if let Some(app) = &mut data.app {
            editor.shared.input.repaint.request(Duration::ZERO);
            editor.shared.input.mute_waypoint_keys();
            let mut installed = false;
            let result = (|| -> Result<()> {
                if !same_generation || !app.merge_plugin_session_controls(text, &expected)? {
                    app.restore_plugin_document(text, &expected)?;
                    installed = true;
                }
                Ok(())
            })();

            editor.shared.publish_key_owner(app, installed);
            result?;
        }
        else if !text.is_empty() { crate::preset::Preset::parse(serde_json::from_str(text).map_err(|e| format!("Invalid preset state: {e}"))?)?; }
        let mut lease = editor.shared.session.borrow_mut();
        lease.as_mut().ok_or("Missing editor session lease.")?.accept_generation(generation)?;
        data.staged_restore = Some(RestoreRequest { document: text.to_owned(), expected });
        data.serialized_document = None;
        data.condition_revision = None; data.changed();
        editor.shared.input.repaint.request(Duration::ZERO); Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_set_model_paths(handle: *mut RocoEditor, analysis: *const c_char, userdata: *const c_char) -> i32 {
    call(-2, || { let editor = unsafe { editor(handle) }?; available(editor)?;
        if analysis.is_null() || userdata.is_null() { return Err("Missing model path string.".into()); }
        let paths = [unsafe { CStr::from_ptr(analysis) }.to_str().map_err(|e| e.to_string())?.to_owned(),
            unsafe { CStr::from_ptr(userdata) }.to_str().map_err(|e| e.to_string())?.to_owned()];
        let Ok(mut data) = editor.shared.data.try_borrow_mut() else { return Ok(2); };
        if data.paths != paths { if let Some(app) = &mut data.app {
                editor.shared.input.mute_waypoint_keys();
                app.set_plugin_model_paths(paths[0].clone(), paths[1].clone());
                editor.shared.publish_key_owner(app, false);
            }
            data.paths = paths; data.changed(); editor.shared.input.repaint.request(Duration::ZERO); }
        Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_report_error(handle: *mut RocoEditor, event_id: u64, message: *const c_char, length: usize) -> i32 {
    call(-2, || { let editor = unsafe { editor(handle) }?; available(editor)?;
        if event_id == 0 { return Err("An error event requires a nonzero identity.".into()); }
        let Ok(mut data) = editor.shared.data.try_borrow_mut() else { return Ok(2); };
        if event_id <= data.accepted_error_event { return Ok(0); }
        if editor.shared.phase.get() != Phase::Ready { return Ok(2); }
        if message.is_null() || length == 0 || length > MAX_ERROR_BYTES { return Err("Invalid plugin error message buffer.".into()); }
        let text = std::str::from_utf8(unsafe { std::slice::from_raw_parts(message.cast::<u8>(), length) }).map_err(|e| e.to_string())?;
        editor.shared.input.mute_waypoint_keys();
        let app = data.app.as_mut().ok_or("Editor is not initialized")?;
        let accepted = app.try_report_plugin_error(text);
        editor.shared.publish_key_owner(app, false);
        if !accepted { return Ok(2); }
        data.accepted_error_event = event_id; drop(data);
        editor.shared.input.repaint.request(Duration::ZERO); Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_editor_last_error(buffer: *mut c_char, capacity: usize) -> usize {
    LAST_ERROR.with(|last| unsafe { copy_text(&last.borrow(), buffer, capacity) })
}
