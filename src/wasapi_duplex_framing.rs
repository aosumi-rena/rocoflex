use crate::wasapi_capture::{self as capture, Header, Packet, Payload};
use std::{collections::TryReserveError, num::NonZeroU32};

#[derive(Clone, Copy, Debug)]
pub struct CaptureSpan { pub header: Header, pub offset_frames: u32, pub frames: u32 }
#[derive(Debug)]
pub struct NativeCursor { pub packet: Packet, pub consumed_frames: u32 }
#[derive(Debug)]
pub enum FramingError { Allocation(TryReserveError), Extent, PacketExtent, Order }
pub(crate) struct ReadyBlock { pub packet: Packet, pub sources: Vec<CaptureSpan> }

pub struct CaptureFraming {
    frames: NonZeroU32,
    channels: usize,
    partial: Vec<f32>,
    sources: Vec<CaptureSpan>,
    all_silent: bool,
    next_ordinal: u64,
    native: Option<NativeCursor>,
}
impl CaptureFraming {
    pub(crate) fn new(frames: NonZeroU32, channels: u16) -> Self {
        Self { frames, channels: usize::from(channels), partial: Vec::new(), sources: Vec::new(),
            all_silent: true, next_ordinal: 0, native: None }
    }
    pub fn callback_frames(&self) -> u32 { self.frames.get() }
    pub fn partial_samples(&self) -> &[f32] { &self.partial }
    pub fn partial_sources(&self) -> &[CaptureSpan] { &self.sources }
    pub fn native_cursor(&self) -> Option<&NativeCursor> { self.native.as_ref() }
    pub fn buffered_frames(&self) -> usize {
        self.partial.len() / self.channels + self.native.as_ref()
            .map_or(0, |p| (p.packet.header.frames - p.consumed_frames) as usize)
    }
    pub(crate) fn reserve(&mut self) -> Result<(), FramingError> {
        let words = (self.frames.get() as usize).checked_mul(self.channels)
            .filter(|n| *n <= isize::MAX as usize / 4).ok_or(FramingError::Extent)?;
        self.partial.try_reserve(words.saturating_sub(self.partial.len())).map_err(FramingError::Allocation)
    }
    pub(crate) fn accept(&mut self, packet: Packet) -> Result<(), FramingError> {
        if self.native.is_some() { return Err(FramingError::Order); }
        self.native = Some(NativeCursor { packet, consumed_frames: 0 });
        Ok(())
    }
    pub(crate) fn next_block(&mut self) -> Result<Option<ReadyBlock>, FramingError> {
        self.reserve()?;
        let Some(cursor) = self.native.as_mut() else { return Ok(None); };
        let expected = (cursor.packet.header.frames as usize).checked_mul(self.channels).ok_or(FramingError::Extent)?;
        let actual = match &cursor.packet.payload {
            Payload::Interleaved(v) => v.len(), Payload::Silent { sample_count } => *sample_count,
        };
        if actual != expected || cursor.consumed_frames > cursor.packet.header.frames { return Err(FramingError::PacketExtent); }
        let available = cursor.packet.header.frames - cursor.consumed_frames;
        let needed = self.frames.get() as usize - self.partial.len() / self.channels;
        let count = needed.min(available as usize);
        if count != 0 {
            self.sources.try_reserve(1).map_err(FramingError::Allocation)?;
            let first = cursor.consumed_frames as usize * self.channels;
            let words = count * self.channels;
            match &cursor.packet.payload {
                Payload::Interleaved(v) => { self.partial.extend_from_slice(&v[first..first + words]); self.all_silent = false; }
                Payload::Silent { .. } => self.partial.resize(self.partial.len() + words, 0.0),
            }
            self.sources.push(CaptureSpan { header: cursor.packet.header, offset_frames: cursor.consumed_frames, frames: count as u32 });
            cursor.consumed_frames += count as u32;
        }
        if cursor.consumed_frames == cursor.packet.header.frames { self.native = None; }
        if self.partial.len() / self.channels != self.frames.get() as usize { return Ok(None); }
        let next = self.next_ordinal.checked_add(1).ok_or(FramingError::Extent)?;
        let first = self.sources.first().ok_or(FramingError::Order)?;
        let first_frame = first.header.first_frame.checked_add(u64::from(first.offset_frames)).ok_or(FramingError::Extent)?;
        let mut header = first.header;

        header.ordinal = self.next_ordinal; header.first_frame = first_frame; header.frames = self.frames.get();
        header.flags = self.sources.iter().fold(0, |flags, s| flags | s.header.flags);
        let payload = if self.all_silent {
            self.partial.clear(); Payload::Silent { sample_count: self.frames.get() as usize * self.channels }
        } else {

            header.flags &= !(windows::Win32::Media::Audio::AUDCLNT_BUFFERFLAGS_SILENT.0 as u32);
            Payload::Interleaved(std::mem::take(&mut self.partial))
        };
        let sources = std::mem::take(&mut self.sources);
        self.next_ordinal = next; self.all_silent = true;
        Ok(Some(ReadyBlock { packet: capture::Packet { header, payload }, sources }))
    }
}
