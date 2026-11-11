use super::finite_power::{ExplicitExceptionState, round_exact_normal_u128};
use super::selected_coefficient_trig::{INV_PIO2, SPLIT_HIGH, SPLIT_LOW};
use super::selected_exp_f64::truncate_positive_trig_count;
use super::vorbis_power::{ArithmeticContext, VorbisScalarError};

const HALF: u64 = 0x3fe0_0000_0000_0000;
const SECOND_SPLIT_HIGH: u64 = 0x3dd0_b461_1a60_0000;
const SECOND_SPLIT_LOW: u64 = 0x3ba3_198a_2e03_7073;

pub(super) enum SeparateReductionError {
    Count { scaled_bits: u64, advanced: ExplicitExceptionState },
    Arithmetic(VorbisScalarError),
}
pub(super) struct SeparateReduction {
    pub(super) count: u32,
    pub(super) head: u64,
    pub(super) tail: u64,
    pub(super) first_exponent: i32,
}

pub(super) fn reduce(a: &mut impl ArithmeticContext, wide: u64, compute_tail: bool)
    -> Result<SeparateReduction, SeparateReductionError>
{
    use self::SeparateReductionError::Arithmetic as E;
    let scaled = a.multiply("SSE short/inverse pi2 MULSD", wide, INV_PIO2.to_bits()).map_err(E)?;
    let scaled = a.add("SSE short/count ADDSD half", scaled, HALF).map_err(E)?;
    let (integer, _, flags) = truncate_positive_trig_count(scaled, a.exception_state())
        .map_err(|_| SeparateReductionError::Count { scaled_bits: scaled, advanced: a.exception_state() })?;
    a.record_completed(flags);
    let count = integer as u32;
    let n = if count == 0 { 0 } else { round_exact_normal_u128(u128::from(count), 0).0 };
    let high_product = a.multiply("SSE short/count high split MULSD", SPLIT_HIGH.to_bits(), n).map_err(E)?;
    let mut first = a.subtract("SSE short/first remainder SUBSD", wide, high_product).map_err(E)?;
    let mut correction = a.multiply("SSE short/count low split MULSD", SPLIT_LOW.to_bits(), n).map_err(E)?;
    let mut head = a.subtract("SSE short/head remainder SUBSD", first, correction).map_err(E)?;

    let first_exponent = ((head & 0x7fff_ffff_ffff_ffff) >> 52) as i32;
    if (wide >> 52) as i32 - first_exponent > 15 {
        let middle = a.multiply("SSE short/refine high split MULSD", SECOND_SPLIT_HIGH, n).map_err(E)?;
        let low = a.multiply("SSE short/refine low split MULSD", SECOND_SPLIT_LOW, n).map_err(E)?;
        let refined = a.subtract("SSE short/refined first SUBSD", first, middle).map_err(E)?;
        let lost = a.subtract("SSE short/refine first loss SUBSD", first, refined).map_err(E)?;
        let lost = a.subtract("SSE short/refine split loss SUBSD", lost, middle).map_err(E)?;
        correction = a.subtract("SSE short/refine tail correction SUBSD", low, lost).map_err(E)?;
        first = refined;
        head = a.subtract("SSE short/refined head SUBSD", first, correction).map_err(E)?;
    }
    let tail = if compute_tail {
        let loss = a.subtract("SSE short/actual F64 head loss SUBSD", first, head).map_err(E)?;
        a.subtract("SSE short/actual F64 tail SUBSD", loss, correction).map_err(E)?
    } else { 0 };
    Ok(SeparateReduction { count, head, tail, first_exponent })
}
