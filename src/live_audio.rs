use std::{num::{NonZeroU32, NonZeroUsize}, panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, thread::{self, JoinHandle}, time::Duration};
use crate::{Result, wasapi_capture as capture, wasapi_duplex as duplex};
pub use duplex::{Configuration, Policy};
#[path = "live_audio_mmcss.rs"]
mod mmcss;
pub use mmcss::SchedulingSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Starting, Receiving, Running, Stopping, Stopped, BackendUnavailable, Failed }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessorFailure { MissingCondition }

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub phase: Phase,
    pub sample_rate: u32,
    pub capture_channels: u16,
    pub render_channels: u16,
    pub callback_frames: u32,
    pub buffered_capture_frames: usize,
    pub capture_buffer_frames: Option<u32>,
    pub render_buffer_frames: Option<u32>,
    pub captured_frames: u64,
    pub produced_frames: u64,
    pub rendered_frames: u64,
    pub callbacks: u64,
    pub queued_frames: usize,
    pub underruns: u64,

    pub input_peak: f32,
    pub output_peak: f32,
    pub input_rms: f32,
    pub output_rms: f32,
    pub error: Option<String>,

    pub processor_failure: Option<ProcessorFailure>,
    pub stop_error: Option<String>,
    pub scheduling: SchedulingSnapshot,
}

pub struct Handle {
    cancel: Arc<AtomicBool>,
    shared: Arc<Mutex<Snapshot>>,
    worker: Option<JoinHandle<()>>,
}
impl Handle {
    pub fn snapshot(&self) -> Snapshot { lock(&self.shared).clone() }
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
        let mut status = lock(&self.shared);
        if matches!(status.phase, Phase::Starting | Phase::Receiving | Phase::Running) {
            status.phase = Phase::Stopping;
        }
    }
    pub fn is_finished(&self) -> bool { self.worker.as_ref().is_none_or(JoinHandle::is_finished) }

    pub fn poll_finished(&mut self) -> bool {
        if !self.is_finished() { return false; }
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                let mut status = lock(&self.shared);
                status.phase = Phase::Failed;
                status.processor_failure = None;
                status.error = Some("Audio worker terminated unexpectedly".into());
            }
        }
        true
    }

}
impl Drop for Handle {
    fn drop(&mut self) {
        self.stop();

        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                let mut status = lock(&self.shared);
                status.phase = Phase::Failed;
                status.processor_failure = None;
                status.error = Some("Audio worker terminated unexpectedly".into());
            }
        }
    }
}

pub(crate) fn classify_processor_failure(error: &duplex::Error<String>) -> Option<ProcessorFailure> {
    match error {
        duplex::Error::Processor(reason)
            if reason == crate::current_condition::PROCESSING_REQUIRES_CONDITION =>
                Some(ProcessorFailure::MissingCondition),
        _ => None,
    }
}

fn lock(shared: &Mutex<Snapshot>) -> std::sync::MutexGuard<'_, Snapshot> {
    shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn interactive_policy() -> Policy {
    Policy { queued_frames: NonZeroUsize::new(19_200).unwrap(), queued_blocks: NonZeroUsize::new(128).unwrap(),
        capture_packets_per_turn: NonZeroUsize::new(16).unwrap(), render_writes_per_turn: NonZeroUsize::new(16).unwrap(),
        prefill: duplex::Prefill::DeviceBuffer, poll_millis: NonZeroU32::new(5).unwrap(),
        startup_timeout: Duration::from_secs(5), capture_stall_timeout: Duration::from_secs(5),
        render_stall_timeout: Duration::from_secs(5), drain_timeout: Duration::from_secs(2),
        discontinuity: crate::wasapi_recording::FlagPolicy::Preserve,
        timestamp_error: crate::wasapi_recording::FlagPolicy::Preserve,
        silence: duplex::SilencePolicy::DeliverFlaggedPacket,
        cancel: duplex::CancelPolicy::StopImmediately, underrun: duplex::UnderrunPolicy::ObserveAndContinue }
}

fn validate_start(configuration: &Configuration, model: bool) -> Result<()> {
    if configuration.capture.sample_rate != configuration.render.sample_rate {
        return Err("Live capture and render client rates must match".into());
    }
    if model && configuration.capture.sample_rate != 48_000 {
        return Err("The live model requires explicit 48000 Hz capture and render client formats".into());
    }
    configuration.capture.format().map_err(|e| format!("Input format: {e:?}"))?;
    configuration.render.format().map_err(|e| format!("Output format: {e:?}"))?;
    Ok(())
}

pub fn start<P, F>(configuration: Configuration, make_processor: F) -> Result<Handle>
where F: FnOnce() -> Result<P> + Send + 'static,
    P: duplex::Processor<Error = String> + 'static,
{
    start_with_policy(configuration, make_processor, true)
}

pub(crate) fn start_direct(configuration: Configuration, processor: DirectProcessor) -> Result<Handle> {
    start_with_policy(configuration, move || Ok(processor), false)
}

