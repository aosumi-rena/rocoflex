use crate::source_scalar::{self as scalar, ExplicitExceptionState, LogArithmeticError, SourceLogWithState};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error, ScalarF32Outcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

#[derive(Clone, Copy, Debug)]
pub struct SourceGainInterpolationRequest { pub sample_index_dword: u32 }
#[derive(Clone, Copy, Debug)]
pub struct SourceGainInterpolationConfiguration {
    pub architecture: ScalarF32Architecture,
    pub xmm7_position_scale_bits: u32,
    pub xmm8_position_divisor_bits: u32,
    pub xmm9_exponent_divisor_bits: u32,
    pub xmm6_power_base_bits: u32,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainInterpolationBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceGainInterpolationBindingFailure {
    configuration: SourceGainInterpolationConfiguration, entry: ExplicitExceptionState,
    cause: SourceGainInterpolationBindingCause,
}
impl SourceGainInterpolationBindingFailure {
    pub fn configuration(&self) -> SourceGainInterpolationConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceGainInterpolationBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainInterpolationOperation {
    Convert { signed_input: i32 }, Multiply { left: u32, right: u32 },
    Subtract { left: u32, right: u32 }, Add { left: u32, right: u32 }, Divide { left: u32, right: u32 },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainInterpolationPacket {
    Scalar(Result<ScalarF32Outcome, ScalarF32Error>),
    Divide(Result<SourceLogWithState, LogArithmeticError>),
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGainInterpolationAttempt {
    pub pc: u32, pub operation: SourceGainInterpolationOperation,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub reported: Option<SourceGainInterpolationPacket>,
    pub scalar_projection: Option<SourceScalarF32Projection>, pub result_bits: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainInterpolationMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), Divide(LogArithmeticError),
}
pub struct SourceGainInterpolationMathFailure {
    cause: SourceGainInterpolationMathCause, attempt: SourceGainInterpolationAttempt,
}
impl SourceGainInterpolationMathFailure {
    pub fn cause(&self) -> SourceGainInterpolationMathCause { self.cause }
    pub fn attempt(&self) -> &SourceGainInterpolationAttempt { &self.attempt }
}
pub struct SourceGainInterpolationMath {
    configuration: SourceGainInterpolationConfiguration, entry: ExplicitExceptionState,
    state: ExplicitExceptionState, scalar: SourceScalarF32,
    reports: [Option<SourceGainInterpolationAttempt>; 9], first_failure: Option<SourceGainInterpolationMathFailure>,
}
impl SourceGainInterpolationMath {
    pub fn from_explicit_parts(configuration: SourceGainInterpolationConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceGainInterpolationBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm {
                return Err(SourceGainInterpolationBindingCause::Architecture);
            }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceGainInterpolationBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
                ScalarF32Opcode::Cvtsi2ssSigned32, ScalarF32Opcode::Mulss, ScalarF32Opcode::Subss, ScalarF32Opcode::Addss])
                .map_err(SourceGainInterpolationBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceGainInterpolationBindingCause::Scalar)
        };
        match bind() {
            Ok(backend) => Ok(Self { configuration, entry, state: entry, scalar: backend,
                reports: [None; 9], first_failure: None }),
            Err(cause) => Err(SourceGainInterpolationBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceGainInterpolationConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }

    pub fn reports(&self) -> &[Option<SourceGainInterpolationAttempt>; 9] { &self.reports }
    pub fn first_failure(&self) -> Option<&SourceGainInterpolationMathFailure> { self.first_failure.as_ref() }
    fn perform(&mut self, slot: usize, pc: u32, operation: SourceGainInterpolationOperation) -> Result<u32, Stopped> {
        if self.first_failure.is_some() { return Err(Stopped); }
        let mut attempt = SourceGainInterpolationAttempt { pc, operation, entry: self.state,
            known_advanced: self.state, reported: None, scalar_projection: None, result_bits: None };
        self.reports[slot] = Some(attempt);
        let result = (|| {
            if let SourceGainInterpolationOperation::Divide { left, right } = operation {
                let reported = scalar::raw_divss_with_explicit_state(left, right, self.state, self.configuration.architecture);
                attempt.reported = Some(SourceGainInterpolationPacket::Divide(reported));
                self.reports[slot] = Some(attempt);
                return match reported {
                    Ok(value) => { self.state = value.exception_state; Ok(value.bits) }
                    Err(error) => {
                        self.state = divide_advanced(error, self.state);
                        Err(SourceGainInterpolationMathCause::Divide(error))
                    }
                };
            }
            let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                .ok_or(SourceGainInterpolationMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
            let instruction = match operation {
                SourceGainInterpolationOperation::Convert { signed_input } => ScalarF32Operation::Cvtsi2ssSigned32 { value: signed_input },
                SourceGainInterpolationOperation::Multiply { left, right } => ScalarF32Operation::Mulss { left, right },
                SourceGainInterpolationOperation::Subtract { left, right } => ScalarF32Operation::Subss { left, right },
                SourceGainInterpolationOperation::Add { left, right } => ScalarF32Operation::Addss { left, right },
                SourceGainInterpolationOperation::Divide { .. } => unreachable!(),
            };
            let reported = self.scalar.execute(instruction, before);

            attempt.reported = Some(SourceGainInterpolationPacket::Scalar(reported));
            self.reports[slot] = Some(attempt);
            let reported = reported.map_err(SourceGainInterpolationMathCause::Scalar)?;
            let projection = project_scalar_f32_outcome(self.state, reported);
            attempt.scalar_projection = Some(projection);
            self.reports[slot] = Some(attempt);
            match projection {
                SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
                SourceScalarF32Projection::Trapped { advanced, .. } => {
                    self.state = advanced; Err(SourceGainInterpolationMathCause::ReportedScalar(projection))
                }
                SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                    self.state = known_advanced; Err(SourceGainInterpolationMathCause::ReportedScalar(projection))
                }
            }
        })();
        attempt.known_advanced = self.state;
        match result {
            Ok(bits) => { attempt.result_bits = Some(bits); self.reports[slot] = Some(attempt); Ok(bits) }
            Err(cause) => {
                self.reports[slot] = Some(attempt);
                self.first_failure = Some(SourceGainInterpolationMathFailure { cause, attempt });
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

#[derive(Clone, Copy, Debug)]
pub struct PowerRequest {

    pub pc: u32,
    pub base_bits: u32,
    pub exponent_bits: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGainInterpolationStage {
    Admission, IntegerQuotient, SampleConvert, PositionMultiply, PositionDivide, QuotientConvert,
    FractionSubtract, FirstSelect, FirstLoad, SecondSelect, SecondLoad, DifferenceSubtract,
    InterpolateMultiply, InterpolateAdd, ExponentDivide, PowerBaseMove, PowerRequired,
}
pub struct SourceGainInterpolationProgress {
    stage: SourceGainInterpolationStage, next_pc: u32,
    product_low64: Option<u64>, quotient_dword: Option<u32>, incremented_quotient_dword: Option<u32>,
    converted_sample_bits: Option<u32>, position_product_bits: Option<u32>, position_bits: Option<u32>,
    converted_quotient_bits: Option<u32>, fraction_bits: Option<u32>, last_index_dword: Option<u32>,
    first_index: Option<usize>, first_gain_bits: Option<u32>, second_index: Option<usize>, second_gain_bits: Option<u32>,
    difference_bits: Option<u32>, interpolation_product_bits: Option<u32>, interpolated_bits: Option<u32>, exponent_bits: Option<u32>,
}
impl SourceGainInterpolationProgress {
    fn new() -> Self {
        Self { stage: SourceGainInterpolationStage::Admission, next_pc: 0x552f30,
            product_low64: None, quotient_dword: None, incremented_quotient_dword: None,
            converted_sample_bits: None, position_product_bits: None, position_bits: None,
            converted_quotient_bits: None, fraction_bits: None, last_index_dword: None,
            first_index: None, first_gain_bits: None, second_index: None, second_gain_bits: None,
            difference_bits: None, interpolation_product_bits: None, interpolated_bits: None, exponent_bits: None }
    }
    pub fn stage(&self) -> SourceGainInterpolationStage { self.stage }
    pub fn next_pc(&self) -> u32 { self.next_pc }
    pub fn product_low64(&self) -> Option<u64> { self.product_low64 }
    pub fn quotient_dword(&self) -> Option<u32> { self.quotient_dword }
    pub fn incremented_quotient_dword(&self) -> Option<u32> { self.incremented_quotient_dword }
    pub fn converted_sample_bits(&self) -> Option<u32> { self.converted_sample_bits }
    pub fn position_product_bits(&self) -> Option<u32> { self.position_product_bits }
    pub fn position_bits(&self) -> Option<u32> { self.position_bits }
    pub fn converted_quotient_bits(&self) -> Option<u32> { self.converted_quotient_bits }
    pub fn fraction_bits(&self) -> Option<u32> { self.fraction_bits }
    pub fn last_index_dword(&self) -> Option<u32> { self.last_index_dword }
    pub fn first_index(&self) -> Option<usize> { self.first_index }
    pub fn first_gain_bits(&self) -> Option<u32> { self.first_gain_bits }
    pub fn second_index(&self) -> Option<usize> { self.second_index }
    pub fn second_gain_bits(&self) -> Option<u32> { self.second_gain_bits }
    pub fn difference_bits(&self) -> Option<u32> { self.difference_bits }
    pub fn interpolation_product_bits(&self) -> Option<u32> { self.interpolation_product_bits }
    pub fn interpolated_bits(&self) -> Option<u32> { self.interpolated_bits }
    pub fn exponent_bits(&self) -> Option<u32> { self.exponent_bits }
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGainInterpolationObservation {
    pub product_low64: u64, pub quotient_dword: u32, pub incremented_quotient_dword: u32,
    pub last_index_dword: u32, pub first_index: usize, pub second_index: usize,
    pub gain_a_bits: u32, pub gain_b_bits: u32, pub fraction_bits: u32, pub interpolated_bits: u32,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainInterpolationFailureCause { GainExtent { actual: usize }, Math }

pub struct SourceGainInterpolationFailure {
    gain: Vec<f32>, request: SourceGainInterpolationRequest, math: SourceGainInterpolationMath,
    progress: SourceGainInterpolationProgress, cause: SourceGainInterpolationFailureCause,
}
impl SourceGainInterpolationFailure {
    pub fn gain(&self) -> &[f32] { &self.gain }
    pub fn request(&self) -> SourceGainInterpolationRequest { self.request }
    pub fn math(&self) -> &SourceGainInterpolationMath { &self.math }
    pub fn progress(&self) -> &SourceGainInterpolationProgress { &self.progress }
    pub fn cause(&self) -> SourceGainInterpolationFailureCause { self.cause }
}

pub struct SourceGainInterpolationCompleted {
    gain: Vec<f32>, request: SourceGainInterpolationRequest, math: SourceGainInterpolationMath,
    progress: SourceGainInterpolationProgress, observation: SourceGainInterpolationObservation, power: PowerRequest,
}
pub struct SourceGainInterpolationParts {
    pub gain: Vec<f32>, pub request: SourceGainInterpolationRequest, pub math: SourceGainInterpolationMath,
    pub progress: SourceGainInterpolationProgress, pub observation: SourceGainInterpolationObservation, pub power: PowerRequest,
}
impl SourceGainInterpolationCompleted {
    pub fn gain(&self) -> &[f32] { &self.gain }
    pub fn request(&self) -> SourceGainInterpolationRequest { self.request }
    pub fn math(&self) -> &SourceGainInterpolationMath { &self.math }
    pub fn progress(&self) -> &SourceGainInterpolationProgress { &self.progress }
    pub fn observation(&self) -> SourceGainInterpolationObservation { self.observation }
    pub fn power_request(&self) -> PowerRequest { self.power }
    pub fn into_parts(self) -> SourceGainInterpolationParts {
        SourceGainInterpolationParts { gain: self.gain, request: self.request, math: self.math,
            progress: self.progress, observation: self.observation, power: self.power }
    }
}
pub fn prepare_gain_power_request(gain: Vec<f32>, request: SourceGainInterpolationRequest, mut math: SourceGainInterpolationMath)
    -> Result<SourceGainInterpolationCompleted, SourceGainInterpolationFailure>
{
    let mut progress = SourceGainInterpolationProgress::new();

    if gain.is_empty() || gain.len() > i32::MAX as usize {
        let cause = SourceGainInterpolationFailureCause::GainExtent { actual: gain.len() };
        return Err(SourceGainInterpolationFailure { gain, request, math, progress, cause });
    }
    if math.first_failure.is_none() { math.reports = [None; 9]; }
    match interpolation_graph(&gain, request, &mut math, &mut progress) {
        Ok((observation, power)) => Ok(SourceGainInterpolationCompleted { gain, request, math, progress, observation, power }),
        Err(Stopped) => Err(SourceGainInterpolationFailure { gain, request, math, progress,
            cause: SourceGainInterpolationFailureCause::Math }),
    }
}
struct Stopped;
fn interpolation_graph(gain: &[f32], request: SourceGainInterpolationRequest,
    math: &mut SourceGainInterpolationMath, progress: &mut SourceGainInterpolationProgress)
    -> Result<(SourceGainInterpolationObservation, PowerRequest), Stopped>
{
    let c = math.configuration;
    progress.stage = SourceGainInterpolationStage::IntegerQuotient;
    progress.next_pc = 0x552f30;

    let product = u64::from(request.sample_index_dword).wrapping_mul(0x8888_8889);
    progress.product_low64 = Some(product);
    let quotient = (product >> 40) as u32;
    progress.quotient_dword = Some(quotient);
    progress.stage = SourceGainInterpolationStage::SampleConvert;
    progress.next_pc = 0x552f3e;
    let sample = math.perform(0, 0x552f3e, SourceGainInterpolationOperation::Convert {
        signed_input: request.sample_index_dword as i32 })?;
    progress.converted_sample_bits = Some(sample);
    progress.stage = SourceGainInterpolationStage::PositionMultiply;
    progress.next_pc = 0x552f43;
    let scaled = math.perform(1, 0x552f43, SourceGainInterpolationOperation::Multiply {
        left: sample, right: c.xmm7_position_scale_bits })?;
    progress.position_product_bits = Some(scaled);
    progress.stage = SourceGainInterpolationStage::PositionDivide;
    progress.next_pc = 0x552f47;
    let position = math.perform(2, 0x552f47, SourceGainInterpolationOperation::Divide {
        left: scaled, right: c.xmm8_position_divisor_bits })?;
    progress.position_bits = Some(position);
    progress.stage = SourceGainInterpolationStage::QuotientConvert;
    progress.next_pc = 0x552f4f;
    let quotient_float = math.perform(3, 0x552f4f, SourceGainInterpolationOperation::Convert { signed_input: quotient as i32 })?;
    progress.converted_quotient_bits = Some(quotient_float);
    progress.stage = SourceGainInterpolationStage::FractionSubtract;
    progress.next_pc = 0x552f53;
    let fraction = math.perform(4, 0x552f53, SourceGainInterpolationOperation::Subtract { left: position, right: quotient_float })?;
    progress.fraction_bits = Some(fraction);
    let last = (gain.len() - 1) as i32;
    progress.last_index_dword = Some(last as u32);
    progress.stage = SourceGainInterpolationStage::FirstSelect;
    progress.next_pc = 0x552f57;
    let first_word = if last < quotient as i32 { last as u32 } else { quotient };
    let first = first_word as usize;
    progress.first_index = Some(first);
    progress.stage = SourceGainInterpolationStage::FirstLoad;
    progress.next_pc = 0x552f61;
    let a = gain[first].to_bits();
    progress.first_gain_bits = Some(a);
    progress.stage = SourceGainInterpolationStage::SecondSelect;
    progress.next_pc = 0x552f67;
    let incremented = quotient.wrapping_add(1);
    progress.incremented_quotient_dword = Some(incremented);
    let second_word = if last < incremented as i32 { last as u32 } else { incremented };
    let second = second_word as usize;
    progress.second_index = Some(second);
    progress.stage = SourceGainInterpolationStage::SecondLoad;
    progress.next_pc = 0x552f70;
    let b = gain[second].to_bits();
    progress.second_gain_bits = Some(b);
    progress.stage = SourceGainInterpolationStage::DifferenceSubtract;
    progress.next_pc = 0x552f76;
    let difference = math.perform(5, 0x552f76, SourceGainInterpolationOperation::Subtract { left: b, right: a })?;
    progress.difference_bits = Some(difference);
    progress.stage = SourceGainInterpolationStage::InterpolateMultiply;
    progress.next_pc = 0x552f7a;
    let weighted = math.perform(6, 0x552f7a, SourceGainInterpolationOperation::Multiply { left: difference, right: fraction })?;
    progress.interpolation_product_bits = Some(weighted);
    progress.stage = SourceGainInterpolationStage::InterpolateAdd;
    progress.next_pc = 0x552f7e;
    let interpolated = math.perform(7, 0x552f7e, SourceGainInterpolationOperation::Add { left: weighted, right: a })?;
    progress.interpolated_bits = Some(interpolated);
    progress.stage = SourceGainInterpolationStage::ExponentDivide;
    progress.next_pc = 0x552f82;
    let exponent = math.perform(8, 0x552f82, SourceGainInterpolationOperation::Divide {
        left: interpolated, right: c.xmm9_exponent_divisor_bits })?;
    progress.exponent_bits = Some(exponent);
    progress.stage = SourceGainInterpolationStage::PowerBaseMove;
    progress.next_pc = 0x552f87;
    let power = PowerRequest { pc: 0x552f8a, base_bits: c.xmm6_power_base_bits, exponent_bits: exponent };
    progress.stage = SourceGainInterpolationStage::PowerRequired;
    progress.next_pc = power.pc;
    Ok((SourceGainInterpolationObservation { product_low64: product, quotient_dword: quotient,
        incremented_quotient_dword: incremented, last_index_dword: last as u32,
        first_index: first, second_index: second, gain_a_bits: a, gain_b_bits: b,
        fraction_bits: fraction, interpolated_bits: interpolated }, power))
}
