#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MaskedTruncationFormat { F32ToSigned32, F32ToSigned64, F64ToSigned64, F64ToSigned32 }

pub(super) fn masked_truncate_bits(bits: u64, format: MaskedTruncationFormat, denormals_are_zero: bool) -> (i64, u32) {
    let (sign, fraction, width, maximum_field, bias, limit, minimum) = match format {
        MaskedTruncationFormat::F32ToSigned32 =>
            (0x8000_0000u64, 0x007f_ffffu64, 23u32, 255i32, 127i32, 31i32, i64::from(i32::MIN)),
        MaskedTruncationFormat::F32ToSigned64 =>
            (0x8000_0000u64, 0x007f_ffffu64, 23u32, 255i32, 127i32, 63i32, i64::MIN),
        MaskedTruncationFormat::F64ToSigned64 =>
            (0x8000_0000_0000_0000u64, 0x000f_ffff_ffff_ffffu64, 52u32, 2047i32, 1023i32, 63i32, i64::MIN),
        MaskedTruncationFormat::F64ToSigned32 =>
            (0x8000_0000_0000_0000u64, 0x000f_ffff_ffff_ffffu64, 52u32, 2047i32, 1023i32, 31i32, i64::from(i32::MIN)),
    };
    let magnitude = bits & !sign;
    if magnitude == 0 { return (0, 0); }
    let field = (magnitude >> width) as i32;

    if field == 0 && denormals_are_zero { return (0, 0); }
    if field == maximum_field { return (minimum, 1); }
    let negative = bits & sign != 0;
    let exponent = field - bias;
    if exponent == limit && negative {
        let fraction_bits = magnitude & fraction;

        let discard = width.saturating_sub(limit as u32);
        if fraction_bits >> discard == 0 {
            return (minimum, if fraction_bits != 0 { 0x20 } else { 0 });
        }
    }
    if exponent >= limit { return (minimum, 1); }

    if exponent < 0 { return (0, 0x20); }
    let significand = (magnitude & fraction) | (1u64 << width);
    let (integer, inexact) = if exponent < width as i32 {
        let discard = (width as i32 - exponent) as u32;
        (significand >> discard, significand & ((1u64 << discard) - 1) != 0)
    } else {

        (significand << ((exponent - width as i32) as u32), false)
    };

    let signed = integer as i64;
    (if negative { -signed } else { signed }, if inexact { 0x20 } else { 0 })
}
