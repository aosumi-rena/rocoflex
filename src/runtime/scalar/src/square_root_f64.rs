use super::finite_power::ExplicitExceptionState;
use super::scalar_f64_operand::F64Operand;
use super::scalar_round_f64::ScalarF64Candidate;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};
use super::square_root::round_normalized_square_root;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceSqrtF64Error {
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
}
impl SourceSqrtF64Error {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "SQRTSD explicit arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "SQRTSD state is not RNE/all-masked/gradual",
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState { entry }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSqrtF64Completion {
    pub source: u64,
    pub bits: u64,
    pub raised_status: u32,
    pub exception_state: ExplicitExceptionState,
}

pub struct SourceRawSquareRootF64 {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
}
impl SourceRawSquareRootF64 {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceSqrtF64Error>
    {
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceSqrtF64Error::ArchitectureNotClosed); }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceSqrtF64Error::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { architecture, state })
    }
    pub fn name(&self) -> &'static str { "sourceSQRTSD/IntelSdm/RNE-masked-gradual" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn evaluate(&mut self, value: f64) -> Result<f64, SourceSqrtF64Error> {
        self.evaluate_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn evaluate_bits(&mut self, bits: u64) -> Result<u64, SourceSqrtF64Error> {
        self.evaluate_bits_with_status(bits).map(|completion| completion.bits)
    }

    pub fn evaluate_bits_with_status(&mut self, source: u64) -> Result<SourceSqrtF64Completion, SourceSqrtF64Error> {
        let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
            .ok_or(SourceSqrtF64Error::UnsupportedEnvironment { mxcsr: self.state.mxcsr() })?;
        let candidate = sqrt_f64_candidate(source, before);
        let result = candidate.bits
            .ok_or(SourceSqrtF64Error::UnsupportedEnvironment { mxcsr: self.state.mxcsr() })?;
        self.state = self.state.with_source_status(u32::from(candidate.raised));
        Ok(SourceSqrtF64Completion { source, bits: result, raised_status: u32::from(candidate.raised),
            exception_state: self.state })
    }
}

pub(super) fn sqrt_f64_candidate(bits: u64, before: ScalarF32Mxcsr) -> ScalarF64Candidate {
    let source = F64Operand::from_bits(bits).with_daz(before.denormals_are_zero());
    if source.nan() {
        return ScalarF64Candidate { bits: Some(source.bits() | 0x0008_0000_0000_0000),
            raised: u8::from(source.signaling()) };
    }

    if source.negative() && !source.zero() {
        return ScalarF64Candidate { bits: Some(0xfff8_0000_0000_0000), raised: 1 };
    }
    if source.zero() || source.infinite() {
        return ScalarF64Candidate { bits: Some(source.bits()), raised: 0 };
    }
    let pre = if source.subnormal() { 2 } else { 0 };
    if pre != 0 && before.with_explicit_raised_status(ScalarF32Exceptions::DENORMAL).1.bits() != 0 {
        return ScalarF64Candidate { bits: None, raised: pre };
    }
    let (mut significand, power) = source.normalized_nonzero_parts();
    let mut exponent = power + 52;
    if exponent & 1 != 0 { significand <<= 1; exponent -= 1; }

    let (mut root, inexact) = round_normalized_square_root(significand, 52, before.rounding());
    let mut result_field = (exponent / 2 + 1023) as u64;
    if root == (1u64 << 53) { root >>= 1; result_field += 1; }
    let result = (result_field << 52) | (root & 0x000f_ffff_ffff_ffff);
    ScalarF64Candidate { bits: Some(result), raised: pre | if inexact { 0x20 } else { 0 } }
}
