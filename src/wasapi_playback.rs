use crate::{conditional_gate::Batch, wasapi_render::{self as render, Channels, Info, Parts, Progress, Stream}};
use std::{num::NonZeroU32, sync::atomic::{AtomicBool, Ordering}, time::{Duration, Instant}};

#[derive(Clone, Copy, Debug)]
pub struct WaitPolicy {
    pub poll_millis: NonZeroU32,

    pub stall_timeout: Duration,
}
#[derive(Debug)]
pub enum Admission {
    MonoRequired, WaitPolicy, Hop, Extent,
    RowWidth { row: usize, expected: usize, actual: usize },
    Nonfinite { row: Option<usize>, sample: usize },
}
#[derive(Debug)]
pub enum Error {
    Admission(Admission), Configuration(render::Error), Open(render::OpenFailure), Stream(render::Error),
    SourceExtent { offset: usize }, NoInitialCapacity, Stalled { elapsed: Duration },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {

    ClientQueueDrained,
    Empty,
    Cancelled,
}
#[derive(Debug)]
pub struct Outcome {
    pub info: Option<Info>,
    pub progress: Option<Progress>,
    pub raw_status: Option<u8>,
    pub result: Result<Exit, Error>,

    pub stop: Result<(), render::Error>,
}
impl Outcome {
    pub fn completed(&self) -> bool {
        matches!(&self.result, Ok(Exit::ClientQueueDrained | Exit::Empty)) && self.stop.is_ok()
    }
}

#[derive(Clone, Copy)]
enum Input<'a> { Samples(&'a [f32]), Rows { batch: &'a Batch, hop: usize } }
impl<'a> Input<'a> {
    fn raw_status(self) -> Option<u8> { match self { Self::Samples(_) => None, Self::Rows { batch, .. } => Some(batch.status) } }
    fn admit(self) -> Result<usize, Admission> {
        match self {
            Self::Samples(samples) => {
                if let Some(sample) = samples.iter().position(|sample| !sample.is_finite()) {
                    return Err(Admission::Nonfinite { row: None, sample });
                }
                Ok(samples.len())
            }
            Self::Rows { batch, hop } => {
                if hop == 0 { return Err(Admission::Hop); }
                let count = batch.rows.len().checked_mul(hop).ok_or(Admission::Extent)?;
                for (row, values) in batch.rows.iter().enumerate() {
                    if values.len() != hop { return Err(Admission::RowWidth { row, expected: hop, actual: values.len() }); }
                }
                for (row, values) in batch.rows.iter().enumerate() {
                    if let Some(sample) = values.iter().position(|sample| !sample.is_finite()) {
                        return Err(Admission::Nonfinite { row: Some(row), sample });
                    }
                }
                Ok(count)
            }
        }
    }
    fn segment(self, offset: usize) -> Option<&'a [f32]> {
        match self {
            Self::Samples(samples) => samples.get(offset..),
            Self::Rows { batch, hop } => batch.rows.get(offset.checked_div(hop)?)?.get(offset % hop..),
        }
    }
}

pub fn play_mono(samples: &[f32], parts: &Parts, wait: WaitPolicy, cancel: &AtomicBool) -> Outcome {
    play(Input::Samples(samples), parts, wait, cancel)
}

pub fn play_mono_rows(batch: &Batch, hop: usize, parts: &Parts, wait: WaitPolicy, cancel: &AtomicBool) -> Outcome {
    play(Input::Rows { batch, hop }, parts, wait, cancel)
}

fn play(input: Input<'_>, parts: &Parts, wait: WaitPolicy, cancel: &AtomicBool) -> Outcome {
    let mut outcome = Outcome { info: None, progress: None, raw_status: input.raw_status(), result: Ok(Exit::Empty), stop: Ok(()) };
    let admission = (|| {
        if parts.channels != Channels::Mono { return Err(Admission::MonoRequired); }
        if wait.poll_millis.get() == u32::MAX || wait.stall_timeout.is_zero() { return Err(Admission::WaitPolicy); }
        input.admit()
    })();
    let frames = match admission {
        Ok(frames) => frames,
        Err(error) => { outcome.result = Err(Error::Admission(error)); return outcome; }
    };
    if let Err(error) = parts.format() {
        outcome.result = Err(Error::Configuration(error)); return outcome;
    }
    if frames == 0 { return outcome; }
    if cancel.load(Ordering::Acquire) { outcome.result = Ok(Exit::Cancelled); return outcome; }

    let mut stream = match Stream::open(parts.clone()) {
        Ok(stream) => stream,
        Err(error) => { outcome.result = Err(Error::Open(error)); return outcome; }
    };
    outcome.info = Some(stream.info().clone());
    outcome.result = drive(&mut stream, input, frames, wait, cancel);
    outcome.stop = stream.stop();
    outcome.progress = Some(stream.progress());
    outcome
}

fn write_segment(stream: &mut Stream, input: Input<'_>, offset: &mut usize) -> Result<usize, Error> {
    let segment = input.segment(*offset).filter(|segment| !segment.is_empty())
        .ok_or(Error::SourceExtent { offset: *offset })?;
    let count = stream.write_interleaved(segment).map_err(Error::Stream)? as usize;
    *offset = offset.checked_add(count).ok_or(Error::SourceExtent { offset: *offset })?;
    Ok(count)
}

fn drive(stream: &mut Stream, input: Input<'_>, frames: usize, wait: WaitPolicy, cancel: &AtomicBool) -> Result<Exit, Error> {
    let mut offset = 0usize;

    while offset < frames {
        if cancel.load(Ordering::Acquire) { return Ok(Exit::Cancelled); }
        if write_segment(stream, input, &mut offset)? == 0 { break; }
    }
    if offset == 0 { return Err(Error::NoInitialCapacity); }
    if cancel.load(Ordering::Acquire) { return Ok(Exit::Cancelled); }
    stream.start().map_err(Error::Stream)?;
    let mut last_progress = Instant::now();
    let mut last_padding = None;
    loop {
        if cancel.load(Ordering::Acquire) { return Ok(Exit::Cancelled); }
        while offset < frames {
            if cancel.load(Ordering::Acquire) { return Ok(Exit::Cancelled); }
            if write_segment(stream, input, &mut offset)? == 0 { break; }
            last_progress = Instant::now();
        }
        if offset == frames {
            let padding = stream.padding().map_err(Error::Stream)?;
            if padding == 0 { return Ok(Exit::ClientQueueDrained); }
            if last_padding.is_some_and(|previous| padding < previous) { last_progress = Instant::now(); }
            last_padding = Some(padding);
        }
        let elapsed = last_progress.elapsed();
        if elapsed >= wait.stall_timeout { return Err(Error::Stalled { elapsed }); }
        let remaining = wait.stall_timeout.saturating_sub(elapsed);
        let millis = remaining.as_millis().clamp(1, u128::from(wait.poll_millis.get())) as u32;

        let _ = stream.wait(millis).map_err(Error::Stream)?;
    }
}