fn start_with_policy<P, F>(configuration: Configuration, make_processor: F, model: bool) -> Result<Handle>
where F: FnOnce() -> Result<P> + Send + 'static,
    P: duplex::Processor<Error = String> + 'static,
{
    validate_start(&configuration, model)?;
    let shared = Arc::new(Mutex::new(Snapshot { phase: Phase::Starting,
        sample_rate: configuration.capture.sample_rate, capture_channels: configuration.capture.channels.count(),
        callback_frames: configuration.callback_frames.get(), buffered_capture_frames: 0,
        render_channels: configuration.render.channels.count(), capture_buffer_frames: None, render_buffer_frames: None,
        captured_frames: 0, produced_frames: 0, rendered_frames: 0, callbacks: 0, queued_frames: 0, underruns: 0,
        input_peak: 0.0, output_peak: 0.0, input_rms: 0.0, output_rms: 0.0, error: None,
        processor_failure: None, stop_error: None,
        scheduling: SchedulingSnapshot::default() }));
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_status = Arc::clone(&shared);
    let worker_cancel = Arc::clone(&cancel);
    let worker = thread::Builder::new().name("Rocoflex live audio".into()).spawn(move || {
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut scheduling = mmcss::Registration::new(Arc::clone(&worker_status));
            if worker_cancel.load(Ordering::Acquire) { lock(&worker_status).phase = Phase::Stopped; return; }
            let processor = match make_processor() {
                Ok(processor) => processor,
                Err(error) => {
                    let mut status = lock(&worker_status);
                    status.phase = Phase::BackendUnavailable; status.error = Some(error); return;
                }
            };

            if worker_cancel.load(Ordering::Acquire) { lock(&worker_status).phase = Phase::Stopped; return; }
            scheduling.register();
            let processor = Metered { inner: processor, shared: Arc::clone(&worker_status) };
            let out = duplex::run_with_observer(configuration, processor, &worker_cancel, |observation| {
                let mut status = lock(&worker_status);
                status.capture_buffer_frames = Some(observation.capture_buffer_frames);
                status.render_buffer_frames = Some(observation.render_buffer_frames);
                status.produced_frames = observation.progress.produced_frames;
                status.rendered_frames = observation.progress.rendered_frames;
                status.callbacks = observation.progress.callback_completed;
                status.buffered_capture_frames = observation.buffered_capture_frames;
                status.queued_frames = observation.progress.queued_frames;
                status.underruns = observation.progress.observed_empty_padding;
                status.phase = if worker_cancel.load(Ordering::Acquire) { Phase::Stopping }
                    else if observation.render_running { Phase::Running }
                    else if observation.capture_running { Phase::Receiving } else { Phase::Starting };
            });
            let mut status = lock(&worker_status);
            status.produced_frames = out.progress.produced_frames;
            status.rendered_frames = out.progress.rendered_frames;
            status.callbacks = out.progress.callback_completed;
            status.buffered_capture_frames = out.framing.buffered_frames();
            status.queued_frames = out.progress.queued_frames;
            status.underruns = out.progress.observed_empty_padding;
            if let Some(progress) = out.capture_progress { status.captured_frames = progress.released_frames; }
            if let Some(info) = &out.capture_info { status.capture_buffer_frames = Some(info.buffer_frames); }
            if let Some(info) = &out.render_info { status.render_buffer_frames = Some(info.buffer_frames); }
            status.phase = match &out.result {
                Ok(_) => Phase::Stopped,
                Err(error) => {
                    status.processor_failure = classify_processor_failure(error);
                    status.error = Some(format!("{error:?}"));
                    Phase::Failed
                }
            };
            let mut stops = Vec::new();
            if let Some(Err(error)) = &out.capture_stop { stops.push(format!("Input stop: {error:?}")); }
            if let Some(Err(error)) = &out.render_stop { stops.push(format!("Output stop: {error:?}")); }
            if !stops.is_empty() { status.stop_error = Some(stops.join("; ")); status.phase = Phase::Failed; }

        }));
        if let Err(payload) = result {
            let reason = payload.downcast_ref::<String>().cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "Unknown panic payload".into());
            let mut status = lock(&worker_status);
            status.phase = Phase::Failed; status.processor_failure = None;
            status.error = Some(format!("Audio worker panic: {reason}"));
        }
    }).map_err(|e| format!("Cannot start audio worker: {e}"))?;
    Ok(Handle { cancel, shared, worker: Some(worker) })
}

struct Metered<P> { inner: P, shared: Arc<Mutex<Snapshot>> }
impl<P: duplex::Processor<Error = String>> duplex::Processor for Metered<P> {
    type Error = String;
    fn process(&mut self, input: &capture::Packet, format: duplex::Format, output: &mut Vec<f32>) -> Result<()> {
        let (peak, rms) = match &input.payload { capture::Payload::Interleaved(samples) => meter(samples),
            capture::Payload::Silent { .. } => (0.0, 0.0) };
        {
            let mut status = lock(&self.shared);
            status.captured_frames = input.header.first_frame.saturating_add(u64::from(input.header.frames));
            status.input_peak = peak; status.input_rms = rms;
        }
        self.inner.process(input, format, output)?;
        let (peak, rms) = meter(output);
        let mut status = lock(&self.shared);
        status.output_peak = peak; status.output_rms = rms;
        Ok(())
    }
}
fn meter(samples: &[f32]) -> (f32, f32) {
    let mut peak = 0.0_f32; let mut squares = 0.0_f64;
    for &sample in samples { if sample.is_finite() { peak = peak.max(sample.abs()); squares += f64::from(sample).powi(2); } }
    (peak, if samples.is_empty() { 0.0 } else { (squares / samples.len() as f64).sqrt() as f32 })
}

