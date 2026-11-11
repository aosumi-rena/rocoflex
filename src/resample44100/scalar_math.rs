use super::scalar_coefficients::*;

pub fn finite_atan(value: f64) -> Result<f64, &'static str> {
    let bits = value.to_bits();
    let magnitude = bits & 0x7fff_ffff_ffff_ffff;
    let negative = magnitude != bits;
    let absolute = f64::from_bits(magnitude);
    if !value.is_finite() || absolute > SCALAR_COEFFICIENT_000 {
        return Err("source scalar exceptional/errno path outside candidate scope");
    }
    let (argument,high,low) = if magnitude > 0x4003_8000_0000_0000 {
        (SCALAR_COEFFICIENT_003 / absolute,SCALAR_COEFFICIENT_002,SCALAR_COEFFICIENT_004)
    } else if magnitude > 0x3ff3_0000_0000_0000 {
        let mut denominator = absolute * SCALAR_COEFFICIENT_005;
        let numerator = absolute - SCALAR_COEFFICIENT_005;
        denominator += SCALAR_COEFFICIENT_008;
        (numerator / denominator,SCALAR_COEFFICIENT_006,SCALAR_COEFFICIENT_007)
    } else if magnitude > 0x3fe6_0000_0000_0000 {
        let numerator = absolute - SCALAR_COEFFICIENT_008;
        let denominator = absolute + SCALAR_COEFFICIENT_008;
        (numerator / denominator,SCALAR_COEFFICIENT_009,SCALAR_COEFFICIENT_010)
    } else if magnitude > 0x3fdc_0000_0000_0000 {
        let mut numerator = absolute + absolute;
        let denominator = absolute + SCALAR_COEFFICIENT_012;
        numerator -= SCALAR_COEFFICIENT_008;
        (numerator / denominator,SCALAR_COEFFICIENT_011,SCALAR_COEFFICIENT_013)
    } else { (absolute,0.0,0.0) };
    let square = argument * argument;
    let mut numerator = square * SCALAR_COEFFICIENT_014;
    let mut denominator = square * SCALAR_COEFFICIENT_015;
    let cube = square * argument;
    numerator += SCALAR_COEFFICIENT_016;
    denominator += SCALAR_COEFFICIENT_017;
    numerator *= square;
    denominator *= square;
    numerator += SCALAR_COEFFICIENT_018;
    denominator += SCALAR_COEFFICIENT_019;
    numerator *= square;
    denominator *= square;
    numerator += SCALAR_COEFFICIENT_020;
    denominator += SCALAR_COEFFICIENT_021;
    numerator *= square;
    denominator *= square;
    numerator += SCALAR_COEFFICIENT_022;
    denominator += SCALAR_COEFFICIENT_023;
    numerator *= cube;
    numerator /= denominator;
    numerator -= low;
    numerator -= argument;
    let result = high - numerator;
    Ok(if negative { f64::from_bits(result.to_bits() ^ 0x8000_0000_0000_0000) } else { result })
}
