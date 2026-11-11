use std::io::{self, Write};
use crate::conditional_gate::Batch;

pub enum Error {
    SampleRate(u32),
    Hop,
    Extent,
    RowWidth { row: usize, expected: usize, actual: usize },

    Io { bytes_written: u64, cause: io::Error },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub sample_rate: u32,
    pub frames: u32,
    pub bytes_written: u64,

    pub raw_status: Option<u8>,
}

struct Plan { sample_rate: u32, byte_rate: u32, frames: u32, data_bytes: u32, riff_bytes: u32 }
impl Plan {
    fn new(sample_rate: u32, frames: usize) -> Result<Self, Error> {
        let byte_rate = sample_rate.checked_mul(4).filter(|_| sample_rate != 0)
            .ok_or(Error::SampleRate(sample_rate))?;
        let frames = u32::try_from(frames).map_err(|_| Error::Extent)?;
        let data_bytes = frames.checked_mul(4).ok_or(Error::Extent)?;
        let riff_bytes = data_bytes.checked_add(50).ok_or(Error::Extent)?;
        Ok(Self { sample_rate, byte_rate, frames, data_bytes, riff_bytes })
    }
    fn header(&self) -> [u8; 58] {
        let mut header = [0; 58];
        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&self.riff_bytes.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&18u32.to_le_bytes());
        header[20..22].copy_from_slice(&3u16.to_le_bytes());
        header[22..24].copy_from_slice(&1u16.to_le_bytes());
        header[24..28].copy_from_slice(&self.sample_rate.to_le_bytes());
        header[28..32].copy_from_slice(&self.byte_rate.to_le_bytes());
        header[32..34].copy_from_slice(&4u16.to_le_bytes());
        header[34..36].copy_from_slice(&32u16.to_le_bytes());

        header[38..42].copy_from_slice(b"fact");
        header[42..46].copy_from_slice(&4u32.to_le_bytes());
        header[46..50].copy_from_slice(&self.frames.to_le_bytes());
        header[50..54].copy_from_slice(b"data");
        header[54..58].copy_from_slice(&self.data_bytes.to_le_bytes());
        header
    }
}

fn write_tracked<W: Write + ?Sized>(writer: &mut W, mut bytes: &[u8], written: &mut u64)
    -> Result<(), Error>
{
    while !bytes.is_empty() {
        match writer.write(bytes) {
            Ok(0) => return Err(Error::Io { bytes_written: *written,
                cause: io::Error::new(io::ErrorKind::WriteZero, "WAVE writer made no progress") }),
            Ok(count) if count <= bytes.len() => {
                *written += count as u64;
                bytes = &bytes[count..];
            }
            Ok(_) => return Err(Error::Io { bytes_written: *written,
                cause: io::Error::new(io::ErrorKind::InvalidData, "WAVE writer reported an invalid length") }),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {},
            Err(cause) => return Err(Error::Io { bytes_written: *written, cause }),
        }
    }
    Ok(())
}

fn write_values<'a, W: Write + ?Sized>(writer: &mut W, plan: Plan,
    values: impl Iterator<Item = &'a f32>, raw_status: Option<u8>) -> Result<Receipt, Error>
{
    let mut written = 0;
    write_tracked(writer, &plan.header(), &mut written)?;
    let mut bytes = [0; 4096];
    let mut used = 0;
    for value in values {
        bytes[used..used + 4].copy_from_slice(&value.to_bits().to_le_bytes());
        used += 4;
        if used == bytes.len() {
            write_tracked(writer, &bytes, &mut written)?;
            used = 0;
        }
    }
    write_tracked(writer, &bytes[..used], &mut written)?;
    writer.flush().map_err(|cause| Error::Io { bytes_written: written, cause })?;
    Ok(Receipt { sample_rate: plan.sample_rate, frames: plan.frames,
        bytes_written: written, raw_status })
}

pub fn write_mono_samples<W: Write + ?Sized>(writer: &mut W, sample_rate: u32, samples: &[f32])
    -> Result<Receipt, Error>
{
    let plan = Plan::new(sample_rate, samples.len())?;
    write_values(writer, plan, samples.iter(), None)
}

pub fn write_mono_rows<W: Write + ?Sized>(writer: &mut W, sample_rate: u32,
    hop: usize, batch: &Batch) -> Result<Receipt, Error>
{
    if hop == 0 { return Err(Error::Hop); }
    for (row, values) in batch.rows.iter().enumerate() {
        if values.len() != hop {
            return Err(Error::RowWidth { row, expected: hop, actual: values.len() });
        }
    }
    let frames = batch.rows.len().checked_mul(hop).ok_or(Error::Extent)?;
    let plan = Plan::new(sample_rate, frames)?;
    write_values(writer, plan, batch.rows.iter().flatten(), Some(batch.status))
}
