//! Windows MIDI 1.0 input. Each worker owns a WinMM thread-message endpoint.
//! No app pointer crosses into a callback and no MIDI call runs on the audio thread.
use serde::{Deserialize, Serialize};
use super::midi_match::{Gate, MidiBinding};
use super::input_publication::Publication;
use std::{sync::{Arc, mpsc, atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering}}, thread, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DeviceKey { pub name: String, pub manufacturer: u16, pub product: u16, pub version: u32 }
#[derive(Clone)]
pub(super) struct Device { pub id: u32, pub key: DeviceKey }
#[repr(C)]
struct Caps { manufacturer: u16, product: u16, version: u32, name: [u16; 32], support: u32 }
#[repr(C)]
#[derive(Default)]
struct WinMessage { hwnd: isize, message: u32, wparam: usize, lparam: isize,
    time: u32, x: i32, y: i32, private: u32 }
#[link(name="winmm")]
unsafe extern "system" {
    fn midiInGetNumDevs() -> u32;
    fn midiInGetDevCapsW(id: usize, caps: *mut Caps, size: u32) -> u32;
    fn midiInOpen(handle: *mut usize, id: u32, callback: usize, instance: usize, flags: u32) -> u32;
    fn midiInStart(handle: usize) -> u32;
    fn midiInStop(handle: usize) -> u32;
    fn midiInClose(handle: usize) -> u32;
}
#[link(name="user32")]
unsafe extern "system" {
    fn PeekMessageW(msg: *mut WinMessage, hwnd: isize, min: u32, max: u32, remove: u32) -> i32;
    fn MsgWaitForMultipleObjectsEx(count: u32, handles: *const usize, timeout: u32, mask: u32, flags: u32) -> u32;
}
#[link(name="kernel32")]
unsafe extern "system" {
    fn GetCurrentThreadId() -> u32;
    fn CreateEventW(attributes: *const std::ffi::c_void, manual: i32, initial: i32, name: *const u16) -> usize;
    fn SetEvent(event: usize) -> i32;
    fn CloseHandle(handle: usize) -> i32;
    fn WaitForSingleObject(handle: usize, timeout: u32) -> u32;
}
const CALLBACK_THREAD: u32 = 0x0002_0000;
const MM_MIM_DATA: u32 = 0x3c3;
const PM_REMOVE: u32 = 1;
const QUEUE_LIMIT: usize = 128;

fn caps_key(id_or_handle: usize) -> Option<DeviceKey> {
    let mut caps = Caps { manufacturer: 0, product: 0, version: 0, name: [0;32], support: 0 };
    if unsafe { midiInGetDevCapsW(id_or_handle, &mut caps, std::mem::size_of::<Caps>() as u32) } != 0 { return None; }
    let length = caps.name.iter().position(|&c| c == 0).unwrap_or(caps.name.len());
    Some(DeviceKey { name: String::from_utf16_lossy(&caps.name[..length]), manufacturer: caps.manufacturer,
        product: caps.product, version: caps.version })
}
pub(super) fn catalog() -> Vec<Device> {
    let mut devices = Vec::new();
    let count = unsafe { midiInGetNumDevs() };
    for id in 0..count {
        if let Some(key) = caps_key(id as usize) { devices.push(Device { id, key }); }
    }
    devices
}
fn unique_id(key: &DeviceKey) -> std::result::Result<u32, String> {
    let devices = catalog();
    let mut matches = devices.iter().filter(|device| &device.key == key);
    let Some(device) = matches.next() else { return Err("The selected MIDI device is unavailable.".into()); };
    if matches.next().is_some() { return Err("Multiple MIDI devices have the same identity; disconnect duplicates before selecting one.".into()); }
    Ok(device.id)
}

