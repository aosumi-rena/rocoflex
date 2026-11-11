use super::coefficient_ordinary::{run_coefficient_ordinary, SourceCoefficientBinaryOpcode,
    SourceCoefficientComparisonFlags, SourceCoefficientExpOperands,
    SourceCoefficientOrdinaryArguments, SourceCoefficientOrdinaryDomainError,
    SourceCoefficientOrdinaryFailure, SourceCoefficientOrdinaryServices};
use super::finite_power::{compare_f32_with_explicit_state, CoefficientScaleResult,
    CoefficientSourceExpBoundary, ExplicitExceptionState, FinitePowerServices,
    PowerError, SourceF32CompareOpcode};
use super::scalar_truncate::{truncate_f32_to_i32_with_explicit_state, SourceScalarTruncationError};
use super::selected_log::{raw_divss_with_explicit_state, LogArithmeticError};
use super::logical_f32_outcome::{project_scalar_f32_outcome, project_xmm_f32_outcome,
    SourceScalarF32Projection, SourceXmmF32Projection};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Error,
    ScalarF32InstructionSet, ScalarF32Mxcsr, ScalarF32Opcode, ScalarF32Operation,
    ScalarF32Outcome, SourceScalarF32, XmmF32Outcome};

#[derive(Clone, Copy, Debug)]
pub enum SourceCoefficientReportedOutcome {
    Scalar { site: u32, instruction: ScalarF32Operation, outcome: ScalarF32Outcome },
    Cvtdq2psXmm { site: u32, source: [i32; 4], outcome: XmmF32Outcome },
}

#[derive(Clone, Debug)]
pub enum SourceCoefficientPrimitiveError {
    SharedCore { error: ScalarF32Error, advanced: ExplicitExceptionState },
    LogicalStateRequired { reported_mxcsr: u32, advanced: ExplicitExceptionState },
    ReportedOutcomeRequired { reported: SourceCoefficientReportedOutcome, reason: PowerError,
        advanced: ExplicitExceptionState },
    TrapRequired { reported_mxcsr: u32, unmasked: u8, advanced: ExplicitExceptionState },
    Divide(LogArithmeticError),
    Truncation(SourceScalarTruncationError),
    Scale(PowerError),
}

#[derive(Clone, Debug)]
pub enum SourceCoefficientScaledExpCause {
    ArchitectureNotClosed { supplied: ScalarF32Architecture },
    UnsupportedEnvironment { mxcsr: u32 },
    Primitive(SourceCoefficientPrimitiveError),
    Power(PowerError),
    OrdinaryDomain(SourceCoefficientOrdinaryDomainError),
    Ordinary(SourceCoefficientOrdinaryFailure<SourceCoefficientPrimitiveError>),
}

#[derive(Clone, Debug)]
pub struct SourceCoefficientScaledExpError {
    pub cause: SourceCoefficientScaledExpCause,

    pub operands: Option<SourceCoefficientExpOperands>,
    pub overflow_word_8360d0: u32,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
}

