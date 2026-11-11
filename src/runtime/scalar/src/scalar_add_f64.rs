use super::finite_power::NormalArithmeticError;
use super::scalar_f64_operand::{F64Operand, binary_nan_result};
use super::scalar_round_f64::{ScalarF64Candidate, finish_binary64_rounding};
use super::scalar_word_arithmetic::{shifted_words, combine_words, round_words_unbounded};
use super::shared_scalar_f32::{ScalarF32Exceptions, ScalarF32Mxcsr, ScalarF32Rounding};

pub(super) struct AdditionRequired { pub reason: NormalArithmeticError, pub raised: u8 }

#[derive(Clone, Copy)]
pub(super) enum ScalarF64SumOperation { Add, Subtract }

pub(super) fn sum_f64_candidate(operation: ScalarF64SumOperation,
    left: u64, right: u64, before: ScalarF32Mxcsr)
    -> Result<ScalarF64Candidate, AdditionRequired> {
    let a = F64Operand::from_bits(left).with_daz(before.denormals_are_zero());
    let b = F64Operand::from_bits(right).with_daz(before.denormals_are_zero());
    if let Some((bits, raised)) = binary_nan_result(a, b) {
        return Ok(ScalarF64Candidate { bits: Some(bits), raised });
    }

    let b = match operation {
        ScalarF64SumOperation::Add => b,
        ScalarF64SumOperation::Subtract => F64Operand::from_bits(b.bits() ^ 0x8000_0000_0000_0000),
    };
    if a.infinite() && b.infinite() && a.negative() != b.negative() {
        return Ok(ScalarF64Candidate { bits: Some(0xfff8_0000_0000_0000), raised: 1 });
    }
    let pre = if a.subnormal() || b.subnormal() { 2 } else { 0 };
    if pre != 0 && before.with_explicit_raised_status(ScalarF32Exceptions::DENORMAL).1.bits() != 0 {
        return Ok(ScalarF64Candidate { bits: None, raised: pre });
    }
    if a.infinite() { return Ok(ScalarF64Candidate { bits: Some(a.bits()), raised: pre }); }
    if b.infinite() { return Ok(ScalarF64Candidate { bits: Some(b.bits()), raised: pre }); }
    let down = before.rounding() == ScalarF32Rounding::TowardNegative;
    if a.zero() && b.zero() {
        let negative = if a.negative() == b.negative() { a.negative() } else { down };
        return Ok(ScalarF64Candidate { bits: Some(if negative { 0x8000_0000_0000_0000 } else { 0 }), raised: pre });
    }
    let (ls, lp) = a.finite_parts();
    let (rs, rp) = b.finite_parts();
    let power = lp.min(rp);

    let lw = shifted_words::<33>(u128::from(ls), (lp - power) as u32)
        .map_err(|reason| AdditionRequired { reason, raised: pre })?;
    let rw = shifted_words::<33>(u128::from(rs), (rp - power) as u32)
        .map_err(|reason| AdditionRequired { reason, raised: pre })?;
    let (words, negative) = combine_words(lw, a.negative(), rw, b.negative())
        .map_err(|reason| AdditionRequired { reason, raised: pre })?;
    let Some(rounded) = round_words_unbounded(words, power, negative, before.rounding()) else {

        return Ok(ScalarF64Candidate { bits: Some(if down { 0x8000_0000_0000_0000 } else { 0 }), raised: pre });
    };
    Ok(finish_binary64_rounding(rounded, negative, before, pre, || {

        (words[0] << ((power + 1074) as u32), false)
    }))
}
