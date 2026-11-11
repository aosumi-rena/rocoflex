use super::selected_phase::{single_large_head, single_large_head_with_explicit_state, PhaseError, SourcePhaseNonzero};
use super::selected_log::Environment;
use super::finite_power::{ExplicitExceptionState, MaskedMathError, MathErrorCallbackPolicy, MathErrorWithState};
use super::finite_power::{MathErrorCall, MathErrorCallbackCompletion, MathErrorCallbackFloat32, MathErrorConversionRequired, MathErrorOperation, MathErrorServiceError, NormalArithmeticError, SourceMathErrorCallback};

const SIGN64: u64 = 0x8000_0000_0000_0000;
pub(super) const DIRECT_LIMIT: u64 = 0x3fe9_21fb_5444_2d18;
const TINY_LIMIT: u64 = 0x3f20_0000_0000_0000;
const SMALL_LIMIT: u64 = 0x3f80_0000_0000_0000;
const SIN_SHORT_LIMIT: u64 = 0x4170_008a_c000_0000;
const COS_SHORT_LIMIT: u64 = 0x41e9_21fb_6000_0000;
const ONE: f64 = f64::from_bits(0x3ff0_0000_0000_0000);
const HALF: f64 = f64::from_bits(0x3fe0_0000_0000_0000);
pub(super) const INV_PIO2: f64 = f64::from_bits(0x3fe4_5f30_6dc9_c883);
pub(super) const SPLIT_HIGH: f64 = f64::from_bits(0x3ff9_21fb_5440_0000);
pub(super) const SPLIT_LOW: f64 = f64::from_bits(0x3dd0_b461_1a62_6331);
pub(super) const SIN_COEFFICIENTS: [f64; 4] = [
    f64::from_bits(0xbfc5_5555_5555_5555),
    f64::from_bits(0x3f81_1111_1111_1111),
    f64::from_bits(0xbf2a_01a0_1a01_a01a),
    f64::from_bits(0x3ec7_1de3_a556_c734),
];
pub(super) const COS_COEFFICIENTS: [f64; 4] = [
    f64::from_bits(0x3fa5_5555_5555_5555),
    f64::from_bits(0xbf56_c16c_16c1_6c16),
    f64::from_bits(0x3efa_01a0_1a01_a019),
    f64::from_bits(0xbe92_7e4f_b778_9f5c),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarTrigError {
    NonzeroSourceProfileRequired,
    UnsupportedEnvironment,
    NonFiniteInput,
    Reduction(PhaseError),
    InfinityDomainPathRequired,
    OutsideTinyDomain,
    OutsideSmallDomain,
    Conversion(MathErrorConversionRequired),
    OutsideRegularDirectDomain,
    NormalArithmeticRequired { operation: &'static str, reason: NormalArithmeticError, advanced: ExplicitExceptionState },
    OutsideShortReductionDomain,
    DomainHandlerRequired { input_bits: u32, requested_result_bits: u32 },
}

impl ScalarTrigError {

    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::NormalArithmeticRequired { advanced, .. }
                | Self::Reduction(PhaseError::NormalArithmeticRequired { advanced, .. }) => advanced,
            Self::Conversion(required) => required.advanced,
            _ => entry,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarTrigWithState {
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarTrigReductionWithState {
    pub quadrant: u32,
    pub head_bits: u64,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScalarTrigEvaluationError {
    Arithmetic(ScalarTrigError),
    MathError(MathErrorServiceError),
}

pub struct SourceCoefficientTrigNonzero {
    phase: SourcePhaseNonzero,
}

impl SourceCoefficientTrigNonzero {
    pub fn bind(word: u32, mxcsr: u32) -> Result<Self, ScalarTrigError> {
        if word == 0 { return Err(ScalarTrigError::NonzeroSourceProfileRequired); }
        let phase = SourcePhaseNonzero::bind(word, mxcsr)
            .map_err(|_| ScalarTrigError::UnsupportedEnvironment)?;
        Ok(Self { phase })
    }

    pub fn explicit_source_cpu_word(&self) -> u32 { self.phase.explicit_source_cpu_word() }
    pub fn environment(&self) -> Environment { self.phase.environment() }

    pub fn tiny_sin_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude >= 0x3900_0000 { return Err(ScalarTrigError::OutsideTinyDomain); }
        let status = tiny_sine_source_status(bits);
        Ok(ScalarTrigWithState { bits, exception_state: state.with_source_status(status) })
    }

    pub fn tiny_cos_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude >= 0x3900_0000 { return Err(ScalarTrigError::OutsideTinyDomain); }
        let status = if magnitude == 0 { 0 }
            else if magnitude < 0x0080_0000 { 0x22 } else { 0x20 };
        Ok(ScalarTrigWithState {
            bits: 0x3f80_0000, exception_state: state.with_source_status(status),
        })
    }

    pub fn small_cos_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> {
        let magnitude = bits & 0x7fff_ffff;
        if !(0x3900_0000..0x3c00_0000).contains(&magnitude) {
            return Err(ScalarTrigError::OutsideSmallDomain);
        }
        let wide = (u64::from(bits & 0x8000_0000) << 32)
            | (u64::from((magnitude >> 23) + 896) << 52)
            | (u64::from(magnitude & 0x007f_ffff) << 29);
        let (result_f64_bits, fused_status) = super::finite_power::small_cosine_fma_bits(wide);
        let converted = MathErrorCallbackCompletion {
            result_f64_bits, advanced: state.with_source_status(fused_status),
        }.into_rne_f32().map_err(ScalarTrigError::Conversion)?;
        Ok(ScalarTrigWithState { bits: converted.bits, exception_state: converted.advanced })
    }

    pub fn small_sin_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> {
        let magnitude = bits & 0x7fff_ffff;
        if !(0x3900_0000..0x3c00_0000).contains(&magnitude) {
            return Err(ScalarTrigError::OutsideSmallDomain);
        }
        let wide = (u64::from(bits & 0x8000_0000) << 32)
            | (u64::from((magnitude >> 23) + 896) << 52)
            | (u64::from(magnitude & 0x007f_ffff) << 29);
        let (result_f64_bits, status) = super::finite_power::small_sine_fma_bits(wide);
        let converted = MathErrorCallbackCompletion {
            result_f64_bits, advanced: state.with_source_status(status),
        }.into_rne_f32().map_err(ScalarTrigError::Conversion)?;
        Ok(ScalarTrigWithState { bits: converted.bits, exception_state: converted.advanced })
    }

    pub fn regular_sin_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> {
        self.regular_direct_with_state(bits, state, true)
    }

    pub fn regular_cos_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> {
        self.regular_direct_with_state(bits, state, false)
    }

    fn regular_direct_with_state(
        &self, bits: u32, mut advanced: ExplicitExceptionState, sine: bool,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude < 0x3c00_0000 || magnitude >= 0x7f80_0000 {
            return Err(ScalarTrigError::OutsideRegularDirectDomain);
        }
        let wide = (u64::from(bits & 0x8000_0000) << 32)
            | (u64::from((magnitude >> 23) + 896) << 52)
            | (u64::from(magnitude & 0x007f_ffff) << 29);
        if wide & !SIGN64 > DIRECT_LIMIT { return Err(ScalarTrigError::OutsideRegularDirectDomain); }
        let (square, flags) = super::finite_power::normal_multiply_source_bits(wide, wide)
            .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation: "direct_square", reason, advanced })?;
        advanced = advanced.with_source_status(flags);
        let mut leading = 0u64;
        if !sine {
            let (half_square, flags) = super::finite_power::normal_multiply_source_bits(square, HALF.to_bits())
                .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation: "direct_half_square", reason, advanced })?;
            advanced = advanced.with_source_status(flags);
            let (value, flags) = super::finite_power::normal_fma_source_bits(ONE.to_bits(), ONE.to_bits(), half_square ^ SIGN64)
                .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation: "direct_cosine_leading_subtract", reason, advanced })?;
            leading = value;
            advanced = advanced.with_source_status(flags);
        }
        let coefficients = if sine { &SIN_COEFFICIENTS } else { &COS_COEFFICIENTS };
        let mut polynomial = coefficients[3].to_bits();
        for coefficient in coefficients[..3].iter().rev() {
            let (next, flags) = super::finite_power::normal_fma_source_bits(polynomial, square, coefficient.to_bits())
                .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation: "direct_horner_fma", reason, advanced })?;
            polynomial = next;
            advanced = advanced.with_source_status(flags);
        }
        let (factor, addend) = if sine {
            let (cube, flags) = super::finite_power::normal_multiply_source_bits(wide, square)
                .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation: "direct_cube", reason, advanced })?;
            advanced = advanced.with_source_status(flags);
            (cube, wide)
        } else {
            let (fourth, flags) = super::finite_power::normal_multiply_source_bits(square, square)
                .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation: "direct_fourth", reason, advanced })?;
            advanced = advanced.with_source_status(flags);
            (fourth, leading)
        };
        let (wide_result, flags) = super::finite_power::normal_fma_source_bits(polynomial, factor, addend)
            .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation: "direct_kernel_fma", reason, advanced })?;
        advanced = advanced.with_source_status(flags);
        let converted = MathErrorCallbackCompletion { result_f64_bits: wide_result, advanced }
            .into_rne_f32().map_err(ScalarTrigError::Conversion)?;
        Ok(ScalarTrigWithState { bits: converted.bits, exception_state: converted.advanced })
    }

    pub fn short_sin_reduction_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigReductionWithState, ScalarTrigError> {
        self.short_reduction_with_state(bits, state, true)
    }

    pub fn short_cos_reduction_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigReductionWithState, ScalarTrigError> {
        self.short_reduction_with_state(bits, state, false)
    }

    fn short_reduction_with_state(
        &self, bits: u32, mut advanced: ExplicitExceptionState, sine: bool,
    ) -> Result<ScalarTrigReductionWithState, ScalarTrigError> {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude < 0x3c00_0000 || magnitude >= 0x7f80_0000 {
            return Err(ScalarTrigError::OutsideShortReductionDomain);
        }
        let wide = (u64::from((magnitude >> 23) + 896) << 52)
            | (u64::from(magnitude & 0x007f_ffff) << 29);
        let limit = if sine { SIN_SHORT_LIMIT } else { COS_SHORT_LIMIT };
        if wide <= DIRECT_LIMIT || wide >= limit { return Err(ScalarTrigError::OutsideShortReductionDomain); }
        let biased = scalar_source_fma(INV_PIO2.to_bits(), wide, HALF.to_bits(), "short_count_fma", &mut advanced)?;
        let (integer, conversion_status) = if biased >= 0x41e0_0000_0000_0000 {
            (i32::MIN, 1)
        } else if biased < 0x3ff0_0000_0000_0000 {
            (0, if biased == 0 { 0 } else { 0x20 })
        } else {
            let exponent = ((biased >> 52) & 0x7ff) as u32 - 1023;
            let significand = (biased & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
            let discard = 52 - exponent;
            let lost = significand & ((1u64 << discard) - 1);
            ((significand >> discard) as i32, if lost != 0 { 0x20 } else { 0 })
        };
        advanced = advanced.with_source_status(conversion_status | (if sine { 0x20 } else { 0 }));
        let magnitude = integer.unsigned_abs();
        let count = if magnitude == 0 { 0 } else {
            let highest = 31 - magnitude.leading_zeros();
            let fraction = u64::from(magnitude ^ (1u32 << highest)) << (52 - highest);
            ((if integer < 0 { 1u64 } else { 0 }) << 63)
                | (u64::from(highest + 1023) << 52) | fraction
        };
        let first = scalar_source_fma(count ^ SIGN64, SPLIT_HIGH.to_bits(), wide, "short_first_remainder_fma", &mut advanced)?;
        let product = scalar_source_multiply(count, SPLIT_LOW.to_bits(), "short_low_product", &mut advanced)?;
        let head = scalar_source_subtract(first, product, "short_head_subtract", &mut advanced)?;
        let lost = scalar_source_subtract(first, head, "short_discarded_head_loss", &mut advanced)?;
        let _tail = scalar_source_subtract(lost, product, "short_discarded_product_loss", &mut advanced)?;
        Ok(ScalarTrigReductionWithState { quadrant: integer as u32 & 3, head_bits: head, exception_state: advanced })
    }

    pub fn sinf_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> { self.source_with_state(bits, state, true) }

    pub fn cosf_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> { self.source_with_state(bits, state, false) }

    pub fn sinf_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<ScalarTrigWithState, ScalarTrigEvaluationError> {
        self.source_with_callback(bits, state, callback, true)
    }

    pub fn cosf_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<ScalarTrigWithState, ScalarTrigEvaluationError> {
        self.source_with_callback(bits, state, callback, false)
    }

    fn source_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback, sine: bool,
    ) -> Result<ScalarTrigWithState, ScalarTrigEvaluationError> {
        if bits & 0x7fff_ffff == 0x7f80_0000 {
            let operation = if sine { MathErrorOperation::SineFloat32 } else { MathErrorOperation::CosineFloat32 };
            let result = self.resolve_infinity_with_callback(bits, operation, state, callback)
                .map_err(ScalarTrigEvaluationError::MathError)?;
            Ok(ScalarTrigWithState { bits: result.bits, exception_state: result.advanced })
        } else { self.source_with_state(bits, state, sine).map_err(ScalarTrigEvaluationError::Arithmetic) }
    }

    fn source_with_state(
        &self, bits: u32, state: ExplicitExceptionState, sine: bool,
    ) -> Result<ScalarTrigWithState, ScalarTrigError> {
        if let Some(result) = nonfinite_bits(bits) {
            return result.map(|bits| ScalarTrigWithState { bits, exception_state: state });
        }
        let magnitude = bits & 0x7fff_ffff;
        if magnitude < 0x3900_0000 {
            return if sine { self.tiny_sin_with_explicit_state(bits, state) } else { self.tiny_cos_with_explicit_state(bits, state) };
        }
        if magnitude < 0x3c00_0000 {
            return if sine { self.small_sin_with_explicit_state(bits, state) } else { self.small_cos_with_explicit_state(bits, state) };
        }
        let wide = (u64::from((magnitude >> 23) + 896) << 52)
            | (u64::from(magnitude & 0x007f_ffff) << 29);
        if wide <= DIRECT_LIMIT { return self.regular_direct_with_state(bits, state, sine); }
        let reduction = if sine {
            if wide < SIN_SHORT_LIMIT { self.short_reduction_with_state(bits, state, true)? }
            else {
                let (quadrant, head_bits, exception_state) = single_large_head_with_explicit_state(wide, state)
                    .map_err(ScalarTrigError::Reduction)?;
                ScalarTrigReductionWithState { quadrant, head_bits, exception_state }
            }
        } else if wide < COS_SHORT_LIMIT { self.short_reduction_with_state(bits, state, false)? }
        else {

            let result = self.phase.large_reduction_with_explicit_state(wide, state).map_err(ScalarTrigError::Reduction)?;
            ScalarTrigReductionWithState { quadrant: result.quadrant, head_bits: result.head_bits, exception_state: result.exception_state }
        };
        let sine_kernel = if sine { reduction.quadrant & 1 == 0 } else { reduction.quadrant & 1 != 0 };
        let (mut wide_result, advanced) = scalar_reduced_kernel_with_state(
            reduction.head_bits, reduction.exception_state, sine_kernel, sine,
        )?;
        let sign = if sine {
            (u64::from(bits & 0x8000_0000) << 32) ^ (u64::from(reduction.quadrant & 2 != 0) << 63)
        } else { u64::from((reduction.quadrant + 1) & 2 != 0) << 63 };
        wide_result ^= sign;
        let converted = MathErrorCallbackCompletion { result_f64_bits: wide_result, advanced }
            .into_rne_f32().map_err(ScalarTrigError::Conversion)?;
        Ok(ScalarTrigWithState { bits: converted.bits, exception_state: converted.advanced })
    }

    pub fn sinf_source_bits(&self, bits: u32) -> Result<u32, ScalarTrigError> {
        if let Some(result) = nonfinite_bits(bits) { return result; }
        self.sinf_bits(bits)
    }

    pub fn cosf_source_bits(&self, bits: u32) -> Result<u32, ScalarTrigError> {
        if let Some(result) = nonfinite_bits(bits) { return result; }
        self.cosf_bits(bits)
    }

    pub fn resolve_infinity_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState, callback: MathErrorCallbackPolicy,
    ) -> Result<MathErrorWithState, ScalarTrigError> {
        if bits & 0x7fff_ffff != 0x7f80_0000 {
            return Err(ScalarTrigError::InfinityDomainPathRequired);
        }
        let requested_result_bits = 0xffc0_0000;
        if callback == MathErrorCallbackPolicy::Present {
            return Err(ScalarTrigError::DomainHandlerRequired { input_bits: bits, requested_result_bits });
        }
        let error = MaskedMathError::Domain;
        Ok(MathErrorWithState {
            bits: requested_result_bits,
            exception_state: state.apply_masked_math_error(error),
            error,
        })
    }

    pub fn resolve_infinity_with_callback(
        &self, bits: u32, operation: MathErrorOperation, state: ExplicitExceptionState,
        callback: &mut impl SourceMathErrorCallback,
    ) -> Result<MathErrorCallbackFloat32, MathErrorServiceError> {
        if bits & 0x7fff_ffff != 0x7f80_0000
            || !matches!(operation, MathErrorOperation::SineFloat32 | MathErrorOperation::CosineFloat32)
        {
            return Err(MathErrorServiceError::PathRequired { advanced: state });
        }
        let (argument1_f64_bits, state) = state.widen_f32_for_math_error(bits);
        let request = MathErrorCall {
            operation, error: MaskedMathError::Domain,
            argument1_f64_bits,
            argument2_f64_bits: 0, proposed_result_f64_bits: 0xfff8_0000_0000_0000,
        };
        let mut completed = state.call_math_error(request, callback).map_err(MathErrorServiceError::Callback)?
            .into_rne_f32().map_err(MathErrorServiceError::Conversion)?;
        completed.bits = 0xffc0_0000;
        Ok(completed)
    }

    pub fn sinf(&self, value: f32) -> Result<f32, ScalarTrigError> {
        if value.to_bits() & 0x7fff_ffff >= 0x7f80_0000 {
            return Err(ScalarTrigError::NonFiniteInput);
        }
        let input = f64::from(value);
        let magnitude = input.to_bits() & !SIGN64;
        if magnitude <= DIRECT_LIMIT {
            if magnitude < TINY_LIMIT { return Ok(value); }
            let square = input * input;
            if magnitude < SMALL_LIMIT {
                let cube = square * input;
                return Ok((-cube).mul_add(f64::from_bits(0x3fc5_5555_5555_5555), input) as f32);
            }
            return Ok(sin_kernel(input) as f32);
        }
        let (quadrant, head) = if magnitude < SIN_SHORT_LIMIT {
            short_reduction(f64::from_bits(magnitude))
        } else { single_large_head(magnitude).map_err(ScalarTrigError::Reduction)? };
        let result = if quadrant & 1 == 0 {
            sin_kernel(head)
        } else { cosine_for_sine(head) };
        let flip = (input.to_bits() & SIGN64) ^ (u64::from(quadrant & 2 != 0) << 63);
        Ok(f64::from_bits(result.to_bits() ^ flip) as f32)
    }

    pub fn cosf(&self, value: f32) -> Result<f32, ScalarTrigError> {
        if value.to_bits() & 0x7fff_ffff >= 0x7f80_0000 {
            return Err(ScalarTrigError::NonFiniteInput);
        }
        let input = f64::from(value);
        let magnitude = input.to_bits() & !SIGN64;
        if magnitude <= DIRECT_LIMIT {
            if magnitude < TINY_LIMIT { return Ok(ONE as f32); }
            if magnitude < SMALL_LIMIT {
                let half_input = input * HALF;
                return Ok((-half_input).mul_add(input, ONE) as f32);
            }
            return Ok(cosine_for_cosine(input) as f32);
        }
        let (quadrant, head) = if magnitude < COS_SHORT_LIMIT {
            short_reduction(f64::from_bits(magnitude))
        } else {

            let reduced = self.phase.reduce_positive(f64::from_bits(magnitude))
                .map_err(ScalarTrigError::Reduction)?;
            (reduced.quadrant, reduced.head)
        };
        let result = if quadrant & 1 == 0 {
            cosine_for_cosine(head)
        } else { sin_kernel(head) };
        let flip = u64::from((quadrant + 1) & 2 != 0) << 63;
        Ok(f64::from_bits(result.to_bits() ^ flip) as f32)
    }

    pub fn sinf_bits(&self, bits: u32) -> Result<u32, ScalarTrigError> {
        self.sinf(f32::from_bits(bits)).map(f32::to_bits)
    }

    pub fn cosf_bits(&self, bits: u32) -> Result<u32, ScalarTrigError> {
        self.cosf(f32::from_bits(bits)).map(f32::to_bits)
    }
}

