#![allow(dead_code)]

use super::{Channel0Cursor, Encoding, Metadata};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Container { Riff, Rf64 }

#[derive(Clone, Copy, Debug)]
pub(crate) struct Ds64 {
    pub riff_bytes: u64,
    pub data_bytes: u64,
    pub ignored_sample_count: u64,
    pub ignored_table_entry_count: u32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Extensible {
    pub ignored_extension_size: u16,
    pub ignored_valid_bits: u16,
    pub channel_mask: u32,
    pub subformat: [u8; 16],
}

#[derive(Clone, Debug)]
pub(crate) enum Error {
    Format(super::Error),
    OwnGuard(&'static str),
    RequiredCodec(u16),
    RequiredStorage { tag: u16, bits: u16 },
    UnknownSubformat([u8; 16]),
}

pub(crate) struct Schema<'a> {
    metadata: Metadata,
    payload: &'a [u8],
    container: Container,
    ds64: Option<Ds64>,
    extensible: Option<Extensible>,
}

impl<'a> Schema<'a> {
    pub(crate) fn metadata(&self) -> &Metadata { &self.metadata }
    pub(crate) fn container(&self) -> Container { self.container }
    pub(crate) fn ds64(&self) -> Option<Ds64> { self.ds64 }
    pub(crate) fn extensible(&self) -> Option<Extensible> { self.extensible }

    pub(crate) fn into_cursor(self) -> Channel0Cursor<'a> {
        Channel0Cursor { metadata: self.metadata, payload: self.payload, source_position: 0 }
    }
}

fn guard(message: &'static str) -> Error { Error::OwnGuard(message) }

fn u64le(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes([
        bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3],
        bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7],
    ])
}

fn extent(start: usize, size: u64, limit: usize) -> Result<(usize, usize), Error> {
    let size = usize::try_from(size).map_err(|_| guard("own addressable chunk size required"))?;
    let end = start.checked_add(size).ok_or_else(|| guard("own chunk end overflow"))?;
    let padded = end.checked_add(size & 1).ok_or_else(|| guard("own chunk padding overflow"))?;
    if padded > limit { return Err(guard("own physically complete chunk required")); }
    Ok((end, padded))
}

struct Format {
    channels: u16,
    rate: u32,
    frame_bytes: usize,
    encoding: Encoding,
    extensible: Option<Extensible>,
}

fn format(bytes: &[u8], expected: u32) -> Result<Format, Error> {
    if bytes.len() < 16 { return Err(guard("own complete fixed fmt fields required")); }
    let tag = super::u16le(bytes, 0);
    let channels = super::u16le(bytes, 2);
    let rate = super::u32le(bytes, 4);
    let bits = super::u16le(bytes, 14);
    if rate != expected { return Err(Error::Format(super::Error::UnsupportedSampleRate(rate))); }
    if channels == 0 { return Err(Error::Format(super::Error::InvalidChannels)); }
    let (float, extensible) = match tag {
        1 => (false, None),
        3 => (true, None),
        0xfffe => {
            if bytes.len() < 40 { return Err(guard("extensible fmt requires at least40 bytes")); }
            let mut guid = [0; 16];
            guid.copy_from_slice(&bytes[24..40]);
            let float_guid = [3,0,0,0,0,0,16,0,128,0,0,170,0,56,155,113];
            let pcm_guid = [1,0,0,0,0,0,16,0,128,0,0,170,0,56,155,113];
            let alternate_pcm_guid = [1,0,0,0,33,7,211,17,134,68,200,193,202,0,0,0];
            let float = if guid == float_guid { true }
                else if guid == pcm_guid || guid == alternate_pcm_guid { false }
                else { return Err(Error::UnknownSubformat(guid)); };
            (float, Some(Extensible {
                ignored_extension_size: super::u16le(bytes, 16),
                ignored_valid_bits: super::u16le(bytes, 18),
                channel_mask: super::u32le(bytes, 20), subformat: guid,
            }))
        }
        _ => return Err(Error::RequiredCodec(tag)),
    };
    let encoding = match (float, bits) {
        (false, 8) => Encoding::Pcm8,
        (false, 16) => Encoding::Pcm16,
        (false, 24) => Encoding::Pcm24,
        (false, 32) => Encoding::Pcm32,
        (true, 32) => Encoding::Float32,
        _ => return Err(Error::RequiredStorage { tag, bits }),
    };
    let frame_bytes = usize::from(channels).checked_mul(encoding.bytes())
        .ok_or_else(|| guard("own frame width overflow"))?;
    if frame_bytes > 5760 {
        return Err(Error::Format(super::Error::UnsupportedFrameBytes(frame_bytes)));
    }

    let align = super::u16le(bytes, 12);
    if usize::from(align) != frame_bytes {
        return Err(Error::Format(super::Error::InconsistentBlockAlign { encoded: align, derived: frame_bytes }));
    }
    let byte_rate = rate.checked_mul(frame_bytes as u32).ok_or_else(|| guard("own byte rate overflow"))?;
    if super::u32le(bytes, 8) != byte_rate {
        return Err(Error::Format(super::Error::InconsistentByteRate));
    }
    Ok(Format { channels, rate, frame_bytes, encoding, extensible })
}

