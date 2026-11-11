use super::{retained_phase_constants::TABLE, selected_log::Environment};
use super::phase_status::PhaseStatusAccumulator;
use super::finite_power::{ExplicitExceptionState, MaskedMathError, MathErrorCallbackPolicy};
use super::finite_power::{MathErrorCall, MathErrorOperation, MathErrorServiceError, NormalArithmeticError, SourceMathErrorCallback};

const SIGN: u64 = 0x8000_0000_0000_0000;
const INFINITY: u64 = 0x7ff0_0000_0000_0000;
const DIRECT_LIMIT: u64 = 0x3fe9_21fb_5444_2d18;
const SMALL_LIMIT: u64 = 0x3f20_0000_0000_0000;
const TINY_LIMIT: u64 = 0x3e40_0000_0000_0000;
const LARGE_LIMIT: u64 = 0x4173_12d0_0000_0000;
const ONE: f64 = f64::from_bits(0x3ff0_0000_0000_0000);
const HALF: f64 = f64::from_bits(0x3fe0_0000_0000_0000);
const PIO2: f64 = f64::from_bits(0x3ff9_21fb_5444_2d18);
const PIO2_TRUNCATED: f64 = f64::from_bits(0x3ff9_21fb_5000_0000);
const PIO2_CORRECTION: f64 = f64::from_bits(0x3e51_10b4_6000_0000);
const PIO2_LOW: f64 = f64::from_bits(0x3c91_a626_3314_5c06);
const INV_PIO2: f64 = f64::from_bits(0x3fe4_5f30_6dc9_c883);
const REDUCTION_MIDDLE: f64 = f64::from_bits(0x3c91_a626_3314_5c00);
const REDUCTION_TAIL: f64 = f64::from_bits(0x397b_839a_2520_49c0);
const ROUND_MAGIC: f64 = f64::from_bits(0x4338_0000_0000_0000);
pub(super) const SIN_COEFFICIENTS: [f64; 6] = [
    f64::from_bits(0xbfc5_5555_5555_5555),
    f64::from_bits(0x3f81_1111_1111_0bb3),
    f64::from_bits(0xbf2a_01a0_19e8_3e5c),
    f64::from_bits(0x3ec7_1de3_796c_de01),
    f64::from_bits(0xbe5a_e600_b42f_dfa7),
    f64::from_bits(0x3de5_e0b2_f9a4_3bb8),
];
pub(super) const COS_COEFFICIENTS: [f64; 6] = [
    f64::from_bits(0x3fa5_5555_5555_5555),
    f64::from_bits(0xbf56_c16c_16c1_6967),
    f64::from_bits(0x3efa_01a0_19f4_ec91),
    f64::from_bits(0xbe92_7e4f_a17f_667b),
    f64::from_bits(0x3e21_eeb6_9038_2eec),
    f64::from_bits(0xbda9_07db_4725_8aa7),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseError {
    NonzeroSourceProfileRequired,
    UnsupportedEnvironment,
    NonFiniteInput,
    OutsidePositiveReductionDomain,
    ConsumedUndefinedBitScan,
    InfinityDomainPathRequired,
    OutsideTinyDomain,
    OutsideSmallDomain,
    OutsideRegularDirectDomain,
    NormalArithmeticRequired { operation: &'static str, reason: NormalArithmeticError, advanced: ExplicitExceptionState },
    DomainHandlerRequired { input_bits: u64, requested_result_bits: u64 },
}

#[derive(Clone, Copy, Debug)]
pub struct PhaseReduction {
    pub quadrant: u32,
    pub head: f64,
    pub tail: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseReductionWithState {
    pub quadrant: u32,
    pub head_bits: u64,
    pub tail_bits: u64,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseErrorWithState {
    pub bits: u64,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseWithStatus {
    pub bits: u64,
    pub exception_state: ExplicitExceptionState,
    pub raised_status: u32,
}
impl PhaseWithStatus {
    fn without_status(self) -> PhaseErrorWithState {
        PhaseErrorWithState { bits: self.bits, exception_state: self.exception_state }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseStatusFailure {
    pub error: PhaseError,
    pub exception_state: ExplicitExceptionState,
    pub raised_status: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhaseEvaluationError {
    Arithmetic(PhaseError),
    MathError(MathErrorServiceError),
}

pub struct SourcePhaseNonzero {
    explicit_source_cpu_word: u32,
    environment: Environment,
}

impl SourcePhaseNonzero {
    pub fn bind(word: u32, mxcsr: u32) -> Result<Self, PhaseError> {
        if word == 0 { return Err(PhaseError::NonzeroSourceProfileRequired); }
        let environment = Environment::from_explicit_current_mxcsr(mxcsr)
            .map_err(|_| PhaseError::UnsupportedEnvironment)?;
        Ok(Self { explicit_source_cpu_word: word, environment })
    }

    pub fn explicit_source_cpu_word(&self) -> u32 { self.explicit_source_cpu_word }
    pub fn environment(&self) -> Environment { self.environment }

    pub fn tiny_sin_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        self.tiny_sin_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state))
    }
    fn tiny_sin_with_status_accumulator(&self, bits: u64, advanced: &mut PhaseStatusAccumulator)
        -> Result<PhaseErrorWithState, PhaseError> {
        let magnitude = bits & !SIGN;
        if magnitude >= TINY_LIMIT { return Err(PhaseError::OutsideTinyDomain); }
        let field = (magnitude >> 52) as i32;
        let fraction = magnitude & 0x000f_ffff_ffff_ffff;
        let status = if magnitude == 0 { 0 }
            else if field == 0 { 0x32 }
            else if magnitude == 0x0010_0000_0000_0000 { 0x30 }
            else if fraction != 0 { 0x20 }
            else {
                let exact_add_threshold = if bits & SIGN == 0 { 971 } else { 970 };
                if field < exact_add_threshold { 0x20 } else { 0 }
            };
        advanced.raise(status);
        Ok(PhaseErrorWithState { bits, exception_state: advanced.state })
    }

    pub fn tiny_cos_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        self.tiny_cos_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state))
    }
    fn tiny_cos_with_status_accumulator(&self, bits: u64, advanced: &mut PhaseStatusAccumulator)
        -> Result<PhaseErrorWithState, PhaseError> {
        let magnitude = bits & !SIGN;
        if magnitude >= TINY_LIMIT { return Err(PhaseError::OutsideTinyDomain); }
        let status = if magnitude == 0 { 0 }
            else if magnitude < 0x0010_0000_0000_0000 { 0x22 } else { 0x20 };
        advanced.raise(status);
        Ok(PhaseErrorWithState { bits: ONE.to_bits(), exception_state: advanced.state })
    }

    pub fn small_cos_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        self.small_cos_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state))
    }
    fn small_cos_with_status_accumulator(&self, bits: u64, advanced: &mut PhaseStatusAccumulator)
        -> Result<PhaseErrorWithState, PhaseError> {
        let magnitude = bits & !SIGN;
        if !(TINY_LIMIT..SMALL_LIMIT).contains(&magnitude) {
            return Err(PhaseError::OutsideSmallDomain);
        }
        let exponent = ((magnitude >> 52) & 0x7ff) as i32 - 1023;
        let significand = (magnitude & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
        let discard = ((if bits & SIGN == 0 { 32 } else { 31 }) - exponent) as u32;
        let add_lost = significand & ((1u64 << discard) - 1);
        let (result, fused_status) = super::finite_power::small_cosine_fma_bits(bits);
        let status = fused_status | (if add_lost != 0 { 0x20 } else { 0 });
        advanced.raise(status);
        Ok(PhaseErrorWithState { bits: result, exception_state: advanced.state })
    }

    pub fn small_sin_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        self.small_sin_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state))
    }
    fn small_sin_with_status_accumulator(&self, bits: u64, advanced: &mut PhaseStatusAccumulator)
        -> Result<PhaseErrorWithState, PhaseError> {
        let magnitude = bits & !SIGN;
        if !(TINY_LIMIT..SMALL_LIMIT).contains(&magnitude) {
            return Err(PhaseError::OutsideSmallDomain);
        }
        let (result, status) = super::finite_power::small_sine_fma_bits(bits);
        advanced.raise(status);
        Ok(PhaseErrorWithState { bits: result, exception_state: advanced.state })
    }

    pub fn regular_sin_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        self.regular_direct_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state), true)
    }

    pub fn regular_cos_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        self.regular_direct_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state), false)
    }
    fn regular_direct_with_status_accumulator(
        &self, bits: u64, advanced: &mut PhaseStatusAccumulator, sine: bool,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        let magnitude = bits & !SIGN;
        if magnitude < SMALL_LIMIT || magnitude > DIRECT_LIMIT || (sine && magnitude == DIRECT_LIMIT) {
            return Err(PhaseError::OutsideRegularDirectDomain);
        }
        let (square, flags) = super::finite_power::normal_multiply_source_bits(bits, bits)
            .map_err(|reason| PhaseError::NormalArithmeticRequired { operation: "direct_square", reason, advanced: advanced.state })?;
        advanced.raise(flags);
        let coefficients = if sine { &SIN_COEFFICIENTS } else { &COS_COEFFICIENTS };
        let mut polynomial = coefficients[5].to_bits();
        let mut cube = 0u64;
        for (index, coefficient) in coefficients[..5].iter().rev().enumerate() {

            if sine && index == 4 {
                let (value, flags) = super::finite_power::normal_multiply_source_bits(bits, square)
                    .map_err(|reason| PhaseError::NormalArithmeticRequired { operation: "direct_cube", reason, advanced: advanced.state })?;
                cube = value;
                advanced.raise(flags);
            }
            let (next, flags) = super::finite_power::normal_fma_source_bits(polynomial, square, coefficient.to_bits())
                .map_err(|reason| PhaseError::NormalArithmeticRequired { operation: "direct_horner_fma", reason, advanced: advanced.state })?;
            polynomial = next;
            advanced.raise(flags);
        }
        let result;
        if sine {
            let (value, flags) = super::finite_power::normal_fma_source_bits(cube, polynomial, bits)
                .map_err(|reason| PhaseError::NormalArithmeticRequired { operation: "direct_sine_fma", reason, advanced: advanced.state })?;
            result = value;
            advanced.raise(flags);
        } else {
            let (leading, flags) = super::finite_power::normal_fma_source_bits(polynomial, square, HALF.to_bits() | SIGN)
                .map_err(|reason| PhaseError::NormalArithmeticRequired { operation: "direct_cosine_half_fma", reason, advanced: advanced.state })?;
            advanced.raise(flags);
            let (value, flags) = super::finite_power::normal_fma_source_bits(leading, square, ONE.to_bits())
                .map_err(|reason| PhaseError::NormalArithmeticRequired { operation: "direct_cosine_one_fma", reason, advanced: advanced.state })?;
            result = value;
            advanced.raise(flags);
        }
        Ok(PhaseErrorWithState { bits: result, exception_state: advanced.state })
    }

    pub fn sin_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        self.sin_with_explicit_status(bits, state).map(PhaseWithStatus::without_status).map_err(|failure| failure.error)
    }

    pub fn cos_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        self.cos_with_explicit_status(bits, state).map(PhaseWithStatus::without_status).map_err(|failure| failure.error)
    }

    pub fn sin_with_explicit_status(&self, bits: u64, state: ExplicitExceptionState)
        -> Result<PhaseWithStatus, PhaseStatusFailure> {
        self.source_with_status(bits, state, true)
    }
    pub fn cos_with_explicit_status(&self, bits: u64, state: ExplicitExceptionState)
        -> Result<PhaseWithStatus, PhaseStatusFailure> {
        self.source_with_status(bits, state, false)
    }
    fn source_with_status(&self, bits: u64, state: ExplicitExceptionState, sine: bool)
        -> Result<PhaseWithStatus, PhaseStatusFailure> {
        let mut advanced = PhaseStatusAccumulator::new(state);
        let result = self.source_with_status_accumulator(bits, &mut advanced, sine);
        match result {
            Ok(result) => Ok(PhaseWithStatus { bits: result.bits,
                exception_state: result.exception_state, raised_status: advanced.raised }),
            Err(error) => Err(PhaseStatusFailure { error, exception_state: advanced.state,
                raised_status: advanced.raised }),
        }
    }

    pub fn sin_with_callback(
        &self, bits: u64, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<PhaseErrorWithState, PhaseEvaluationError> {
        if bits & !SIGN == INFINITY {
            self.resolve_infinity_with_callback(bits, MathErrorOperation::Sine, state, callback)
                .map_err(PhaseEvaluationError::MathError)
        } else { self.sin_with_explicit_state(bits, state).map_err(PhaseEvaluationError::Arithmetic) }
    }

    pub fn cos_with_callback(
        &self, bits: u64, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<PhaseErrorWithState, PhaseEvaluationError> {
        if bits & !SIGN == INFINITY {
            self.resolve_infinity_with_callback(bits, MathErrorOperation::Cosine, state, callback)
                .map_err(PhaseEvaluationError::MathError)
        } else { self.cos_with_explicit_state(bits, state).map_err(PhaseEvaluationError::Arithmetic) }
    }

    fn source_with_status_accumulator(
        &self, bits: u64, advanced: &mut PhaseStatusAccumulator, sine: bool,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        if let Some(result) = nonfinite_bits(bits) {
            return result.map(|bits| PhaseErrorWithState { bits, exception_state: advanced.state });
        }
        let magnitude = bits & !SIGN;
        if magnitude < DIRECT_LIMIT || (!sine && magnitude == DIRECT_LIMIT) {
            if magnitude < TINY_LIMIT {
                return if sine { self.tiny_sin_with_status_accumulator(bits, advanced) }
                    else { self.tiny_cos_with_status_accumulator(bits, advanced) };
            }
            if magnitude < SMALL_LIMIT {
                return if sine { self.small_sin_with_status_accumulator(bits, advanced) }
                    else { self.small_cos_with_status_accumulator(bits, advanced) };
            }
            return self.regular_direct_with_status_accumulator(bits, advanced, sine);
        }
        let reduction = self.reduction_with_status_accumulator(magnitude, advanced)?;
        let sine_kernel = if sine { reduction.quadrant & 1 == 0 } else { reduction.quadrant & 1 != 0 };
        let mut result = reduced_kernel_with_status_accumulator(
            reduction.head_bits, reduction.tail_bits, advanced, sine_kernel,
        )?;
        let sign = if sine {
            (bits & SIGN) ^ (u64::from(reduction.quadrant & 2 != 0) << 63)
        } else { u64::from((reduction.quadrant + 1) & 2 != 0) << 63 };
        result.bits ^= sign;
        Ok(result)
    }

    pub fn evaluate_sin_source_bits(&self, bits: u64) -> Result<u64, PhaseError> {
        if let Some(result) = nonfinite_bits(bits) { return result; }
        self.evaluate_sin(f64::from_bits(bits)).map(f64::to_bits)
    }

    pub fn evaluate_cos_source_bits(&self, bits: u64) -> Result<u64, PhaseError> {
        if let Some(result) = nonfinite_bits(bits) { return result; }
        self.evaluate_cos(f64::from_bits(bits)).map(f64::to_bits)
    }

    pub fn resolve_infinity_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState, callback: MathErrorCallbackPolicy,
    ) -> Result<PhaseErrorWithState, PhaseError> {
        if bits & !SIGN != INFINITY { return Err(PhaseError::InfinityDomainPathRequired); }
        let requested_result_bits = 0xfff8_0000_0000_0000;
        if callback == MathErrorCallbackPolicy::Present {
            return Err(PhaseError::DomainHandlerRequired { input_bits: bits, requested_result_bits });
        }
        Ok(PhaseErrorWithState {
            bits: requested_result_bits,
            exception_state: state.apply_masked_math_error(MaskedMathError::Domain),
        })
    }

    pub fn resolve_infinity_with_callback(
        &self, bits: u64, operation: MathErrorOperation, state: ExplicitExceptionState,
        callback: &mut impl SourceMathErrorCallback,
    ) -> Result<PhaseErrorWithState, MathErrorServiceError> {
        if bits & !SIGN != INFINITY || !matches!(operation, MathErrorOperation::Sine | MathErrorOperation::Cosine) {
            return Err(MathErrorServiceError::PathRequired { advanced: state });
        }
        let request = MathErrorCall {
            operation, error: MaskedMathError::Domain,
            argument1_f64_bits: bits, argument2_f64_bits: 0,
            proposed_result_f64_bits: 0xfff8_0000_0000_0000,
        };
        let completed = state.call_math_error(request, callback).map_err(MathErrorServiceError::Callback)?;
        Ok(PhaseErrorWithState { bits: request.proposed_result_f64_bits, exception_state: completed.advanced })
    }

    pub fn reduce_positive(&self, value: f64) -> Result<PhaseReduction, PhaseError> {
        let bits = value.to_bits();
        if !(DIRECT_LIMIT..INFINITY).contains(&bits) {
            return Err(PhaseError::OutsidePositiveReductionDomain);
        }
        if bits < LARGE_LIMIT { Ok(reduce_moderate(value)) } else { reduce_large(bits) }
    }

    pub fn moderate_reduction_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseReductionWithState, PhaseError> {
        self.moderate_reduction_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state))
    }
    fn moderate_reduction_with_status_accumulator(
        &self, bits: u64, advanced: &mut PhaseStatusAccumulator,
    ) -> Result<PhaseReductionWithState, PhaseError> {
        if !(DIRECT_LIMIT..LARGE_LIMIT).contains(&bits) {
            return Err(PhaseError::OutsidePositiveReductionDomain);
        }
        let rounded = source_phase_fma(bits, INV_PIO2.to_bits(), ROUND_MAGIC.to_bits(), "moderate_magic_fma", advanced)?;
        let count = source_phase_subtract(rounded, ROUND_MAGIC.to_bits(), "moderate_count_subtract", advanced)?;
        let magnitude = count & !SIGN;
        let integer = if magnitude == 0 { 0 } else {

            let exponent = ((magnitude >> 52) & 0x7ff) as u32 - 1023;
            let significand = (magnitude & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
            (significand >> (52 - exponent)) as u32
        };
        let first = source_phase_fma(count ^ SIGN, PIO2.to_bits(), bits, "moderate_first_remainder_fma", advanced)?;
        let product = source_phase_multiply(count, REDUCTION_MIDDLE.to_bits(), "moderate_middle_product", advanced)?;
        let product_error = source_phase_fma(count, REDUCTION_MIDDLE.to_bits(), product ^ SIGN, "moderate_product_error_fma", advanced)?;
        let difference = source_phase_subtract(first, product, "moderate_difference", advanced)?;
        let lost = source_phase_subtract(first, difference, "moderate_lost_head", advanced)?;
        let lost = source_phase_subtract(lost, product, "moderate_lost_product", advanced)?;
        let head = source_phase_fma(count ^ SIGN, REDUCTION_MIDDLE.to_bits(), first, "moderate_head_fma", advanced)?;
        let tail = source_phase_subtract(difference, head, "moderate_tail_difference", advanced)?;
        let tail = source_phase_fma(tail, ONE.to_bits(), lost, "moderate_tail_add", advanced)?;
        let tail = source_phase_subtract(tail, product_error, "moderate_tail_product_error", advanced)?;
        let tail = source_phase_fma(count ^ SIGN, REDUCTION_TAIL.to_bits(), tail, "moderate_tail_fma", advanced)?;
        Ok(PhaseReductionWithState { quadrant: integer & 3, head_bits: head, tail_bits: tail, exception_state: advanced.state })
    }

    pub fn large_reduction_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseReductionWithState, PhaseError> {
        self.large_reduction_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state))
    }
    fn large_reduction_with_status_accumulator(
        &self, bits: u64, advanced: &mut PhaseStatusAccumulator,
    ) -> Result<PhaseReductionWithState, PhaseError> {
        if !(LARGE_LIMIT..INFINITY).contains(&bits) {
            return Err(PhaseError::OutsidePositiveReductionDomain);
        }
        let IntegerPhaseHeadTail { quadrant, head_bits, tail_bits } = integer_large_head_tail(bits)?;
        let truncated = head_bits & 0xffff_ffff_f800_0000;
        let delta = source_phase_subtract(head_bits, truncated, "large_truncated_delta", advanced)?;
        let product = source_phase_multiply(head_bits, PIO2.to_bits(), "large_head_product", advanced)?;
        let correction = source_phase_multiply(truncated, PIO2_TRUNCATED.to_bits(), "large_truncated_product", advanced)?;
        let correction = source_phase_subtract(correction, product, "large_product_difference", advanced)?;
        let correction = source_phase_fma(delta, PIO2_TRUNCATED.to_bits(), correction, "large_delta_high_fma", advanced)?;
        let correction = source_phase_fma(truncated, PIO2_CORRECTION.to_bits(), correction, "large_truncated_low_fma", advanced)?;
        let correction = source_phase_fma(delta, PIO2_CORRECTION.to_bits(), correction, "large_delta_low_fma", advanced)?;
        let small = source_phase_multiply(tail_bits, PIO2.to_bits(), "large_tail_product", advanced)?;
        let small = source_phase_fma(head_bits, PIO2_LOW.to_bits(), small, "large_head_tail_fma", advanced)?;
        let correction = source_phase_fma(correction, ONE.to_bits(), small, "large_tail_add", advanced)?;
        let result = source_phase_fma(product, ONE.to_bits(), correction, "large_result_add", advanced)?;
        let lost = source_phase_subtract(product, result, "large_result_lost", advanced)?;
        let tail = source_phase_fma(lost, ONE.to_bits(), correction, "large_tail_finish_add", advanced)?;
        Ok(PhaseReductionWithState { quadrant, head_bits: result, tail_bits: tail, exception_state: advanced.state })
    }

    pub fn reduction_with_explicit_state(
        &self, bits: u64, state: ExplicitExceptionState,
    ) -> Result<PhaseReductionWithState, PhaseError> {
        self.reduction_with_status_accumulator(bits, &mut PhaseStatusAccumulator::new(state))
    }
    fn reduction_with_status_accumulator(&self, bits: u64, advanced: &mut PhaseStatusAccumulator)
        -> Result<PhaseReductionWithState, PhaseError> {
        if !(DIRECT_LIMIT..INFINITY).contains(&bits) {
            return Err(PhaseError::OutsidePositiveReductionDomain);
        }
        if bits < LARGE_LIMIT { self.moderate_reduction_with_status_accumulator(bits, advanced) }
        else { self.large_reduction_with_status_accumulator(bits, advanced) }
    }

    pub fn evaluate_sin(&self, value: f64) -> Result<f64, PhaseError> {
        let bits = value.to_bits();
        let magnitude = bits & !SIGN;
        if magnitude >= INFINITY { return Err(PhaseError::NonFiniteInput); }
        if magnitude < DIRECT_LIMIT {
            if magnitude < TINY_LIMIT { return Ok(value); }
            let square = value * value;
            if magnitude < SMALL_LIMIT {
                let cube = square * value;
                return Ok((-cube).mul_add(f64::from_bits(0x3fc5_5555_5555_5555), value));
            }
            let polynomial = sin_polynomial(square);
            let cube = value * square;
            return Ok(cube.mul_add(polynomial, value));
        }
        let reduced = self.reduce_positive(f64::from_bits(magnitude))?;
        let result = if reduced.quadrant & 1 == 0 {
            reduced_sin(reduced.head, reduced.tail)
        } else { reduced_cos(reduced.head, reduced.tail) };
        let flip = (bits & SIGN) ^ (u64::from(reduced.quadrant & 2 != 0) << 63);
        Ok(f64::from_bits(result.to_bits() ^ flip))
    }

    pub fn evaluate_cos(&self, value: f64) -> Result<f64, PhaseError> {
        let magnitude = value.to_bits() & !SIGN;
        if magnitude >= INFINITY { return Err(PhaseError::NonFiniteInput); }

        if magnitude <= DIRECT_LIMIT {
            if magnitude < TINY_LIMIT { return Ok(ONE); }
            if magnitude < SMALL_LIMIT {
                let half_input = value * HALF;
                return Ok((-value).mul_add(half_input, ONE));
            }
            let square = value * value;
            let polynomial = cos_polynomial(square);
            return Ok(polynomial.mul_add(square, -HALF).mul_add(square, ONE));
        }
        let reduced = self.reduce_positive(f64::from_bits(magnitude))?;
        let result = if reduced.quadrant & 1 == 0 {
            reduced_cos(reduced.head, reduced.tail)
        } else { reduced_sin(reduced.head, reduced.tail) };
        let flip = u64::from((reduced.quadrant + 1) & 2 != 0) << 63;
        Ok(f64::from_bits(result.to_bits() ^ flip))
    }
}

