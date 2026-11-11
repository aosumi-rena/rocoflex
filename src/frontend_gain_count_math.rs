use crate::source_scalar::{self as scalar, ExplicitExceptionState, LogArithmeticError, SourceLogWithState,
    SourceF32I32Truncation, SourceScalarTruncationError};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error, ScalarF32Outcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

#[derive(Clone, Copy, Debug)]
pub struct SourceGainCountRequest {

    pub source_count_dword: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGainCountConfiguration {
    pub architecture: ScalarF32Architecture,

    pub divisor_bits: u32,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainCountBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceGainCountBindingFailure {
    configuration: SourceGainCountConfiguration, entry: ExplicitExceptionState, cause: SourceGainCountBindingCause,
}
impl SourceGainCountBindingFailure {
    pub fn configuration(&self) -> SourceGainCountConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceGainCountBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainCountOperation {
    Convert { signed_input: i32 }, Divide { left: u32, right: u32 }, Truncate { bits: u32 },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainCountPacket {
    Scalar(Result<ScalarF32Outcome, ScalarF32Error>),
    Divide(Result<SourceLogWithState, LogArithmeticError>),
    Truncate(Result<SourceF32I32Truncation, SourceScalarTruncationError>),
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainCountValue { FloatBits(u32), RawCount(SourceF32I32Truncation) }
#[derive(Clone, Copy, Debug)]
pub struct SourceGainCountAttempt {
    pub pc: u32, pub operation: SourceGainCountOperation,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub reported: Option<SourceGainCountPacket>, pub scalar_projection: Option<SourceScalarF32Projection>,
    pub result: Option<SourceGainCountValue>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainCountMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 }, ReportedScalar(SourceScalarF32Projection),
    Divide(LogArithmeticError), Truncate(SourceScalarTruncationError),
}
pub struct SourceGainCountMathFailure { cause: SourceGainCountMathCause, attempt: SourceGainCountAttempt }
impl SourceGainCountMathFailure {
    pub fn cause(&self) -> SourceGainCountMathCause { self.cause }
    pub fn attempt(&self) -> &SourceGainCountAttempt { &self.attempt }
}
pub struct SourceGainCountMath {
    configuration: SourceGainCountConfiguration, entry: ExplicitExceptionState,
    state: ExplicitExceptionState, scalar: SourceScalarF32,
    reports: [Option<SourceGainCountAttempt>; 3], first_failure: Option<SourceGainCountMathFailure>,
}
impl SourceGainCountMath {
    pub fn from_explicit_parts(configuration: SourceGainCountConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceGainCountBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm {
                return Err(SourceGainCountBindingCause::Architecture);
            }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceGainCountBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Cvtsi2ssSigned32])
                .map_err(SourceGainCountBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceGainCountBindingCause::Scalar)
        };
        match bind() {
            Ok(backend) => Ok(Self { configuration, entry, state: entry, scalar: backend,
                reports: [None; 3], first_failure: None }),
            Err(cause) => Err(SourceGainCountBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceGainCountConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn reports(&self) -> &[Option<SourceGainCountAttempt>; 3] { &self.reports }
    pub fn first_failure(&self) -> Option<&SourceGainCountMathFailure> { self.first_failure.as_ref() }
    fn perform(&mut self, slot: usize, pc: u32, operation: SourceGainCountOperation)
        -> Result<SourceGainCountValue, Stopped>
    {
        if self.first_failure.is_some() { return Err(Stopped); }
        let mut attempt = SourceGainCountAttempt { pc, operation,
            entry: self.state, known_advanced: self.state, reported: None, scalar_projection: None, result: None };
        self.reports[slot] = Some(attempt);
        let result = (|| {
            match operation {
                SourceGainCountOperation::Convert { signed_input } => {
                    let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                        .ok_or(SourceGainCountMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
                    let reported = self.scalar.execute(ScalarF32Operation::Cvtsi2ssSigned32 { value: signed_input }, before);
                    attempt.reported = Some(SourceGainCountPacket::Scalar(reported));
                    self.reports[slot] = Some(attempt);
                    let reported = reported.map_err(SourceGainCountMathCause::Scalar)?;
                    let projection = project_scalar_f32_outcome(self.state, reported);
                    attempt.scalar_projection = Some(projection);
                    self.reports[slot] = Some(attempt);
                    match projection {
                        SourceScalarF32Projection::Completed { bits, advanced } => {
                            self.state = advanced; Ok(SourceGainCountValue::FloatBits(bits))
                        }
                        SourceScalarF32Projection::Trapped { advanced, .. } => {
                            self.state = advanced; Err(SourceGainCountMathCause::ReportedScalar(projection))
                        }
                        SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                            self.state = known_advanced; Err(SourceGainCountMathCause::ReportedScalar(projection))
                        }
                    }
                }
                SourceGainCountOperation::Divide { left, right } => {
                    let reported = scalar::raw_divss_with_explicit_state(left, right, self.state, self.configuration.architecture);
                    attempt.reported = Some(SourceGainCountPacket::Divide(reported));
                    self.reports[slot] = Some(attempt);
                    match reported {
                        Ok(value) => { self.state = value.exception_state; Ok(SourceGainCountValue::FloatBits(value.bits)) }
                        Err(error) => {
                            self.state = divide_advanced(error, self.state);
                            Err(SourceGainCountMathCause::Divide(error))
                        }
                    }
                }
                SourceGainCountOperation::Truncate { bits } => {
                    let reported = scalar::truncate_f32_to_i32_with_explicit_state(bits, self.state, self.configuration.architecture);
                    attempt.reported = Some(SourceGainCountPacket::Truncate(reported));
                    self.reports[slot] = Some(attempt);
                    match reported {
                        Ok(value) => {
                            self.state = value.exception_state;
                            Ok(SourceGainCountValue::RawCount(value))
                        }
                        Err(error) => {
                            self.state = error.known_advanced_state(self.state);
                            Err(SourceGainCountMathCause::Truncate(error))
                        }
                    }
                }
            }
        })();
        attempt.known_advanced = self.state;
        match result {
            Ok(value) => { attempt.result = Some(value); self.reports[slot] = Some(attempt); Ok(value) }
            Err(cause) => {
                self.reports[slot] = Some(attempt);
                self.first_failure = Some(SourceGainCountMathFailure { cause, attempt });
                Err(Stopped)
            }
        }
    }
}
fn divide_advanced(error: LogArithmeticError, entry: ExplicitExceptionState) -> ExplicitExceptionState {
    match error {
        LogArithmeticError::ArithmeticRequired { advanced, .. }
        | LogArithmeticError::ScalarServiceRequired { advanced, .. }
        | LogArithmeticError::UnexpectedScalarState { advanced, .. }
        | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => advanced,
        LogArithmeticError::Dispatch(scalar::Error::CallbackRequiredFromEntry { entry }) => entry.exception_state,
        LogArithmeticError::Dispatch(_) => entry,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGainCountStage { Subtract, Convert, Divide, Truncate, Select, AllocationExtent, RetainRawCount, Complete }
pub struct SourceGainCountProgress {
    stage: SourceGainCountStage, next_pc: u32, wrapped_input_dword: Option<u32>,
    converted_bits: Option<u32>, divided_bits: Option<u32>, truncation: Option<SourceF32I32Truncation>,
    raw_selector_dword: Option<u32>, selected_count: Option<u32>, selected_allocation_bytes: Option<u64>,
}
impl SourceGainCountProgress {
    fn new() -> Self {
        Self { stage: SourceGainCountStage::Subtract, next_pc: 0x552659, wrapped_input_dword: None,
            converted_bits: None, divided_bits: None, truncation: None, raw_selector_dword: None,
            selected_count: None, selected_allocation_bytes: None }
    }
    pub fn stage(&self) -> SourceGainCountStage { self.stage }
    pub fn next_pc(&self) -> u32 { self.next_pc }
    pub fn wrapped_input_dword(&self) -> Option<u32> { self.wrapped_input_dword }
    pub fn converted_bits(&self) -> Option<u32> { self.converted_bits }
    pub fn divided_bits(&self) -> Option<u32> { self.divided_bits }
    pub fn truncation(&self) -> Option<SourceF32I32Truncation> { self.truncation }
    pub fn raw_selector_dword(&self) -> Option<u32> { self.raw_selector_dword }
    pub fn selected_count(&self) -> Option<u32> { self.selected_count }
    pub fn selected_allocation_bytes(&self) -> Option<u64> { self.selected_allocation_bytes }
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGainCountObservation {
    pub wrapped_input_dword: u32, pub converted_bits: u32, pub divided_bits: u32,
    pub truncation: SourceF32I32Truncation,

    pub raw_selector_dword: u32,
    pub selected_count: u32,

    pub selected_allocation_bytes: u64,
}
impl SourceGainCountObservation {
    pub fn raw_signed_count(self) -> i32 { self.raw_selector_dword as i32 }
}

pub struct SourceGainCountFailure {
    request: SourceGainCountRequest, math: SourceGainCountMath, progress: SourceGainCountProgress,
}
impl SourceGainCountFailure {
    pub fn request(&self) -> SourceGainCountRequest { self.request }
    pub fn math(&self) -> &SourceGainCountMath { &self.math }
    pub fn progress(&self) -> &SourceGainCountProgress { &self.progress }
}
pub struct SourceGainCountCompleted {
    request: SourceGainCountRequest, math: SourceGainCountMath, progress: SourceGainCountProgress,
    observation: SourceGainCountObservation,
}
pub struct SourceGainCountParts {
    pub request: SourceGainCountRequest, pub math: SourceGainCountMath,
    pub progress: SourceGainCountProgress, pub observation: SourceGainCountObservation,
}
impl SourceGainCountCompleted {
    pub fn request(&self) -> SourceGainCountRequest { self.request }
    pub fn math(&self) -> &SourceGainCountMath { &self.math }
    pub fn progress(&self) -> &SourceGainCountProgress { &self.progress }
    pub fn observation(&self) -> SourceGainCountObservation { self.observation }
    pub fn into_parts(self) -> SourceGainCountParts {
        SourceGainCountParts { request: self.request, math: self.math, progress: self.progress, observation: self.observation }
    }
}
pub fn count_source_windows(request: SourceGainCountRequest, mut math: SourceGainCountMath)
    -> Result<SourceGainCountCompleted, SourceGainCountFailure>
{
    let mut progress = SourceGainCountProgress::new();

    if math.first_failure.is_none() { math.reports = [None; 3]; }
    match count_graph(request, &mut math, &mut progress) {
        Ok(observation) => Ok(SourceGainCountCompleted { request, math, progress, observation }),
        Err(Stopped) => Err(SourceGainCountFailure { request, math, progress }),
    }
}
struct Stopped;
fn count_graph(request: SourceGainCountRequest, math: &mut SourceGainCountMath, progress: &mut SourceGainCountProgress)
    -> Result<SourceGainCountObservation, Stopped>
{
    let wrapped = request.source_count_dword.wrapping_sub(0x780);
    progress.wrapped_input_dword = Some(wrapped);
    progress.stage = SourceGainCountStage::Convert;
    progress.next_pc = 0x55265f;
    let converted = match math.perform(0, 0x55265f, SourceGainCountOperation::Convert { signed_input: wrapped as i32 })? {
        SourceGainCountValue::FloatBits(bits) => bits, _ => unreachable!(),
    };
    progress.converted_bits = Some(converted);
    progress.stage = SourceGainCountStage::Divide;
    progress.next_pc = 0x552663;
    let divisor = math.configuration.divisor_bits;
    let divided = match math.perform(1, 0x552663, SourceGainCountOperation::Divide { left: converted, right: divisor })? {
        SourceGainCountValue::FloatBits(bits) => bits, _ => unreachable!(),
    };
    progress.divided_bits = Some(divided);
    progress.stage = SourceGainCountStage::Truncate;
    progress.next_pc = 0x55266b;
    let truncation = match math.perform(2, 0x55266b, SourceGainCountOperation::Truncate { bits: divided })? {
        SourceGainCountValue::RawCount(value) => value, _ => unreachable!(),
    };
    progress.truncation = Some(truncation);
    progress.stage = SourceGainCountStage::Select;
    progress.next_pc = 0x55266f;
    let raw = truncation.low_dword;

    let selected = if (raw as i32) >= 2 { raw } else { 1 };
    progress.selected_count = Some(selected);
    progress.stage = SourceGainCountStage::AllocationExtent;
    progress.next_pc = 0x55267c;
    let selected_allocation_bytes = u64::from(selected) * 4;
    progress.selected_allocation_bytes = Some(selected_allocation_bytes);
    progress.stage = SourceGainCountStage::RetainRawCount;
    progress.next_pc = 0x552684;
    progress.raw_selector_dword = Some(raw);
    progress.stage = SourceGainCountStage::Complete;
    progress.next_pc = 0x552687;
    Ok(SourceGainCountObservation { wrapped_input_dword: wrapped, converted_bits: converted, divided_bits: divided,
        truncation, raw_selector_dword: raw, selected_count: selected, selected_allocation_bytes })
}