pub(crate) fn admit(bytes: &[u8], expected_rate: u32) -> Result<Schema<'_>, Error> {
    if expected_rate == 0 || expected_rate > 384_000 {
        return Err(Error::Format(super::Error::UnsupportedSampleRate(expected_rate)));
    }
    if bytes.len() < 12 { return Err(Error::Format(super::Error::ShortHeader)); }
    let container = match &bytes[..4] {
        b"RIFF" => Container::Riff,
        b"RF64" => Container::Rf64,
        _ => return Err(Error::Format(super::Error::UnsupportedContainer)),
    };
    if &bytes[8..12] != b"WAVE" { return Err(Error::Format(super::Error::NotWave)); }
    let mut cursor = 12usize;
    let ds64 = if container == Container::Rf64 {
        if bytes.len() < 20 || &bytes[12..16] != b"ds64" {
            return Err(guard("RF64 requires initial ds64 for this source branch"));
        }
        let size = u64::from(super::u32le(bytes, 16));
        if size < 28 { return Err(guard("ds64 requires at least28 bytes")); }
        let (_, padded) = extent(20, size, bytes.len())?;
        cursor = padded;
        Some(Ds64 {
            riff_bytes: u64le(bytes, 20), data_bytes: u64le(bytes, 28),
            ignored_sample_count: u64le(bytes, 36),
            ignored_table_entry_count: super::u32le(bytes, 44),
        })
    } else { None };
    let riff_bytes = match ds64 {
        Some(value) => value.riff_bytes,
        None => u64::from(super::u32le(bytes, 4)),
    };
    let declared_end = riff_bytes.checked_add(8).ok_or_else(|| guard("own container end overflow"))?;
    if declared_end > i64::MAX as u64 || declared_end != bytes.len() as u64 {
        return Err(guard("own exact physical container extent required"));
    }
    if cursor > bytes.len() { return Err(guard("own initial chunk exceeds container")); }
    let mut fmt = None;
    let mut data = None;
    while cursor < bytes.len() {
        let start = cursor.checked_add(8).ok_or_else(|| guard("own chunk header overflow"))?;
        if start > bytes.len() { return Err(Error::Format(super::Error::TruncatedChunkHeader)); }
        let id = &bytes[cursor..cursor + 4];
        let size32 = super::u32le(bytes, cursor + 4);
        let size = if id == b"data" {
            if let Some(value) = ds64 {
                if value.data_bytes > i64::MAX as u64 { return Err(guard("own nonnegative signed data size required")); }
                if value.data_bytes == 0 && size32 != 0 {
                    return Err(guard("own zero RF64 data subset requires zero encoded chunk size"));
                }
                value.data_bytes
            } else { u64::from(size32) }
        } else { u64::from(size32) };
        let (end, padded) = extent(start, size, bytes.len())?;
        match id {
            b"fmt " => {
                if fmt.is_some() { return Err(guard("own unique fmt required; native float flag can be sticky")); }
                fmt = Some(format(&bytes[start..end], expected_rate)?);
            }
            b"data" => {
                if fmt.is_none() { return Err(Error::Format(super::Error::DataBeforeFmt)); }
                if data.is_some() { return Err(guard("own unique data required")); }
                data = Some((start, end - start));
            }
            b"ds64" => return Err(guard("own initial-only ds64 subset required")),
            _ => {}
        }
        cursor = padded;
    }
    let fmt = fmt.ok_or(Error::Format(super::Error::MissingFmt))?;
    let (data_offset, data_bytes) = data.ok_or(Error::Format(super::Error::MissingData))?;
    if data_bytes % fmt.frame_bytes != 0 { return Err(Error::Format(super::Error::IncompleteFrame)); }
    let frames = data_bytes / fmt.frame_bytes;
    super::check_frames(frames, fmt.rate).map_err(Error::Format)?;
    let metadata = Metadata {
        source_channels: fmt.channels, sample_rate: fmt.rate, encoding: fmt.encoding,
        frame_bytes: fmt.frame_bytes, frames, data_offset, data_bytes,
    };
    Ok(Schema {
        metadata, payload: &bytes[data_offset..data_offset + data_bytes], container, ds64,
        extensible: fmt.extensible,
    })
}
