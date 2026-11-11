use crate::{ltv_coefficient_math::ScalarUpper, source_scalar::{self as scalar,
    ExplicitExceptionState, SelectedProfilePositiveLog, SourceExpDispatchReceipt,
    SourceExpDispatchOutcome, SourceExpDispatchError, SourceExpEntry, SourceLogEntry,
    SourceLogPlan, LogArithmeticError, SourceLogArithmeticStep, SourceLogArithmeticFailure,
    SourceLogFailureCapture}};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error,
    ScalarF32Outcome, ScalarF32SelectionOutcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection,
    project_selection_f32_outcome, SourceSelectionF32Projection};

#[derive(Clone, Copy)]
pub struct SourceJoinConfiguration {
    pub exp_source_word: u32, pub log_source_word: u32, pub architecture: ScalarF32Architecture,
    pub pitch_divisor_bits: u32, pub pitch_multiplier_bits: u32, pub pitch_offset_bits: u32,
    pub energy_floor_bits: u32,
}
#[derive(Clone, Copy)]
pub enum SourceJoinBindingCause { Architecture, Controls, Scalar(ScalarF32Error), Log(scalar::Error) }
pub struct SourceJoinMathBindingFailure<U: ScalarUpper> {
    configuration: SourceJoinConfiguration, state: ExplicitExceptionState, upper: U, cause: SourceJoinBindingCause,
}
impl<U: ScalarUpper> SourceJoinMathBindingFailure<U> {
    pub fn configuration(&self) -> SourceJoinConfiguration { self.configuration }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn upper_supplier(&self) -> &U { &self.upper }
    pub fn cause(&self) -> SourceJoinBindingCause { self.cause }
}
#[derive(Clone, Copy)]
pub enum SourceJoinSite {
    PitchExp, PitchDivide, PitchLog, PitchMultiply, PitchOffset,
    EnergyExp, EnergySum, EnergyCount, EnergyDivide, EnergyMaximum, EnergyLog,
}
#[derive(Clone, Copy)]
pub struct SourceJoinMathStopped;
pub enum SourceJoinMathCause<E> {
    Upper(E), Exp(SourceExpDispatchError), ExpRangeRequired(SourceExpEntry),
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), ReportedSelection(SourceSelectionF32Projection),
    Divide(LogArithmeticError), Log(LogArithmeticError), LogRequired(SourceLogEntry),
}
#[derive(Clone, Copy)]
pub struct SourceJoinMathAttempt {
    pub site: SourceJoinSite, pub left_bits: u32, pub right_bits: Option<u32>, pub integer: Option<i32>,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub result_bits: Option<u32>, pub upper_bits: Option<u64>,
    pub scalar_reported: Option<ScalarF32Outcome>, pub selection_reported: Option<ScalarF32SelectionOutcome>,
    pub log_entry: Option<SourceLogEntry>, pub last_log_step: Option<SourceLogArithmeticStep>,
    pub log_failure: Option<SourceLogArithmeticFailure>,
}
pub struct SourceJoinMathLeafFailure<E> {
    cause: SourceJoinMathCause<E>, attempt: SourceJoinMathAttempt,
}
impl<E> SourceJoinMathLeafFailure<E> {
    pub fn cause(&self) -> &SourceJoinMathCause<E> { &self.cause }
    pub fn attempt(&self) -> &SourceJoinMathAttempt { &self.attempt }
}