impl SourceCoefficientScaledExpError {
    pub fn reason(&self) -> &'static str {
        match &self.cause {
            SourceCoefficientScaledExpCause::ArchitectureNotClosed { .. } =>
                "562834 requires explicit Intel instruction semantics",
            SourceCoefficientScaledExpCause::UnsupportedEnvironment { .. } =>
                "562834 requires RNE/all-masked/gradual logical controls",
            SourceCoefficientScaledExpCause::Primitive(_) =>
                "562834 canonical scalar primitive admission is required",
            SourceCoefficientScaledExpCause::Power(_) =>
                "562834 existing source boundary or integer scale requirement",
            SourceCoefficientScaledExpCause::OrdinaryDomain(_) =>
                "562834 remaining ordinary operand domain is required",
            SourceCoefficientScaledExpCause::Ordinary(_) =>
                "562834 ordinary operation requires its retained typed continuation",
        }
    }
    pub fn known_advanced_state(&self, _entry: ExplicitExceptionState) -> ExplicitExceptionState {
        self.advanced
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCoefficientScaledExpCompletion {
    pub operands: SourceCoefficientExpOperands,
    pub overflow_word_8360d0: u32,
    pub result: CoefficientScaleResult,
    pub entry: ExplicitExceptionState,
    pub exception_state: ExplicitExceptionState,
}

pub struct SourceCoefficientScaledExp562834 {
    scalar: SourceScalarF32,
    power: FinitePowerServices,
    state: ExplicitExceptionState,
    overflow_word_8360d0: u32,
    failure: Option<SourceCoefficientScaledExpError>,
    last_completion: Option<SourceCoefficientScaledExpCompletion>,
}

impl SourceCoefficientScaledExp562834 {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState,
        overflow_word_8360d0: u32) -> Result<Self, SourceCoefficientScaledExpError>
    {
        let error = |cause| SourceCoefficientScaledExpError { cause, operands: None,
            overflow_word_8360d0, entry: state, advanced: state };
        if architecture != ScalarF32Architecture::IntelSdm {
            return Err(error(SourceCoefficientScaledExpCause::ArchitectureNotClosed { supplied: architecture }));
        }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(error(SourceCoefficientScaledExpCause::UnsupportedEnvironment { mxcsr: state.mxcsr() }));
        }
        let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
            ScalarF32Opcode::Addss, ScalarF32Opcode::Subss, ScalarF32Opcode::Mulss,
            ScalarF32Opcode::Cvtdq2psXmm,
        ]).map_err(|cause| error(SourceCoefficientScaledExpCause::Primitive(
            SourceCoefficientPrimitiveError::SharedCore { error: cause, advanced: state })))?;
        let scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions)
            .map_err(|cause| error(SourceCoefficientScaledExpCause::Primitive(
                SourceCoefficientPrimitiveError::SharedCore { error: cause, advanced: state })))?;
        let power = FinitePowerServices::bind(state.mxcsr())
            .map_err(|cause| error(SourceCoefficientScaledExpCause::Power(cause)))?;
        Ok(Self { scalar, power, state, overflow_word_8360d0,
            failure: None, last_completion: None })
    }

    pub fn name(&self) -> &'static str { "source562834/explicit-state/ordinary+integer-scale" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.scalar.architecture() }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn overflow_word_8360d0(&self) -> u32 { self.overflow_word_8360d0 }
    pub fn failure(&self) -> Option<&SourceCoefficientScaledExpError> { self.failure.as_ref() }
    pub fn last_completion(&self) -> Option<&SourceCoefficientScaledExpCompletion> { self.last_completion.as_ref() }

    pub fn evaluate_bits(&mut self, input_bits: u32, multiplier_bits: u32, exponent2: i16)
        -> Result<CoefficientScaleResult, SourceCoefficientScaledExpError>
    {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let entry = self.state;
        let operands = SourceCoefficientExpOperands { input_bits, multiplier_bits, exponent2 };
        let result = self.evaluate_once(operands);
        match result {
            Ok(result) => {
                self.last_completion = Some(SourceCoefficientScaledExpCompletion { operands,
                    overflow_word_8360d0: self.overflow_word_8360d0, result, entry,
                    exception_state: self.state });
                Ok(result)
            }
            Err(cause) => {
                let error = SourceCoefficientScaledExpError { cause, operands: Some(operands),
                    overflow_word_8360d0: self.overflow_word_8360d0, entry, advanced: self.state };
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }

    fn evaluate_once(&mut self, operands: SourceCoefficientExpOperands)
        -> Result<CoefficientScaleResult, SourceCoefficientScaledExpCause>
    {

        let entry = self.power.coefficient_scaled_exp_source_entry(operands.input_bits,
            operands.multiplier_bits, operands.exponent2, self.overflow_word_8360d0, self.state);
        self.state = entry.exception_state;
        let boundary = self.power.coefficient_source_boundary_from_entry(entry, self.overflow_word_8360d0)
            .map_err(SourceCoefficientScaledExpCause::Power)?;
        self.state = boundary.exception_state;
        if let CoefficientSourceExpBoundary::Complete(result) = boundary.boundary { return Ok(result); }
        let arguments = SourceCoefficientOrdinaryArguments::from_finite_bits(
            operands.input_bits, operands.multiplier_bits, operands.exponent2)
            .map_err(SourceCoefficientScaledExpCause::OrdinaryDomain)?;
        let architecture = self.scalar.architecture();
        let mut services = StateServices { scalar: &mut self.scalar, power: &self.power,
            state: &mut self.state, architecture, overflow_word: self.overflow_word_8360d0 };
        run_coefficient_ordinary(&mut services, arguments)
            .map_err(SourceCoefficientScaledExpCause::Ordinary)
    }
}

struct StateServices<'a> {
    scalar: &'a mut SourceScalarF32,
    power: &'a FinitePowerServices,
    state: &'a mut ExplicitExceptionState,
    architecture: ScalarF32Architecture,
    overflow_word: u32,
}

impl StateServices<'_> {
    fn scalar_state(&self) -> Result<ScalarF32Mxcsr, SourceCoefficientPrimitiveError> {
        ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr()).ok_or(
            SourceCoefficientPrimitiveError::LogicalStateRequired {
                reported_mxcsr: self.state.mxcsr(), advanced: *self.state })
    }
}

