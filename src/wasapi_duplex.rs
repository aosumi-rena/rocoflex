use crate::{wasapi, wasapi_capture as capture, wasapi_render as render,
    wasapi_recording::FlagPolicy};
use std::{collections::{TryReserveError, VecDeque}, num::{NonZeroU32, NonZeroUsize},
    os::windows::io::AsRawHandle, sync::atomic::{AtomicBool, Ordering}, time::{Duration, Instant}};
use windows::Win32::{Foundation::{HANDLE, RPC_S_CALLPENDING}, System::Com::CoWaitForMultipleHandles};
#[path = "wasapi_duplex_framing.rs"]
mod framing;
pub use framing::{CaptureFraming, CaptureSpan, FramingError, NativeCursor};

#[derive(Clone, Copy, Debug)]
pub struct Format {
    pub sample_rate: u32,
    pub capture_channels: capture::Channels,
    pub render_channels: render::Channels,
}
pub trait Processor {
    type Error;

    fn process(&mut self, input: &capture::Packet, format: Format, output: &mut Vec<f32>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SilencePolicy { Reject, DeliverFlaggedPacket }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelPolicy { StopImmediately, DrainProcessed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnderrunPolicy {

    FailOnObservedEmpty,
    ObserveAndContinue,
}
#[derive(Clone, Copy, Debug)]
pub enum Prefill { DeviceBuffer, Frames(NonZeroU32) }
#[derive(Clone, Copy, Debug)]
pub struct Policy {

    pub queued_frames: NonZeroUsize,
    pub queued_blocks: NonZeroUsize,

    pub capture_packets_per_turn: NonZeroUsize,
    pub render_writes_per_turn: NonZeroUsize,
    pub prefill: Prefill,
    pub poll_millis: NonZeroU32,
    pub startup_timeout: Duration,
    pub capture_stall_timeout: Duration,
    pub render_stall_timeout: Duration,
    pub drain_timeout: Duration,
    pub discontinuity: FlagPolicy,
    pub timestamp_error: FlagPolicy,
    pub silence: SilencePolicy,
    pub cancel: CancelPolicy,
    pub underrun: UnderrunPolicy,
}
#[derive(Clone, Debug)]
pub struct Configuration {
    pub capture: capture::Parts, pub render: render::Parts, pub policy: Policy,

    pub callback_frames: NonZeroU32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputStage { Captured, Processing, Processed }
#[derive(Debug)]
pub struct Current {
    pub input: capture::Packet,
    pub sources: Vec<CaptureSpan>,
    pub output: Vec<f32>,
    pub stage: InputStage,
}
#[derive(Debug)]
pub struct Block {
    pub input: capture::Packet,
    pub sources: Vec<CaptureSpan>,
    pub output: Vec<f32>,

    pub accepted_frames: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake { Capture, Render, TimedOut }
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub callback_attempts: u64,
    pub callback_completed: u64,
    pub produced_frames: u64,
    pub rendered_frames: u64,
    pub queued_frames: usize,
    pub observed_empty_padding: u64,
    pub last_padding: Option<u32>,
    pub last_wake: Option<Wake>,

    pub cancel_observed: bool,
    pub draining: bool,
}
#[derive(Debug)]
pub enum Error<E> {
    RateMismatch { capture: u32, render: u32 }, Policy, Framing(FramingError),
    CaptureConfiguration(render::Error), RenderConfiguration(render::Error),
    CaptureOpen(capture::OpenFailure), RenderOpen(render::OpenFailure),
    Capture(capture::Error), Render(render::Error), Processor(E), Allocation(TryReserveError),
    Prefill { frames: u32, capacity: u32, queue_limit: usize },
    Discontinuity, TimestampError, SilentPacket,
    OutputChannels { samples: usize, channels: u16 }, Extent,
    QueueBlocks { limit: usize }, QueueFrames { requested: usize, limit: usize },
    UnexpectedRenderCount { actual: u32, remaining: usize },
    StartupStalled { elapsed: Duration }, CaptureStalled { elapsed: Duration },
    RenderStalled { elapsed: Duration }, DrainStalled { elapsed: Duration },
    ObservedEmptyPadding, CaptureStopDuringDrain,
    Wait(wasapi::Error), WaitIndex(u32), NoActiveStream,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Cancelled,

    CancelledDrained,
}
pub struct Outcome<P: Processor> {
    pub configuration: Configuration,
    pub processor: P,
    pub capture_info: Option<capture::Info>,
    pub render_info: Option<render::Info>,
    pub capture_progress: Option<capture::Progress>,
    pub render_progress: Option<render::Progress>,
    pub current: Option<Current>,
    pub framing: CaptureFraming,
    pub pending: VecDeque<Block>,

    pub last_retired: Option<Block>,
    pub progress: Progress,
    pub result: Result<Exit, Error<P::Error>>,

    pub capture_stop: Option<Result<(), capture::Error>>,
    pub render_stop: Option<Result<(), render::Error>>,
}
impl<P: Processor> Outcome<P> {
    pub fn processed_output_drained(&self) -> bool {
        matches!(&self.result, Ok(Exit::CancelledDrained))
            && matches!(&self.capture_stop, Some(Ok(())))
            && matches!(&self.render_stop, Some(Ok(())))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Observation {
    pub progress: Progress,
    pub capture_running: bool,
    pub render_running: bool,
    pub capture_buffer_frames: u32,
    pub render_buffer_frames: u32,
    pub buffered_capture_frames: usize,
}

pub fn run<P: Processor>(configuration: Configuration, processor: P, cancel: &AtomicBool) -> Outcome<P> {
    run_with_observer(configuration, processor, cancel, |_| {})
}

pub fn run_with_observer<P: Processor>(configuration: Configuration, processor: P, cancel: &AtomicBool,
    mut observer: impl FnMut(Observation)) -> Outcome<P>
{
    let framing = CaptureFraming::new(configuration.callback_frames, configuration.capture.channels.count());
    let mut out = Outcome { configuration, processor, capture_info: None, render_info: None,
        capture_progress: None, render_progress: None, current: None, framing, pending: VecDeque::new(),
        last_retired: None, capture_stop: None, render_stop: None, result: Ok(Exit::Cancelled),
        progress: Progress { callback_attempts: 0, callback_completed: 0, produced_frames: 0,
            rendered_frames: 0, queued_frames: 0, observed_empty_padding: 0,
            last_padding: None, last_wake: None, cancel_observed: false, draining: false } };
    if let Err(error) = preflight(&out.configuration) { out.result = Err(error); return out; }
    if let Err(error) = out.framing.reserve() { out.result = Err(Error::Framing(error)); return out; }
    if observe_cancel(&mut out, cancel) { return out; }

    let mut input = match capture::Stream::open(out.configuration.capture.clone()) {
        Ok(stream) => stream,
        Err(error) => { out.result = Err(Error::CaptureOpen(error)); return out; }
    };
    out.capture_info = Some(input.info().clone());
    let mut output = match render::Stream::open(out.configuration.render.clone()) {
        Ok(stream) => stream,
        Err(error) => {
            out.result = Err(Error::RenderOpen(error));
            out.capture_stop = Some(input.stop()); out.capture_progress = Some(input.progress());
            return out;
        }
    };
    out.render_info = Some(output.info().clone());
    out.result = drive(&mut input, &mut output, &mut out, cancel, &mut observer);
    if !input.progress().stop_attempted { out.capture_stop = Some(input.stop()); }
    out.capture_progress = Some(input.progress());
    out.render_stop = Some(output.stop());
    out.render_progress = Some(output.progress());

    out
}

fn preflight<E>(config: &Configuration) -> Result<(), Error<E>> {
    if config.capture.sample_rate != config.render.sample_rate {
        return Err(Error::RateMismatch { capture: config.capture.sample_rate, render: config.render.sample_rate });
    }
    config.capture.format().map_err(Error::CaptureConfiguration)?;
    config.render.format().map_err(Error::RenderConfiguration)?;
    let p = config.policy;
    if p.poll_millis.get() == u32::MAX || p.startup_timeout.is_zero() || p.capture_stall_timeout.is_zero()
        || p.render_stall_timeout.is_zero() || p.drain_timeout.is_zero() { return Err(Error::Policy); }
    Ok(())
}

struct Clocks { started: Instant, capture: Instant, render: Instant, drain: Option<Instant> }
impl Clocks {
    fn new() -> Self { let now = Instant::now(); Self { started: now, capture: now, render: now, drain: None } }
    fn check<E>(&self, p: Policy, render_running: bool, need_render: bool) -> Result<(), Error<E>> {
        if let Some(drain) = self.drain {
            let elapsed = drain.elapsed();
            if elapsed >= p.drain_timeout { return Err(Error::DrainStalled { elapsed }); }
        } else {
            let elapsed = self.capture.elapsed();
            if elapsed >= p.capture_stall_timeout { return Err(Error::CaptureStalled { elapsed }); }
            let elapsed = self.started.elapsed();
            if !render_running && elapsed >= p.startup_timeout { return Err(Error::StartupStalled { elapsed }); }
        }
        let elapsed = self.render.elapsed();
        if render_running && need_render && elapsed >= p.render_stall_timeout { return Err(Error::RenderStalled { elapsed }); }
        Ok(())
    }
    fn wait_millis(&self, p: Policy, render_running: bool, need_render: bool) -> u32 {
        let mut remaining = Duration::from_millis(u64::from(p.poll_millis.get()));
        if let Some(drain) = self.drain { remaining = remaining.min(p.drain_timeout.saturating_sub(drain.elapsed())); }
        else {
            remaining = remaining.min(p.capture_stall_timeout.saturating_sub(self.capture.elapsed()));
            if !render_running { remaining = remaining.min(p.startup_timeout.saturating_sub(self.started.elapsed())); }
        }
        if render_running && need_render { remaining = remaining.min(p.render_stall_timeout.saturating_sub(self.render.elapsed())); }
        remaining.as_millis().clamp(1, u128::from(p.poll_millis.get())) as u32
    }
}

fn drive<P: Processor>(input: &mut capture::Stream, output: &mut render::Stream,
    out: &mut Outcome<P>, cancel: &AtomicBool, observer: &mut impl FnMut(Observation)) -> Result<Exit, Error<P::Error>> {
    let p = out.configuration.policy;
    let capacity = output.info().buffer_frames;
    let prefill = match p.prefill { Prefill::DeviceBuffer => capacity, Prefill::Frames(frames) => frames.get() };
    if prefill > capacity { return Err(Error::Prefill { frames: prefill, capacity, queue_limit: p.queued_frames.get() }); }
    if observe_cancel(out, cancel) { return Ok(Exit::Cancelled); }
    input.start().map_err(Error::Capture)?;
    let mut clocks = Clocks::new();
    'turn: loop {
        observer(Observation { progress: out.progress, capture_running: input.progress().running,
            render_running: output.progress().running, capture_buffer_frames: input.info().buffer_frames,
            render_buffer_frames: output.info().buffer_frames, buffered_capture_frames: out.framing.buffered_frames() });
        if observe_cancel(out, cancel) && !out.progress.draining {
            if p.cancel == CancelPolicy::StopImmediately { return Ok(Exit::Cancelled); }
            out.progress.draining = true; clocks.drain = Some(Instant::now());
            out.capture_stop = Some(input.stop());
            if out.capture_stop.as_ref().is_some_and(Result::is_err) { return Err(Error::CaptureStopDuringDrain); }
        }
        let mut budget = p.render_writes_per_turn.get();
        let mut advanced = flush(output, out, &mut budget, &mut clocks, cancel)?;
        let cancelled = observe_cancel(out, cancel);
        start_output(output, prefill, out.progress.draining, &mut clocks, p, cancelled)?;
        if !out.progress.draining {
            for _ in 0..p.capture_packets_per_turn.get() {
                if observe_cancel(out, cancel) { continue 'turn; }

                if out.pending.len() == p.queued_blocks.get() { return Err(Error::QueueBlocks { limit: p.queued_blocks.get() }); }
                out.pending.try_reserve(1).map_err(Error::Allocation)?;
                if out.framing.native_cursor().is_none() {
                    let Some(packet) = input.read_packet().map_err(Error::Capture)? else { break; };
                    clocks.capture = Instant::now(); advanced = true;
                    out.framing.accept(packet).map_err(Error::Framing)?;
                    let header = out.framing.native_cursor().expect("same committed packet").packet.header;
                    if header.discontinuity() && p.discontinuity == FlagPolicy::Reject { return Err(Error::Discontinuity); }
                    if !header.timestamps_valid() && p.timestamp_error == FlagPolicy::Reject { return Err(Error::TimestampError); }
                    if header.silent() && p.silence == SilencePolicy::Reject { return Err(Error::SilentPacket); }
                }

                if observe_cancel(out, cancel) { continue 'turn; }
                let Some(block) = out.framing.next_block().map_err(Error::Framing)? else { continue; };
                advanced = true;
                out.current = Some(Current { input: block.packet, sources: block.sources, output: Vec::new(), stage: InputStage::Captured });
                process_current(out)?;
                advanced |= flush(output, out, &mut budget, &mut clocks, cancel)?;
                let cancelled = observe_cancel(out, cancel);
                start_output(output, prefill, false, &mut clocks, p, cancelled)?;
                if budget == 0 { break; }
            }
        }
        if observe_cancel(out, cancel) && !out.progress.draining { continue; }
        let running = output.progress().running;
        let padding = output.padding().map_err(Error::Render)?;
        if out.progress.last_padding.is_some_and(|previous| padding < previous) { clocks.render = Instant::now(); }
        out.progress.last_padding = Some(padding);
        if out.progress.draining {
            if out.pending.is_empty() && padding == 0 { return Ok(Exit::CancelledDrained); }
        } else if running && padding == 0 {
            out.progress.observed_empty_padding = out.progress.observed_empty_padding.checked_add(1).ok_or(Error::Extent)?;
            if p.underrun == UnderrunPolicy::FailOnObservedEmpty { return Err(Error::ObservedEmptyPadding); }
        }
        let need_render = !out.pending.is_empty() || padding != 0;
        clocks.check(p, running, need_render)?;
        if advanced { continue; }
        let millis = clocks.wait_millis(p, running, need_render);
        out.progress.last_wake = Some(wait_streams(
            if out.progress.draining { None } else { Some(input) },
            if running { Some(output) } else { None }, millis)?);
    }
}

fn process_current<P: Processor>(out: &mut Outcome<P>) -> Result<(), Error<P::Error>> {
    let p = out.configuration.policy;
    let current = out.current.as_ref().expect("committed input retained");
    if current.input.header.discontinuity() && p.discontinuity == FlagPolicy::Reject { return Err(Error::Discontinuity); }
    if !current.input.header.timestamps_valid() && p.timestamp_error == FlagPolicy::Reject { return Err(Error::TimestampError); }
    if current.input.header.silent() && p.silence == SilencePolicy::Reject { return Err(Error::SilentPacket); }
    out.progress.callback_attempts = out.progress.callback_attempts.checked_add(1).ok_or(Error::Extent)?;
    let format = Format { sample_rate: out.configuration.capture.sample_rate,
        capture_channels: out.configuration.capture.channels, render_channels: out.configuration.render.channels };
    let current = out.current.as_mut().expect("same committed input");
    current.stage = InputStage::Processing;
    let returned = out.processor.process(&current.input, format, &mut current.output);
    returned.map_err(Error::Processor)?;
    current.stage = InputStage::Processed;
    out.progress.callback_completed = out.progress.callback_completed.checked_add(1).ok_or(Error::Extent)?;
    let channels = usize::from(format.render_channels.count());
    if current.output.len() % channels != 0 { return Err(Error::OutputChannels { samples: current.output.len(), channels: channels as u16 }); }
    let frames = current.output.len() / channels;
    out.progress.produced_frames = out.progress.produced_frames.checked_add(u64::try_from(frames).map_err(|_| Error::Extent)?).ok_or(Error::Extent)?;
    let queued = out.progress.queued_frames.checked_add(frames).ok_or(Error::Extent)?;
    if queued > p.queued_frames.get() { return Err(Error::QueueFrames { requested: queued, limit: p.queued_frames.get() }); }
    let Current { input, sources, output, .. } = out.current.take().expect("same processed input");
    let block = Block { input, sources, output, accepted_frames: 0 };
    if frames == 0 { out.last_retired = Some(block); }
    else {
        out.pending.push_back(block); out.progress.queued_frames = queued;
    }
    Ok(())
}

fn flush<P: Processor>(output: &mut render::Stream, out: &mut Outcome<P>, budget: &mut usize,
    clocks: &mut Clocks, cancel: &AtomicBool) -> Result<bool, Error<P::Error>> {
    let channels = usize::from(out.configuration.render.channels.count());
    let mut advanced = false;
    while *budget != 0 {

        if observe_cancel(out, cancel) && !out.progress.draining { break; }
        let Some(block) = out.pending.front_mut() else { break; };
        let first = block.accepted_frames.checked_mul(channels).ok_or(Error::Extent)?;
        let remaining = block.output.len() / channels - block.accepted_frames;
        *budget -= 1;
        let frames = output.write_interleaved(&block.output[first..]).map_err(Error::Render)?;
        if frames == 0 { break; }
        if frames as usize > remaining { return Err(Error::UnexpectedRenderCount { actual: frames, remaining }); }
        block.accepted_frames += frames as usize;
        out.progress.queued_frames -= frames as usize;
        out.progress.rendered_frames = output.progress().accepted_frames;
        advanced = true; clocks.render = Instant::now();
        if block.accepted_frames == block.output.len() / channels {
            out.last_retired = out.pending.pop_front();
        }
    }
    Ok(advanced)
}

fn start_output<E>(output: &mut render::Stream, prefill: u32, draining: bool,
    clocks: &mut Clocks, p: Policy, cancelled: bool) -> Result<(), Error<E>> {
    if !draining && cancelled { return Ok(()); }
    if !output.progress().start_attempted && output.progress().accepted_frames != 0
        && (draining || output.progress().accepted_frames >= u64::from(prefill)) {

        let elapsed = clocks.started.elapsed();
        if !draining && elapsed >= p.startup_timeout { return Err(Error::StartupStalled { elapsed }); }
        output.start().map_err(Error::Render)?;
        clocks.render = Instant::now();
    }
    Ok(())
}

fn observe_cancel<P: Processor>(out: &mut Outcome<P>, cancel: &AtomicBool) -> bool {
    out.progress.cancel_observed |= cancel.load(Ordering::Acquire);
    out.progress.cancel_observed
}

fn wait_streams<E>(input: Option<&capture::Stream>, output: Option<&render::Stream>, millis: u32) -> Result<Wake, Error<E>> {
    let input_handle = input.map(capture::Stream::wait_handle).transpose().map_err(Error::Capture)?;
    let output_handle = output.map(render::Stream::wait_handle).transpose().map_err(Error::Render)?;
    let mut handles = [HANDLE(std::ptr::null_mut()); 2];
    let mut count = 0;
    if let Some(handle) = &input_handle { handles[count] = HANDLE(handle.as_raw_handle()); count += 1; }
    if let Some(handle) = &output_handle { handles[count] = HANDLE(handle.as_raw_handle()); count += 1; }
    if count == 0 { return Err(Error::NoActiveStream); }

    match unsafe { CoWaitForMultipleHandles(0, millis, &handles[..count]) } {
        Ok(0) if input_handle.is_some() => Ok(Wake::Capture),
        Ok(0) if output_handle.is_some() => Ok(Wake::Render),
        Ok(1) if input_handle.is_some() && output_handle.is_some() => Ok(Wake::Render),
        Ok(index) => Err(Error::WaitIndex(index)),
        Err(error) if error.code() == RPC_S_CALLPENDING => Ok(Wake::TimedOut),
        Err(error) => Err(Error::Wait(wasapi::native("CoWaitForMultipleHandles(duplex)", error))),
    }
}
