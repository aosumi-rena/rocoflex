use crate::{wasapi_capture::Payload, wasapi_recording::{FlagPolicy, Outcome}, wave_output as wave};
use std::{fmt, io::Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SilencePolicy {
    Reject,

    MaterializeFlaggedSilenceAsPositiveZero,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionPolicy {
    CompletedOnly,

    ReleasedPrefix,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {

    pub sample_rate: u32,

    pub source_channel: u16,
    pub silence: SilencePolicy,
    pub completion: CompletionPolicy,
    pub discontinuity: FlagPolicy,
    pub timestamp_error: FlagPolicy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Admission, Packets, Selection, Writing, Complete }
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub stage: Stage,
    pub validated_packets: usize,
    pub validated_frames: u64,
    pub selected_packets: usize,
    pub selected_frames: usize,
}
#[derive(Debug)]
pub enum Admission {
    RecordingIncomplete, MissingInfo, MissingProgress, NoPackets,
    SampleRate { requested: u32, captured: u32 },
    Channel { selected: u16, channels: u16 },
    Extent,
    PacketOrdinal { packet: usize, reported: u64 },
    FramePrefix { packet: usize, expected: u64, reported: u64 },
    PacketFrames { packet: usize, frames: u32, capacity: u32 },
    Payload { packet: usize, expected_samples: usize, actual_samples: usize },
    SilenceFlag { packet: usize }, SilentPacket { packet: usize },
    Discontinuity { packet: usize }, TimestampError { packet: usize },
    ReleasedTotals { packets: u64, frames: u64, reported_packets: u64, reported_frames: u64 },
}
pub enum Cause {
    Admission(Admission),
    Allocation(std::collections::TryReserveError),

    Wave(wave::Error),
}
impl fmt::Debug for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(error) => f.debug_tuple("Admission").field(error).finish(),
            Self::Allocation(error) => f.debug_tuple("Allocation").field(error).finish(),
            Self::Wave(error) => match error {
                wave::Error::SampleRate(rate) => f.debug_tuple("WaveSampleRate").field(rate).finish(),
                wave::Error::Hop => f.write_str("WaveHop"),
                wave::Error::Extent => f.write_str("WaveExtent"),
                wave::Error::RowWidth { row, expected, actual } => f.debug_struct("WaveRowWidth")
                    .field("row", row).field("expected", expected).field("actual", actual).finish(),
                wave::Error::Io { bytes_written, cause } => f.debug_struct("WaveIo")
                    .field("bytes_written", bytes_written).field("cause", cause).finish(),
            },
        }
    }
}

#[derive(Debug)]
struct Prepared<'a> {
    recording: &'a Outcome,
    options: Options,
    progress: Progress,
    mono: Vec<f32>,
}
#[derive(Debug)]
pub struct Completed<'a> { prepared: Prepared<'a>, receipt: wave::Receipt }
impl<'a> Completed<'a> {
    pub fn recording(&self) -> &'a Outcome { self.prepared.recording }
    pub fn options(&self) -> Options { self.prepared.options }
    pub fn progress(&self) -> Progress { self.prepared.progress }
    pub fn samples(&self) -> &[f32] { &self.prepared.mono }
    pub fn receipt(&self) -> wave::Receipt { self.receipt }

    pub fn whole_recording_completed(&self) -> bool { self.prepared.recording.completed() }

    pub fn into_parts(self) -> (&'a Outcome, Options, Progress, Vec<f32>, wave::Receipt) {
        let Prepared { recording, options, progress, mono } = self.prepared;
        (recording, options, progress, mono, self.receipt)
    }
}
#[derive(Debug)]
pub struct Failure<'a> { prepared: Prepared<'a>, cause: Cause }
impl<'a> Failure<'a> {
    pub fn recording(&self) -> &'a Outcome { self.prepared.recording }
    pub fn options(&self) -> Options { self.prepared.options }
    pub fn progress(&self) -> Progress { self.prepared.progress }
    pub fn selected_samples(&self) -> &[f32] { &self.prepared.mono }
    pub fn cause(&self) -> &Cause { &self.cause }

    pub fn into_parts(self) -> (&'a Outcome, Options, Progress, Vec<f32>, Cause) {
        let Prepared { recording, options, progress, mono } = self.prepared;
        (recording, options, progress, mono, self.cause)
    }
}