fn source_phase_fma(
    left: u64, right: u64, addend: u64, operation: &'static str, advanced: &mut PhaseStatusAccumulator,
) -> Result<u64, PhaseError> {
    let (bits, flags) = super::finite_power::normal_fma_source_bits(left, right, addend)
        .map_err(|reason| PhaseError::NormalArithmeticRequired { operation, reason, advanced: advanced.state })?;
    advanced.raise(flags);
    Ok(bits)
}

fn source_phase_multiply(
    left: u64, right: u64, operation: &'static str, advanced: &mut PhaseStatusAccumulator,
) -> Result<u64, PhaseError> {
    let (bits, flags) = super::finite_power::normal_multiply_source_bits(left, right)
        .map_err(|reason| PhaseError::NormalArithmeticRequired { operation, reason, advanced: advanced.state })?;
    advanced.raise(flags);
    Ok(bits)
}

fn source_phase_subtract(
    left: u64, right: u64, operation: &'static str, advanced: &mut PhaseStatusAccumulator,
) -> Result<u64, PhaseError> {

    source_phase_fma(left, ONE.to_bits(), right ^ SIGN, operation, advanced)
}

fn reduced_kernel_with_status_accumulator(
    head: u64, tail: u64, advanced: &mut PhaseStatusAccumulator, sine: bool,
) -> Result<PhaseErrorWithState, PhaseError> {

    let square = source_phase_multiply(head, head, "reduced_square", advanced)?;
    let result;
    if sine {
        let mut polynomial = SIN_COEFFICIENTS[5].to_bits();
        for coefficient in SIN_COEFFICIENTS[1..5].iter().rev() {
            polynomial = source_phase_fma(polynomial, square, coefficient.to_bits(), "reduced_sine_horner_fma", advanced)?;
        }
        let cube = source_phase_multiply(head, square, "reduced_cube", advanced)?;
        let product = source_phase_multiply(cube, polynomial, "reduced_polynomial_product", advanced)?;
        let half_tail = source_phase_multiply(tail, HALF.to_bits(), "reduced_half_tail", advanced)?;
        let difference = source_phase_subtract(half_tail, product, "reduced_tail_product_difference", advanced)?;
        let difference = source_phase_multiply(square, difference, "reduced_square_difference", advanced)?;
        let difference = source_phase_subtract(difference, tail, "reduced_tail_subtract", advanced)?;
        let difference = source_phase_fma(cube ^ SIGN, SIN_COEFFICIENTS[0].to_bits(), difference, "reduced_cubic_correction_fma", advanced)?;
        result = source_phase_subtract(head, difference, "reduced_sine_finish", advanced)?;
    } else {
        let half_square = source_phase_multiply(square, HALF.to_bits(), "reduced_half_square", advanced)?;
        let leading = source_phase_subtract(ONE.to_bits(), half_square, "reduced_cosine_leading", advanced)?;
        let correction = source_phase_subtract(ONE.to_bits(), leading, "reduced_leading_loss", advanced)?;
        let correction = source_phase_subtract(correction, half_square, "reduced_square_loss", advanced)?;
        let correction = source_phase_fma(head ^ SIGN, tail, correction, "reduced_head_tail_fma", advanced)?;
        let fourth = source_phase_multiply(square, square, "reduced_fourth", advanced)?;
        let mut polynomial = COS_COEFFICIENTS[5].to_bits();
        for coefficient in COS_COEFFICIENTS[..5].iter().rev() {
            polynomial = source_phase_fma(polynomial, square, coefficient.to_bits(), "reduced_cosine_horner_fma", advanced)?;
        }
        let correction = source_phase_fma(polynomial, fourth, correction, "reduced_cosine_correction_fma", advanced)?;
        result = source_phase_fma(correction, ONE.to_bits(), leading, "reduced_cosine_finish_add", advanced)?;
    }
    Ok(PhaseErrorWithState { bits: result, exception_state: advanced.state })
}

