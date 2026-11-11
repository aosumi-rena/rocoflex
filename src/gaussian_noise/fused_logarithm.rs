
pub(super) fn positive_finite(
    value: f32,
    bits: u32,
    reciprocal: &[u32; 129],
    log_high: &[u32; 129],
    log_low: &[u32; 129],
) -> f32 {
    let mut index_bits = bits;
    let mut mantissa = bits & 0x007f_ffff;
    let mut exponent = ((bits >> 23) as i32 - 127) as f32;
    let difference = value - f32::from_bits(0x3f80_0000);

    if exponent == -127.0 {
        let normalized = f32::from_bits(mantissa | 0x3f80_0000)
            - f32::from_bits(0x3f80_0000);
        let normalized_bits = normalized.to_bits();
        mantissa = normalized_bits & 0x007f_ffff;
        index_bits = mantissa;
        exponent = ((normalized_bits >> 23) as i32 - 253) as f32;
    }

    let rounded_index_bits = (index_bits & 0x007f_0000)
        + ((index_bits & 0x0000_8000) << 1);

    if f32::from_bits(difference.to_bits() & 0x7fff_ffff)
        < f32::from_bits(0x3d80_0000)
    {
        let delta = value - f32::from_bits(0x3f80_0000);
        let denominator = f32::from_bits(0x4000_0000) + delta;
        let ratio = delta / denominator;
        let correction = delta * ratio;
        let doubled_ratio = ratio + ratio;
        let square = doubled_ratio * doubled_ratio;
        let cube = doubled_ratio * square;
        let polynomial = square.mul_add(
            f32::from_bits(0x3c4c_cccd), f32::from_bits(0x3daa_aaab),
        );
        let tail = polynomial.mul_add(cube, -correction);
        return delta + tail;
    }

    let index = (rounded_index_bits >> 16) as usize;
    let normalized = f32::from_bits(mantissa | 0x3f00_0000);
    let rounded = f32::from_bits(rounded_index_bits | 0x3f00_0000);
    let difference = rounded - normalized;
    let residual = difference * f32::from_bits(reciprocal[index]);
    let polynomial = f32::from_bits(0x3eaa_aaab)
        .mul_add(residual, f32::from_bits(0x3f00_0000));
    let square = residual * residual;
    let corrected_residual = polynomial.mul_add(square, residual);
    let low = f32::from_bits(0x3805_fdf4).mul_add(exponent, -corrected_residual);
    let high = f32::from_bits(0x3f31_7000)
        .mul_add(exponent, f32::from_bits(log_high[index]));
    let low = low + f32::from_bits(log_low[index]);
    high + low
}