pub fn write_mono<'a, W: Write + ?Sized>(writer: &mut W, recording: &'a Outcome, options: Options)
    -> Result<Completed<'a>, Failure<'a>> {
    let mut prepared = Prepared { recording, options, mono: Vec::new(), progress: Progress {
        stage: Stage::Admission, validated_packets: 0, validated_frames: 0,
        selected_packets: 0, selected_frames: 0,
    } };
    if let Err(cause) = select(&mut prepared) { return Err(Failure { prepared, cause }); }
    prepared.progress.stage = Stage::Writing;
    match wave::write_mono_samples(writer, options.sample_rate, &prepared.mono) {
        Ok(receipt) => { prepared.progress.stage = Stage::Complete; Ok(Completed { prepared, receipt }) }
        Err(error) => Err(Failure { prepared, cause: Cause::Wave(error) }),
    }
}

fn select(prepared: &mut Prepared<'_>) -> Result<(), Cause> {
    let recording = prepared.recording;
    let options = prepared.options;
    let admit = |error| Cause::Admission(error);
    if options.completion == CompletionPolicy::CompletedOnly && !recording.completed() {
        return Err(admit(Admission::RecordingIncomplete));
    }
    let info = recording.info.as_ref().ok_or_else(|| admit(Admission::MissingInfo))?;
    let progress = recording.progress.as_ref().ok_or_else(|| admit(Admission::MissingProgress))?;
    if recording.packets.is_empty() { return Err(admit(Admission::NoPackets)); }
    if info.sample_rate != options.sample_rate { return Err(admit(Admission::SampleRate { requested: options.sample_rate, captured: info.sample_rate })); }
    let channels = info.channels.count();
    if options.source_channel >= channels { return Err(admit(Admission::Channel { selected: options.source_channel, channels })); }
    prepared.progress.stage = Stage::Packets;
    let mut total = 0u64;
    for (index, packet) in recording.packets.iter().enumerate() {
        let header = packet.header;
        if header.ordinal != u64::try_from(index).map_err(|_| admit(Admission::Extent))? {
            return Err(admit(Admission::PacketOrdinal { packet: index, reported: header.ordinal }));
        }
        if header.first_frame != total { return Err(admit(Admission::FramePrefix { packet: index, expected: total, reported: header.first_frame })); }
        if header.frames == 0 || header.frames > info.buffer_frames {
            return Err(admit(Admission::PacketFrames { packet: index, frames: header.frames, capacity: info.buffer_frames }));
        }
        let count = (header.frames as usize).checked_mul(usize::from(channels)).ok_or_else(|| admit(Admission::Extent))?;
        let (actual, silent_payload) = match &packet.payload {
            Payload::Interleaved(samples) => (samples.len(), false),
            Payload::Silent { sample_count } => (*sample_count, true),
        };
        if actual != count { return Err(admit(Admission::Payload { packet: index, expected_samples: count, actual_samples: actual })); }
        if silent_payload != header.silent() { return Err(admit(Admission::SilenceFlag { packet: index })); }
        if silent_payload && options.silence == SilencePolicy::Reject { return Err(admit(Admission::SilentPacket { packet: index })); }
        if header.discontinuity() && options.discontinuity == FlagPolicy::Reject { return Err(admit(Admission::Discontinuity { packet: index })); }
        if !header.timestamps_valid() && options.timestamp_error == FlagPolicy::Reject { return Err(admit(Admission::TimestampError { packet: index })); }
        total = total.checked_add(u64::from(header.frames)).ok_or_else(|| admit(Admission::Extent))?;
        prepared.progress.validated_packets = index + 1;
        prepared.progress.validated_frames = total;
    }
    let packets = u64::try_from(recording.packets.len()).map_err(|_| admit(Admission::Extent))?;
    if total != progress.released_frames || packets != progress.released_packets {
        return Err(admit(Admission::ReleasedTotals { packets, frames: total,
            reported_packets: progress.released_packets, reported_frames: progress.released_frames }));
    }
    let frames = usize::try_from(total).map_err(|_| admit(Admission::Extent))?;
    prepared.mono.try_reserve_exact(frames).map_err(Cause::Allocation)?;
    prepared.progress.stage = Stage::Selection;
    for packet in &recording.packets {
        match &packet.payload {
            Payload::Interleaved(samples) => {
                for frame in samples.chunks_exact(usize::from(channels)) {

                    prepared.mono.push(f32::from_bits(frame[usize::from(options.source_channel)].to_bits()));
                }
            }
            Payload::Silent { .. } => {

                prepared.mono.resize(prepared.mono.len() + packet.header.frames as usize, f32::from_bits(0));
            }
        }
        prepared.progress.selected_packets += 1;
        prepared.progress.selected_frames = prepared.mono.len();
    }
    Ok(())
}
