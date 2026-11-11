pub fn encode_f32_bits(bits: u32) -> u16 {
    let upper = bits >> 16;
    let sign = upper & 0x8000;
    let exponent = bits & 0x7f80_0000;

    if exponent >= 0x4780_0000 {
        if exponent != 0x7f80_0000 {
            return (sign | 0x7c00) as u16;
        }
        let payload = bits & 0x007f_ffff;
        if payload == 0 {
            return (upper & 0xfc00) as u16;
        }
        let nan = if payload < 0x2000 { 0x7c01 } else { 0x7c00 | (payload >> 13) };
        return (sign | nan) as u16;
    }

    if exponent <= 0x3800_0000 {
        if exponent < 0x3300_0000 {
            return sign as u16;
        }
        let shift = 0x71 - (exponent >> 23);
        let significand = (bits & 0x007f_ffff) | 0x0080_0000;
        let rounded = ((significand >> shift) + 0x1000) >> 13;
        return (sign | rounded) as u16;
    }

    let encoded_exponent = (bits.wrapping_add(0x0800_0000) >> 13) & 0xfc00;
    let rounded_significand = ((bits & 0x007f_f000) + 0x1000) >> 13;
    (encoded_exponent + sign + rounded_significand) as u16
}

pub fn decode_to_f32_bits(word: u16) -> u32 {
    let raw = u32::from(word);
    let sign = (raw & 0x8000) << 16;
    let exponent = raw & 0x7c00;
    if exponent == 0x7c00 {
        return sign | (raw << 13) | 0x7f80_0000;
    }
    if exponent != 0 {
        return sign.wrapping_add((raw << 13) & 0x0fff_e000).wrapping_add(0x3800_0000);
    }
    let mut significand = raw & 0x03ff;
    if significand == 0 {
        return sign;
    }
    let mut adjusted_exponent = 1u32;
    loop {
        significand += significand;
        adjusted_exponent = adjusted_exponent.wrapping_sub(1);
        if significand & 0x0400 != 0 {
            break;
        }
    }
    let mantissa = (significand << 13) & 0x007f_c000;
    sign.wrapping_add(adjusted_exponent << 23).wrapping_add(0x3800_0000) | mantissa
}

