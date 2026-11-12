//! Windows MIDI device selection and editor-side delivery.
use super::*;
use super::midi_port::{self, Device, DeviceKey, Event, Input, Worker};
use std::{collections::VecDeque, sync::{Arc, atomic::Ordering}, thread};

const OPEN_ERROR: u8 = 1;
const SYNC_ERROR: u8 = 2;
const OVERFLOW_ERROR: u8 = 4;
const STOP_ERROR: u8 = 8;
const CLOSE_ERROR: u8 = 16;
const CATALOG_ERROR: u8 = 32;
const MAX_PENDING_ERRORS: usize = 6;

fn enqueue(pending: &mut VecDeque<(u8, String)>, flag: u8, message: String) {
    if pending.len() < MAX_PENDING_ERRORS && !pending.iter().any(|(kind, _)| *kind == flag) {
        pending.push_back((flag, message));
    }
}

fn report_once(flags: &mut u8, pending: &mut VecDeque<(u8, String)>, flag: u8, message: String) {
    if *flags & flag == 0 {
        *flags |= flag;
        enqueue(pending, flag, message);
    }
}

pub(super) struct State {
    pub selected: Option<DeviceKey>,
    devices: Option<Vec<Device>>,
    query: Option<thread::JoinHandle<Vec<Device>>>,
    catalog_attempted: bool,
    catalog_error: Option<String>,
    worker: Option<Worker>,
    closing: Option<Worker>,
    pending_start: bool,
    opening: bool,
    opened: bool,
    attempted: Option<DeviceKey>,
    pub status: String,
    pending_errors: VecDeque<(u8, String)>,
    reported_errors: u8,
    scope: Option<(u64, u64, u8)>,
    epoch: u64,
    reset_held: bool,
}
impl Default for State {
    fn default() -> Self { Self { selected: None, devices: None, query: None,
        catalog_attempted: false, catalog_error: None, worker: None,
        closing: None, pending_start: false, opening: false, opened: false, attempted: None,
        status: String::new(), pending_errors: VecDeque::new(), reported_errors: 0,
        scope: None, epoch: 1, reset_held: false } }
}
impl State {
    pub fn decode_preference(value: &Value) -> Result<Option<DeviceKey>> {
        let selected: Option<DeviceKey> = serde_json::from_value(value.clone())
            .map_err(|error| format!("Invalid MIDI input selection: {error}"))?;
        if selected.as_ref().is_some_and(|key| key.name.is_empty()) { return Err("Invalid MIDI input device name.".into()); }
        Ok(selected)
    }
    pub fn preference(&self) -> Value { json!(self.selected) }
    pub fn take_reset(&mut self) -> bool { std::mem::take(&mut self.reset_held) }
    pub fn take_error(&mut self) -> Option<String> { self.pending_errors.pop_front().map(|(_, message)| message) }
    fn refresh(&mut self, ctx: &egui::Context) {
        if self.query.is_none() {
            self.catalog_attempted = true;
            match thread::Builder::new().name("rocoflex-midi-list".into()).spawn(midi_port::catalog) {
                Ok(job) => { self.query = Some(job); self.catalog_error = None; ctx.request_repaint_after(Duration::from_millis(50)); }
                Err(error) => {
                    let message = format!("Could not list MIDI devices: {error}");
                    self.catalog_error = Some(message.clone());
                    enqueue(&mut self.pending_errors, CATALOG_ERROR, message);
                    ctx.request_repaint();
                }
            }
        }
    }
    pub fn reconcile(&mut self, ctx: &egui::Context, plugin: bool, scope: (u64,u64,u8)) -> Vec<Input> {
        if self.scope != Some(scope) {
            self.scope = Some(scope); self.epoch = self.epoch.checked_add(1).unwrap_or(0); self.reset_held = true;
        }
        if plugin {
            if let Some(worker) = self.worker.take() { worker.stop(); self.closing = Some(worker); }
            self.attempted = None; self.pending_start = false;
            return Vec::new();
        }
        if self.attempted != self.selected {
            if let Some(worker) = self.worker.take() { worker.stop(); self.closing = Some(worker); }
            self.attempted = self.selected.clone(); self.pending_start = self.selected.is_some(); self.opening = false; self.opened = false;
            self.reset_held = true; self.status.clear();
        }
        if self.closing.as_ref().is_some_and(Worker::finished) {
            self.closing = None; self.status.clear();
        }
        if let Some(closing) = &self.closing {
            let error = closing.close_error.load(Ordering::Acquire);
            self.status = if error == 0 { "Disconnecting…".into() }
                else { format!("MIDI close is retrying (Windows error {error}).") };
            if error != 0 {
                report_once(&mut self.reported_errors, &mut self.pending_errors, CLOSE_ERROR, self.status.clone());
            }
            ctx.request_repaint_after(Duration::from_millis(30));
        } else if self.pending_start {
            self.pending_start = false;
            if let Some(key) = self.selected.clone() {
                self.reported_errors = 0;
                let repaint_ctx = ctx.clone();
                match midi_port::worker(key, Arc::new(move || repaint_ctx.request_repaint())) { Ok(worker) => { self.worker = Some(worker); self.opening = true; self.status = "Connecting…".into(); },
                    Err(error) => { self.status = error.clone(); report_once(&mut self.reported_errors, &mut self.pending_errors, OPEN_ERROR, error); } }
            }
        }
        if self.query.as_ref().is_some_and(thread::JoinHandle::is_finished) {
            if let Some(job) = self.query.take() {
                match job.join() {
                    Ok(devices) => { self.devices = Some(devices); self.catalog_error = None; }
                    Err(_) => {
                        let message = "MIDI device listing stopped unexpectedly.".to_owned();
                        self.catalog_error = Some(message.clone());
                        enqueue(&mut self.pending_errors, CATALOG_ERROR, message);
                    }
                }
            }
            ctx.request_repaint();
        }
        if self.query.is_some() { ctx.request_repaint_after(Duration::from_millis(50)); }
        let Some(worker) = self.worker.as_ref() else { return Vec::new(); };
        worker.set_epoch(if scope.2 == 0 { 0 } else { self.epoch });
        let mut inputs = Vec::new();
        let guard = match worker.publication.ui() {
            Ok(Some(guard)) => guard,
            Ok(None) => { ctx.request_repaint(); return inputs; }
            Err(()) => {
                self.status = "MIDI input synchronization failed.".into();
                report_once(&mut self.reported_errors, &mut self.pending_errors, SYNC_ERROR, self.status.clone());
                let close_error = worker.close_error.load(Ordering::Acquire);
                if close_error != 0 {
                    report_once(&mut self.reported_errors, &mut self.pending_errors, CLOSE_ERROR,
                        format!("MIDI close is retrying (Windows error {close_error})."));
                }
                return inputs;
            }
        };
        let mut processed = 0;
        for _ in 0..128 {
            let event = match worker.events.try_recv() {
                Ok(event) => event,
                Err(_) => break,
            };
            processed += 1;
            match event {
                Event::Opened(Ok(())) => { self.opening = false; self.opened = true; self.status = "Connected".into(); }
                Event::Opened(Err(error)) => {
                    self.opening = false; self.opened = false; self.status = error.clone();
                    report_once(&mut self.reported_errors, &mut self.pending_errors, OPEN_ERROR, error);
                }
                Event::Failed(error) => {
                    self.status = error.clone();
                    report_once(&mut self.reported_errors, &mut self.pending_errors, STOP_ERROR, error);
                }
                Event::Data(data) if self.epoch != 0 && data.epoch == self.epoch => inputs.push(data),
                Event::Data(_) => {},
            }
        }
        if *guard {
            inputs.clear(); self.reset_held = true;
            self.status = "MIDI input overflow.".into();
            report_once(&mut self.reported_errors, &mut self.pending_errors, OVERFLOW_ERROR, self.status.clone());
        }
        drop(guard);
        if worker.failed.load(Ordering::Acquire) {
            inputs.clear(); self.status = "MIDI input stopped unexpectedly.".into();
            report_once(&mut self.reported_errors, &mut self.pending_errors, STOP_ERROR, self.status.clone());
        } else if (worker.terminal.load(Ordering::Acquire) || worker.finished()) && (self.opened || self.opening) {
            inputs.clear(); self.opened = false; self.opening = false;
            self.status = "MIDI input stopped unexpectedly.".into();
            report_once(&mut self.reported_errors, &mut self.pending_errors, STOP_ERROR, self.status.clone());
        }
        let close_error = worker.close_error.load(Ordering::Acquire);
        if close_error != 0 {
            inputs.clear();
            if self.opened || self.opening { self.status = "MIDI input stopped; driver close is retrying.".into(); }
            report_once(&mut self.reported_errors, &mut self.pending_errors, CLOSE_ERROR,
                format!("MIDI close is retrying (Windows error {close_error})."));
        }
        if processed == 128 { ctx.request_repaint(); }
        inputs
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, width: f32) {
        use super::controls::{settings_row, settings_value_rect, settings_option, settings_option_raw,
            settings_reference_combo_icon, settings_reference_text};
        if !self.catalog_attempted { self.refresh(ui.ctx()); }
        let row = settings_row(ui, width, "MIDI Control Device");
        let value = settings_value_rect(row);
        let text = self.selected.as_ref().map_or_else(|| localization::tr(ui.ctx(), "None"), |key| key.name.as_str());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(value));
        child.spacing_mut().interact_size.y = 24.0;
        let device_list = egui::ComboBox::from_id_salt("midi-input-device").width(value.width()).height(200.0)
            .truncate().icon(settings_reference_combo_icon)
            .selected_text(settings_reference_text(text.to_owned(), child.is_enabled()))
            .show_ui(&mut child, |ui| {
                if settings_option(ui, "None", self.selected.is_none()).clicked() && ui.ctx().current_pass_index() == 0 { self.selected = None; }
                if let Some(devices) = &self.devices {
                    for device in devices {
                        let duplicate = devices.iter().filter(|other| other.key == device.key).count() != 1;
                        ui.add_enabled_ui(!duplicate, |ui| {
                            let label = if duplicate { localization::input_value(ui.ctx(), "{value} (duplicate)", &device.key.name) }
                                else { device.key.name.clone() };
                            if settings_option_raw(ui, &label, self.selected.as_ref() == Some(&device.key)).clicked()
                                && ui.ctx().current_pass_index() == 0 {
                                if self.selected.as_ref() == Some(&device.key) { self.attempted = None; }
                                self.selected = Some(device.key.clone());
                            }
                        });
                    }
                }
            });
        if device_list.response.clicked() && ui.ctx().current_pass_index() == 0 { self.refresh(ui.ctx()); }
    }
}
