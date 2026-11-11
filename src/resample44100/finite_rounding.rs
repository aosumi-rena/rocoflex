fn parts(value: f64) -> Result<(u64,u64,bool), &'static str> {
    let bits = value.to_bits();
    let magnitude = bits & 0x7fff_ffff_ffff_ffff;
    if magnitude >= 0x7ff0_0000_0000_0000 { return Err("finite source rounding scope required"); }
    Ok((bits,magnitude,bits != magnitude))
}

fn truncated_bits(bits: u64, magnitude: u64) -> u64 {
    let exponent = (magnitude >> 52) as u32;
    let fractional_bits = 1075-exponent;
    bits & !((1u64 << fractional_bits)-1)
}

pub fn floor_finite(value: f64) -> Result<f64, &'static str> {
    let (bits,magnitude,negative) = parts(value)?;
    if magnitude >= 0x4340_0000_0000_0000 || magnitude == 0 { return Ok(value); }
    if magnitude < 0x3ff0_0000_0000_0000 { return Ok(if negative { -1.0 } else { 0.0 }); }
    let truncated = truncated_bits(bits,magnitude);
    let value = f64::from_bits(truncated);
    Ok(if negative && truncated != bits { value-1.0 } else { value })
}

pub fn ceil_finite(value: f64) -> Result<f64, &'static str> {
    let (bits,magnitude,negative) = parts(value)?;
    if magnitude >= 0x4340_0000_0000_0000 || magnitude == 0 { return Ok(value); }
    if magnitude < 0x3ff0_0000_0000_0000 {
        return Ok(if negative { f64::from_bits(0x8000_0000_0000_0000) } else { 1.0 });
    }
    let truncated = truncated_bits(bits,magnitude);
    let value = f64::from_bits(truncated);
    Ok(if !negative && truncated != bits { value+1.0 } else { value })
}