#[derive(Clone, Copy)]
pub(super) struct Input { pub packed: u32, pub epoch: u64, pub trigger: Option<MidiBinding> }
pub(super) enum Event { Opened(std::result::Result<(), String>), Failed(String), Data(Input) }
struct StopSignal(usize);
impl Drop for StopSignal { fn drop(&mut self) { unsafe { CloseHandle(self.0); } } }
struct InputHandle { handle: usize, close_error: Arc<AtomicU32>, wake: Arc<dyn Fn() + Send + Sync> }
impl Drop for InputHandle {
    fn drop(&mut self) {
        unsafe { midiInStop(self.handle); }
        let mut msg = WinMessage::default();
        // Keep the owning thread and handle alive until native close succeeds.
        loop {
            let result = unsafe { midiInClose(self.handle) };
            if result == 0 { break; }
            if self.close_error.swap(result, Ordering::AcqRel) != result { notify(&self.wake); }
            for _ in 0..256 { if unsafe { PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) } == 0 { break; } }
            thread::sleep(Duration::from_millis(250));
        }
    }
}
fn notify(wake: &Arc<dyn Fn() + Send + Sync>) {
    // A UI repaint failure must not unwind past a live native input handle.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| wake()));
}
struct WakeOnExit { wake: Arc<dyn Fn() + Send + Sync>, terminal: Arc<AtomicBool> }
impl Drop for WakeOnExit {
    fn drop(&mut self) { self.terminal.store(true,Ordering::Release); notify(&self.wake); }
}
pub(super) struct Worker { stop: Arc<StopSignal>, scope_signal: Arc<StopSignal>, pub events: mpsc::Receiver<Event>,
    pub publication: Arc<Publication>, pub failed: Arc<AtomicBool>, pub terminal: Arc<AtomicBool>,
    pub close_error: Arc<AtomicU32>, epoch: Arc<AtomicU64>,
    ready_epoch: Arc<AtomicU64>, _thread: thread::JoinHandle<()> }
impl Worker {
    #[allow(dead_code)]
    pub fn stop(&self) { unsafe { SetEvent(self.stop.0); } }
    #[allow(dead_code)]
    pub fn finished(&self) -> bool { self._thread.is_finished() }
    pub fn set_epoch(&self, value: u64) {
        if self.epoch.swap(value, Ordering::AcqRel) != value { unsafe { SetEvent(self.scope_signal.0); } }
    }
    #[allow(dead_code)]
    pub fn ready_epoch(&self) -> u64 { self.ready_epoch.load(Ordering::Acquire) }
}
impl Drop for Worker { fn drop(&mut self) { self.stop(); } }

