use super::finite_power::RoundedBinary64;
use super::shared_scalar_f32::{ScalarF32Mxcsr, ScalarF32Rounding};

pub(super) struct ScalarF64Candidate { pub bits: Option<u64>, pub raised: u8 }

pub(super) fn finish_binary64_rounding(rounded: RoundedBinary64, negative: bool,
    before: ScalarF32Mxcsr, pre: u8, exact_gradual: impl FnOnce() -> (u64, bool)) -> ScalarF64Candidate {
    let sign = if negative { 0x8000_0000_0000_0000 } else { 0 };
    if rounded.field >= 2047 {
        if before.masks().bits() & 8 == 0 {
            return ScalarF64Candidate { bits: None, raised: pre | 8 | if rounded.inexact { 0x20 } else { 0 } };
        }
        let infinity = match before.rounding() {
            ScalarF32Rounding::NearestEven => true,
            ScalarF32Rounding::TowardNegative => negative,
            ScalarF32Rounding::TowardPositive => !negative,
            ScalarF32Rounding::TowardZero => false,
        };
        return ScalarF64Candidate { bits: Some(sign | if infinity { 0x7ff0_0000_0000_0000 } else { 0x7fef_ffff_ffff_ffff }),
            raised: pre | 0x28 };
    }
    let tiny = rounded.field < 1;
    if tiny && before.masks().bits() & 0x10 == 0 {
        return ScalarF64Candidate { bits: None, raised: pre | 0x10 | if rounded.inexact { 0x20 } else { 0 } };
    }
    if tiny && before.flush_to_zero() {
        return ScalarF64Candidate { bits: Some(sign), raised: pre | 0x30 };
    }
    if tiny {
        let (subnormal, inexact) = exact_gradual();
        return ScalarF64Candidate { bits: Some(sign | subnormal),
            raised: pre | if inexact { 0x30 } else { 0 } };
    }
    ScalarF64Candidate { bits: Some(sign | ((rounded.field as u64) << 52)
        | (rounded.significand & 0x000f_ffff_ffff_ffff)), raised: pre | if rounded.inexact { 0x20 } else { 0 } }
}
