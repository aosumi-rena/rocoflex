use crate::source_scalar::{self as scalar, ExplicitExceptionState, SelectedProfilePositiveLog,
    SourceLogEntry, SourceLogPlan, LogArithmeticError, SourceLogArithmeticStep,
    SourceLogArithmeticFailure, SourceLogFailureCapture};
use scalar::selected_log::Error as LogError;
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture,
    ScalarF32InstructionSet, ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome,
    ScalarF32Mxcsr, ScalarF32Error};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

#[derive(Clone, Copy)]
pub enum SourcePitchPostlogBindingCause { Architecture, Controls, Scalar(ScalarF32Error), Log(LogError) }
pub struct SourcePitchPostlogBindingFailure {
    source_word: u32, architecture: ScalarF32Architecture,
    entry: ExplicitExceptionState, scale_bits: u32, cause: SourcePitchPostlogBindingCause,
}
impl SourcePitchPostlogBindingFailure {
    pub fn source_word(&self) -> u32 { self.source_word }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn scale_bits(&self) -> u32 { self.scale_bits }
    pub fn cause(&self) -> SourcePitchPostlogBindingCause { self.cause }
}
#[derive(Clone, Copy)]
pub enum SourcePitchPostlogStage { Add, Log, Multiply }
#[derive(Clone, Copy)]
pub enum SourcePitchPostlogCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 }, ReportedScalar(SourceScalarF32Projection),
    LogRequired(SourceLogEntry), Log(LogArithmeticError),
}
#[derive(Clone, Copy)]
pub struct SourcePitchPostlogAttempt {
    pub stage: SourcePitchPostlogStage,
    pub magnitude_bits: u32, pub epsilon_bits: u32, pub scale_bits: u32,
    pub shifted_bits: Option<u32>, pub logarithm_bits: Option<u32>, pub output_bits: Option<u32>,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub add_reported: Option<ScalarF32Outcome>, pub multiply_reported: Option<ScalarF32Outcome>,
    pub log_entry: Option<SourceLogEntry>, pub last_log_step: Option<SourceLogArithmeticStep>,
    pub log_failure: Option<SourceLogArithmeticFailure>,
}
pub struct SourcePitchPostlogFailure {
    cause: SourcePitchPostlogCause, attempt: SourcePitchPostlogAttempt,
}
impl SourcePitchPostlogFailure {
    pub fn cause(&self) -> SourcePitchPostlogCause { self.cause }
    pub fn attempt(&self) -> &SourcePitchPostlogAttempt { &self.attempt }
}

