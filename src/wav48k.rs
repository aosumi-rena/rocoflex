pub const SAMPLE_RATE: u32 = 48_000;
pub const MAX_FRAMES: usize = 300 * SAMPLE_RATE as usize;

#[path = "wav_source_container.rs"]
pub(crate) mod source_container;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Pcm8,
    Pcm16,
    Pcm24,
    Pcm32,
    Float32,
}

impl Encoding {
    fn bytes(self) -> usize {
        match self {
            Self::Pcm8 => 1,
            Self::Pcm16 => 2,
            Self::Pcm24 => 3,
            Self::Pcm32 | Self::Float32 => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    ShortHeader,
    UnsupportedContainer,
    NotWave,
    DeclaredSizeMismatch,
    TruncatedChunkHeader,
    ChunkOutOfBounds,
    MissingFmt,
    MissingData,
    DataBeforeFmt,
    DuplicateFmt,
    DuplicateData,
    UnsupportedFmtSize,
    UnsupportedFormat { tag: u16, bits: u16 },
    UnsupportedSampleRate(u32),
    InvalidChannels,
    UnsupportedFrameBytes(usize),
    InconsistentBlockAlign { encoded: u16, derived: usize },
    InconsistentByteRate,
    IncompleteFrame,
    TooLong(usize),
    Overflow,
    Allocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub source_channels: u16,
    pub sample_rate: u32,
    pub encoding: Encoding,
    pub frame_bytes: usize,
    pub frames: usize,
    pub data_offset: usize,
    pub data_bytes: usize,
}

#[derive(Debug)]
pub struct Decoded {
    pub metadata: Metadata,

    pub channel0: Vec<f32>,
}

#[derive(Clone, Copy)]
struct Format {
    channels: u16,
    sample_rate: u32,
    encoding: Encoding,
    frame_bytes: usize,
}

fn u16le(v: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([v[offset], v[offset + 1]])
}

fn u32le(v: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([v[offset], v[offset + 1], v[offset + 2], v[offset + 3]])
}

fn parse_format(
    v: &[u8], include_resampled_rates: bool, expected_source_rate: Option<u32>,
) -> Result<Format, Error> {

    if v.len() != 16 && !(v.len() == 18 && u16le(v, 16) == 0) {
        return Err(Error::UnsupportedFmtSize);
    }
    let tag = u16le(v, 0);
    let bits = u16le(v, 14);
    let encoding = match (tag, bits) {
        (1, 8) => Encoding::Pcm8,
        (1, 16) => Encoding::Pcm16,
        (1, 24) => Encoding::Pcm24,
        (1, 32) => Encoding::Pcm32,
        (3, 32) => Encoding::Float32,
        _ => return Err(Error::UnsupportedFormat { tag, bits }),
    };
    let channels = u16le(v, 2);
    if channels == 0 {
        return Err(Error::InvalidChannels);
    }
    let rate = u32le(v, 4);
    if let Some(expected) = expected_source_rate {
        if rate != expected { return Err(Error::UnsupportedSampleRate(rate)); }
    } else if rate != SAMPLE_RATE
        && !(include_resampled_rates
            && matches!(rate, 4_000 | 6_000 | 8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000
                | 44_100 | 64_000 | 72_000 | 88_200 | 96_000 | 144_000
                | 176_400 | 192_000 | 288_000 | 352_800 | 384_000))
    {
        return Err(Error::UnsupportedSampleRate(rate));
    }

    let frame_bytes = usize::from(channels)
        .checked_mul(encoding.bytes())
        .ok_or(Error::Overflow)?;
    if frame_bytes > 5760 {
        return Err(Error::UnsupportedFrameBytes(frame_bytes));
    }

    let encoded = u16le(v, 12);
    if usize::from(encoded) != frame_bytes {
        return Err(Error::InconsistentBlockAlign { encoded, derived: frame_bytes });
    }
    let expected_rate = rate
        .checked_mul(frame_bytes as u32)
        .ok_or(Error::Overflow)?;
    if u32le(v, 8) != expected_rate {
        return Err(Error::InconsistentByteRate);
    }
    Ok(Format { channels, sample_rate: rate, encoding, frame_bytes })
}

fn check_frames(frames: usize, sample_rate: u32) -> Result<(), Error> {
    if frames > 300 * sample_rate as usize { Err(Error::TooLong(frames)) } else { Ok(()) }
}

pub fn inspect(bytes: &[u8]) -> Result<Metadata, Error> {
    inspect_domain(bytes, false)
}

pub fn inspect_supported(bytes: &[u8]) -> Result<Metadata, Error> {
    inspect_domain(bytes, true)
}

fn inspect_domain(bytes: &[u8], include_resampled_rates: bool) -> Result<Metadata, Error> {
    inspect_rate_domain(bytes, include_resampled_rates, None)
}

pub fn inspect_source_plan(bytes: &[u8], expected_source_rate: u32) -> Result<Metadata, Error> {
    if expected_source_rate == 0 || expected_source_rate > 384_000 {
        return Err(Error::UnsupportedSampleRate(expected_source_rate));
    }
    inspect_rate_domain(bytes, false, Some(expected_source_rate))
}

fn inspect_rate_domain(
    bytes: &[u8], include_resampled_rates: bool, expected_source_rate: Option<u32>,
) -> Result<Metadata, Error> {
    if bytes.len() < 12 {
        return Err(Error::ShortHeader);
    }
    if &bytes[..4] != b"RIFF" {
        return Err(Error::UnsupportedContainer);
    }
    if &bytes[8..12] != b"WAVE" {
        return Err(Error::NotWave);
    }
    let end = (u32le(bytes, 4) as usize).checked_add(8).ok_or(Error::Overflow)?;
    if end != bytes.len() {
        return Err(Error::DeclaredSizeMismatch);
    }
    let mut cursor = 12_usize;
    let mut format = None;
    let mut data = None;
    while cursor < end {
        let start = cursor.checked_add(8).ok_or(Error::Overflow)?;
        if start > end {
            return Err(Error::TruncatedChunkHeader);
        }
        let size = u32le(bytes, cursor + 4) as usize;
        let payload_end = start.checked_add(size).ok_or(Error::Overflow)?;
        let padded_end = payload_end.checked_add(size & 1).ok_or(Error::Overflow)?;
        if padded_end > end {
            return Err(Error::ChunkOutOfBounds);
        }
        match &bytes[cursor..cursor + 4] {
            b"fmt " => {
                if format.is_some() {
                    return Err(Error::DuplicateFmt);
                }
                format = Some(parse_format(
                    &bytes[start..payload_end], include_resampled_rates, expected_source_rate,
                )?);
            }
            b"data" => {
                if format.is_none() {
                    return Err(Error::DataBeforeFmt);
                }
                if data.is_some() {
                    return Err(Error::DuplicateData);
                }
                data = Some((start, size));
            }
            _ => {}
        }
        cursor = padded_end;
    }
    let f = format.ok_or(Error::MissingFmt)?;
    let (data_offset, data_bytes) = data.ok_or(Error::MissingData)?;
    if data_bytes % f.frame_bytes != 0 {
        return Err(Error::IncompleteFrame);
    }
    let frames = data_bytes / f.frame_bytes;
    check_frames(frames, f.sample_rate)?;
    Ok(Metadata {
        source_channels: f.channels,
        sample_rate: f.sample_rate,
        encoding: f.encoding,
        frame_bytes: f.frame_bytes,
        frames,
        data_offset,
        data_bytes,
    })
}

fn channel_sample(v: &[u8], encoding: Encoding) -> f32 {
    let word = match encoding {
        Encoding::Pcm8 => (u32::from(v[0] ^ 0x80) << 24) as i32,
        Encoding::Pcm16 => (u32::from(u16le(v, 0)) << 16) as i32,
        Encoding::Pcm24 => ((u32::from(v[0]) | (u32::from(v[1]) << 8)
            | (u32::from(v[2]) << 16)) << 8) as i32,
        Encoding::Pcm32 => u32le(v, 0) as i32,
        Encoding::Float32 => return f32::from_bits(u32le(v, 0)),
    };

    (word as f32) * f32::from_bits(0x30000000)
}

pub fn decode(bytes: &[u8]) -> Result<Decoded, Error> {
    let metadata = inspect(bytes)?;
    decode_metadata(bytes, metadata)
}

pub fn decode_supported(bytes: &[u8]) -> Result<Decoded, Error> {
    let metadata = inspect_supported(bytes)?;
    decode_metadata(bytes, metadata)
}

fn decode_metadata(bytes: &[u8], metadata: Metadata) -> Result<Decoded, Error> {
    let mut channel0 = Vec::new();
    channel0.try_reserve_exact(metadata.frames).map_err(|_| Error::Allocation)?;
    let payload = &bytes[metadata.data_offset..metadata.data_offset + metadata.data_bytes];
    for frame in payload.chunks_exact(metadata.frame_bytes) {
        channel0.push(channel_sample(frame, metadata.encoding));
    }
    Ok(Decoded { metadata, channel0 })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct Channel0Read {
    pub source_start: usize,
    pub requested_frames: usize,
    pub decoded_frames: usize,
    pub zero_filled_frames: usize,
    pub next_source_position: usize,
}

#[derive(Clone)]
pub(crate) struct Channel0Cursor<'a> {
    metadata: Metadata,
    payload: &'a [u8],
    source_position: usize,
}

impl<'a> Channel0Cursor<'a> {
    #[allow(dead_code)]
    pub(crate) fn from_supported(bytes: &'a [u8]) -> Result<Self, Error> {
        let metadata = inspect_supported(bytes)?;
        Self::from_metadata(bytes, metadata)
    }

    pub(crate) fn from_source_plan(bytes: &'a [u8], expected_source_rate: u32) -> Result<Self, Error> {
        let metadata = inspect_source_plan(bytes, expected_source_rate)?;
        Self::from_metadata(bytes, metadata)
    }

    fn from_metadata(bytes: &'a [u8], metadata: Metadata) -> Result<Self, Error> {
        let end = metadata.data_offset.checked_add(metadata.data_bytes).ok_or(Error::Overflow)?;
        let payload = bytes.get(metadata.data_offset..end).ok_or(Error::ChunkOutOfBounds)?;
        Ok(Self { metadata, payload, source_position: 0 })
    }

    pub(crate) fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub(crate) fn source_position(&self) -> usize {
        self.source_position
    }

    pub(crate) fn first_nonfinite_frame(&self) -> Option<usize> {
        if self.metadata.encoding != Encoding::Float32 { return None; }

        self.payload.chunks_exact(self.metadata.frame_bytes)
            .position(|frame| channel_sample(frame, self.metadata.encoding).to_bits()
                & 0x7f80_0000 == 0x7f80_0000)
    }

    pub(crate) fn rewind(&mut self) {
        self.source_position = 0;
    }

    pub(crate) fn read_into(&mut self, output: &mut [f32]) -> Result<Channel0Read, Error> {
        if output.len() > i32::MAX as usize { return Err(Error::Overflow); }
        let next_source_position = self.source_position.checked_add(output.len())
            .ok_or(Error::Overflow)?;
        let begin = self.source_position.min(self.metadata.frames);
        let available = output.len().min(self.metadata.frames - begin);
        let byte_begin = begin.checked_mul(self.metadata.frame_bytes).ok_or(Error::Overflow)?;
        let byte_count = available.checked_mul(self.metadata.frame_bytes).ok_or(Error::Overflow)?;
        let byte_end = byte_begin.checked_add(byte_count).ok_or(Error::Overflow)?;
        let input = self.payload.get(byte_begin..byte_end).ok_or(Error::ChunkOutOfBounds)?;
        for (sample, frame) in output[..available].iter_mut()
            .zip(input.chunks_exact(self.metadata.frame_bytes))
        {
            *sample = channel_sample(frame, self.metadata.encoding);
        }

        output[available..].fill(0.0);
        let state = Channel0Read {
            source_start: self.source_position, requested_frames: output.len(),
            decoded_frames: available, zero_filled_frames: output.len() - available,
            next_source_position,
        };
        self.source_position = next_source_position;
        Ok(state)
    }
}

