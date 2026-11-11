use super::finite_power::{exact_f64_quotient_head, round_u128_binary64_unbounded,
    round_u128_right_to_grid};
use super::scalar_f64_operand::{F64Operand, binary_nan_result};
use super::scalar_round_f64::{ScalarF64Candidate, finish_binary64_rounding};
use super::shared_scalar_f32::{ScalarF32Exceptions, ScalarF32Mxcsr};

pub(super) fn divide_f64_candidate(left: u64, right: u64, before: ScalarF32Mxcsr) -> ScalarF64Candidate {
    let a = F64Operand::from_bits(left).with_daz(before.denormals_are_zero());
    let b = F64Operand::from_bits(right).with_daz(before.denormals_are_zero());
    if let Some((bits, raised)) = binary_nan_result(a, b) {
        return ScalarF64Candidate { bits: Some(bits), raised };
    }
    if (a.zero() && b.zero()) || (a.infinite() && b.infinite()) {
        return ScalarF64Candidate { bits: Some(0xfff8_0000_0000_0000), raised: 1 };
    }
    let negative = a.negative() ^ b.negative();
    let sign = if negative { 0x8000_0000_0000_0000 } else { 0 };

    if b.zero() && !a.infinite() {
        return ScalarF64Candidate { bits: Some(sign | 0x7ff0_0000_0000_0000), raised: 4 };
    }
    let pre = if a.subnormal() || b.subnormal() { 2 } else { 0 };
    if pre != 0 && before.with_explicit_raised_status(ScalarF32Exceptions::DENORMAL).1.bits() != 0 {
        return ScalarF64Candidate { bits: None, raised: pre };
    }
    if a.infinite() {
        return ScalarF64Candidate { bits: Some(sign | 0x7ff0_0000_0000_0000), raised: pre };
    }
    if b.infinite() || a.zero() {
        return ScalarF64Candidate { bits: Some(sign), raised: pre };
    }

    let (ls, lp) = a.normalized_nonzero_parts();
    let (rs, rp) = b.normalized_nonzero_parts();
    let head = exact_f64_quotient_head(ls, rs);
    let power = lp - rp - head.shift;
    let rounded = round_u128_binary64_unbounded(head.jammed, power, negative, before.rounding());
    finish_binary64_rounding(rounded, negative, before, pre, || {

        let (subnormal, inexact) = round_u128_right_to_grid(head.jammed,
            (-1074 - power) as u32, negative, before.rounding());
        (subnormal as u64, inexact)
    })
}