pub(super) fn tiny_sine_source_status(bits: u32) -> u32 {
    let magnitude = bits & 0x7fff_ffff;
    if magnitude == 0 { 0 }
        else if magnitude < 0x0080_0000 { 0x32 }
        else if magnitude == 0x0080_0000 { 0x30 } else { 0x20 }
}

fn scalar_source_fma(
    left: u64, right: u64, addend: u64, operation: &'static str, advanced: &mut ExplicitExceptionState,
) -> Result<u64, ScalarTrigError> {
    let (bits, flags) = super::finite_power::normal_fma_source_bits(left, right, addend)
        .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation, reason, advanced: *advanced })?;
    *advanced = advanced.with_source_status(flags);
    Ok(bits)
}

fn scalar_source_multiply(
    left: u64, right: u64, operation: &'static str, advanced: &mut ExplicitExceptionState,
) -> Result<u64, ScalarTrigError> {
    let (bits, flags) = super::finite_power::normal_multiply_source_bits(left, right)
        .map_err(|reason| ScalarTrigError::NormalArithmeticRequired { operation, reason, advanced: *advanced })?;
    *advanced = advanced.with_source_status(flags);
    Ok(bits)
}

fn scalar_source_subtract(
    left: u64, right: u64, operation: &'static str, advanced: &mut ExplicitExceptionState,
) -> Result<u64, ScalarTrigError> {
    scalar_source_fma(left, ONE.to_bits(), right ^ SIGN64, operation, advanced)
}