pub struct SourcePitchPostlogMath {
    source_word: u32, architecture: ScalarF32Architecture, scale_bits: u32,
    state: ExplicitExceptionState, scalar: SourceScalarF32, logarithm: SelectedProfilePositiveLog,
    latest_attempt: Option<SourcePitchPostlogAttempt>, failure: Option<SourcePitchPostlogFailure>,
}
impl SourcePitchPostlogMath {
    pub fn from_explicit_parts(source_word: u32, architecture: ScalarF32Architecture,
        logical_postlog_entry: ExplicitExceptionState, scale_bits: u32)
        -> Result<Self, SourcePitchPostlogBindingFailure>
    {
        let bind = || -> Result<_, SourcePitchPostlogBindingCause> {
            if architecture != ScalarF32Architecture::IntelSdm { return Err(SourcePitchPostlogBindingCause::Architecture); }
            if logical_postlog_entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourcePitchPostlogBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Addss, ScalarF32Opcode::Mulss])
                .map_err(SourcePitchPostlogBindingCause::Scalar)?;
            let scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions)
                .map_err(SourcePitchPostlogBindingCause::Scalar)?;
            let logarithm = SelectedProfilePositiveLog::bind(source_word, logical_postlog_entry.mxcsr())
                .map_err(SourcePitchPostlogBindingCause::Log)?;
            Ok((scalar, logarithm))
        };
        let (scalar, logarithm) = bind().map_err(|cause| SourcePitchPostlogBindingFailure {
            source_word, architecture, entry: logical_postlog_entry, scale_bits, cause })?;
        Ok(Self { source_word, architecture, scale_bits, state: logical_postlog_entry,
            scalar, logarithm, latest_attempt: None, failure: None })
    }
    pub fn source_word(&self) -> u32 { self.source_word }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn scale_bits(&self) -> u32 { self.scale_bits }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn latest_attempt(&self) -> Option<&SourcePitchPostlogAttempt> { self.latest_attempt.as_ref() }
    pub fn failure(&self) -> Option<&SourcePitchPostlogFailure> { self.failure.as_ref() }

    fn binary(&mut self, operation: ScalarF32Operation, attempt: &mut SourcePitchPostlogAttempt,
        multiply: bool) -> Result<u32, SourcePitchPostlogCause>
    {

        let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
            .ok_or(SourcePitchPostlogCause::LogicalState { mxcsr: self.state.mxcsr() })?;
        let reported = self.scalar.execute(operation, before).map_err(SourcePitchPostlogCause::Scalar)?;
        if multiply { attempt.multiply_reported = Some(reported); }
        else { attempt.add_reported = Some(reported); }
        let projection = project_scalar_f32_outcome(self.state, reported);
        match projection {
            SourceScalarF32Projection::Completed { bits, advanced } => {
                self.state = advanced; attempt.known_advanced = advanced; Ok(bits)
            }
            SourceScalarF32Projection::Trapped { advanced, .. } => {
                self.state = advanced; attempt.known_advanced = advanced;
                Err(SourcePitchPostlogCause::ReportedScalar(projection))
            }
            SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                self.state = known_advanced; attempt.known_advanced = known_advanced;
                Err(SourcePitchPostlogCause::ReportedScalar(projection))
            }
        }
    }
    fn run(&mut self, attempt: &mut SourcePitchPostlogAttempt) -> Result<u32, SourcePitchPostlogCause> {
        let shifted = self.binary(ScalarF32Operation::Addss {
            left: attempt.magnitude_bits, right: attempt.epsilon_bits }, attempt, false)?;
        attempt.shifted_bits = Some(shifted);
        attempt.stage = SourcePitchPostlogStage::Log;
        let before = self.state;
        let entry = self.logarithm.source_entry_with_explicit_state(shifted, before);
        attempt.log_entry = Some(entry);
        let logged = match entry.plan {
            SourceLogPlan::CompleteBits { bits } => {
                self.state = entry.exception_state; bits
            }
            SourceLogPlan::MaskedError { .. } => {
                self.state = entry.exception_state;
                attempt.known_advanced = self.state;
                return Err(SourcePitchPostlogCause::LogRequired(entry));
            }
            SourceLogPlan::PositiveFinite { .. } => {
                let mut capture = SourceLogFailureCapture::from_explicit_observer(None);
                let result = self.logarithm.positive_with_observer(shifted, before, self.architecture, &mut capture);
                attempt.last_log_step = capture.last_completed().copied();
                attempt.log_failure = capture.first_failure().copied();
                match result {
                    Ok(result) => { self.state = result.exception_state; result.bits }
                    Err(error) => {
                        self.state = match error {
                            LogArithmeticError::ArithmeticRequired { advanced, .. }
                            | LogArithmeticError::ScalarServiceRequired { advanced, .. }
                            | LogArithmeticError::UnexpectedScalarState { advanced, .. }
                            | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => advanced,
                            LogArithmeticError::Dispatch(_) => before,
                        };
                        if let Some(failure) = attempt.log_failure { self.state = failure.known_advanced; }
                        attempt.known_advanced = self.state;
                        return Err(SourcePitchPostlogCause::Log(error));
                    }
                }
            }
        };
        attempt.known_advanced = self.state;
        attempt.logarithm_bits = Some(logged);
        attempt.stage = SourcePitchPostlogStage::Multiply;
        let result = self.binary(ScalarF32Operation::Mulss { left: logged, right: self.scale_bits }, attempt, true)?;
        attempt.output_bits = Some(result);
        Ok(result)
    }
}
impl super::FramePostlog for SourcePitchPostlogMath {
    type Error = SourcePitchPostlogCause;
    fn apply(&mut self, magnitude: f32, epsilon: f32) -> Result<f32, Self::Error> {
        if let Some(failure) = &self.failure { return Err(failure.cause); }
        let mut attempt = SourcePitchPostlogAttempt {
            stage: SourcePitchPostlogStage::Add, magnitude_bits: magnitude.to_bits(),
            epsilon_bits: epsilon.to_bits(), scale_bits: self.scale_bits,
            shifted_bits: None, logarithm_bits: None, output_bits: None,
            entry: self.state, known_advanced: self.state, add_reported: None, multiply_reported: None,
            log_entry: None, last_log_step: None, log_failure: None,
        };
        let result = self.run(&mut attempt);
        self.latest_attempt = Some(attempt);
        if let Err(cause) = result { self.failure = Some(SourcePitchPostlogFailure { cause, attempt }); }
        result.map(f32::from_bits)
    }
}