fn nonfinite_bits(bits: u64) -> Option<Result<u64, PhaseError>> {
    let magnitude = bits & !SIGN;
    if magnitude > INFINITY { return Some(Ok(bits | 0x0008_0000_0000_0000)); }
    if magnitude == INFINITY {
        return Some(Err(PhaseError::DomainHandlerRequired {
            input_bits: bits, requested_result_bits: 0xfff8_0000_0000_0000,
        }));
    }
    None
}

fn sin_polynomial(square: f64) -> f64 {
    let mut value = SIN_COEFFICIENTS[5];
    for coefficient in SIN_COEFFICIENTS[..5].iter().rev() {
        value = value.mul_add(square, *coefficient);
    }
    value
}

fn cos_polynomial(square: f64) -> f64 {
    let mut value = COS_COEFFICIENTS[5];
    for coefficient in COS_COEFFICIENTS[..5].iter().rev() {
        value = value.mul_add(square, *coefficient);
    }
    value
}

fn reduced_sin(head: f64, tail: f64) -> f64 {
    let square = head * head;
    let mut polynomial = SIN_COEFFICIENTS[5];
    for coefficient in SIN_COEFFICIENTS[1..5].iter().rev() {
        polynomial = polynomial.mul_add(square, *coefficient);
    }
    let cube = head * square;
    let product = cube * polynomial;
    let half_tail = tail * HALF;
    let difference = half_tail - product;
    let difference = square * difference;
    let difference = difference - tail;
    let difference = (-cube).mul_add(SIN_COEFFICIENTS[0], difference);
    head - difference
}

