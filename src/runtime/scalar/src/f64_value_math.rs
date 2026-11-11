use super::finite_power::{ExplicitExceptionState, NormalArithmeticError, SourceScalarConversionError,
    cvtsd2ss_with_explicit_status, signed_i64_to_f64_candidate_bits};
use super::raw_scalar_select_f64::{selection_f64_bits, SourceF64SelectionOpcode};
use super::scalar_add_f64::{sum_f64_candidate, ScalarF64SumOperation};
use super::scalar_compare_f64::f64_comparison_candidate;
use super::scalar_divide_f64::divide_f64_candidate;
use super::scalar_multiply_f64::multiply_f64_candidate;
use super::scalar_round_f64::ScalarF64Candidate;
use super::scalar_widen_f32::widen_f32_candidate;
use super::selected_finite_phase_word_zero::{SourceFinitePhaseWordZero, SourceFinitePhaseWordZeroError};
use super::selected_phase::{SourcePhaseNonzero, PhaseError};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF64ValueOperation { Add, Subtract, Multiply, Divide, Minimum,
    OrderedLessSignaling, WidenF32, SignedI32ToF64, NarrowF64, UnsignedU32ToF64,
    Sine586960, Cosine57ddb0 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF64ValueOperands {
    Binary { left: u64, right: u64 }, F32 { source: u32 },
    SignedI32 { value: i32 }, F64 { source: u64 }, UnsignedU32 { value: u32 },
    PhaseF64 { source: u64, explicit_source_word: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF64ValueResult { F64Bits(u64), F32Bits(u32), OrderedLess(bool) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF64ValueCause {
    UnsupportedEnvironment, AlignmentRequired(NormalArithmeticError),
    RaisedStatusRequired { reported: u8 }, TrapRequired { unmasked: u8 },
    CandidateRequired, ComparisonMaskRequired { mask: u64 },
    Conversion(SourceScalarConversionError),
    PhaseWordZero(SourceFinitePhaseWordZeroError), PhaseNonzero(PhaseError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF64ValueFailure {
    pub operation: SourceF64ValueOperation, pub operands: SourceF64ValueOperands,
    pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState,
    pub raised_status: Option<u8>, pub cause: SourceF64ValueCause,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF64ValueCompletion {
    pub operation: SourceF64ValueOperation, pub operands: SourceF64ValueOperands,
    pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState,
    pub result: SourceF64ValueResult,

    pub raised_status: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF64ValueCandidateReceipt {
    pub operation: SourceF64ValueOperation,
    pub operands: SourceF64ValueOperands,
    pub entry: ExplicitExceptionState,
    pub before_mxcsr: u32,
    pub candidate_bits: Option<u64>,
    pub raised_status: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF64ValueError {
    ArchitectureNotClosed { architecture: ScalarF32Architecture, advanced: ExplicitExceptionState },
    UnsupportedEnvironment { advanced: ExplicitExceptionState },
    OperationFailed(SourceF64ValueFailure), AlreadyFailed(SourceF64ValueFailure),
}
impl SourceF64ValueError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed { .. } => "F64 value service requires explicit IntelSdm architecture",
            Self::UnsupportedEnvironment { .. } => "F64 value service requires strict RNE/allmasked/gradual state",
            Self::AlreadyFailed(_) => "F64 value service stopped after its retained original failure",
            Self::OperationFailed(failure) => match failure.cause {
                SourceF64ValueCause::UnsupportedEnvironment => "F64 value operation controls left the admitted profile",
                SourceF64ValueCause::AlignmentRequired(_) => "F64 value exact aligned continuation required",
                SourceF64ValueCause::RaisedStatusRequired { .. } => "F64 value candidate status not admitted",
                SourceF64ValueCause::TrapRequired { .. } => "F64 value unmasked instruction continuation required",
                SourceF64ValueCause::CandidateRequired => "F64 value source candidate continuation required",
                SourceF64ValueCause::ComparisonMaskRequired { .. } => "LT_OS candidate is not the literal full64 mask",
                SourceF64ValueCause::Conversion(_) => "F64 value borrowed narrowing continuation required",
                SourceF64ValueCause::PhaseWordZero(error) => error.reason(),
                SourceF64ValueCause::PhaseNonzero(_) => "F64 value explicit nonzero source phase continuation required",
            },
        }
    }
    pub fn known_advanced_state(self, _entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::ArchitectureNotClosed { advanced, .. } | Self::UnsupportedEnvironment { advanced } => advanced,
            Self::OperationFailed(failure) | Self::AlreadyFailed(failure) => failure.advanced,
        }
    }
}

#[derive(Debug)]
pub struct SourceF64ValueParts {
    pub architecture: ScalarF32Architecture, pub exception_state: ExplicitExceptionState,
    pub failure: Option<SourceF64ValueFailure>, pub last_completion: Option<SourceF64ValueCompletion>,
    pub last_candidate: Option<SourceF64ValueCandidateReceipt>,
}

#[derive(Debug)]
pub struct SourceF64ValueMath {
    architecture: ScalarF32Architecture, state: ExplicitExceptionState,
    failure: Option<SourceF64ValueFailure>, last: Option<SourceF64ValueCompletion>,
    last_candidate: Option<SourceF64ValueCandidateReceipt>,
}
impl SourceF64ValueMath {

    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState) -> Result<Self, SourceF64ValueError> {
        if architecture != ScalarF32Architecture::IntelSdm {
            return Err(SourceF64ValueError::ArchitectureNotClosed { architecture, advanced: state });
        }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceF64ValueError::UnsupportedEnvironment { advanced: state });
        }
        Ok(Self { architecture, state, failure: None, last: None, last_candidate: None })
    }
    pub fn name(&self) -> &'static str { "source-f64-values/IntelSdm/RNE-masked-gradual" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn failure(&self) -> Option<&SourceF64ValueFailure> { self.failure.as_ref() }
    pub fn last_completion(&self) -> Option<SourceF64ValueCompletion> { self.last }
    pub fn last_candidate(&self) -> Option<SourceF64ValueCandidateReceipt> { self.last_candidate }
    pub fn into_parts(self) -> SourceF64ValueParts {
        SourceF64ValueParts { architecture: self.architecture, exception_state: self.state,
            failure: self.failure, last_completion: self.last, last_candidate: self.last_candidate }
    }
    pub fn add_bits(&mut self, left: u64, right: u64) -> Result<u64, SourceF64ValueError> {
        self.binary(SourceF64ValueOperation::Add, left, right)
    }
    pub fn subtract_bits(&mut self, left: u64, right: u64) -> Result<u64, SourceF64ValueError> {
        self.binary(SourceF64ValueOperation::Subtract, left, right)
    }
    pub fn multiply_bits(&mut self, left: u64, right: u64) -> Result<u64, SourceF64ValueError> {
        self.binary(SourceF64ValueOperation::Multiply, left, right)
    }
    pub fn divide_bits(&mut self, left: u64, right: u64) -> Result<u64, SourceF64ValueError> {
        self.binary(SourceF64ValueOperation::Divide, left, right)
    }

    pub fn minimum_bits(&mut self, left: u64, right: u64) -> Result<u64, SourceF64ValueError> {
        self.binary(SourceF64ValueOperation::Minimum, left, right)
    }

    pub fn ordered_less_bits(&mut self, left: u64, right: u64) -> Result<bool, SourceF64ValueError> {
        self.binary(SourceF64ValueOperation::OrderedLessSignaling, left, right).map(|mask| mask == u64::MAX)
    }
    pub fn widen_f32_bits(&mut self, source: u32) -> Result<u64, SourceF64ValueError> {
        let op = SourceF64ValueOperation::WidenF32;
        let operands = SourceF64ValueOperands::F32 { source };
        let entry = self.state;
        let before = self.begin(op, operands)?;
        let (bits, raised) = widen_f32_candidate(source, before.denormals_are_zero());
        self.finish(op, operands, entry, before, ScalarF64Candidate { bits: Some(bits), raised })
    }

    pub fn signed_i32_to_f64_bits(&mut self, value: i32) -> Result<u64, SourceF64ValueError> {
        let op = SourceF64ValueOperation::SignedI32ToF64;
        let operands = SourceF64ValueOperands::SignedI32 { value };
        let entry = self.state;
        let before = self.begin(op, operands)?;
        let (bits, status) = signed_i64_to_f64_candidate_bits(i64::from(value), before.rounding());
        self.finish(op, operands, entry, before, ScalarF64Candidate { bits: Some(bits), raised: status as u8 })
    }

    pub fn unsigned_u32_to_f64_bits(&mut self, value: u32) -> Result<u64, SourceF64ValueError> {
        let op = SourceF64ValueOperation::UnsignedU32ToF64;
        let operands = SourceF64ValueOperands::UnsignedU32 { value };
        let entry = self.state;
        let before = self.begin(op, operands)?;
        let (bits, status) = signed_i64_to_f64_candidate_bits(i64::from(value), before.rounding());
        self.finish(op, operands, entry, before, ScalarF64Candidate { bits: Some(bits), raised: status as u8 })
    }
    pub fn narrow_f64_bits(&mut self, source: u64) -> Result<u32, SourceF64ValueError> {
        let op = SourceF64ValueOperation::NarrowF64;
        let operands = SourceF64ValueOperands::F64 { source };
        let entry = self.state;
        self.begin(op, operands)?;
        match cvtsd2ss_with_explicit_status(source, entry, self.architecture) {
            Ok(result) => {
                self.state = result.advanced;
                self.remember(op, operands, entry, SourceF64ValueResult::F32Bits(result.bits), Some(result.raised_status));
                Ok(result.bits)
            }
            Err(error) => {
                self.state = error.known_advanced_state(entry);
                Err(self.stop(op, operands, entry, None, SourceF64ValueCause::Conversion(error)))
            }
        }
    }

    pub fn sine_586960_bits(&mut self, source: u64, explicit_source_word: u32)
        -> Result<u64, SourceF64ValueError> {
        self.phase_bits(source, explicit_source_word, true)
    }

    pub fn cosine_57ddb0_bits(&mut self, source: u64, explicit_source_word: u32)
        -> Result<u64, SourceF64ValueError> {
        self.phase_bits(source, explicit_source_word, false)
    }
    fn phase_bits(&mut self, source: u64, explicit_source_word: u32, sine: bool)
        -> Result<u64, SourceF64ValueError> {
        let op = if sine { SourceF64ValueOperation::Sine586960 }
            else { SourceF64ValueOperation::Cosine57ddb0 };
        let operands = SourceF64ValueOperands::PhaseF64 { source, explicit_source_word };
        let entry = self.state;
        self.begin(op, operands)?;
        let result = if explicit_source_word == 0 {
            match SourceFinitePhaseWordZero::bind(explicit_source_word, self.architecture, entry) {
                Err(error) => Err((SourceF64ValueCause::PhaseWordZero(error), error.known_advanced_state(entry), None)),
                Ok(mut leaf) => {
                    let result = if sine { leaf.sine_bits_with_status(source) }
                        else { leaf.cosine_bits_with_status(source) };
                    result.map(|result| (result.bits, result.exception_state, Some(result.raised_status as u8)))
                        .map_err(|failure| (SourceF64ValueCause::PhaseWordZero(failure.error),
                            failure.exception_state, Some(failure.raised_status as u8)))
                }
            }
        } else {
            match SourcePhaseNonzero::bind(explicit_source_word, entry.mxcsr()) {
                Err(error) => Err((SourceF64ValueCause::PhaseNonzero(error), entry, None)),
                Ok(leaf) => {
                    let result = if sine { leaf.sin_with_explicit_status(source, entry) }
                        else { leaf.cos_with_explicit_status(source, entry) };

                    result.map(|result| (result.bits, result.exception_state, Some(result.raised_status as u8)))
                        .map_err(|failure| (SourceF64ValueCause::PhaseNonzero(failure.error),
                            failure.exception_state, Some(failure.raised_status as u8)))
                }
            }
        };
        match result {
            Ok((bits, advanced, raised_status)) => {
                self.state = advanced;
                self.remember(op, operands, entry, SourceF64ValueResult::F64Bits(bits), raised_status);
                Ok(bits)
            }
            Err((cause, advanced, raised_status)) => {
                self.state = advanced;
                Err(self.stop(op, operands, entry, raised_status, cause))
            }
        }
    }

    fn begin(&mut self, op: SourceF64ValueOperation, operands: SourceF64ValueOperands)
        -> Result<ScalarF32Mxcsr, SourceF64ValueError> {
        if let Some(failure) = self.failure { return Err(SourceF64ValueError::AlreadyFailed(failure)); }
        self.last_candidate = None;
        if self.state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(self.stop(op, operands, self.state, None, SourceF64ValueCause::UnsupportedEnvironment));
        }
        ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr()).ok_or_else(||
            self.stop(op, operands, self.state, None, SourceF64ValueCause::UnsupportedEnvironment))
    }
    fn stop(&mut self, operation: SourceF64ValueOperation, operands: SourceF64ValueOperands,
        entry: ExplicitExceptionState, raised_status: Option<u8>, cause: SourceF64ValueCause) -> SourceF64ValueError {
        let failure = SourceF64ValueFailure { operation, operands, entry, advanced: self.state, raised_status, cause };
        self.failure = Some(failure);
        SourceF64ValueError::OperationFailed(failure)
    }
    fn remember(&mut self, operation: SourceF64ValueOperation, operands: SourceF64ValueOperands,
        entry: ExplicitExceptionState, result: SourceF64ValueResult, raised_status: Option<u8>) {
        self.last = Some(SourceF64ValueCompletion { operation, operands, entry,
            advanced: self.state, result, raised_status });
    }
    fn accumulate(&mut self, op: SourceF64ValueOperation, operands: SourceF64ValueOperands,
        entry: ExplicitExceptionState, before: ScalarF32Mxcsr, raised_status: u8) -> Result<(), SourceF64ValueError> {
        let Some(raised) = ScalarF32Exceptions::from_explicit_bits(raised_status) else {
            return Err(self.stop(op, operands, entry, Some(raised_status),
                SourceF64ValueCause::RaisedStatusRequired { reported: raised_status }));
        };
        let (_, unmasked) = before.with_explicit_raised_status(raised);
        self.state = self.state.with_source_status(u32::from(raised.bits()));
        if unmasked.bits() != 0 {
            return Err(self.stop(op, operands, entry, Some(raised_status),
                SourceF64ValueCause::TrapRequired { unmasked: unmasked.bits() }));
        }
        Ok(())
    }
    fn finish(&mut self, op: SourceF64ValueOperation, operands: SourceF64ValueOperands,
        entry: ExplicitExceptionState, before: ScalarF32Mxcsr, candidate: ScalarF64Candidate) -> Result<u64, SourceF64ValueError> {
        self.last_candidate = Some(SourceF64ValueCandidateReceipt { operation: op, operands,
            entry, before_mxcsr: before.word(), candidate_bits: candidate.bits, raised_status: candidate.raised });
        self.accumulate(op, operands, entry, before, candidate.raised)?;
        let Some(bits) = candidate.bits else {
            return Err(self.stop(op, operands, entry, Some(candidate.raised), SourceF64ValueCause::CandidateRequired));
        };
        let result = if op == SourceF64ValueOperation::OrderedLessSignaling {
            if bits != 0 && bits != u64::MAX {
                return Err(self.stop(op, operands, entry, Some(candidate.raised),
                    SourceF64ValueCause::ComparisonMaskRequired { mask: bits }));
            }
            SourceF64ValueResult::OrderedLess(bits == u64::MAX)
        } else { SourceF64ValueResult::F64Bits(bits) };
        self.remember(op, operands, entry, result, Some(candidate.raised));
        Ok(bits)
    }
    fn binary(&mut self, op: SourceF64ValueOperation, left: u64, right: u64) -> Result<u64, SourceF64ValueError> {
        let operands = SourceF64ValueOperands::Binary { left, right };
        let entry = self.state;
        let before = self.begin(op, operands)?;
        let candidate = match op {
            SourceF64ValueOperation::Add | SourceF64ValueOperation::Subtract => {
                let sum = if op == SourceF64ValueOperation::Add { ScalarF64SumOperation::Add }
                    else { ScalarF64SumOperation::Subtract };
                match sum_f64_candidate(sum, left, right, before) {
                    Ok(candidate) => candidate,
                    Err(required) => {
                        self.accumulate(op, operands, entry, before, required.raised)?;
                        return Err(self.stop(op, operands, entry, Some(required.raised),
                            SourceF64ValueCause::AlignmentRequired(required.reason)));
                    }
                }
            }
            SourceF64ValueOperation::Multiply => multiply_f64_candidate(left, right, before),
            SourceF64ValueOperation::Divide => divide_f64_candidate(left, right, before),
            SourceF64ValueOperation::Minimum | SourceF64ValueOperation::OrderedLessSignaling => {
                let opcode = if op == SourceF64ValueOperation::Minimum { SourceF64SelectionOpcode::Minsd }
                    else { SourceF64SelectionOpcode::Cmpltsd };
                let (relation, raised) = f64_comparison_candidate(left, right, false, before.denormals_are_zero());
                ScalarF64Candidate { bits: selection_f64_bits(opcode, left, right, relation, before.denormals_are_zero()), raised }
            }
            _ => return Err(self.stop(op, operands, entry, None, SourceF64ValueCause::CandidateRequired)),
        };
        self.finish(op, operands, entry, before, candidate)
    }
}
