pub(super) fn widen_f32_candidate(bits: u32, denormals_are_zero: bool) -> (u64, u8) {
    let sign = u64::from(bits & 0x8000_0000) << 32;
    let field = (bits >> 23) & 255;
    let fraction = bits & 0x007f_ffff;
    if field == 255 {
        let result = sign | 0x7ff0_0000_0000_0000 | (u64::from(fraction) << 29)
            | (if fraction == 0 { 0 } else { 0x0008_0000_0000_0000 });
        let invalid = fraction != 0 && fraction & 0x0040_0000 == 0;
        return (result, u8::from(invalid));
    }
    if field != 0 {
        return (sign | (u64::from(field + 896) << 52) | (u64::from(fraction) << 29), 0);
    }
    if fraction == 0 || denormals_are_zero { return (sign, 0); }
    let highest = 31 - fraction.leading_zeros();
    let result = sign | (u64::from(highest + 874) << 52)
        | (u64::from(fraction ^ (1u32 << highest)) << (52 - highest));
    (result, 2)
}
