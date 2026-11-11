use super::selected_log::Environment;
use super::finite_power::{ExplicitExceptionState, MaskedMathError, MathErrorCallbackPolicy, MathErrorWithState};
use super::finite_power::{MathErrorCall, MathErrorCallbackFloat32, MathErrorOperation, MathErrorServiceError, SourceMathErrorCallback};
use super::finite_power::binary_round_increment;
use super::shared_scalar_f32::ScalarF32Rounding;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquareRootPlan {
    HardwareSquareRoot { input_bits: u32 },
    QuietNaN { input_bits: u32, result_bits: u32 },
    DomainHandlerRequired { input_bits: u32, requested_result_bits: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquareRootError {
    UnsupportedEnvironment,
    DomainExceptionPathRequired,
    DomainHandlerRequired { input_bits: u32, requested_result_bits: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SquareRootWithState {
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SquareRootEvaluationError {
    Arithmetic(SquareRootError),
    MathError(MathErrorServiceError),
}

pub struct SourceSquareRoot {
    environment: Environment,
}

impl SourceSquareRoot {
    pub fn bind(mxcsr: u32) -> Result<Self, SquareRootError> {
        let environment = Environment::from_explicit_current_mxcsr(mxcsr)
            .map_err(|_| SquareRootError::UnsupportedEnvironment)?;
        Ok(Self { environment })
    }

    pub fn environment(&self) -> Environment {
        self.environment
    }

    pub fn plan(&self, bits: u32) -> SquareRootPlan {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude > 0x7f80_0000 {
            return SquareRootPlan::QuietNaN {
                input_bits: bits, result_bits: bits | 0x0040_0000,
            };
        }

        if bits & 0x8000_0000 != 0 && magnitude != 0 {
            return SquareRootPlan::DomainHandlerRequired {
                input_bits: bits,
                requested_result_bits: 0xffc0_0000,
            };
        }
        SquareRootPlan::HardwareSquareRoot { input_bits: bits }
    }

    pub fn evaluate_bits(&self, bits: u32) -> Result<u32, SquareRootError> {
        self.evaluate(f32::from_bits(bits)).map(f32::to_bits)
    }

    pub fn raw_sqrtss_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<SquareRootWithState, SquareRootError> {
        let candidate = sqrt_f32_candidate(bits, false, true, ScalarF32Rounding::NearestEven);
        let result = candidate.bits.ok_or(SquareRootError::UnsupportedEnvironment)?;
        Ok(SquareRootWithState { bits: result,
            exception_state: state.with_source_status(u32::from(candidate.raised)) })
    }

    pub fn direct_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<SquareRootWithState, SquareRootError> {
        match self.plan(bits) {
            SquareRootPlan::DomainHandlerRequired { input_bits, requested_result_bits } =>
                Err(SquareRootError::DomainHandlerRequired { input_bits, requested_result_bits }),
            SquareRootPlan::QuietNaN { result_bits, .. } =>
                Ok(SquareRootWithState { bits: result_bits, exception_state: state }),
            SquareRootPlan::HardwareSquareRoot { input_bits } => {
                self.raw_sqrtss_with_explicit_state(input_bits, state)
            }
        }
    }

    pub fn evaluate_with_explicit_policy(
        &self, bits: u32, state: ExplicitExceptionState, callback: MathErrorCallbackPolicy,
    ) -> Result<SquareRootWithState, SquareRootError> {
        if matches!(self.plan(bits), SquareRootPlan::DomainHandlerRequired { .. }) {
            let result = self.resolve_domain_with_explicit_state(bits, state, callback)?;
            Ok(SquareRootWithState { bits: result.bits, exception_state: result.exception_state })
        } else { self.direct_with_explicit_state(bits, state) }
    }

    pub fn evaluate_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<SquareRootWithState, SquareRootEvaluationError> {
        if matches!(self.plan(bits), SquareRootPlan::DomainHandlerRequired { .. }) {
            let result = self.resolve_domain_with_callback(bits, state, callback)
                .map_err(SquareRootEvaluationError::MathError)?;
            Ok(SquareRootWithState { bits: result.bits, exception_state: result.advanced })
        } else { self.direct_with_explicit_state(bits, state).map_err(SquareRootEvaluationError::Arithmetic) }
    }

    pub fn resolve_domain_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState, callback: MathErrorCallbackPolicy,
    ) -> Result<MathErrorWithState, SquareRootError> {
        let SquareRootPlan::DomainHandlerRequired { input_bits, requested_result_bits } = self.plan(bits)
            else { return Err(SquareRootError::DomainExceptionPathRequired); };
        if callback == MathErrorCallbackPolicy::Present {
            return Err(SquareRootError::DomainHandlerRequired { input_bits, requested_result_bits });
        }
        let error = MaskedMathError::Domain;
        Ok(MathErrorWithState {
            bits: requested_result_bits,
            exception_state: state.apply_masked_math_error(error),
            error,
        })
    }

    pub fn resolve_domain_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<MathErrorCallbackFloat32, MathErrorServiceError> {
        if !matches!(self.plan(bits), SquareRootPlan::DomainHandlerRequired { .. }) {
            return Err(MathErrorServiceError::PathRequired { advanced: state });
        }
        let (argument1_f64_bits, state) = state.widen_f32_for_math_error(bits);
        let request = MathErrorCall {
            operation: MathErrorOperation::SquareRoot,
            error: MaskedMathError::Domain,
            argument1_f64_bits,
            argument2_f64_bits: 0,
            proposed_result_f64_bits: 0xfff8_0000_0000_0000,
        };
        state.call_math_error(request, callback).map_err(MathErrorServiceError::Callback)?
            .into_rne_f32().map_err(MathErrorServiceError::Conversion)
    }

    pub fn evaluate(&self, value: f32) -> Result<f32, SquareRootError> {
        match self.plan(value.to_bits()) {
            SquareRootPlan::HardwareSquareRoot { input_bits } =>
                Ok(f32::from_bits(input_bits).sqrt()),
            SquareRootPlan::QuietNaN { result_bits, .. } => Ok(f32::from_bits(result_bits)),
            SquareRootPlan::DomainHandlerRequired { input_bits, requested_result_bits } =>
                Err(SquareRootError::DomainHandlerRequired { input_bits, requested_result_bits }),
        }
    }
}

pub(super) struct ScalarF32SqrtCandidate { pub bits: Option<u32>, pub raised: u8 }

pub(super) fn sqrt_f32_candidate(mut bits: u32, daz: bool, denormal_masked: bool,
    rounding: ScalarF32Rounding) -> ScalarF32SqrtCandidate {
    let mut magnitude = bits & 0x7fff_ffff;
    if magnitude > 0x7f80_0000 {
        return ScalarF32SqrtCandidate { bits: Some(bits | 0x0040_0000),
            raised: u8::from(bits & 0x0040_0000 == 0) };
    }
    let subnormal = magnitude != 0 && magnitude < 0x0080_0000;
    if daz && subnormal { bits &= 0x8000_0000; magnitude = 0; }
    if bits >> 31 != 0 && magnitude != 0 {
        return ScalarF32SqrtCandidate { bits: Some(0xffc0_0000), raised: 1 };
    }
    if magnitude == 0 || magnitude == 0x7f80_0000 {
        return ScalarF32SqrtCandidate { bits: Some(bits), raised: 0 };
    }
    let pre = if subnormal { 2 } else { 0 };
    if pre != 0 && !denormal_masked {
        return ScalarF32SqrtCandidate { bits: None, raised: pre };
    }
    let field = magnitude >> 23;
    let fraction = magnitude & 0x007f_ffff;
    let (mut exponent, mut significand) = if field == 0 {
        let highest = 31 - fraction.leading_zeros();
        (highest as i32 - 149, fraction << (23 - highest))
    } else { (field as i32 - 127, fraction | 0x0080_0000) };
    if exponent & 1 != 0 { significand <<= 1; exponent -= 1; }
    let (mut root, inexact) = round_normalized_square_root(u64::from(significand), 23, rounding);
    let mut result_field = (exponent / 2 + 127) as u32;
    if root == 0x0100_0000 { root >>= 1; result_field += 1; }
    ScalarF32SqrtCandidate { bits: Some((result_field << 23) | ((root as u32) & 0x007f_ffff)),
        raised: pre | if inexact { 0x20 } else { 0 } }
}

pub(super) fn round_normalized_square_root(significand: u64, fraction_bits: u32,
    rounding: ScalarF32Rounding) -> (u64, bool) {
    let (root, remainder) = restoring_square_root(u128::from(significand) << fraction_bits,
        1u128 << (fraction_bits * 2));
    let increment = binary_round_increment(rounding, false,
        remainder << 2, (root << 2) + 1, false);
    ((root + u128::from(increment)) as u64, remainder != 0)
}

pub(super) fn restoring_square_root(mut remainder: u128, mut place: u128) -> (u128, u128) {
    let mut root = 0u128;
    while place != 0 {
        let trial = root + place;
        if remainder >= trial {
            remainder -= trial;
            root = (root >> 1) + place;
        } else { root >>= 1; }
        place >>= 2;
    }
    (root, remainder)
}