fn reduced_cos(head: f64, tail: f64) -> f64 {
    let square = head * head;
    let half_square = square * HALF;
    let leading = ONE - half_square;
    let correction = ONE - leading;
    let correction = correction - half_square;
    let correction = (-head).mul_add(tail, correction);
    let fourth = square * square;
    let polynomial = cos_polynomial(square);
    polynomial.mul_add(fourth, correction) + leading
}

fn reduce_moderate(input: f64) -> PhaseReduction {
    let rounded = input.mul_add(INV_PIO2, ROUND_MAGIC);
    let count = rounded - ROUND_MAGIC;
    let quadrant = (count as i32 as u32) & 3;
    let first = (-count).mul_add(PIO2, input);
    let product = count * REDUCTION_MIDDLE;
    let product_error = count.mul_add(REDUCTION_MIDDLE, -product);
    let difference = first - product;
    let lost = first - difference;
    let lost = lost - product;
    let head = (-count).mul_add(REDUCTION_MIDDLE, first);
    let tail = difference - head;
    let tail = tail + lost;
    let tail = tail - product_error;
    let tail = (-count).mul_add(REDUCTION_TAIL, tail);
    PhaseReduction { quadrant, head, tail }
}

fn table_qword(index: usize) -> u64 {
    u64::from_le_bytes(TABLE[index..index + 8].try_into().unwrap())
}