impl SourceCoefficientOrdinaryServices for StateServices<'_> {
    type Error = SourceCoefficientPrimitiveError;

    fn binary(&mut self, site: u32, opcode: SourceCoefficientBinaryOpcode,
        left: u32, right: u32) -> Result<u32, Self::Error>
    {
        let operation = match opcode {
            SourceCoefficientBinaryOpcode::Divss => return match raw_divss_with_explicit_state(left, right, *self.state, self.architecture) {
                Ok(result) => { *self.state = result.exception_state; Ok(result.bits) }
                Err(error) => {
                    let advanced = match &error {
                        LogArithmeticError::ArithmeticRequired { advanced, .. }
                        | LogArithmeticError::ScalarServiceRequired { advanced, .. }
                        | LogArithmeticError::UnexpectedScalarState { advanced, .. }
                        | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => *advanced,
                        LogArithmeticError::Dispatch(_) => *self.state,
                    };
                    *self.state = advanced;
                    Err(SourceCoefficientPrimitiveError::Divide(error))
                }
            },
            SourceCoefficientBinaryOpcode::Addss => ScalarF32Operation::Addss { left, right },
            SourceCoefficientBinaryOpcode::Subss => ScalarF32Operation::Subss { left, right },
            SourceCoefficientBinaryOpcode::Mulss => ScalarF32Operation::Mulss { left, right },
        };
        let before = self.scalar_state()?;
        let outcome = self.scalar.execute(operation, before).map_err(|error|
            SourceCoefficientPrimitiveError::SharedCore { error, advanced: *self.state })?;
        match project_scalar_f32_outcome(*self.state, outcome) {
            SourceScalarF32Projection::Completed { bits, advanced } => { *self.state = advanced; Ok(bits) }
            SourceScalarF32Projection::Trapped { reported_mxcsr, unmasked, advanced } => {
                *self.state = advanced;
                Err(SourceCoefficientPrimitiveError::TrapRequired {
                    reported_mxcsr, unmasked, advanced })
            }
            SourceScalarF32Projection::ReportedStateRequired { reported, reason, known_advanced } => {
                *self.state = known_advanced;
                Err(SourceCoefficientPrimitiveError::ReportedOutcomeRequired {
                    reported: SourceCoefficientReportedOutcome::Scalar { site, instruction: operation, outcome: reported },
                    reason, advanced: known_advanced })
            }
        }
    }

    fn comiss(&mut self, _site: u32, left: u32, right: u32)
        -> Result<SourceCoefficientComparisonFlags, Self::Error>
    {
        let result = compare_f32_with_explicit_state(SourceF32CompareOpcode::Comiss, left, right, *self.state);
        *self.state = result.exception_state;
        Ok(SourceCoefficientComparisonFlags { carry: result.carry, parity: result.parity, zero: result.zero })
    }

    fn cvttss2si_i32(&mut self, _site: u32, source: u32) -> Result<u32, Self::Error> {
        match truncate_f32_to_i32_with_explicit_state(source, *self.state, self.architecture) {
            Ok(result) => { *self.state = result.exception_state; Ok(result.low_dword) }
            Err(error) => {
                *self.state = error.known_advanced_state(*self.state);
                Err(SourceCoefficientPrimitiveError::Truncation(error))
            }
        }
    }

    fn cvtdq2ps_xmm(&mut self, site: u32, source: [i32; 4]) -> Result<[u32; 4], Self::Error> {
        let before = self.scalar_state()?;
        let outcome = self.scalar.execute_cvtdq2ps_xmm(source, before).map_err(|error|
            SourceCoefficientPrimitiveError::SharedCore { error, advanced: *self.state })?;
        match project_xmm_f32_outcome(*self.state, outcome) {
            SourceXmmF32Projection::Completed { bits, advanced } => { *self.state = advanced; Ok(bits) }
            SourceXmmF32Projection::Trapped { reported_mxcsr, unmasked, advanced } => {
                *self.state = advanced;
                Err(SourceCoefficientPrimitiveError::TrapRequired {
                    reported_mxcsr, unmasked, advanced })
            }
            SourceXmmF32Projection::ReportedStateRequired { reported, reason, known_advanced } => {
                *self.state = known_advanced;
                Err(SourceCoefficientPrimitiveError::ReportedOutcomeRequired {
                    reported: SourceCoefficientReportedOutcome::Cvtdq2psXmm { site, source, outcome: reported },
                    reason, advanced: known_advanced })
            }
        }
    }

    fn scale_562bf0(&mut self, _site: u32, bits: u32, shift: i32)
        -> Result<CoefficientScaleResult, Self::Error>
    {

        self.power.coefficient_scale_with_explicit_overflow_word(bits, shift, self.overflow_word)
            .map_err(SourceCoefficientPrimitiveError::Scale)
    }
}
