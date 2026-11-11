use crate::wasapi_capture::{self as capture, Info, Packet, Parts, Progress, Stream};
use std::{num::{NonZeroU32, NonZeroUsize}, sync::atomic::{AtomicBool, Ordering}, time::{Duration, Instant}};

#[derive(Clone, Copy, Debug)]
pub struct WaitPolicy {
    pub poll_millis: NonZeroU32,

    pub stall_timeout: Duration,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlagPolicy { Preserve, Reject }
#[derive(Clone, Copy, Debug)]
pub struct Request {

    pub at_least_frames: u64,

    pub packet_limit: NonZeroUsize,
    pub discontinuity: FlagPolicy,
    pub timestamp_error: FlagPolicy,
    pub wait: WaitPolicy,
}
#[derive(Debug)]
pub enum Error {
    WaitPolicy, Configuration(crate::wasapi_render::Error), Open(capture::OpenFailure), Stream(capture::Error),
    PacketAllocation(std::collections::TryReserveError), PacketLimit { limit: usize },
    Discontinuity { packet: usize }, TimestampError { packet: usize }, Stalled { elapsed: Duration },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit { TargetReached, Empty, Cancelled }
#[derive(Debug)]
pub struct Outcome {
    pub info: Option<Info>,
    pub progress: Option<Progress>,

    pub packets: Vec<Packet>,
    pub result: Result<Exit, Error>,

    pub stop: Result<(), capture::Error>,
}
impl Outcome {
    pub fn completed(&self) -> bool { matches!(&self.result, Ok(Exit::TargetReached | Exit::Empty)) && self.stop.is_ok() }
}

pub fn record(parts: Parts, request: Request, cancel: &AtomicBool) -> Outcome {
    record_with_mode(parts, capture::Mode::Capture, request, cancel)
}

pub fn record_with_mode(parts: Parts, mode: capture::Mode, request: Request, cancel: &AtomicBool) -> Outcome {
    let mut outcome = Outcome { info: None, progress: None, packets: Vec::new(), result: Ok(Exit::Empty), stop: Ok(()) };
    if request.wait.poll_millis.get() == u32::MAX || request.wait.stall_timeout.is_zero() {
        outcome.result = Err(Error::WaitPolicy); return outcome;
    }
    if let Err(error) = parts.format() { outcome.result = Err(Error::Configuration(error)); return outcome; }
    if request.at_least_frames == 0 { return outcome; }
    if cancel.load(Ordering::Acquire) { outcome.result = Ok(Exit::Cancelled); return outcome; }
    let mut stream = match Stream::open_with_mode(parts, mode) {
        Ok(stream) => stream,
        Err(error) => { outcome.result = Err(Error::Open(error)); return outcome; }
    };
    outcome.info = Some(stream.info().clone());
    outcome.result = drive(&mut stream, request, cancel, &mut outcome.packets);
    outcome.stop = stream.stop();
    outcome.progress = Some(stream.progress());
    outcome
}

fn drive(stream: &mut Stream, request: Request, cancel: &AtomicBool, packets: &mut Vec<Packet>) -> Result<Exit, Error> {
    if cancel.load(Ordering::Acquire) { return Ok(Exit::Cancelled); }
    stream.start().map_err(Error::Stream)?;
    let mut last_progress = Instant::now();
    loop {
        if cancel.load(Ordering::Acquire) { return Ok(Exit::Cancelled); }
        if stream.progress().released_frames >= request.at_least_frames { return Ok(Exit::TargetReached); }
        if packets.len() == request.packet_limit.get() { return Err(Error::PacketLimit { limit: request.packet_limit.get() }); }

        packets.try_reserve(1).map_err(Error::PacketAllocation)?;
        match stream.read_packet().map_err(Error::Stream)? {
            Some(packet) => {
                let header = packet.header;
                let ordinal = packets.len();
                packets.push(packet);
                last_progress = Instant::now();
                if header.discontinuity() && request.discontinuity == FlagPolicy::Reject { return Err(Error::Discontinuity { packet: ordinal }); }
                if !header.timestamps_valid() && request.timestamp_error == FlagPolicy::Reject { return Err(Error::TimestampError { packet: ordinal }); }

                continue;
            }
            None => {}
        }
        let elapsed = last_progress.elapsed();
        if elapsed >= request.wait.stall_timeout { return Err(Error::Stalled { elapsed }); }
        let remaining = request.wait.stall_timeout.saturating_sub(elapsed);
        let millis = remaining.as_millis().clamp(1, u128::from(request.wait.poll_millis.get())) as u32;
        let _ = stream.wait(millis).map_err(Error::Stream)?;
    }
}