pub(crate) struct DirectProcessor {
    selected_input_channel: usize,
    meter: crate::morph_processor::input_meter::Writer,
    meter_gain: crate::morph_processor::input_gain::InputGain,
}
impl DirectProcessor {
    pub(crate) fn new(selected_input_channel: usize, meter: crate::morph_processor::input_meter::Writer) -> Self {
        Self { selected_input_channel, meter, meter_gain: crate::morph_processor::input_gain::InputGain::new() }
    }
}
impl duplex::Processor for DirectProcessor {
    type Error = String;
    fn process(&mut self, input: &capture::Packet, format: duplex::Format, output: &mut Vec<f32>) -> Result<()> {
        let channels = usize::from(format.capture_channels.count());
        if self.selected_input_channel >= channels {
            return Err(format!("Input channel {} unavailable in {channels}-channel format", self.selected_input_channel));
        }
        if channels != usize::from(format.render_channels.count()) {
            return Err("Direct audio requires matching capture and render channels".into());
        }
        let expected = (input.header.frames as usize).checked_mul(channels)
            .ok_or("Capture sample extent overflow")?;
        let end = output.len().checked_add(expected).ok_or("Render sample extent overflow")?;
        let pending = match &input.payload {
            capture::Payload::Interleaved(samples) => {
                if samples.len() != expected { return Err("Capture packet extent does not match its header".into()); }
                if samples.iter().any(|sample| !sample.is_finite()) { return Err("Nonfinite input audio".into()); }
                let pending = self.meter.begin_interleaved_block(
                    samples, channels, self.selected_input_channel, &self.meter_gain);
                output.try_reserve(expected).map_err(|e| format!("Cannot reserve direct output: {e}"))?;
                output.extend_from_slice(samples);
                pending
            }
            capture::Payload::Silent { sample_count } => {
                if *sample_count != expected { return Err("Silent packet extent does not match its header".into()); }
                let pending = self.meter.begin_silent_block(input.header.frames as usize, &self.meter_gain);
                output.try_reserve(expected).map_err(|e| format!("Cannot reserve direct output: {e}"))?;
                output.resize(end, 0.0);
                pending
            }
        };
        self.meter.finish_block(pending);
        Ok(())
    }
}

pub struct MonoProcessor<F> { input_channel: usize, process: F, mono_input: Vec<f32>, mono_output: Vec<f32> }
impl<F> MonoProcessor<F> {
    pub fn new(input_channel: usize, process: F) -> Self {
        Self { input_channel, process, mono_input: Vec::new(), mono_output: Vec::new() }
    }
}
impl<F: FnMut(&[f32], &mut Vec<f32>) -> Result<()>> duplex::Processor for MonoProcessor<F> {
    type Error = String;
    fn process(&mut self, input: &capture::Packet, format: duplex::Format, output: &mut Vec<f32>) -> Result<()> {
        if format.sample_rate != 48_000 { return Err("Live model client rate must be 48000 Hz".into()); }
        let channels = usize::from(format.capture_channels.count());
        if self.input_channel >= channels { return Err(format!("Input channel {} unavailable in {channels}-channel format", self.input_channel)); }
        let frames = input.header.frames as usize;
        let expected = frames.checked_mul(channels).ok_or("Capture sample extent overflow")?;
        self.mono_input.clear(); self.mono_input.reserve(frames);
        match &input.payload {
            capture::Payload::Interleaved(samples) => {
                if samples.len() != expected { return Err("Capture packet extent does not match its header".into()); }
                self.mono_input.extend(samples.chunks_exact(channels).map(|frame| frame[self.input_channel]));
            }
            capture::Payload::Silent { sample_count } => {
                if *sample_count != expected { return Err("Silent packet extent does not match its header".into()); }
                self.mono_input.resize(frames, 0.0);
            }
        }
        if self.mono_input.iter().any(|sample| !sample.is_finite()) { return Err("Nonfinite input audio".into()); }
        self.mono_output.clear();
        (self.process)(&self.mono_input, &mut self.mono_output)?;
        if self.mono_output.iter().any(|sample| !sample.is_finite()) { return Err("Nonfinite model output".into()); }
        let render_channels = usize::from(format.render_channels.count());
        let added = self.mono_output.len().checked_mul(render_channels).ok_or("Render sample extent overflow")?;
        output.reserve(added);
        for &sample in &self.mono_output { for _ in 0..render_channels { output.push(sample); } }
        Ok(())
    }
}