fn scalar_reduced_kernel_with_state(
    head: u64, mut advanced: ExplicitExceptionState, sine_kernel: bool, cosine_for_sine: bool,
) -> Result<(u64, ExplicitExceptionState), ScalarTrigError> {
    let square = scalar_source_multiply(head, head, "reduced_square", &mut advanced)?;
    let mut leading = 0u64;
    if !sine_kernel {
        leading = if cosine_for_sine {
            scalar_source_fma(square, HALF.to_bits() ^ SIGN64, ONE.to_bits(), "reduced_fused_leading", &mut advanced)?
        } else {
            let half_square = scalar_source_multiply(square, HALF.to_bits(), "reduced_half_square", &mut advanced)?;
            scalar_source_subtract(ONE.to_bits(), half_square, "reduced_separate_leading", &mut advanced)?
        };
    }
    let coefficients = if sine_kernel { &SIN_COEFFICIENTS } else { &COS_COEFFICIENTS };
    let mut polynomial = coefficients[3].to_bits();
    for coefficient in coefficients[..3].iter().rev() {
        polynomial = scalar_source_fma(polynomial, square, coefficient.to_bits(), "reduced_horner_fma", &mut advanced)?;
    }
    let (factor, addend) = if sine_kernel {
        (scalar_source_multiply(head, square, "reduced_cube", &mut advanced)?, head)
    } else {
        (scalar_source_multiply(square, square, "reduced_fourth", &mut advanced)?, leading)
    };
    let result = scalar_source_fma(polynomial, factor, addend, "reduced_kernel_fma", &mut advanced)?;
    Ok((result, advanced))
}

