use super::{retained_power::ScalarPowerServices, selected_log::Environment};
use super::shared_scalar_f32::comparison::{f32_comparison_candidate, F32Relation};

pub(super) fn small_cosine_fma_bits(bits: u64) -> (u64, u32) {
    let field = ((bits >> 52) & 0x7ff) as i32;
    let exponent = field - 1023;
    let significand = (bits & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let square = u128::from(significand) * u128::from(significand);
    let discard = (52 - 2 * exponent) as u32;
    let whole = square >> discard;
    let lost = square & ((1u128 << discard) - 1);
    let half = 1u128 << (discard - 1);
    let increment = lost > half || (lost == half && whole & 1 != 0);
    let rounded = whole + u128::from(increment);
    let below_one_significand = (1u64 << 53) - rounded as u64;
    let result = if below_one_significand == (1u64 << 53) {
        0x3ff0_0000_0000_0000
    } else { 0x3fe0_0000_0000_0000 | (below_one_significand & 0x000f_ffff_ffff_ffff) };
    (result, if lost != 0 { 0x20 } else { 0 })
}

pub(super) fn small_sine_fma_bits(bits: u64) -> (u64, u32) {
    let magnitude = bits & 0x7fff_ffff_ffff_ffff;
    let (square, square_status) = normal_product_bits(magnitude, magnitude);
    let (cube, cube_status) = normal_product_bits(square, magnitude);
    let coefficient = 0x3fc5_5555_5555_5555u64;
    let cube_sig = (cube & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let coefficient_sig = (coefficient & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let product = u128::from(cube_sig) * u128::from(coefficient_sig);
    let product_power = ((cube >> 52) & 0x7ff) as i32 - 1075
        + ((coefficient >> 52) & 0x7ff) as i32 - 1075;
    let input_sig = (magnitude & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let input_power = ((magnitude >> 52) & 0x7ff) as i32 - 1075;
    let shift = (input_power - product_power) as u32;
    let mut exact = [0u64; 3];
    let word = (shift / 64) as usize;
    let offset = shift % 64;
    exact[word] = input_sig << offset;
    if offset != 0 { exact[word + 1] = input_sig >> (64 - offset); }
    let (low, borrow_low) = exact[0].overflowing_sub(product as u64);
    let (middle, borrow_middle) = exact[1].overflowing_sub((product >> 64) as u64);
    let (middle, borrow_carry) = middle.overflowing_sub(u64::from(borrow_low));
    exact = [low, middle, exact[2] - u64::from(borrow_middle || borrow_carry)];
    let (result, fused_status) = round_normal_192(exact, product_power);
    (result | (bits & 0x8000_0000_0000_0000), square_status | cube_status | fused_status)
}

fn normal_product_bits(left: u64, right: u64) -> (u64, u32) {
    let ls = (left & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let rs = (right & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let product = u128::from(ls) * u128::from(rs);
    let power = ((left >> 52) & 0x7ff) as i32 - 1075
        + ((right >> 52) & 0x7ff) as i32 - 1075;
    round_exact_normal_u128(product, power)
}

pub(super) fn round_exact_normal_u128(product: u128, power: i32) -> (u64, u32) {
    round_exact_normal_u128_with_rounding(product, power, false,
        super::shared_scalar_f32::ScalarF32Rounding::NearestEven)
}

pub(super) fn round_exact_normal_u128_with_rounding(product: u128, power: i32,
    negative: bool, rounding: super::shared_scalar_f32::ScalarF32Rounding) -> (u64, u32) {
    let sign = if negative { 0x8000_0000_0000_0000 } else { 0 };
    let rounded = round_u128_binary64_unbounded(product, power, negative, rounding);
    (sign | ((rounded.field as u64) << 52) | (rounded.significand & 0x000f_ffff_ffff_ffff),
        if rounded.inexact { 0x20 } else { 0 })
}

pub(super) fn signed_i64_to_f64_candidate_bits(value: i64,
    rounding: super::shared_scalar_f32::ScalarF32Rounding) -> (u64, u32) {
    if value == 0 { (0, 0) } else {
        round_exact_normal_u128_with_rounding(u128::from(value.unsigned_abs()), 0, value < 0, rounding)
    }
}

pub(super) struct RoundedBinary64 {
    pub significand: u64,
    pub field: i32,
    pub inexact: bool,
}

pub(super) fn round_u128_binary64_unbounded(product: u128, power: i32,
    negative: bool, rounding: super::shared_scalar_f32::ScalarF32Rounding) -> RoundedBinary64 {
    let highest = 127 - product.leading_zeros();
    if highest <= 52 {
        let significand = (product as u64) << (52 - highest);
        let field = highest as i32 + power + 1023;
        return RoundedBinary64 { significand, field, inexact: false };
    }
    let discard = highest - 52;
    let (retained, inexact) = round_u128_right_to_grid(product, discard, negative, rounding);
    let mut retained = retained as u64;
    let mut field = highest as i32 + power + 1023;
    if retained == (1u64 << 53) { retained >>= 1; field += 1; }
    RoundedBinary64 { significand: retained, field, inexact }
}

pub(super) fn round_u128_right_to_grid(product: u128, discard: u32, negative: bool,
    rounding: super::shared_scalar_f32::ScalarF32Rounding) -> (u128, bool) {
    if discard == 0 { return (product, false); }
    if discard > 128 {
        let increment = binary_round_increment(rounding, negative, u128::from(product != 0), 2, false);
        return (u128::from(increment), product != 0);
    }
    let (retained, lost, half) = if discard == 128 { (0, product, 1u128 << 127) }
        else { (product >> discard, product & ((1u128 << discard) - 1), 1u128 << (discard - 1)) };
    let increment = binary_round_increment(rounding, negative, lost, half, retained & 1 != 0);
    (retained + u128::from(increment), lost != 0)
}

pub(super) fn binary_round_increment(rounding: super::shared_scalar_f32::ScalarF32Rounding,
    negative: bool, lost: u128, half: u128, odd: bool) -> bool {
    use super::shared_scalar_f32::ScalarF32Rounding;
    match rounding {
        ScalarF32Rounding::NearestEven => lost > half || (lost == half && odd),
        ScalarF32Rounding::TowardNegative => negative && lost != 0,
        ScalarF32Rounding::TowardPositive => !negative && lost != 0,
        ScalarF32Rounding::TowardZero => false,
    }
}

fn any_low_192(words: [u64; 3], count: u32) -> bool {
    let whole = (count / 64) as usize;
    let part = count % 64;
    words[..whole].iter().any(|word| *word != 0)
        || (part != 0 && words[whole] & ((1u64 << part) - 1) != 0)
}

fn round_normal_192(words: [u64; 3], power: i32) -> (u64, u32) {
    let high_word = if words[2] != 0 { 2usize } else if words[1] != 0 { 1 } else { 0 };
    let highest = high_word as u32 * 64 + 63 - words[high_word].leading_zeros();
    let discard = highest - 52;
    let word = (discard / 64) as usize;
    let offset = discard % 64;
    let mut retained = words[word] >> offset;
    if offset != 0 { retained |= words[word + 1] << (64 - offset); }
    let guard_position = discard - 1;
    let guard = words[(guard_position / 64) as usize] & (1u64 << (guard_position % 64)) != 0;
    let sticky = any_low_192(words, guard_position);
    retained += u64::from(guard && (sticky || retained & 1 != 0));
    let mut field = highest as i32 + power + 1023;
    if retained == (1u64 << 53) { retained >>= 1; field += 1; }
    let inexact = guard || sticky;
    (((field as u64) << 52) | (retained & 0x000f_ffff_ffff_ffff), if inexact { 0x20 } else { 0 })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalArithmeticError { NonNormalOperand, AlignmentOutsideDomain, NonNormalResult }

fn normal_components(bits: u64) -> Result<(bool, u64, i32), NormalArithmeticError> {
    let operand = super::scalar_f64_operand::F64Operand::from_bits(bits);
    let negative = operand.negative();
    if operand.zero() { return Ok((negative, 0, 0)); }
    if operand.field() == 0 || operand.field() == 0x7ff { return Err(NormalArithmeticError::NonNormalOperand); }
    let (significand, power) = operand.finite_parts();
    Ok((negative, significand, power))
}

pub(super) fn exact_f64_product(ls: u64, lp: i32, rs: u64, rp: i32) -> (u128, i32) {
    (u128::from(ls) * u128::from(rs), lp + rp)
}

fn shifted_1024(value: u128, shift: u32) -> Result<[u64; 16], NormalArithmeticError> {
    super::scalar_word_arithmetic::shifted_words::<16>(value, shift)
}

fn round_normal_1024(words: [u64; 16], power: i32, negative: bool) -> Result<(u64, u32), NormalArithmeticError> {
    let Some(high_word) = words.iter().rposition(|word| *word != 0) else { return Ok((0, 0)); };
    let highest = high_word as u32 * 64 + 63 - words[high_word].leading_zeros();
    let field = highest as i32 + power + 1023;
    if !(1..=2046).contains(&field) { return Err(NormalArithmeticError::NonNormalResult); }
    let rounded = super::scalar_word_arithmetic::round_words_unbounded(words, power, negative,
        super::shared_scalar_f32::ScalarF32Rounding::NearestEven)
        .ok_or(NormalArithmeticError::NonNormalResult)?;
    if rounded.field == 2047 { return Err(NormalArithmeticError::NonNormalResult); }
    let bits = ((if negative { 1u64 } else { 0 }) << 63)
        | ((rounded.field as u64) << 52) | (rounded.significand & 0x000f_ffff_ffff_ffff);
    Ok((bits, if rounded.inexact { 0x20 } else { 0 }))
}

pub(super) fn normal_fma_source_bits(left: u64, right: u64, addend: u64) -> Result<(u64, u32), NormalArithmeticError> {
    let (lsign, ls, lp) = normal_components(left)?;
    let (rsign, rs, rp) = normal_components(right)?;
    let (asign, ads, ap) = normal_components(addend)?;
    let product_sign = lsign ^ rsign;
    if ls == 0 || rs == 0 {
        if ads != 0 { return Ok((addend, 0)); }
        let negative = product_sign && asign;
        return Ok(((if negative { 1u64 } else { 0 }) << 63, 0));
    }
    let product = u128::from(ls) * u128::from(rs);
    let product_power = lp + rp;
    if ads == 0 { return round_normal_1024(shifted_1024(product, 0)?, product_power, product_sign); }
    let power = product_power.min(ap);
    let product_words = shifted_1024(product, (product_power - power) as u32)?;
    let addend_words = shifted_1024(u128::from(ads), (ap - power) as u32)?;
    let (exact, negative) = super::scalar_word_arithmetic::combine_words(
        product_words, product_sign, addend_words, asign)?;
    round_normal_1024(exact, power, negative)
}

pub(super) fn normal_multiply_source_bits(left: u64, right: u64) -> Result<(u64, u32), NormalArithmeticError> {
    let (lsign, ls, lp) = normal_components(left)?;
    let (rsign, rs, rp) = normal_components(right)?;
    let negative = lsign ^ rsign;
    if ls == 0 || rs == 0 { return Ok(((if negative { 1u64 } else { 0 }) << 63, 0)); }
    let (product, power) = exact_f64_product(ls, lp, rs, rp);
    round_normal_1024(shifted_1024(product, 0)?, power, negative)
}

pub(super) fn normal_divide_source_bits(left: u64, right: u64) -> Result<(u64, u32), NormalArithmeticError> {
    let (lsign, ls, lp) = normal_components(left)?;
    let (rsign, rs, rp) = normal_components(right)?;
    if rs == 0 { return Err(NormalArithmeticError::NonNormalOperand); }
    let sign = u64::from(lsign ^ rsign) << 63;
    if ls == 0 { return Ok((sign, 0)); }
    let below = ls < rs;
    let field = lp - rp + 1023 - i32::from(below);
    if !(1..=2046).contains(&field) { return Err(NormalArithmeticError::NonNormalResult); }
    let head = exact_f64_quotient_head(ls, rs);
    let (bits, status) = round_exact_normal_u128(head.jammed, lp - rp - head.shift);
    if bits >> 52 == 2047 { return Err(NormalArithmeticError::NonNormalResult); }
    Ok((sign | bits, status))
}

pub(super) struct F64QuotientHead { pub jammed: u128, pub shift: i32 }
pub(super) fn exact_f64_quotient_head(ls: u64, rs: u64) -> F64QuotientHead {
    let shift = if ls < rs { 56 } else { 55 };

    let numerator = u128::from(ls) << shift;
    let divisor = u128::from(rs);
    let quotient = numerator / divisor;
    let remainder = numerator % divisor;
    F64QuotientHead { jammed: quotient | u128::from(remainder != 0), shift }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequiredException {
    Underflow,
    Overflow,
}

impl RequiredException {

    pub fn source_request(self) -> u32 {
        match self {
            Self::Underflow => 0x10,
            Self::Overflow => 0x08,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerError {
    UnsupportedEnvironment,
    PositiveFiniteRequired,
    FiniteRequired,
    ExactZeroCoefficientInputRequired,
    NaNCoefficientInputRequired,
    ExternalExceptionRequired {
        staged_bits: Option<u32>,
        exception: RequiredException,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalePlan {
    Complete {
        bits: u32,

        helper_status: Option<i16>,
    },
    ExternalExceptionRequired {
        staged_bits: Option<u32>,
        exception: RequiredException,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExplicitExceptionState {
    mxcsr: u32,
    errno: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExceptionNotificationPlan {
    pub mxcsr: u32,
    pub errno_write: Option<i32>,
}

fn source_notification_plan_bits(mxcsr: u32, mut request: u32) -> SourceExceptionNotificationPlan {
    if request & 0x18 != 0 { request |= 0x20; }
    let errno_write = if request & 1 != 0 { Some(0x21) }
        else if request & 0x1c != 0 { Some(0x22) } else { None };
    SourceExceptionNotificationPlan { mxcsr: mxcsr | (request & 0x3d), errno_write }
}

pub fn plan_source_notification_profile_neutral(
    explicit_mxcsr: u32, request: u32,
) -> Result<SourceExceptionNotificationPlan, PowerError> {
    if explicit_mxcsr & 0xffff_0000 != 0 { return Err(PowerError::UnsupportedEnvironment); }
    Ok(source_notification_plan_bits(explicit_mxcsr, request))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathErrorCallbackPolicy { ExplicitlyAbsent, Present }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskedMathError { Domain, Singularity, Overflow, Underflow }

impl MaskedMathError {
    pub fn source_class(self) -> i32 {
        match self { Self::Domain => 1, Self::Singularity => 2, Self::Overflow => 3, Self::Underflow => 4 }
    }
    pub fn source_flags(self) -> u32 {
        match self { Self::Domain => 8, Self::Singularity => 4, Self::Overflow => 0x11, Self::Underflow => 0x12 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathErrorWithState {
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
    pub error: MaskedMathError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathErrorOperation { Logarithm, Exponential, SquareRoot, Sine, Cosine, SineFloat32, CosineFloat32 }

impl MathErrorOperation {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Logarithm => "logf", Self::Exponential => "expf", Self::SquareRoot => "sqrtf",
            Self::Sine => "sin", Self::Cosine => "cos", Self::SineFloat32 => "sinf", Self::CosineFloat32 => "cosf",
        }
    }
    pub fn source_operation_code(self) -> u32 {
        match self {
            Self::Logarithm => 0x1a, Self::Exponential => 0x14, Self::SquareRoot => 5,
            Self::Sine | Self::SineFloat32 => 0x1e, Self::Cosine | Self::CosineFloat32 => 0x12,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathErrorCall {
    pub operation: MathErrorOperation,
    pub error: MaskedMathError,
    pub argument1_f64_bits: u64,
    pub argument2_f64_bits: u64,
    pub proposed_result_f64_bits: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathErrorCallbackResponse {
    pub handled: bool,
    pub result_f64_bits: u64,
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathErrorCallbackFailure {
    pub reason: String,
    pub advanced: ExplicitExceptionState,
}

pub trait SourceMathErrorCallback {
    fn call(
        &mut self, request: MathErrorCall, state: ExplicitExceptionState,
    ) -> Result<MathErrorCallbackResponse, MathErrorCallbackFailure>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathErrorCallbackCompletion {
    pub result_f64_bits: u64,
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathErrorCallbackFloat32 {
    pub bits: u32,
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceScalarF32ConversionWithStatus {
    pub bits: u32,
    pub advanced: ExplicitExceptionState,
    pub raised_status: u8,
}

impl SourceScalarF32ConversionWithStatus {
    fn without_status(self) -> MathErrorCallbackFloat32 {
        MathErrorCallbackFloat32 { bits: self.bits, advanced: self.advanced }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathErrorConversionRequired {
    pub result_f64_bits: u64,
    pub advanced: ExplicitExceptionState,
}

impl MathErrorConversionRequired {

    pub fn convert_rne_source_bits(self) -> Result<MathErrorCallbackFloat32, Self> {
        MathErrorCallbackCompletion {
            result_f64_bits: self.result_f64_bits, advanced: self.advanced,
        }.into_rne_f32()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MathErrorServiceError {
    PathRequired { advanced: ExplicitExceptionState },
    Callback(MathErrorCallbackFailure),
    Conversion(MathErrorConversionRequired),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceScalarConversionError {
    ArchitectureNotClosed,
    ConversionRequired(MathErrorConversionRequired),
}

impl SourceScalarConversionError {
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self { Self::ConversionRequired(required) => required.advanced, _ => entry }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePackedF32ToF64WithState {
    pub bits: [u64; 2],
    pub exception_state: ExplicitExceptionState,
}

pub fn cvtps2pd_with_explicit_state(source: [u32; 2], state: ExplicitExceptionState,
    architecture: super::shared_scalar_f32::ScalarF32Architecture)
    -> Result<SourcePackedF32ToF64WithState, SourceScalarConversionError>
{
    if architecture != super::shared_scalar_f32::ScalarF32Architecture::IntelSdm {
        return Err(SourceScalarConversionError::ArchitectureNotClosed);
    }
    let (low, advanced) = state.widen_f32_for_math_error(source[0]);
    let (high, advanced) = advanced.widen_f32_for_math_error(source[1]);
    Ok(SourcePackedF32ToF64WithState { bits: [low, high], exception_state: advanced })
}

pub fn cvtsd2ss_with_explicit_state(source: u64, state: ExplicitExceptionState,
    architecture: super::shared_scalar_f32::ScalarF32Architecture)
    -> Result<MathErrorCallbackFloat32, SourceScalarConversionError>
{
    cvtsd2ss_with_explicit_status(source, state, architecture)
        .map(SourceScalarF32ConversionWithStatus::without_status)
}

pub fn cvtsd2ss_with_explicit_status(source: u64, state: ExplicitExceptionState,
    architecture: super::shared_scalar_f32::ScalarF32Architecture)
    -> Result<SourceScalarF32ConversionWithStatus, SourceScalarConversionError>
{
    if architecture != super::shared_scalar_f32::ScalarF32Architecture::IntelSdm {
        return Err(SourceScalarConversionError::ArchitectureNotClosed);
    }
    MathErrorCallbackCompletion { result_f64_bits: source, advanced: state }.into_rne_f32_with_status()
        .map_err(SourceScalarConversionError::ConversionRequired)
}

impl MathErrorCallbackCompletion {

    pub fn into_rne_f32(self) -> Result<MathErrorCallbackFloat32, MathErrorConversionRequired> {
        self.into_rne_f32_with_status().map(SourceScalarF32ConversionWithStatus::without_status)
    }

    fn into_rne_f32_with_status(mut self)
        -> Result<SourceScalarF32ConversionWithStatus, MathErrorConversionRequired> {

        let before = super::shared_scalar_f32::ScalarF32Mxcsr::from_explicit_word(self.advanced.mxcsr)
            .ok_or(MathErrorConversionRequired { result_f64_bits: self.result_f64_bits, advanced: self.advanced })?;
        let candidate = super::scalar_narrow_f64::narrow_f64_candidate(self.result_f64_bits, before);
        self.advanced = self.advanced.with_source_status(u32::from(candidate.raised));
        let bits = candidate.bits.ok_or(MathErrorConversionRequired {
            result_f64_bits: self.result_f64_bits, advanced: self.advanced })?;
        Ok(SourceScalarF32ConversionWithStatus {
            bits, advanced: self.advanced, raised_status: candidate.raised,
        })
    }

    pub fn into_exact_f32(mut self) -> Result<MathErrorCallbackFloat32, MathErrorConversionRequired> {
        let wide = self.result_f64_bits;
        let sign = ((wide >> 32) as u32) & 0x8000_0000;
        let field = ((wide >> 52) & 0x7ff) as i32;
        let fraction = wide & 0x000f_ffff_ffff_ffff;
        let bits = if let Some((bits, raised)) = super::scalar_narrow_f64::special_f64_to_f32(wide) {
            self.advanced.mxcsr |= u32::from(raised);
            bits
        } else if (897..=1150).contains(&field) && fraction & ((1u64 << 29) - 1) == 0 {
            sign | (((field - 896) as u32) << 23) | ((fraction >> 29) as u32)
        } else if (874..=896).contains(&field) {
            let significand = fraction | (1u64 << 52);
            let discard = (926 - field) as u32;
            if significand & ((1u64 << discard) - 1) != 0 {
                return Err(MathErrorConversionRequired { result_f64_bits: wide, advanced: self.advanced });
            }
            sign | ((significand >> discard) as u32)
        } else {
            return Err(MathErrorConversionRequired { result_f64_bits: wide, advanced: self.advanced });
        };
        Ok(MathErrorCallbackFloat32 { bits, advanced: self.advanced })
    }
}

impl ExplicitExceptionState {
    pub fn from_explicit_parts(mxcsr: u32, errno: i32) -> Result<Self, PowerError> {
        Environment::from_explicit_current_mxcsr(mxcsr)
            .map_err(|_| PowerError::UnsupportedEnvironment)?;
        if mxcsr & 0xffff_0000 != 0 {
            return Err(PowerError::UnsupportedEnvironment);
        }
        Ok(Self { mxcsr, errno })
    }

    pub fn mxcsr(self) -> u32 { self.mxcsr }
    pub fn errno(self) -> i32 { self.errno }

    pub(super) fn with_source_status(mut self, status: u32) -> Self {
        self.mxcsr |= status & 0x3f;
        self
    }

    pub(super) fn widen_f32_for_math_error(self, bits: u32) -> (u64, Self) {
        let (result, raised) = super::scalar_widen_f32::widen_f32_candidate(bits, false);
        (result, self.with_source_status(u32::from(raised)))
    }

    pub fn raise_source_request(mut self, request: u32) -> Self {
        let plan = source_notification_plan_bits(self.mxcsr, request);
        self.mxcsr = plan.mxcsr;
        if let Some(errno) = plan.errno_write { self.errno = errno; }
        self
    }

    pub fn apply_masked_math_error(mut self, error: MaskedMathError) -> Self {
        self = self.raise_masked_math_flags(error);
        self.apply_default_math_errno(error)
    }

    fn raise_masked_math_flags(mut self, error: MaskedMathError) -> Self {
        self.mxcsr |= match error {
            MaskedMathError::Domain => 1,
            MaskedMathError::Singularity => 4,
            MaskedMathError::Overflow => 0x28,
            MaskedMathError::Underflow => 0x30,
        };
        self
    }

    fn apply_default_math_errno(mut self, error: MaskedMathError) -> Self {
        match error {
            MaskedMathError::Domain => self.errno = 0x21,
            MaskedMathError::Singularity | MaskedMathError::Overflow => self.errno = 0x22,
            MaskedMathError::Underflow => {},
        }
        self
    }

    pub fn call_math_error(
        self, request: MathErrorCall, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<MathErrorCallbackCompletion, MathErrorCallbackFailure> {
        let state = self.raise_masked_math_flags(request.error);
        let response = callback.call(request, state)?;
        let advanced = if response.handled { response.advanced }
            else { response.advanced.apply_default_math_errno(request.error) };
        Ok(MathErrorCallbackCompletion { result_f64_bits: response.result_f64_bits, advanced })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScaleWithState {
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
    pub raised: Option<RequiredException>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoefficientScaleResult {
    pub bits: u32,
    pub source_status: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoefficientScaledExpBoundary {
    Complete(CoefficientScaleResult),
    OrdinaryFinite { input_bits: u32, multiplier_bits: u32, exponent2: i16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoefficientSourceExpBoundary {
    Complete(CoefficientScaleResult),
    ArithmeticRequired { input_bits: u32, multiplier_bits: u32, exponent2: i16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoefficientSourceExpEntry {
    pub boundary: CoefficientSourceExpBoundary,

    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EntryComparison { Less, Equal, Greater, Unordered }

impl From<F32Relation> for EntryComparison {
    fn from(relation: F32Relation) -> Self {
        match relation {
            F32Relation::Less => Self::Less,
            F32Relation::Equal => Self::Equal,
            F32Relation::Greater => Self::Greater,
            F32Relation::Unordered => Self::Unordered,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF32CompareOpcode { Comiss, Ucomiss }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF32Comparison {
    pub carry: bool,
    pub parity: bool,
    pub zero: bool,

    pub exception_state: ExplicitExceptionState,
}

pub fn compare_f32_with_explicit_state(
    opcode: SourceF32CompareOpcode, left: u32, right: u32, mut state: ExplicitExceptionState,
) -> SourceF32Comparison {
    let (comparison, raised) = f32_comparison_candidate(left, right, opcode == SourceF32CompareOpcode::Ucomiss, false);
    state.mxcsr |= u32::from(raised);
    let (carry, parity, zero) = comparison.flag_bits();
    SourceF32Comparison { carry, parity, zero, exception_state: state }
}

pub(super) fn comparison_flag_bits(comparison: EntryComparison) -> (bool, bool, bool) {
    match comparison {
        EntryComparison::Less => (true, false, false),
        EntryComparison::Equal => (false, false, true),
        EntryComparison::Greater => (false, false, false),
        EntryComparison::Unordered => (true, true, true),
    }
}

fn coefficient_entry_comparison(
    left: u32, right: u32, quiet: bool, state: &mut ExplicitExceptionState,
) -> EntryComparison {
    let (comparison, raised) = f32_comparison_candidate(left, right, quiet, false);
    state.mxcsr |= u32::from(raised);
    comparison.into()
}

pub fn source_exponent_bits_profile_neutral(bits: u32) -> i32 {
    let magnitude = bits & 0x7fff_ffff;
    if magnitude == 0 { return i32::MIN; }
    if magnitude >= 0x7f80_0000 { return i32::MAX; }
    let field = magnitude >> 23;
    if field != 0 { return field as i32 - 127; }
    let fraction = magnitude & 0x007f_ffff;
    (31 - fraction.leading_zeros()) as i32 - 149
}

pub fn plan_source_scale_profile_neutral(bits: u32, shift: i32) -> Result<ScalePlan, PowerError> {
    if shift == 0 || bits & 0x7fff_ffff >= 0x7f80_0000 {
        return Ok(ScalePlan::Complete { bits, helper_status: None });
    }
    plan_scale_finite_profile_neutral(bits, shift)
}

fn plan_scale_finite_profile_neutral(bits: u32, shift: i32) -> Result<ScalePlan, PowerError> {
    let sign = bits & 0x8000_0000;
    let magnitude = bits & 0x7fff_ffff;
    if magnitude >= 0x7f80_0000 {
        return Err(PowerError::FiniteRequired);
    }
    if magnitude == 0 || shift == 0 {
        return Ok(ScalePlan::Complete { bits, helper_status: None });
    }

    let field = (magnitude >> 23) as i32;
    let fraction = magnitude & 0x007f_ffff;
    let (exponent, significand) = if field == 0 {

        let highest = 31 - fraction.leading_zeros();
        (highest as i32 - 22, fraction << (23 - highest))
    } else {
        (field, fraction | 0x0080_0000)
    };

    let target = i64::from(exponent) + i64::from(shift);
    if target >= 255 {
        return Ok(ScalePlan::ExternalExceptionRequired {

            staged_bits: Some(sign | 0x7f80_0000),
            exception: RequiredException::Overflow,
        });
    }
    if target > 0 {
        let bits = sign | ((target as u32) << 23) | (significand & 0x007f_ffff);
        return Ok(ScalePlan::Complete { bits, helper_status: Some(-1) });
    }

    let discard = 1 - target;

    let rounded = if discard > 24 {
        0
    } else {
        let discard = discard as u32;
        let retained = significand >> discard;
        let lost = significand & ((1u32 << discard) - 1);
        let halfway = 1u32 << (discard - 1);
        let increment = lost > halfway || (lost == halfway && retained & 1 != 0);
        retained + u32::from(increment)
    };
    if rounded == 0 {
        return Ok(ScalePlan::ExternalExceptionRequired {
            staged_bits: Some(sign),
            exception: RequiredException::Underflow,
        });
    }

    Ok(ScalePlan::Complete { bits: sign | rounded, helper_status: Some(-1) })
}

pub struct FinitePowerServices {
    environment: Environment,
    exponent_service: ScalarPowerServices,
}

impl FinitePowerServices {
    pub fn bind(mxcsr: u32) -> Result<Self, PowerError> {
        let environment = Environment::from_explicit_current_mxcsr(mxcsr)
            .map_err(|_| PowerError::UnsupportedEnvironment)?;
        let exponent_service = ScalarPowerServices::bind(mxcsr)
            .map_err(|_| PowerError::UnsupportedEnvironment)?;
        Ok(Self { environment, exponent_service })
    }

    pub fn environment(&self) -> Environment {
        self.environment
    }

    pub fn exponent(&self, positive_finite: f32) -> Result<i32, PowerError> {
        self.exponent_service.exponent(positive_finite)
            .map_err(|_| PowerError::PositiveFiniteRequired)
    }

    pub fn source_exponent_bits(&self, bits: u32) -> i32 {
        source_exponent_bits_profile_neutral(bits)
    }

    pub fn scale(&self, value: f32, shift: i32) -> Result<f32, PowerError> {
        match self.plan_scale(value.to_bits(), shift)? {
            ScalePlan::Complete { bits, .. } => Ok(f32::from_bits(bits)),
            ScalePlan::ExternalExceptionRequired { staged_bits, exception } =>
                Err(PowerError::ExternalExceptionRequired { staged_bits, exception }),
        }
    }

    pub fn scale_bits(&self, bits: u32, shift: i32) -> Result<u32, PowerError> {
        self.scale(f32::from_bits(bits), shift).map(f32::to_bits)
    }

    pub fn plan_source_scale(&self, bits: u32, shift: i32) -> Result<ScalePlan, PowerError> {
        plan_source_scale_profile_neutral(bits, shift)
    }

    pub fn scale_with_explicit_state(
        &self, bits: u32, shift: i32, state: ExplicitExceptionState,
    ) -> Result<ScaleWithState, PowerError> {
        match self.plan_source_scale(bits, shift)? {
            ScalePlan::Complete { bits, .. } => Ok(ScaleWithState {
                bits, exception_state: state, raised: None,
            }),
            ScalePlan::ExternalExceptionRequired { staged_bits: Some(bits), exception } =>
                Ok(ScaleWithState {
                    bits,
                    exception_state: state.raise_source_request(exception.source_request()),
                    raised: Some(exception),
                }),
            ScalePlan::ExternalExceptionRequired { staged_bits, exception } =>
                Err(PowerError::ExternalExceptionRequired { staged_bits, exception }),
        }
    }

    pub fn coefficient_scale_with_explicit_overflow_word(
        &self, bits: u32, shift: i32, overflow_word: u32,
    ) -> Result<CoefficientScaleResult, PowerError> {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude >= 0x7f80_0000 {
            return Ok(CoefficientScaleResult {
                bits, source_status: if magnitude == 0x7f80_0000 { 1 } else { 2 },
            });
        }
        if magnitude == 0 { return Ok(CoefficientScaleResult { bits, source_status: 0 }); }
        match self.plan_scale(bits, shift)? {
            ScalePlan::Complete { bits, .. } => Ok(CoefficientScaleResult { bits, source_status: u32::MAX }),
            ScalePlan::ExternalExceptionRequired { exception: RequiredException::Overflow, .. } =>
                Ok(CoefficientScaleResult {
                    bits: overflow_word ^ (bits & 0x8000_0000), source_status: 1,
                }),
            ScalePlan::ExternalExceptionRequired { staged_bits: Some(bits), exception: RequiredException::Underflow } =>
                Ok(CoefficientScaleResult { bits, source_status: 0 }),
            ScalePlan::ExternalExceptionRequired { staged_bits, exception } =>
                Err(PowerError::ExternalExceptionRequired { staged_bits, exception }),
        }
    }

    pub fn coefficient_scaled_exp_boundary(
        &self, input_bits: u32, multiplier_bits: u32, exponent2: i16, overflow_word: u32,
    ) -> Result<CoefficientScaledExpBoundary, PowerError> {
        if input_bits & 0x7fff_ffff >= 0x7f80_0000
            || multiplier_bits & 0x7fff_ffff >= 0x7f80_0000
        { return Err(PowerError::FiniteRequired); }
        let magnitude = input_bits & 0x7fff_ffff;
        if (input_bits & 0x8000_0000 != 0 && magnitude > 0x4365_0000)
            || multiplier_bits & 0x7fff_ffff == 0
        {
            return Ok(CoefficientScaledExpBoundary::Complete(CoefficientScaleResult { bits: 0, source_status: 0 }));
        }
        if input_bits & 0x8000_0000 == 0 && magnitude > 0x4365_0000 {
            return Ok(CoefficientScaledExpBoundary::Complete(CoefficientScaleResult { bits: overflow_word, source_status: 1 }));
        }
        Ok(CoefficientScaledExpBoundary::OrdinaryFinite { input_bits, multiplier_bits, exponent2 })
    }

    pub fn coefficient_scaled_exp_source_entry(
        &self, input_bits: u32, multiplier_bits: u32, exponent2: i16,
        overflow_word: u32, mut state: ExplicitExceptionState,
    ) -> CoefficientSourceExpEntry {

        let first = coefficient_entry_comparison(0xc365_0000, input_bits, false, &mut state);
        let boundary = if first == EntryComparison::Greater {
            CoefficientSourceExpBoundary::Complete(CoefficientScaleResult { bits: 0, source_status: 0 })
        } else {

            let second = coefficient_entry_comparison(multiplier_bits, 0, true, &mut state);
            if second == EntryComparison::Equal {
                CoefficientSourceExpBoundary::Complete(CoefficientScaleResult { bits: 0, source_status: 0 })
            } else {

                let third = coefficient_entry_comparison(input_bits, 0x4365_0000, false, &mut state);
                if third == EntryComparison::Greater {
                    CoefficientSourceExpBoundary::Complete(CoefficientScaleResult {
                        bits: overflow_word, source_status: 1,
                    })
                } else {
                    CoefficientSourceExpBoundary::ArithmeticRequired { input_bits, multiplier_bits, exponent2 }
                }
            }
        };
        CoefficientSourceExpEntry { boundary, exception_state: state }
    }

    pub fn coefficient_zero_real_with_explicit_state(
        &self, input_bits: u32, multiplier_bits: u32, exponent2: i16,
        overflow_word: u32, state: ExplicitExceptionState,
    ) -> Result<CoefficientSourceExpEntry, PowerError> {
        if input_bits & 0x7fff_ffff != 0 { return Err(PowerError::ExactZeroCoefficientInputRequired); }
        let entry = self.coefficient_scaled_exp_source_entry(input_bits, multiplier_bits, exponent2, overflow_word, state);
        self.coefficient_zero_real_from_entry(entry, multiplier_bits, exponent2, overflow_word)
    }

    fn coefficient_zero_real_from_entry(&self, entry: CoefficientSourceExpEntry,
        multiplier_bits: u32, exponent2: i16, overflow_word: u32,
    ) -> Result<CoefficientSourceExpEntry, PowerError> {
        if matches!(entry.boundary, CoefficientSourceExpBoundary::Complete(_)) { return Ok(entry); }
        let result = self.coefficient_scale_with_explicit_overflow_word(multiplier_bits, i32::from(exponent2), overflow_word)?;
        Ok(CoefficientSourceExpEntry {
            boundary: CoefficientSourceExpBoundary::Complete(result),
            exception_state: entry.exception_state.with_source_status(0x20),
        })
    }

    pub fn coefficient_nan_real_with_explicit_state(
        &self, input_bits: u32, multiplier_bits: u32, exponent2: i16,
        overflow_word: u32, state: ExplicitExceptionState,
    ) -> Result<CoefficientSourceExpEntry, PowerError> {
        if input_bits & 0x7fff_ffff <= 0x7f80_0000 { return Err(PowerError::NaNCoefficientInputRequired); }
        let entry = self.coefficient_scaled_exp_source_entry(input_bits, multiplier_bits, exponent2, overflow_word, state);
        Ok(self.coefficient_nan_real_from_entry(entry, input_bits))
    }

    fn coefficient_nan_real_from_entry(&self, entry: CoefficientSourceExpEntry,
        input_bits: u32,
    ) -> CoefficientSourceExpEntry {
        if matches!(entry.boundary, CoefficientSourceExpBoundary::Complete(_)) { return entry; }
        CoefficientSourceExpEntry {
            boundary: CoefficientSourceExpBoundary::Complete(CoefficientScaleResult {
                bits: input_bits | 0x0040_0000, source_status: 2,
            }),
            exception_state: entry.exception_state,
        }
    }

    pub fn coefficient_source_boundary_with_explicit_state(
        &self, input_bits: u32, multiplier_bits: u32, exponent2: i16,
        overflow_word: u32, state: ExplicitExceptionState,
    ) -> Result<CoefficientSourceExpEntry, PowerError> {
        let entry = self.coefficient_scaled_exp_source_entry(input_bits, multiplier_bits, exponent2, overflow_word, state);
        self.coefficient_source_boundary_from_entry(entry, overflow_word)
    }

    pub(super) fn coefficient_source_boundary_from_entry(&self,
        entry: CoefficientSourceExpEntry, overflow_word: u32,
    ) -> Result<CoefficientSourceExpEntry, PowerError> {
        let CoefficientSourceExpBoundary::ArithmeticRequired {
            input_bits, multiplier_bits, exponent2,
        } = entry.boundary else { return Ok(entry); };
        let magnitude = input_bits & 0x7fff_ffff;
        if magnitude == 0 {
            self.coefficient_zero_real_from_entry(entry, multiplier_bits, exponent2, overflow_word)
        } else if magnitude > 0x7f80_0000 {
            Ok(self.coefficient_nan_real_from_entry(entry, input_bits))
        } else { Ok(entry) }
    }

    pub fn plan_scale(&self, bits: u32, shift: i32) -> Result<ScalePlan, PowerError> {
        plan_scale_finite_profile_neutral(bits, shift)
    }
}