pub(super) fn worker(key: DeviceKey, wake: Arc<dyn Fn() + Send + Sync>) -> std::result::Result<Worker, String> {
    let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
    if event == 0 { return Err("Could not create the MIDI shutdown event.".into()); }
    let stop = Arc::new(StopSignal(event));
    let scope_event = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
    if scope_event == 0 { return Err("Could not create the MIDI scope event.".into()); }
    let scope_signal = Arc::new(StopSignal(scope_event));
    let stop_thread = stop.clone();
    let scope_thread = scope_signal.clone();
    let (events_tx, events) = mpsc::sync_channel(QUEUE_LIMIT);
    let publication = Arc::new(Publication::default());
    let publication_thread = publication.clone();
    let failed = Arc::new(AtomicBool::new(false));
    let failed_thread = failed.clone();
    let terminal = Arc::new(AtomicBool::new(false));
    let terminal_thread = terminal.clone();
    let close_error = Arc::new(AtomicU32::new(0));
    let close_error_thread = close_error.clone();
    let epoch = Arc::new(AtomicU64::new(0));
    let epoch_thread = epoch.clone();
    let ready_epoch = Arc::new(AtomicU64::new(0));
    let ready_thread = ready_epoch.clone();
    let handle = thread::Builder::new().name("rocoflex-midi-input".into()).spawn(move || {
        let _exit_wake = WakeOnExit { wake: wake.clone(), terminal: terminal_thread };
        let mut msg = WinMessage::default();
        // Force creation of the message queue before midiInOpen posts to it.
        unsafe { PeekMessageW(&mut msg, 0, 0, 0, 0); }
        let id = match unique_id(&key) {
            Ok(id) => id,
            Err(error) => { let _ = events_tx.try_send(Event::Opened(Err(error))); notify(&wake); return; }
        };
        let mut handle = 0usize;
        let opened = unsafe { midiInOpen(&mut handle, id, GetCurrentThreadId() as usize, 0, CALLBACK_THREAD) };
        if opened != 0 {
            let _ = events_tx.try_send(Event::Opened(Err(format!("MIDI device open failed (Windows error {opened})."))));
            notify(&wake); return;
        }
        let _handle_owner = InputHandle { handle, close_error: close_error_thread, wake: wake.clone() };
        // IDs may move as devices are plugged/unplugged. Check the opened
        // handle and refuse an identity that changed during open.
        let started = if caps_key(handle).as_ref() == Some(&key) { unsafe { midiInStart(handle) } }
            else { u32::MAX };
        if started == 0 { let _ = events_tx.try_send(Event::Opened(Ok(()))); }
        else { let _ = events_tx.try_send(Event::Opened(Err(format!("MIDI device start failed (Windows error {started}).")))); }
        notify(&wake);
        if started == 0 {
            let mut physical = Gate::default();
            let handles = [stop_thread.0, scope_thread.0];
            'worker: loop {
                let wait = unsafe { MsgWaitForMultipleObjectsEx(2, handles.as_ptr(), u32::MAX, 0x04ff, 0x0004) };
                if wait == 0 { break; }
                if wait != 1 && wait != 2 {
                    failed_thread.store(true,Ordering::Release);
                    let _ = events_tx.try_send(Event::Failed("Windows MIDI message wait failed.".into()));
                    notify(&wake); break;
                }
                if wait == 1 || ready_thread.load(Ordering::Acquire) != epoch_thread.load(Ordering::Acquire) {
                    // Retire messages queued under the old owner before acknowledging
                    // a new one. Shutdown retains priority even under device flooding.
                    loop {
                        let requested = epoch_thread.load(Ordering::Acquire);
                        let mut empty = false;
                        for _ in 0..256 {
                            if unsafe { PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) } == 0 { empty = true; break; }
                            if msg.message == MM_MIM_DATA && msg.wparam == handle {
                                match publication_thread.worker() {
                                    Ok(_guard) => { physical.input(msg.lparam as u32); }
                                    Err(_) => { failed_thread.store(true,Ordering::Release); notify(&wake); break 'worker; }
                                }
                            }
                        }
                        if unsafe { WaitForSingleObject(stop_thread.0, 0) } == 0 { break 'worker; }
                        if empty && epoch_thread.load(Ordering::Acquire) == requested {
                            ready_thread.store(requested, Ordering::Release);
                            notify(&wake); break;
                        }
                    }
                    continue;
                }
                for _ in 0..256 {
                    if ready_thread.load(Ordering::Acquire) != epoch_thread.load(Ordering::Acquire) { break; }
                    if unsafe { PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) } == 0 { break; }
                    if msg.message == MM_MIM_DATA && msg.wparam == handle {
                        // Physical state survives document-owner changes, including
                        // releases drained during a scope handoff or queue overflow.
                        let mut guard = match publication_thread.worker() {
                            Ok(guard) => guard,
                            Err(_) => { failed_thread.store(true,Ordering::Release); notify(&wake); break 'worker; }
                        };
                        let packed = msg.lparam as u32;
                        let trigger = physical.input(packed);
                        let mut repaint = false;
                        let active_epoch = ready_thread.load(Ordering::Acquire);
                        if !*guard && active_epoch != 0 {
                            let value = Input { packed, epoch: active_epoch, trigger };
                            if events_tx.try_send(Event::Data(value)).is_err() { *guard = true; }
                            repaint = true;
                        }
                        drop(guard);
                        if repaint { notify(&wake); }
                    }
                }
            }
        }
    }).map_err(|e| format!("Could not start MIDI input: {e}"))?;
    Ok(Worker { stop, scope_signal, events, publication, failed, terminal, close_error, epoch, ready_epoch, _thread: handle })
}
