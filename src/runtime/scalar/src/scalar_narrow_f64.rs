use super::finite_power::binary_round_increment;
use super::shared_scalar_f32::{ScalarF32Exceptions, ScalarF32Mxcsr, ScalarF32Rounding};

pub(super) struct NarrowingCandidate { pub bits: Option<u32>, pub raised: u8 }

pub(super) fn special_f64_to_f32(wide: u64) -> Option<(u32, u8)> {
    let sign = ((wide >> 32) as u32) & 0x8000_0000;
    let field = ((wide >> 52) & 0x7ff) as i32;
    let fraction = wide & 0x000f_ffff_ffff_ffff;
    if field == 0x7ff {
        if fraction == 0 { Some((sign | 0x7f80_0000, 0)) } else {
            let invalid = fraction & 0x0008_0000_0000_0000 == 0;
            Some((sign | 0x7f80_0000 | ((fraction >> 29) as u32) | 0x0040_0000,
                u8::from(invalid)))
        }
    } else if field == 0 && fraction == 0 { Some((sign, 0)) } else { None }
}

pub(super) fn narrow_f64_candidate(wide: u64, before: ScalarF32Mxcsr) -> NarrowingCandidate {
    if let Some((bits, raised)) = special_f64_to_f32(wide) {
        let trapped = raised != 0
            && before.with_explicit_raised_status(ScalarF32Exceptions::INVALID).1.bits() != 0;
        return NarrowingCandidate { bits: if trapped { None } else { Some(bits) }, raised };
    }
    let sign = ((wide >> 32) as u32) & 0x8000_0000;
    let negative = sign != 0;
    let mut field = ((wide >> 52) & 0x7ff) as i32;
    let fraction = wide & 0x000f_ffff_ffff_ffff;
    let mut pre = 0;
    let significand = if field == 0 {
        if before.denormals_are_zero() { return NarrowingCandidate { bits: Some(sign), raised: 0 }; }
        pre = 2;
        let (_, unmasked) = before.with_explicit_raised_status(ScalarF32Exceptions::DENORMAL);
        if unmasked.bits() != 0 { return NarrowingCandidate { bits: None, raised: pre }; }

        let highest = 63 - fraction.leading_zeros();
        field = highest as i32 - 51;
        fraction << (52 - highest)
    } else { fraction | (1u64 << 52) };
    let rounding = before.rounding();
    let unbounded_retained = significand >> 29;
    let unbounded_lost = significand & ((1u64 << 29) - 1);
    let unbounded_increment = binary_round_increment(rounding, negative,
        u128::from(unbounded_lost), 1u128 << 28, unbounded_retained & 1 != 0);
    let unbounded_rounded = unbounded_retained + u64::from(unbounded_increment);
    let carry = unbounded_rounded == (1u64 << 24);
    let result_field = field - 896 + i32::from(carry);
    let unbounded_inexact = unbounded_lost != 0;
    if result_field >= 255 {
        if before.masks().bits() & 8 == 0 {
            return NarrowingCandidate { bits: None,
                raised: pre | 8 | if unbounded_inexact { 0x20 } else { 0 } };
        }
        let infinity = match rounding {
            ScalarF32Rounding::NearestEven => true,
            ScalarF32Rounding::TowardNegative => negative,
            ScalarF32Rounding::TowardPositive => !negative,
            ScalarF32Rounding::TowardZero => false,
        };
        return NarrowingCandidate { bits: Some(sign | if infinity { 0x7f80_0000 } else { 0x7f7f_ffff }),
            raised: pre | 0x28 };
    }
    let tiny = result_field < 1;
    if tiny && before.masks().bits() & 0x10 == 0 {
        return NarrowingCandidate { bits: None,
            raised: pre | 0x10 | if unbounded_inexact { 0x20 } else { 0 } };
    }
    if tiny && before.flush_to_zero() {
        return NarrowingCandidate { bits: Some(sign), raised: pre | 0x30 };
    }
    let (bits, inexact) = if field < 873 {

        let increment = binary_round_increment(rounding, negative,
            u128::from(significand), 1u128 << 53, false);
        (sign | u32::from(increment), true)
    } else if field <= 896 {
        let discard = (926 - field) as u32;
        let retained = (significand >> discard) as u32;
        let lost = significand & ((1u64 << discard) - 1);
        let half = 1u64 << (discard - 1);
        let increment = binary_round_increment(rounding, negative,
            u128::from(lost), u128::from(half), retained & 1 != 0);
        (sign | (retained + u32::from(increment)), lost != 0)
    } else {
        let rounded = if carry { unbounded_rounded >> 1 } else { unbounded_rounded };
        (sign | ((result_field as u32) << 23) | (rounded as u32 & 0x007f_ffff), unbounded_inexact)
    };
    NarrowingCandidate { bits: Some(bits), raised: pre | if inexact {
        if tiny { 0x30 } else { 0x20 }
    } else { 0 } }
}
