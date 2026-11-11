use crate::Result;

const ALPHABET: &[u8; 64] = b".ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+";

fn plausible(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes.len().is_multiple_of(4)
        && bytes.chunks_exact(4).all(|b| {
            let f = f32::from_le_bytes(b.try_into().unwrap());
            f.is_finite() && f.abs() <= 10.0
        })
}

fn parameters(bytes: &[u8]) -> (u64, u64) {
    bytes.iter().fold((0, 0), |(seed, increment), &b| {
        let n = i16::from(b as i8);
        (seed + n.max(0) as u64, increment + n.unsigned_abs() as u64)
    })
}

pub fn decode(text: &str, width: usize) -> Result<Vec<f32>> {
    let (count, encoded) = text.split_once('.').ok_or("Invalid payload header")?;
    let count: usize = count.parse().map_err(|_| "Invalid payload length")?;
    if count != width.checked_mul(4).ok_or("Payload too large")? || count == 0 || count > 4096 {
        return Err(format!("Expected {width} float values"));
    }
    if encoded.len() != (count * 8).div_ceil(6) {
        return Err("Truncated or oversized payload".into());
    }
    let mut bytes = Vec::with_capacity(count);
    let (mut bits, mut bit_count) = (0_u32, 0);
    for ch in encoded.bytes() {
        let n = ALPHABET
            .iter()
            .position(|&v| v == ch)
            .ok_or("Invalid payload character")?;
        bits |= (n as u32) << bit_count;
        bit_count += 6;
        if bit_count >= 8 {
            bytes.push(bits as u8);
            bits >>= 8;
            bit_count -= 8;
        }
    }
    if bytes.len() != count || bits != 0 {
        return Err("Invalid payload padding".into());
    }
    if !plausible(&bytes) {
        let (seed, increment) = parameters(&bytes);
        for i in (0..count).rev() {
            let j = i + ((seed + i as u64 * increment) % (count - i) as u64) as usize;
            bytes.swap(i, j);
        }
        if !plausible(&bytes) {
            return Err("Payload contains invalid float values".into());
        }
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
        .collect())
}

pub fn encode(values: &[f32]) -> Result<String> {
    if values.is_empty()
        || values.len() > 1024
        || values.iter().any(|v| !v.is_finite() || v.abs() > 10.0)
    {
        return Err("Invalid float payload".into());
    }
    let raw: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut shuffled = raw.clone();
    let (seed, increment) = parameters(&raw);
    let n = raw.len();
    for i in 0..n {
        let j = i + ((seed + i as u64 * increment) % (n - i) as u64) as usize;
        shuffled.swap(i, j);
    }
    let bytes = if plausible(&shuffled) {
        &raw
    } else {
        &shuffled
    };
    let mut result = format!("{n}.");
    for bit in (0..n * 8).step_by(6) {
        let byte = bit / 8;
        let shift = bit % 8;
        let word = u16::from(bytes[byte]) | (bytes.get(byte + 1).copied().unwrap_or(0) as u16) << 8;
        result.push(ALPHABET[((word >> shift) & 63) as usize] as char);
    }
    Ok(result)
}
