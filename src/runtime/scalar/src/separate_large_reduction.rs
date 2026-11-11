use super::finite_power::normal_multiply_source_bits;
use super::retained_phase_constants::{C1, C3, C4};
use super::selected_phase::{integer_large_head_tail, IntegerPhaseHeadTail, PhaseError};
use super::vorbis_power::{ArithmeticContext, VorbisScalarError};

pub(super) enum SeparateLargeReductionError { Integer(PhaseError), Arithmetic(VorbisScalarError) }
pub(super) struct SeparateLargeReduction {
    pub(super) quadrant: u32,
    pub(super) head: u64,
    pub(super) tail: u64,
}

pub(super) fn reduce(a: &mut impl ArithmeticContext, positive_bits: u64)
    -> Result<SeparateLargeReduction, SeparateLargeReductionError> {
    use self::SeparateLargeReductionError::Arithmetic as E;
    let IntegerPhaseHeadTail { quadrant, head_bits, tail_bits } = integer_large_head_tail(positive_bits)
        .map_err(SeparateLargeReductionError::Integer)?;
    let truncated = head_bits & 0xffff_ffff_f800_0000;

    let delta = a.subtract("597550/large SSE truncated delta SUBSD", head_bits, truncated).map_err(E)?;
    let head_product = multiply_pair(a, "59759d/large SSE head MULPD",
        [head_bits, head_bits], [C1[0].to_bits(), C1[1].to_bits()]).map_err(E)?;
    let tail_product = multiply_pair(a, "5975a1/large SSE tail MULPD",
        [tail_bits, 0], [C1[0].to_bits(), C1[1].to_bits()]).map_err(E)?;
    let high_product = multiply_pair(a, "5975a5/large SSE high split MULPD",
        [C3[0].to_bits(), C3[1].to_bits()], [delta, truncated]).map_err(E)?;
    let low_product = multiply_pair(a, "5975a9/large SSE low split MULPD",
        [C4[0].to_bits(), C4[1].to_bits()], [delta, truncated]).map_err(E)?;
    let correction = a.subtract("5975b3/large SSE product difference SUBSD", high_product[1], head_product[0]).map_err(E)?;
    let correction = a.add("5975b7/large SSE delta high ADDSD", correction, high_product[0]).map_err(E)?;
    let correction = a.add("5975be/large SSE truncated low ADDSD", correction, low_product[1]).map_err(E)?;
    let small = a.add("5975c2/large SSE head+tail low ADDSD", head_product[1], tail_product[0]).map_err(E)?;
    let correction = a.add("5975c6/large SSE delta low ADDSD", correction, low_product[0]).map_err(E)?;
    let correction = a.add("5975ca/large SSE correction ADDSD", correction, small).map_err(E)?;
    let head = a.add("5975d2/large SSE head ADDSD", head_product[0], correction).map_err(E)?;
    let loss = a.subtract("5975d6/large SSE lost head SUBSD", head_product[0], head).map_err(E)?;
    let tail = a.add("5975da/large SSE tail ADDSD", correction, loss).map_err(E)?;
    Ok(SeparateLargeReduction { quadrant, head, tail })
}

fn multiply_pair(a: &mut impl ArithmeticContext, operation: &'static str, left: [u64; 2], right: [u64; 2])
    -> Result<[u64; 2], VorbisScalarError> {
    let entry = a.exception_state();
    let (low, low_flags) = normal_multiply_source_bits(left[0], right[0])
        .map_err(|reason| VorbisScalarError::ArithmeticRequired { operation, reason, advanced: entry })?;
    let (high, high_flags) = normal_multiply_source_bits(left[1], right[1])
        .map_err(|reason| VorbisScalarError::ArithmeticRequired { operation, reason, advanced: entry })?;
    a.record_completed(low_flags | high_flags);
    Ok([low, high])
}