struct IntegerReduction {
    quadrant: u32,
    head_bits: u64,
    middle: u64,
    biased: i64,
    sign: u64,
}

pub(super) struct IntegerPhaseHeadTail {
    pub(super) quadrant: u32,
    pub(super) head_bits: u64,
    pub(super) tail_bits: u64,
}

pub(super) fn integer_large_head_tail(bits: u64) -> Result<IntegerPhaseHeadTail, PhaseError> {
    if !(0x411e_8480_0000_0000..INFINITY).contains(&bits) {
        return Err(PhaseError::OutsidePositiveReductionDomain);
    }
    let IntegerReduction { quadrant, head_bits, middle, biased, sign } = decompose_large(bits)?;
    if middle == 0 { return Err(PhaseError::ConsumedUndefinedBitScan); }
    let tail_top = 63 - i64::from(middle.leading_zeros());
    let left = 64 - tail_top;
    let fraction = middle.wrapping_shl(left as u32) >> 12;
    let tail_bits = fraction | sign | ((biased - left - 52) as u64).wrapping_shl(52);
    Ok(IntegerPhaseHeadTail { quadrant, head_bits, tail_bits })
}

fn decompose_large(bits: u64) -> Result<IntegerReduction, PhaseError> {
    let exponent = (bits >> 52) - 1023;
    let index = 134 - (exponent >> 3) as usize;
    let mantissa = (bits & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let first = u128::from(mantissa) * u128::from(table_qword(index));
    let second = u128::from(mantissa) * u128::from(table_qword(index + 8));
    let third = u128::from(mantissa) * u128::from(table_qword(index + 16));
    let mut low = first as u64;
    let (mut middle, carry) = (second as u64).overflowing_add((first >> 64) as u64);
    let mut high = ((second >> 64) as u64).wrapping_add(u64::from(carry))
        .wrapping_add(third as u64);
    let residue = exponent & 7;
    let shift = 54 - residue;
    let round_up = high >> (shift - 1) & 1 != 0;
    let quadrant = ((high >> shift).wrapping_add(u64::from(round_up)) & 3) as u32;
    let sign = if round_up {
        high = !high; middle = !middle; low = !low; SIGN
    } else { 0 };
    high = high.wrapping_shl((residue + 10) as u32)
        .wrapping_shr((residue + 10) as u32);
    let mut head_exponent = residue as i64 - 54;
    if high == 0 {
        high = middle; middle = low; low = 0; head_exponent -= 64;
    }
    if high == 0 { return Err(PhaseError::ConsumedUndefinedBitScan); }
    let top = 63 - i64::from(high.leading_zeros());
    head_exponent += top;
    let adjustment = top - 52;
    if adjustment > 0 {
        let saved = high;
        high = high.wrapping_shr(adjustment as u32);
        middle = middle.wrapping_shr(adjustment as u32)
            | saved.wrapping_shl((64 - adjustment) as u32);
    } else if adjustment < 0 {
        let left = -adjustment;
        let saved = middle;
        high = high.wrapping_shl(left as u32)
            | saved.wrapping_shr((64 - left) as u32);
        middle = middle.wrapping_shl(left as u32)
            | low.wrapping_shr((64 - left) as u32);
    }
    let biased = head_exponent + 1023;
    let head_bits = (high & !(1u64 << 52)) | sign | (biased as u64).wrapping_shl(52);
    Ok(IntegerReduction { quadrant, head_bits, middle, biased, sign })
}

pub(crate) fn single_large_head(bits: u64) -> Result<(u32, f64), PhaseError> {
    if !(0x4170_008a_c000_0000..INFINITY).contains(&bits) {
        return Err(PhaseError::OutsidePositiveReductionDomain);
    }
    let parts = decompose_large(bits)?;
    Ok((parts.quadrant, f64::from_bits(parts.head_bits) * PIO2))
}

pub(super) fn single_large_head_with_explicit_state(
    bits: u64, advanced: ExplicitExceptionState,
) -> Result<(u32, u64, ExplicitExceptionState), PhaseError> {
    if !(0x4170_008a_c000_0000..INFINITY).contains(&bits) {
        return Err(PhaseError::OutsidePositiveReductionDomain);
    }
    large_head_product_with_explicit_state(bits, advanced, "single_large_head_product")
}

pub(super) fn cpu0_cosine_large_head_with_explicit_state(
    bits: u64, advanced: ExplicitExceptionState,
) -> Result<(u32, u64, ExplicitExceptionState), PhaseError> {
    if !(0x411e_8480_0000_0000..INFINITY).contains(&bits) {
        return Err(PhaseError::OutsidePositiveReductionDomain);
    }
    large_head_product_with_explicit_state(bits, advanced, "597c83/CPU0 cosine head MULSD")
}

pub(super) fn cpu0_sine_large_head_with_explicit_state(
    bits: u64, advanced: ExplicitExceptionState,
) -> Result<(u32, u64, ExplicitExceptionState), PhaseError> {
    if !(0x416e_8480_0000_0000..INFINITY).contains(&bits) {
        return Err(PhaseError::OutsidePositiveReductionDomain);
    }
    large_head_product_with_explicit_state(bits, advanced, "59bace/CPU0 sine head MULSD")
}

fn large_head_product_with_explicit_state(bits: u64, state: ExplicitExceptionState,
    operation: &'static str) -> Result<(u32, u64, ExplicitExceptionState), PhaseError> {
    let parts = decompose_large(bits)?;
    let mut advanced = PhaseStatusAccumulator::new(state);
    let head = source_phase_multiply(parts.head_bits, PIO2.to_bits(), operation, &mut advanced)?;
    Ok((parts.quadrant, head, advanced.state))
}

fn reduce_large(bits: u64) -> Result<PhaseReduction, PhaseError> {
    let IntegerReduction { quadrant, head_bits, middle, biased, sign } = decompose_large(bits)?;
    if middle == 0 { return Err(PhaseError::ConsumedUndefinedBitScan); }
    let tail_top = 63 - i64::from(middle.leading_zeros());
    let left = 64 - tail_top;
    let fraction = middle.wrapping_shl(left as u32) >> 12;
    let tail_bits = fraction | sign | ((biased - left - 52) as u64).wrapping_shl(52);
    let head = f64::from_bits(head_bits);
    let small = f64::from_bits(tail_bits);
    let truncated = f64::from_bits(head_bits & 0xffff_ffff_f800_0000);
    let delta = head - truncated;
    let product = head * PIO2;
    let correction = truncated * PIO2_TRUNCATED;
    let correction = correction - product;
    let correction = delta.mul_add(PIO2_TRUNCATED, correction);
    let correction = truncated.mul_add(PIO2_CORRECTION, correction);
    let correction = delta.mul_add(PIO2_CORRECTION, correction);
    let small = small * PIO2;
    let small = head.mul_add(PIO2_LOW, small);
    let correction = correction + small;
    let result = product + correction;
    let lost = product - result;
    let tail = lost + correction;
    Ok(PhaseReduction { quadrant, head: result, tail })
}
