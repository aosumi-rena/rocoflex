use crate::{logical_read_rows::ReadRows, wasapi_capture as capture,
    wasapi_recording::FlagPolicy};
use std::{collections::{TryReserveError, VecDeque}, num::{NonZeroU32, NonZeroUsize}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SilencePolicy { Reject, MaterializeFlaggedPositiveZero }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndResidualPolicy { Reject, PreserveUnpadded }
#[derive(Clone, Copy, Debug)]
pub struct Configuration {

    pub source_rate: NonZeroU32,
    pub hop: NonZeroUsize,
    pub source_channel: u16,
    pub queued_rows: NonZeroUsize,
    pub packet_receipts: NonZeroUsize,

    pub first_packet: u64,
    pub first_frame: u64,
    pub silence: SilencePolicy,
    pub discontinuity: FlagPolicy,
    pub timestamp_error: FlagPolicy,
    pub end_residual: EndResidualPolicy,
}
#[derive(Debug)]
pub enum InitializationCause {
    Rate { engine: u32, client: u32 }, Channel { selected: u16, channels: u16 },
    HopExtent, ZeroCaptureCapacity,
}
#[derive(Debug)]
pub struct InitializationFailure {
    pub configuration: Configuration, pub info: capture::Info, pub cause: InitializationCause,
}
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub accepted_samples: u64,
    pub completed_rows: u64,
    pub next_packet: u64,
    pub next_frame: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct Receipt {
    pub header: capture::Header,

    pub released: capture::Progress,
    pub first_sample: u64,
    pub accepted_frames: usize,
}
pub struct State {
    configuration: Configuration,
    info: capture::Info,
    progress: Progress,
    rows: VecDeque<Vec<f32>>,
    partial: Vec<f32>,
    receipts: VecDeque<Receipt>,
    first_queued_row: u64,
}
#[derive(Debug)]
pub enum Cause {
    Sequence { expected_packet: u64, expected_frame: u64 }, ReleaseReceipt,
    FrameCount { frames: u32, capacity: u32 }, Payload, Extent,
    Discontinuity, TimestampError, SilentPacket,
    ReceiptLimit { limit: usize }, RowLimit { limit: usize },
    Allocation { at: AllocationSite, cause: TryReserveError },
}
#[derive(Clone, Copy, Debug)]
pub enum AllocationSite { Receipt, RowQueue, PartialRow }

pub struct Failure { state: State, packet: capture::Packet, released: capture::Progress, cause: Cause }
impl Failure {
    pub fn state(&self) -> &State { &self.state }
    pub fn packet(&self) -> &capture::Packet { &self.packet }
    pub fn released(&self) -> capture::Progress { self.released }
    pub fn cause(&self) -> &Cause { &self.cause }

    pub fn accepted_frames(&self) -> usize {
        self.state.receipts.back().filter(|receipt| same_header(receipt.header, self.packet.header))
            .map_or(0, |receipt| receipt.accepted_frames)
    }
}
pub struct EndFailure { state: State, status: u8 }
impl EndFailure {
    pub fn state(&self) -> &State { &self.state }
    pub fn supplied_status(&self) -> u8 { self.status }
}

pub struct Rows { rows: VecDeque<Vec<f32>>, status: u8, first_row: u64 }
impl Rows {
    pub fn first_row(&self) -> u64 { self.first_row }
    pub fn into_parts(self) -> (VecDeque<Vec<f32>>, u8, u64) { (self.rows, self.status, self.first_row) }
}
impl ReadRows for Rows {
    fn row_count(&self) -> usize { self.rows.len() }
    fn source_status(&self) -> u8 { self.status }
    fn read_row(&self, index: usize) -> Option<&[f32]> { self.rows.get(index).map(Vec::as_slice) }
}
pub struct Finished {
    pub configuration: Configuration,
    pub info: capture::Info,
    pub progress: Progress,
    pub rows: Rows,

    pub residual: Vec<f32>,
    pub receipts: VecDeque<Receipt>,
}
impl State {
    pub fn new(configuration: Configuration, info: capture::Info) -> Result<Self, InitializationFailure> {
        let channels = info.channels.count();
        let cause = if configuration.source_rate.get() != info.sample_rate {
            Some(InitializationCause::Rate { engine: configuration.source_rate.get(), client: info.sample_rate })
        } else if configuration.source_channel >= channels {
            Some(InitializationCause::Channel { selected: configuration.source_channel, channels })
        } else if configuration.hop.get() > i32::MAX as usize
            || configuration.hop.get().checked_mul(std::mem::size_of::<f32>())
                .is_none_or(|bytes| bytes > isize::MAX as usize) {
            Some(InitializationCause::HopExtent)
        } else if info.buffer_frames == 0 { Some(InitializationCause::ZeroCaptureCapacity) }
        else { None };
        if let Some(cause) = cause { return Err(InitializationFailure { configuration, info, cause }); }
        Ok(Self { progress: Progress { accepted_samples: 0, completed_rows: 0,
                next_packet: configuration.first_packet, next_frame: configuration.first_frame },
            configuration, info, rows: VecDeque::new(), partial: Vec::new(),
            receipts: VecDeque::new(), first_queued_row: 0 })
    }
    pub fn configuration(&self) -> Configuration { self.configuration }
    pub fn info(&self) -> &capture::Info { &self.info }
    pub fn progress(&self) -> Progress { self.progress }
    pub fn queued_rows(&self) -> &VecDeque<Vec<f32>> { &self.rows }
    pub fn partial(&self) -> &[f32] { &self.partial }
    pub fn receipts(&self) -> &VecDeque<Receipt> { &self.receipts }

    pub fn push(mut self, packet: capture::Packet, released: capture::Progress) -> Result<Self, Failure> {
        if let Err(cause) = self.append(&packet, released) {
            return Err(Failure { state: self, packet, released, cause });
        }
        Ok(self)
    }

    fn append(&mut self, packet: &capture::Packet, released: capture::Progress) -> Result<(), Cause> {
        let h = packet.header;
        if h.ordinal != self.progress.next_packet || h.first_frame != self.progress.next_frame {
            return Err(Cause::Sequence { expected_packet: self.progress.next_packet, expected_frame: self.progress.next_frame });
        }
        let next_packet = h.ordinal.checked_add(1).ok_or(Cause::Extent)?;
        let next_frame = h.first_frame.checked_add(u64::from(h.frames)).ok_or(Cause::Extent)?;
        let accepted_end = self.progress.accepted_samples.checked_add(u64::from(h.frames)).ok_or(Cause::Extent)?;
        let Some(attempt) = released.packet else { return Err(Cause::ReleaseReceipt); };
        let Some(reported) = attempt.header else { return Err(Cause::ReleaseReceipt); };
        if !same_header(h, reported) || attempt.stage != capture::PacketStage::Released
            || !attempt.release_succeeded || attempt.release_frames != Some(h.frames)
            || released.released_packets != next_packet || released.released_frames != next_frame
            || released.failure.is_some() {
            return Err(Cause::ReleaseReceipt);
        }
        if h.frames == 0 || h.frames > self.info.buffer_frames {
            return Err(Cause::FrameCount { frames: h.frames, capacity: self.info.buffer_frames });
        }
        let frames = usize::try_from(h.frames).map_err(|_| Cause::Extent)?;
        let channels = usize::from(self.info.channels.count());
        let count = frames.checked_mul(channels).ok_or(Cause::Extent)?;
        match &packet.payload {
            capture::Payload::Interleaved(values) if !h.silent() && values.len() == count => {},
            capture::Payload::Silent { sample_count } if h.silent() && *sample_count == count => {},
            _ => return Err(Cause::Payload),
        }
        if h.discontinuity() && self.configuration.discontinuity == FlagPolicy::Reject { return Err(Cause::Discontinuity); }
        if !h.timestamps_valid() && self.configuration.timestamp_error == FlagPolicy::Reject { return Err(Cause::TimestampError); }
        if h.silent() && self.configuration.silence == SilencePolicy::Reject { return Err(Cause::SilentPacket); }
        if self.receipts.len() >= self.configuration.packet_receipts.get() {
            return Err(Cause::ReceiptLimit { limit: self.configuration.packet_receipts.get() });
        }
        self.receipts.try_reserve(1).map_err(|cause| Cause::Allocation { at: AllocationSite::Receipt, cause })?;
        self.receipts.push_back(Receipt { header: h, released,
            first_sample: self.progress.accepted_samples, accepted_frames: 0 });
        let hop = self.configuration.hop.get();
        let mut cursor = 0;
        while cursor < frames {
            let take = (frames - cursor).min(hop - self.partial.len());
            let full = self.partial.len() + take == hop;
            let next_rows = if full {
                if self.rows.len() >= self.configuration.queued_rows.get() {
                    return Err(Cause::RowLimit { limit: self.configuration.queued_rows.get() });
                }
                let next = self.progress.completed_rows.checked_add(1).ok_or(Cause::Extent)?;
                self.rows.try_reserve(1).map_err(|cause| Cause::Allocation { at: AllocationSite::RowQueue, cause })?;
                Some(next)
            } else { None };

            self.partial.try_reserve(hop - self.partial.len())
                .map_err(|cause| Cause::Allocation { at: AllocationSite::PartialRow, cause })?;
            for frame in cursor..cursor + take {
                let bits = match &packet.payload {
                    capture::Payload::Interleaved(values) => values[frame * channels + usize::from(self.configuration.source_channel)].to_bits(),
                    capture::Payload::Silent { .. } => 0,
                };
                self.partial.push(f32::from_bits(bits));
            }
            cursor += take;
            self.progress.accepted_samples += take as u64;
            self.receipts.back_mut().expect("reserved current packet receipt").accepted_frames = cursor;
            if let Some(next) = next_rows {
                self.rows.push_back(std::mem::take(&mut self.partial));
                self.progress.completed_rows = next;
            }
        }

        self.progress.accepted_samples = accepted_end;
        self.progress.next_packet = next_packet;
        self.progress.next_frame = next_frame;
        Ok(())
    }

    pub fn take_rows(&mut self, status: u8) -> Rows {
        let rows = Rows { rows: std::mem::take(&mut self.rows), status, first_row: self.first_queued_row };
        self.first_queued_row = self.progress.completed_rows;
        rows
    }
    pub fn take_receipts(&mut self) -> VecDeque<Receipt> { std::mem::take(&mut self.receipts) }

    pub fn finish(self, status: u8) -> Result<Finished, EndFailure> {
        if !self.partial.is_empty() && self.configuration.end_residual == EndResidualPolicy::Reject {
            return Err(EndFailure { state: self, status });
        }
        Ok(Finished { configuration: self.configuration, info: self.info, progress: self.progress,
            rows: Rows { rows: self.rows, status, first_row: self.first_queued_row },
            residual: self.partial, receipts: self.receipts })
    }
}

fn same_header(a: capture::Header, b: capture::Header) -> bool {
    a.ordinal == b.ordinal && a.first_frame == b.first_frame && a.frames == b.frames
        && a.flags == b.flags && a.device_position == b.device_position
        && a.qpc_position_100ns == b.qpc_position_100ns
}
