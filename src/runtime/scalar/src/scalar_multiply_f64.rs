use super::finite_power::{exact_f64_product, round_u128_binary64_unbounded, round_u128_right_to_grid};
use super::scalar_f64_operand::{F64Operand, binary_nan_result};
use super::scalar_round_f64::{ScalarF64Candidate, finish_binary64_rounding};
use super::shared_scalar_f32::{ScalarF32Exceptions, ScalarF32Mxcsr};

pub(super) fn multiply_f64_candidate(left: u64, right: u64, before: ScalarF32Mxcsr) -> ScalarF64Candidate {
    let a = F64Operand::from_bits(left).with_daz(before.denormals_are_zero());
    let b = F64Operand::from_bits(right).with_daz(before.denormals_are_zero());
    if let Some((bits, raised)) = binary_nan_result(a, b) {
        return ScalarF64Candidate { bits: Some(bits), raised };
    }
    if (a.infinite() && b.zero()) || (b.infinite() && a.zero()) {
        return ScalarF64Candidate { bits: Some(0xfff8_0000_0000_0000), raised: 1 };
    }
    let pre = if a.subnormal() || b.subnormal() { 2 } else { 0 };
    if pre != 0 && before.with_explicit_raised_status(ScalarF32Exceptions::DENORMAL).1.bits() != 0 {
        return ScalarF64Candidate { bits: None, raised: pre };
    }
    let negative = a.negative() ^ b.negative();
    let sign = if negative { 0x8000_0000_0000_0000 } else { 0 };
    if a.infinite() || b.infinite() {
        return ScalarF64Candidate { bits: Some(sign | 0x7ff0_0000_0000_0000), raised: pre };
    }
    if a.zero() || b.zero() { return ScalarF64Candidate { bits: Some(sign), raised: pre }; }
    let (ls, lp) = a.finite_parts();
    let (rs, rp) = b.finite_parts();
    let (product, power) = exact_f64_product(ls, lp, rs, rp);
    let rounded = round_u128_binary64_unbounded(product, power, negative, before.rounding());
    finish_binary64_rounding(rounded, negative, before, pre, || {

        let (subnormal, inexact) = if power >= -1074 {
            (product << ((power + 1074) as u32), false)
        } else { round_u128_right_to_grid(product, (-1074 - power) as u32, negative, before.rounding()) };
        (subnormal as u64, inexact)
    })
}
