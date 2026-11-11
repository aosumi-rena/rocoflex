use super::retained_exp_constants::{EXP_TABLE, LN2_DIV64, LOG2_64, POLY_HALF, POLY_THIRD};
use super::selected_log::Environment;
use super::finite_power::{ExplicitExceptionState, MaskedMathError, MathErrorCallbackPolicy, MathErrorWithState};
use super::finite_power::{MathErrorCall, MathErrorCallbackCompletion, MathErrorCallbackFloat32, MathErrorConversionRequired, MathErrorOperation, MathErrorServiceError, NormalArithmeticError, SourceMathErrorCallback};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpError {
    NonzeroSourceProfileRequired,
    UnsupportedEnvironment,
    NonFiniteInput { input_bits: u32 },
    BelowOrdinaryRange { scaled_bits: u64 },
    AtOrAboveUpperOrdinaryRange { scaled_bits: u64 },
    RangeExceptionPathRequired,
    MathErrorCallbackRequired { input_bits: u32, requested_result_bits: u32 },
    OrdinaryPathRequired { entry: SourceExpEntry },
    NormalArithmeticRequired { operation: &'static str, reason: NormalArithmeticError, advanced: ExplicitExceptionState },
    ConversionRequired(MathErrorConversionRequired),
    CallbackRequiredFromEntry { entry: SourceExpEntry },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExpPlan {
    Ordinary { input_bits: u32, scaled_bits: u64 },
    CompleteBits { bits: u32 },
    MaskedRangeError {
        input_bits: u32,
        helper_input_bits: u32,
        requested_result_bits: u32,
        error: MaskedMathError,
        scaled_bits: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExpEntry {
    pub plan: SourceExpPlan,

    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExpReduction {
    pub input_bits: u32,
    pub scaled_bits: u64,
    pub integer: i32,
    pub integer_f64_bits: u64,
    pub table_index: u32,
    pub binary_scale_exponent: i32,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExpPackedReduction {
    pub low: SourceExpReduction,
    pub upper_integer: i32,
}

pub(super) fn rne_i32_lane(bits: u64) -> (i32, u32) {
    let magnitude = bits & 0x7fff_ffff_ffff_ffff;
    let negative = bits >> 63 != 0;
    let field = ((magnitude >> 52) & 0x7ff) as i32;
    if field == 0x7ff || field > 1054 { return (i32::MIN, 1); }
    if magnitude == 0 { return (0, 0); }
    if field < 1022 { return (0, 0x20); }
    if field == 1022 {
        return (if magnitude == 0x3fe0_0000_0000_0000 { 0 } else if negative { -1 } else { 1 }, 0x20);
    }

    let exponent = field - 1023;
    let discard = (52 - exponent) as u32;
    let significand = (magnitude & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let whole = significand >> discard;
    let lost = significand & ((1u64 << discard) - 1);
    let half = 1u64 << (discard - 1);
    let rounded = whole + u64::from(lost > half || (lost == half && whole & 1 != 0));
    let maximum = if negative { 0x8000_0000u64 } else { 0x7fff_ffffu64 };
    if rounded > maximum { return (i32::MIN, 1); }
    let integer = if negative { -(rounded as i64) } else { rounded as i64 } as i32;
    (integer, if lost != 0 { 0x20 } else { 0 })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExpRemainder {
    pub reduction: SourceExpReduction,
    pub remainder_f64_bits: u64,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExpWithState {
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExpDispatchOutcome {
    Complete(SourceExpWithState),
    RangeRequired(SourceExpEntry),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceExpDispatchError {
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    WordZero(super::selected_exp_word_zero::SourceWordZeroExpError),
    Nonzero(ExpError),
}
impl SourceExpDispatchError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "exp explicit arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "exp state is not RNE/all-masked/gradual",
            Self::WordZero(error) => error.reason(),
            Self::Nonzero(_) => "57f8e0 explicit nonzero source continuation required",
        }
    }
    pub fn known_advanced_state(&self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::WordZero(error) => error.known_advanced_state(entry),
            Self::Nonzero(ExpError::NormalArithmeticRequired { advanced, .. }) => *advanced,
            Self::Nonzero(ExpError::ConversionRequired(required)) => required.advanced,
            Self::Nonzero(ExpError::OrdinaryPathRequired { entry } | ExpError::CallbackRequiredFromEntry { entry }) => entry.exception_state,
            _ => entry,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceExpDispatchReceipt {
    pub explicit_source_word: u32,
    pub architecture: super::shared_scalar_f32::ScalarF32Architecture,
    pub input_bits: u32,
    pub original_upper_f64_bits: u64,
    pub entry: ExplicitExceptionState,
    pub known_advanced: ExplicitExceptionState,
    pub word_zero_call: Option<super::selected_exp_word_zero::SourceWordZeroExpCallReceipt>,
    pub outcome: Result<SourceExpDispatchOutcome, SourceExpDispatchError>,
}

pub fn selected_exp_with_explicit_upper_lane(
    explicit_source_word: u32,
    architecture: super::shared_scalar_f32::ScalarF32Architecture,
    input_bits: u32,
    original_upper_f64_bits: u64,
    state: ExplicitExceptionState,
) -> Result<SourceExpDispatchOutcome, SourceExpDispatchError> {
    selected_exp_with_explicit_upper_lane_receipt(explicit_source_word, architecture,
        input_bits, original_upper_f64_bits, state).outcome
}

pub fn selected_exp_with_explicit_upper_lane_receipt(
    explicit_source_word: u32,
    architecture: super::shared_scalar_f32::ScalarF32Architecture,
    input_bits: u32,
    original_upper_f64_bits: u64,
    state: ExplicitExceptionState,
) -> SourceExpDispatchReceipt {
    let mut word_zero_call = None;
    let outcome = selected_exp_dispatch_once(explicit_source_word, architecture,
        input_bits, original_upper_f64_bits, state, &mut word_zero_call);
    let known_advanced = match &outcome {
        Ok(SourceExpDispatchOutcome::Complete(completed)) => completed.exception_state,
        Ok(SourceExpDispatchOutcome::RangeRequired(entry)) => entry.exception_state,
        Err(error) => error.known_advanced_state(state),
    };
    SourceExpDispatchReceipt { explicit_source_word, architecture, input_bits,
        original_upper_f64_bits, entry: state, known_advanced, word_zero_call, outcome }
}

fn selected_exp_dispatch_once(
    explicit_source_word: u32,
    architecture: super::shared_scalar_f32::ScalarF32Architecture,
    input_bits: u32,
    original_upper_f64_bits: u64,
    state: ExplicitExceptionState,
    word_zero_call: &mut Option<super::selected_exp_word_zero::SourceWordZeroExpCallReceipt>,
) -> Result<SourceExpDispatchOutcome, SourceExpDispatchError> {
    if architecture != super::shared_scalar_f32::ScalarF32Architecture::IntelSdm {
        return Err(SourceExpDispatchError::ArchitectureNotClosed);
    }
    if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
        return Err(SourceExpDispatchError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
    }

    let (plan, flags) = source_entry_plan(input_bits);
    let entry = SourceExpEntry { plan, exception_state: state.with_source_status(flags) };
    match entry.plan {
        SourceExpPlan::CompleteBits { bits } => Ok(SourceExpDispatchOutcome::Complete(SourceExpWithState {
            bits, exception_state: entry.exception_state,
        })),
        SourceExpPlan::MaskedRangeError { .. } => Ok(SourceExpDispatchOutcome::RangeRequired(entry)),
        SourceExpPlan::Ordinary { .. } => {
            let value = if explicit_source_word == 0 {
                let mut leaf = super::selected_exp_word_zero::SourceScalarExpWordZero::bind(
                    explicit_source_word, architecture, state,
                ).map_err(SourceExpDispatchError::WordZero)?;
                let result = leaf.evaluate_bits(input_bits, original_upper_f64_bits);
                *word_zero_call = leaf.last_call().cloned();
                let bits = result.map_err(SourceExpDispatchError::WordZero)?;
                SourceExpWithState { bits, exception_state: leaf.exception_state() }
            } else {
                let leaf = SourceScalarExpNonzero::bind(explicit_source_word, state.mxcsr()).map_err(SourceExpDispatchError::Nonzero)?;
                leaf.ordinary_with_explicit_upper_lane(input_bits, original_upper_f64_bits, state)
                    .map_err(SourceExpDispatchError::Nonzero)?
            };
            Ok(SourceExpDispatchOutcome::Complete(value))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceExpEvaluationError {
    Arithmetic(ExpError),
    MathError(MathErrorServiceError),
}

pub struct SourceScalarExpNonzero {
    explicit_source_cpu_word: u32,
    environment: Environment,
}

impl SourceScalarExpNonzero {

    pub fn bind(explicit_source_cpu_word: u32, mxcsr: u32) -> Result<Self, ExpError> {
        if explicit_source_cpu_word == 0 {
            return Err(ExpError::NonzeroSourceProfileRequired);
        }
        let environment = Environment::from_explicit_current_mxcsr(mxcsr)
            .map_err(|_| ExpError::UnsupportedEnvironment)?;
        Ok(Self { explicit_source_cpu_word, environment })
    }

    pub fn explicit_source_cpu_word(&self) -> u32 {
        self.explicit_source_cpu_word
    }

    pub fn environment(&self) -> Environment {
        self.environment
    }

    pub fn evaluate_bits(&self, input_bits: u32) -> Result<u32, ExpError> {
        self.exp_finite(f32::from_bits(input_bits)).map(f32::to_bits)
    }

    pub fn plan_bits(&self, bits: u32) -> SourceExpPlan {
        source_entry_plan(bits).0
    }

    pub fn source_entry_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> SourceExpEntry {
        let (plan, status) = source_entry_plan(bits);
        SourceExpEntry { plan, exception_state: state.with_source_status(status) }
    }

    pub fn reduction_with_explicit_upper_lane(
        &self, bits: u32, original_upper_f64_bits: u64, state: ExplicitExceptionState,
    ) -> Result<SourceExpPackedReduction, ExpError> {
        let mut low = self.coefficient_1342d0_reduction_from_entry(bits, state)?;
        let (upper_integer, flags) = rne_i32_lane(original_upper_f64_bits);
        low.exception_state = low.exception_state.with_source_status(flags);
        Ok(SourceExpPackedReduction { low, upper_integer })
    }

    pub fn ordinary_with_explicit_upper_lane(
        &self, bits: u32, original_upper_f64_bits: u64, state: ExplicitExceptionState,
    ) -> Result<SourceExpWithState, ExpError> {
        let entry = self.source_entry_with_explicit_state(bits, state);
        if !matches!(entry.plan, SourceExpPlan::Ordinary { .. }) {
            return Err(ExpError::OrdinaryPathRequired { entry });
        }
        let (_, flags) = rne_i32_lane(original_upper_f64_bits);
        self.coefficient_1342d0_ordinary_with_explicit_state(bits, state.with_source_status(flags))
    }

    pub fn with_explicit_upper_lane_and_policy(
        &self, bits: u32, original_upper_f64_bits: u64, state: ExplicitExceptionState,
        callback: MathErrorCallbackPolicy,
    ) -> Result<SourceExpWithState, ExpError> {
        if matches!(self.plan_bits(bits), SourceExpPlan::Ordinary { .. }) {
            self.ordinary_with_explicit_upper_lane(bits, original_upper_f64_bits, state)
        } else {
            self.coefficient_1342d0_with_explicit_state(bits, state, callback)
        }
    }

    pub fn with_explicit_upper_lane_and_callback(
        &self, bits: u32, original_upper_f64_bits: u64, state: ExplicitExceptionState,
        callback: &mut impl SourceMathErrorCallback,
    ) -> Result<SourceExpWithState, SourceExpEvaluationError> {
        if matches!(self.plan_bits(bits), SourceExpPlan::Ordinary { .. }) {
            self.ordinary_with_explicit_upper_lane(bits, original_upper_f64_bits, state)
                .map_err(SourceExpEvaluationError::Arithmetic)
        } else {
            self.coefficient_1342d0_with_callback(bits, state, callback)
        }
    }

    pub fn coefficient_1342d0_reduction_from_entry(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<SourceExpReduction, ExpError> {
        let entry = self.source_entry_with_explicit_state(bits, state);
        let SourceExpPlan::Ordinary { input_bits, scaled_bits } = entry.plan
            else { return Err(ExpError::OrdinaryPathRequired { entry }); };
        let magnitude = scaled_bits & 0x7fff_ffff_ffff_ffff;
        let (rounded, inexact) = if magnitude == 0 { (0u32, false) }
            else if magnitude < 0x3ff0_0000_0000_0000 {
                (u32::from(magnitude > 0x3fe0_0000_0000_0000), true)
            } else {
                let exponent = ((magnitude >> 52) & 0x7ff) as u32 - 1023;

                let discard = 52 - exponent;
                let significand = (magnitude & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
                let whole = (significand >> discard) as u32;
                let lost = significand & ((1u64 << discard) - 1);
                let half = 1u64 << (discard - 1);
                let increment = lost > half || (lost == half && whole & 1 != 0);
                (whole + u32::from(increment), lost != 0)
            };
        let integer = if scaled_bits >> 63 != 0 { -(rounded as i32) } else { rounded as i32 };
        let integer_f64_bits = if rounded == 0 { 0 } else {
            let highest = 31 - rounded.leading_zeros();
            let fraction = u64::from(rounded ^ (1u32 << highest)) << (52 - highest);
            ((if integer < 0 { 1u64 } else { 0 }) << 63)
                | (u64::from(highest + 1023) << 52) | fraction
        };
        let table_index = (integer as u32) & 63;
        let binary_scale_exponent = (integer - table_index as i32) >> 6;
        Ok(SourceExpReduction {
            input_bits, scaled_bits, integer, integer_f64_bits, table_index, binary_scale_exponent,
            exception_state: entry.exception_state.with_source_status(if inexact { 0x20 } else { 0 }),
        })
    }

    pub fn coefficient_1342d0_remainder_from_entry(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<SourceExpRemainder, ExpError> {
        let reduction = self.coefficient_1342d0_reduction_from_entry(bits, state)?;
        let state = reduction.exception_state;
        if reduction.integer == 0 {
            let (remainder_f64_bits, state) = state.widen_f32_for_math_error(bits);
            return Ok(SourceExpRemainder { reduction, remainder_f64_bits, exception_state: state });
        }
        let magnitude = bits & 0x7fff_ffff;
        let input_sig = (magnitude & 0x007f_ffff) | 0x0080_0000;
        let input_power = (magnitude >> 23) as i32 - 150;
        let coefficient = LN2_DIV64.to_bits();
        let coefficient_sig = (coefficient & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
        let coefficient_power = ((coefficient >> 52) & 0x7ff) as i32 - 1075;

        let input = u128::from(input_sig) << ((input_power - coefficient_power) as u32);
        let product = u128::from(reduction.integer.unsigned_abs()) * u128::from(coefficient_sig);
        let (exact, negative_relative_to_input) = if input >= product {
            (input - product, false)
        } else { (product - input, true) };
        let (remainder_f64_bits, status) = if exact == 0 { (0, 0) } else {
            let (rounded, status) = super::finite_power::round_exact_normal_u128(exact, coefficient_power);
            let negative = (bits >> 31 != 0) ^ negative_relative_to_input;
            (rounded | ((if negative { 1u64 } else { 0 }) << 63), status)
        };
        Ok(SourceExpRemainder {
            reduction, remainder_f64_bits, exception_state: state.with_source_status(status),
        })
    }

    pub fn coefficient_1342d0_ordinary_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<SourceExpWithState, ExpError> {
        let remainder = self.coefficient_1342d0_remainder_from_entry(bits, state)?;
        let mut advanced = remainder.exception_state;
        let r = remainder.remainder_f64_bits;
        let (square, flags) = super::finite_power::normal_multiply_source_bits(r, r)
            .map_err(|reason| ExpError::NormalArithmeticRequired { operation: "remainder_square", reason, advanced })?;
        advanced = advanced.with_source_status(flags);
        let (polynomial, flags) = super::finite_power::normal_fma_source_bits(POLY_THIRD.to_bits(), r, POLY_HALF.to_bits())
            .map_err(|reason| ExpError::NormalArithmeticRequired { operation: "polynomial_fma", reason, advanced })?;
        advanced = advanced.with_source_status(flags);
        let (reduced, flags) = super::finite_power::normal_fma_source_bits(square, polynomial, r)
            .map_err(|reason| ExpError::NormalArithmeticRequired { operation: "reduced_fma", reason, advanced })?;
        advanced = advanced.with_source_status(flags);
        let table = EXP_TABLE[remainder.reduction.table_index as usize].to_bits();
        let (expanded, flags) = super::finite_power::normal_fma_source_bits(reduced, table, table)
            .map_err(|reason| ExpError::NormalArithmeticRequired { operation: "table_fma", reason, advanced })?;
        advanced = advanced.with_source_status(flags);
        let scale = ((remainder.reduction.binary_scale_exponent + 1023) as u64) << 52;
        let (wide, flags) = super::finite_power::normal_multiply_source_bits(expanded, scale)
            .map_err(|reason| ExpError::NormalArithmeticRequired { operation: "binary_scale_multiply", reason, advanced })?;
        advanced = advanced.with_source_status(flags);
        let result = MathErrorCallbackCompletion { result_f64_bits: wide, advanced }
            .into_rne_f32().map_err(ExpError::ConversionRequired)?;
        Ok(SourceExpWithState { bits: result.bits, exception_state: result.advanced })
    }

    pub fn coefficient_1342d0_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState, callback: MathErrorCallbackPolicy,
    ) -> Result<SourceExpWithState, ExpError> {
        let entry = self.source_entry_with_explicit_state(bits, state);
        match entry.plan {
            SourceExpPlan::CompleteBits { bits } =>
                Ok(SourceExpWithState { bits, exception_state: entry.exception_state }),
            SourceExpPlan::Ordinary { .. } => self.coefficient_1342d0_ordinary_with_explicit_state(bits, state),
            SourceExpPlan::MaskedRangeError { requested_result_bits, error, .. } => {
                if callback == MathErrorCallbackPolicy::Present {
                    return Err(ExpError::CallbackRequiredFromEntry { entry });
                }
                Ok(SourceExpWithState {
                    bits: requested_result_bits,
                    exception_state: entry.exception_state.apply_masked_math_error(error),
                })
            }
        }
    }

    pub fn coefficient_1342d0_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<SourceExpWithState, SourceExpEvaluationError> {
        let entry = self.source_entry_with_explicit_state(bits, state);
        match entry.plan {
            SourceExpPlan::CompleteBits { bits } =>
                Ok(SourceExpWithState { bits, exception_state: entry.exception_state }),
            SourceExpPlan::Ordinary { .. } => self.coefficient_1342d0_ordinary_with_explicit_state(bits, state)
                .map_err(SourceExpEvaluationError::Arithmetic),
            SourceExpPlan::MaskedRangeError { .. } => {
                let completed = self.resolve_range_with_callback(bits, entry.exception_state, callback)
                    .map_err(SourceExpEvaluationError::MathError)?;
                Ok(SourceExpWithState { bits: completed.bits, exception_state: completed.advanced })
            }
        }
    }

    pub fn evaluate_source_bits(&self, bits: u32) -> Result<u32, ExpError> {
        match self.plan_bits(bits) {
            SourceExpPlan::CompleteBits { bits } => Ok(bits),
            SourceExpPlan::Ordinary { input_bits, scaled_bits } => Ok(ordinary_exp(
                f64::from(f32::from_bits(input_bits)), f64::from_bits(scaled_bits),
            ).to_bits()),
            SourceExpPlan::MaskedRangeError { error: MaskedMathError::Overflow, scaled_bits, .. } =>
                Err(ExpError::AtOrAboveUpperOrdinaryRange { scaled_bits }),
            SourceExpPlan::MaskedRangeError { scaled_bits, .. } =>
                Err(ExpError::BelowOrdinaryRange { scaled_bits }),
        }
    }

    pub fn resolve_range_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState, callback: MathErrorCallbackPolicy,
    ) -> Result<MathErrorWithState, ExpError> {
        let SourceExpPlan::MaskedRangeError { input_bits, requested_result_bits, error, .. } = self.plan_bits(bits)
            else { return Err(ExpError::RangeExceptionPathRequired); };
        if callback == MathErrorCallbackPolicy::Present {
            return Err(ExpError::MathErrorCallbackRequired { input_bits, requested_result_bits });
        }
        Ok(MathErrorWithState {
            bits: requested_result_bits,
            exception_state: state.apply_masked_math_error(error),
            error,
        })
    }

    pub fn resolve_range_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<MathErrorCallbackFloat32, MathErrorServiceError> {
        resolve_source_exp_range_with_callback(self.plan_bits(bits), state, callback)
    }

    pub fn exp_finite(&self, value: f32) -> Result<f32, ExpError> {
        let input_bits = value.to_bits();
        if input_bits & 0x7fff_ffff >= 0x7f80_0000 {
            return Err(ExpError::NonFiniteInput { input_bits });
        }
        let input = f64::from(value);
        let scaled = input * LOG2_64;

        if scaled >= 8192.0 {
            return Err(ExpError::AtOrAboveUpperOrdinaryRange {
                scaled_bits: scaled.to_bits(),
            });
        }
        if scaled < -9600.0 {
            return Err(ExpError::BelowOrdinaryRange { scaled_bits: scaled.to_bits() });
        }
        Ok(ordinary_exp(input, scaled))
    }
}

pub(super) fn source_entry_plan(bits: u32) -> (SourceExpPlan, u32) {
    let magnitude = bits & 0x7fff_ffff;
    if magnitude >= 0x7f80_0000 {
        let result = if magnitude > 0x7f80_0000 { bits | 0x0040_0000 }
            else if bits == 0x7f80_0000 { bits } else { 0 };
        return (SourceExpPlan::CompleteBits { bits: result }, 0);
    }
    let sign = u64::from(bits & 0x8000_0000) << 32;
    if magnitude == 0 {
        return (SourceExpPlan::Ordinary { input_bits: bits, scaled_bits: sign }, 0);
    }
    let field = (magnitude >> 23) as i32;
    let fraction = magnitude & 0x007f_ffff;
    let (significand, input_power) = if field == 0 {
        (fraction, -149)
    } else { (fraction | 0x0080_0000, field - 150) };
    let constant = LOG2_64.to_bits();
    let constant_significand = (constant & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let constant_power = ((constant >> 52) & 0x7ff) as i32 - 1023 - 52;
    let product = u128::from(significand) * u128::from(constant_significand);
    let highest = 127 - product.leading_zeros();
    let discard = highest - 52;
    let mut retained = (product >> discard) as u64;
    let lost = product & ((1u128 << discard) - 1);
    if discard != 0 {
        let half = 1u128 << (discard - 1);
        retained += u64::from(lost > half || (lost == half && retained & 1 != 0));
    }
    let mut result_field = highest as i32 + input_power + constant_power + 1023;
    if retained == (1u64 << 53) { retained >>= 1; result_field += 1; }
    let scaled_bits = sign | ((result_field as u64) << 52) | (retained & 0x000f_ffff_ffff_ffff);
    let status = (if field == 0 { 2 } else { 0 }) | (if lost != 0 { 0x20 } else { 0 });
    let scaled_magnitude = scaled_bits & 0x7fff_ffff_ffff_ffff;
    let plan = if sign == 0 && scaled_magnitude >= 0x40c0_0000_0000_0000 {
        SourceExpPlan::MaskedRangeError {
            input_bits: bits, helper_input_bits: magnitude, requested_result_bits: 0x7f80_0000,
            error: MaskedMathError::Overflow, scaled_bits,
        }
    } else if sign != 0 && scaled_magnitude > 0x40c2_c000_0000_0000 {
        SourceExpPlan::MaskedRangeError {
            input_bits: bits, helper_input_bits: magnitude, requested_result_bits: 0,
            error: MaskedMathError::Underflow, scaled_bits,
        }
    } else { SourceExpPlan::Ordinary { input_bits: bits, scaled_bits } };
    (plan, status)
}

pub(super) fn resolve_source_exp_range_with_callback(
    plan: SourceExpPlan, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
) -> Result<MathErrorCallbackFloat32, MathErrorServiceError> {
    let SourceExpPlan::MaskedRangeError { helper_input_bits, requested_result_bits, error, .. } = plan
        else { return Err(MathErrorServiceError::PathRequired { advanced: state }); };
    let (argument1_f64_bits, state) = state.widen_f32_for_math_error(helper_input_bits);
    let request = MathErrorCall {
        operation: MathErrorOperation::Exponential,
        error,
        argument1_f64_bits,
        argument2_f64_bits: 0,
        proposed_result_f64_bits: if requested_result_bits == 0 { 0 } else { 0x7ff0_0000_0000_0000 },
    };
    let mut result = state.call_math_error(request, callback).map_err(MathErrorServiceError::Callback)?
        .into_rne_f32().map_err(MathErrorServiceError::Conversion)?;

    result.bits = requested_result_bits;
    Ok(result)
}

fn ordinary_exp(input: f64, scaled: f64) -> f32 {

    let integer = scaled.round_ties_even() as i32;
    let index = (integer & 63) as usize;
    let multiple = (integer - index as i32) >> 6;

    let remainder = (-(integer as f64)).mul_add(LN2_DIV64, input);
    let square = remainder * remainder;
    let polynomial = POLY_THIRD.mul_add(remainder, POLY_HALF);
    let reduced = square.mul_add(polynomial, remainder);
    let table = EXP_TABLE[index];
    let expanded = reduced.mul_add(table, table);
    let scale = f64::from_bits(((multiple + 1023) as u64) << 52);
    (expanded * scale) as f32
}