pub struct SourceJoinMath<U: ScalarUpper> {
    configuration: SourceJoinConfiguration, state: ExplicitExceptionState, upper: U,
    scalar: SourceScalarF32, logarithm: SelectedProfilePositiveLog,
    latest_attempt: Option<SourceJoinMathAttempt>, latest_exp_receipt: Option<SourceExpDispatchReceipt>,
    failure: Option<SourceJoinMathLeafFailure<U::Error>>,
}
impl<U: ScalarUpper> SourceJoinMath<U> {
    pub fn from_explicit_parts(configuration: SourceJoinConfiguration, logical_entry: ExplicitExceptionState, upper: U)
        -> Result<Self, SourceJoinMathBindingFailure<U>>
    {
        let bind = || -> Result<_, SourceJoinBindingCause> {
            if configuration.architecture != ScalarF32Architecture::IntelSdm { return Err(SourceJoinBindingCause::Architecture); }
            if logical_entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceJoinBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
                ScalarF32Opcode::Addss, ScalarF32Opcode::Mulss, ScalarF32Opcode::Cvtsi2ssSigned32, ScalarF32Opcode::Maxss])
                .map_err(SourceJoinBindingCause::Scalar)?;
            let scalar = SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceJoinBindingCause::Scalar)?;
            let logarithm = SelectedProfilePositiveLog::bind(configuration.log_source_word, logical_entry.mxcsr())
                .map_err(SourceJoinBindingCause::Log)?;
            Ok((scalar, logarithm))
        };
        let (scalar, logarithm) = match bind() {
            Ok(value) => value,
            Err(cause) => return Err(SourceJoinMathBindingFailure { configuration, state: logical_entry, upper, cause }),
        };
        Ok(Self { configuration, state: logical_entry, upper, scalar, logarithm,
            latest_attempt: None, latest_exp_receipt: None, failure: None })
    }
    pub fn configuration(&self) -> SourceJoinConfiguration { self.configuration }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn upper_supplier(&self) -> &U { &self.upper }
    pub fn latest_attempt(&self) -> Option<&SourceJoinMathAttempt> { self.latest_attempt.as_ref() }

    pub fn latest_exp_receipt(&self) -> Option<&SourceExpDispatchReceipt> { self.latest_exp_receipt.as_ref() }
    pub fn failure(&self) -> Option<&SourceJoinMathLeafFailure<U::Error>> { self.failure.as_ref() }
    fn perform(&mut self, site: SourceJoinSite, left_bits: u32, right_bits: Option<u32>, integer: Option<i32>,
        operation: impl FnOnce(&mut Self, &mut SourceJoinMathAttempt) -> Result<u32, SourceJoinMathCause<U::Error>>)
        -> Result<f32, SourceJoinMathStopped>
    {
        if self.failure.is_some() { return Err(SourceJoinMathStopped); }
        let mut attempt = SourceJoinMathAttempt { site, left_bits, right_bits, integer,
            entry: self.state, known_advanced: self.state, result_bits: None, upper_bits: None,
            scalar_reported: None, selection_reported: None, log_entry: None, last_log_step: None, log_failure: None };
        let result = operation(self, &mut attempt);
        attempt.known_advanced = self.state;
        match result {
            Ok(bits) => { attempt.result_bits = Some(bits); self.latest_attempt = Some(attempt); Ok(f32::from_bits(bits)) }
            Err(cause) => {
                self.latest_attempt = Some(attempt);
                self.failure = Some(SourceJoinMathLeafFailure { cause, attempt });
                Err(SourceJoinMathStopped)
            }
        }
    }
    fn scalar_once(&mut self, operation: ScalarF32Operation, attempt: &mut SourceJoinMathAttempt)
        -> Result<u32, SourceJoinMathCause<U::Error>>
    {
        let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
            .ok_or(SourceJoinMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
        let reported = self.scalar.execute(operation, before).map_err(SourceJoinMathCause::Scalar)?;
        attempt.scalar_reported = Some(reported);
        let projection = project_scalar_f32_outcome(self.state, reported);
        match projection {
            SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
            SourceScalarF32Projection::Trapped { advanced, .. } => { self.state = advanced; Err(SourceJoinMathCause::ReportedScalar(projection)) }
            SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                self.state = known_advanced; Err(SourceJoinMathCause::ReportedScalar(projection))
            }
        }
    }
    fn log_once(&mut self, input: u32, attempt: &mut SourceJoinMathAttempt)
        -> Result<u32, SourceJoinMathCause<U::Error>>
    {
        let before = self.state;
        let entry = self.logarithm.source_entry_with_explicit_state(input, before);
        attempt.log_entry = Some(entry);
        match entry.plan {
            SourceLogPlan::CompleteBits { bits } => { self.state = entry.exception_state; Ok(bits) }
            SourceLogPlan::MaskedError { .. } => { self.state = entry.exception_state; Err(SourceJoinMathCause::LogRequired(entry)) }
            SourceLogPlan::PositiveFinite { .. } => {
                let mut capture = SourceLogFailureCapture::from_explicit_observer(None);
                let result = self.logarithm.positive_with_observer(input, before, self.configuration.architecture, &mut capture);
                attempt.last_log_step = capture.last_completed().copied();
                attempt.log_failure = capture.first_failure().copied();
                match result {
                    Ok(result) => { self.state = result.exception_state; Ok(result.bits) }
                    Err(error) => {
                        self.state = log_advanced(error, before);
                        if let Some(failure) = attempt.log_failure { self.state = failure.known_advanced; }
                        Err(SourceJoinMathCause::Log(error))
                    }
                }
            }
        }
    }
}
fn log_advanced(error: LogArithmeticError, entry: ExplicitExceptionState) -> ExplicitExceptionState {
    match error {
        LogArithmeticError::ArithmeticRequired { advanced, .. } | LogArithmeticError::ScalarServiceRequired { advanced, .. }
        | LogArithmeticError::UnexpectedScalarState { advanced, .. } | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => advanced,
        LogArithmeticError::Dispatch(scalar::Error::CallbackRequiredFromEntry { entry }) => entry.exception_state,
        LogArithmeticError::Dispatch(_) => entry,
    }
}
impl<U: ScalarUpper> super::JoinMath for SourceJoinMath<U> {
    type Error = SourceJoinMathStopped;
    fn pitch_divisor(&self) -> f32 { f32::from_bits(self.configuration.pitch_divisor_bits) }
    fn pitch_multiplier(&self) -> f32 { f32::from_bits(self.configuration.pitch_multiplier_bits) }
    fn pitch_offset(&self) -> f32 { f32::from_bits(self.configuration.pitch_offset_bits) }
    fn energy_floor(&self) -> f32 { f32::from_bits(self.configuration.energy_floor_bits) }
    fn exp(&mut self, site: SourceJoinSite, input: f32, energy: bool) -> Result<f32, Self::Error> {
        self.perform(site, input.to_bits(), None, None, |this, attempt| {
            this.latest_exp_receipt = None;

            let upper = if energy { this.upper.original_upper_f64(input.to_bits()).map_err(SourceJoinMathCause::Upper)? } else { 0 };
            attempt.upper_bits = Some(upper);
            let receipt = scalar::selected_exp_with_explicit_upper_lane_receipt(this.configuration.exp_source_word,
                this.configuration.architecture, input.to_bits(), upper, this.state);
            let outcome = receipt.outcome.clone();
            this.state = receipt.known_advanced;
            this.latest_exp_receipt = Some(receipt);
            match outcome {
                Ok(SourceExpDispatchOutcome::Complete(result)) => Ok(result.bits),
                Ok(SourceExpDispatchOutcome::RangeRequired(entry)) => Err(SourceJoinMathCause::ExpRangeRequired(entry)),
                Err(error) => Err(SourceJoinMathCause::Exp(error)),
            }
        })
    }
    fn divide(&mut self, site: SourceJoinSite, left: f32, right: f32) -> Result<f32, Self::Error> {
        self.perform(site, left.to_bits(), Some(right.to_bits()), None, |this, _| {
            match scalar::raw_divss_with_explicit_state(left.to_bits(), right.to_bits(), this.state, this.configuration.architecture) {
                Ok(result) => { this.state = result.exception_state; Ok(result.bits) }
                Err(error) => { this.state = log_advanced(error, this.state); Err(SourceJoinMathCause::Divide(error)) }
            }
        })
    }
    fn log(&mut self, site: SourceJoinSite, input: f32) -> Result<f32, Self::Error> {
        self.perform(site, input.to_bits(), None, None, |this, attempt| this.log_once(input.to_bits(), attempt))
    }
    fn multiply(&mut self, site: SourceJoinSite, left: f32, right: f32) -> Result<f32, Self::Error> {
        self.perform(site, left.to_bits(), Some(right.to_bits()), None, |this, attempt|
            this.scalar_once(ScalarF32Operation::Mulss { left: left.to_bits(), right: right.to_bits() }, attempt))
    }
    fn add(&mut self, site: SourceJoinSite, left: f32, right: f32) -> Result<f32, Self::Error> {
        self.perform(site, left.to_bits(), Some(right.to_bits()), None, |this, attempt|
            this.scalar_once(ScalarF32Operation::Addss { left: left.to_bits(), right: right.to_bits() }, attempt))
    }
    fn count(&mut self, value: i32) -> Result<f32, Self::Error> {
        self.perform(SourceJoinSite::EnergyCount, value as u32, None, Some(value), |this, attempt|
            this.scalar_once(ScalarF32Operation::Cvtsi2ssSigned32 { value }, attempt))
    }
    fn maximum(&mut self, left: f32, right: f32) -> Result<f32, Self::Error> {
        self.perform(SourceJoinSite::EnergyMaximum, left.to_bits(), Some(right.to_bits()), None, |this, attempt| {
            let before = ScalarF32Mxcsr::from_explicit_word(this.state.mxcsr())
                .ok_or(SourceJoinMathCause::LogicalState { mxcsr: this.state.mxcsr() })?;
            let reported = this.scalar.execute_maxss(left.to_bits(), right.to_bits(), before).map_err(SourceJoinMathCause::Scalar)?;
            attempt.selection_reported = Some(reported);
            let projection = project_selection_f32_outcome(this.state, reported);
            match projection {
                SourceSelectionF32Projection::Completed { bits, advanced } => { this.state = advanced; Ok(bits) }
                SourceSelectionF32Projection::Trapped { advanced, .. } | SourceSelectionF32Projection::Required { advanced, .. } => {
                    this.state = advanced; Err(SourceJoinMathCause::ReportedSelection(projection))
                }
                SourceSelectionF32Projection::ReportedStateRequired { known_advanced, .. } => {
                    this.state = known_advanced; Err(SourceJoinMathCause::ReportedSelection(projection))
                }
            }
        })
    }
}
