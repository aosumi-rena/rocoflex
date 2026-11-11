use crate::{Result, codec};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct FactoryPoint {
    pub data: [f32; 128],
    pub reduced: [f32; 4],
}

#[derive(Clone)]
pub struct FactoryVoice {
    pub name: String,
    pub pitch_stats: [f32; 5],
    pub data: [f32; 128],
    pub points: Vec<FactoryPoint>,
}

pub struct FactoryLibrary {
    pub voices: Vec<FactoryVoice>,
}

impl FactoryVoice {
    pub fn source_document(&self) -> Result<Value> {
        let points = self
            .points
            .iter()
            .map(|point| {
                Ok(json!({
                    "type": "point", "name": "", "enabled": true,
                    "data": codec::encode(&point.data)?,
                    "reducedData": codec::encode(&point.reduced)?,
                }))
            })
            .collect::<Result<Vec<Value>>>()?;
        let [q10, q25, q50, q75, q90] = self.pitch_stats;
        Ok(json!({
            "type": "trajectory", "name": self.name, "enabled": true,
            "data": codec::encode(&self.data)?, "points": points,
            "pitchStats": {"q10":q10, "q25":q25, "q50":q50, "q75":q75, "q90":q90},
        }))
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn take(&mut self, len: usize) -> Result<&[u8]> {
        let end = self
            .position
            .checked_add(len)
            .ok_or("Catalog size overflow")?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or("Truncated voice catalog")?;
        self.position = end;
        Ok(bytes)
    }

    fn floats<const N: usize>(&mut self, limit: f32) -> Result<[f32; N]> {
        let bytes = self.take(N * 4)?;
        let mut values = [0.0; N];
        for (dst, raw) in values.iter_mut().zip(bytes.chunks_exact(4)) {
            *dst = f32::from_le_bytes(raw.try_into().unwrap());
            if !dst.is_finite() || dst.abs() > limit {
                return Err("Invalid voice catalog value".into());
            }
        }
        Ok(values)
    }
}

fn decode(bytes: &[u8]) -> Result<(FactoryLibrary, usize)> {
    let mut reader = Reader { bytes, position: 0 };
    let count = reader.take(1)?[0];
    if count == 0 || count > 127 {
        return Err("Unsupported voice catalog count".into());
    }
    let mut voices = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let point_count = reader.take(1)?[0];
        if point_count == 0 || point_count > 127 {
            return Err("Unsupported factory voice point count".into());
        }
        let pitch_stats = reader.floats::<5>(256.0)?;
        if pitch_stats.windows(2).any(|v| v[0] > v[1]) {
            return Err("Unordered factory pitch statistics".into());
        }
        let data = reader.floats::<128>(10.0)?;
        let remaining = &reader.bytes[reader.position..];
        let end = remaining
            .iter()
            .take(256)
            .position(|&v| v == 0)
            .ok_or("Invalid factory voice name")?;
        let name =
            std::str::from_utf8(&remaining[..end]).map_err(|_| "Invalid factory voice name")?;
        if name.is_empty()
            || name.chars().any(char::is_control)
            || !name.chars().any(char::is_alphabetic)
        {
            return Err("Invalid factory voice name".into());
        }
        let name = name.to_owned();
        reader.take(end + 1)?;
        let mut points = Vec::with_capacity(point_count as usize);
        for _ in 0..point_count {
            points.push(FactoryPoint {
                data: reader.floats::<128>(10.0)?,
                reduced: reader.floats::<4>(10.0)?,
            });
        }
        voices.push(FactoryVoice {
            name,
            pitch_stats,
            data,
            points,
        });
    }
    Ok((FactoryLibrary { voices }, reader.position))
}

impl FactoryLibrary {
    pub fn bundled() -> Result<Self> {
        Self::from_blob(include_bytes!("../assets/factory-voices.bin"))
    }

    pub fn from_blob(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 8 * 1024 * 1024 {
            return Err("Voice catalog is too large".into());
        }
        let (library, consumed) = decode(bytes)?;
        if consumed != bytes.len() {
            return Err("Unexpected trailing voice catalog data".into());
        }
        Ok(library)
    }
}