fn nonfinite_bits(bits: u32) -> Option<Result<u32, ScalarTrigError>> {
    let magnitude = bits & 0x7fff_ffff;
    if magnitude > 0x7f80_0000 { return Some(Ok(bits | 0x0040_0000)); }
    if magnitude == 0x7f80_0000 {
        return Some(Err(ScalarTrigError::DomainHandlerRequired {
            input_bits: bits, requested_result_bits: 0xffc0_0000,
        }));
    }
    None
}

fn sin_kernel(head: f64) -> f64 {
    let square = head * head;
    let polynomial = SIN_COEFFICIENTS[3].mul_add(square, SIN_COEFFICIENTS[2]);
    let polynomial = polynomial.mul_add(square, SIN_COEFFICIENTS[1]);
    let polynomial = polynomial.mul_add(square, SIN_COEFFICIENTS[0]);
    let cube = head * square;
    polynomial.mul_add(cube, head)
}

fn cosine_polynomial(square: f64) -> f64 {
    let polynomial = COS_COEFFICIENTS[3].mul_add(square, COS_COEFFICIENTS[2]);
    let polynomial = polynomial.mul_add(square, COS_COEFFICIENTS[1]);
    polynomial.mul_add(square, COS_COEFFICIENTS[0])
}

fn cosine_for_cosine(head: f64) -> f64 {
    let square = head * head;
    let half_square = square * HALF;
    let leading = ONE - half_square;
    let polynomial = cosine_polynomial(square);
    let fourth = square * square;
    polynomial.mul_add(fourth, leading)
}

fn cosine_for_sine(head: f64) -> f64 {
    let square = head * head;
    let leading = square.mul_add(-HALF, ONE);
    let polynomial = cosine_polynomial(square);
    let fourth = square * square;
    polynomial.mul_add(fourth, leading)
}

fn short_reduction(input: f64) -> (u32, f64) {
    let biased = INV_PIO2.mul_add(input, HALF);

    let integer = if biased >= 2_147_483_648.0 { i32::MIN } else { biased as i32 };
    let count = f64::from(integer);
    let quadrant = integer as u32 & 3;
    let first = (-count).mul_add(SPLIT_HIGH, input);
    let low = count * SPLIT_LOW;
    (quadrant, first - low)
}
