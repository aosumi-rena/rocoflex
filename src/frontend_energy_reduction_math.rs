use std::convert::Infallible;
use crate::source_scalar::{self as scalar, ExplicitExceptionState, LogArithmeticError, SourceLogWithState};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error, ScalarF32Outcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

pub const WINDOW_SAMPLES: usize = 1920;
const MULTIPLY_PCS: [u32; 8] = [0x5528c7, 0x5528dc, 0x5528f1, 0x552906,
    0x55291b, 0x552930, 0x552945, 0x552959];
const ADD_PCS: [u32; 8] = [0x5528cb, 0x5528e0, 0x5528f5, 0x55290a,
    0x55291f, 0x552934, 0x552949, 0x55295d];
const STORE_PCS: [u32; 8] = [0x5528cf, 0x5528e4, 0x5528f9, 0x55290e,
    0x552923, 0x552938, 0x55294d, 0x552961];

#[derive(Clone, Copy, Debug)]
pub struct SourceEnergyConfiguration {
    pub architecture: ScalarF32Architecture,

    pub divisor_bits: u32,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceEnergyBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceEnergyBindingFailure {
    configuration: SourceEnergyConfiguration, entry: ExplicitExceptionState, cause: SourceEnergyBindingCause,
}
impl SourceEnergyBindingFailure {
    pub fn configuration(&self) -> SourceEnergyConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceEnergyBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEnergyOperation { Multiply, Add, Divide }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEnergyStage { Multiply, Add, Store, Divide, FinalStore, Complete }
#[derive(Clone, Copy, Debug)]
pub struct SourceEnergyAttempt {
    pub pc: u32, pub sample_index: usize, pub operation: SourceEnergyOperation,
    pub left_bits: u32, pub right_bits: u32,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,

    pub result_bits: Option<u32>,
    pub scalar_reported: Option<Result<ScalarF32Outcome, ScalarF32Error>>,
    pub scalar_projection: Option<SourceScalarF32Projection>,
    pub divide_reported: Option<Result<SourceLogWithState, LogArithmeticError>>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceEnergyMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), Divide(LogArithmeticError),
}
pub struct SourceEnergyMathFailure { cause: SourceEnergyMathCause, attempt: SourceEnergyAttempt }
impl SourceEnergyMathFailure {
    pub fn cause(&self) -> SourceEnergyMathCause { self.cause }
    pub fn attempt(&self) -> &SourceEnergyAttempt { &self.attempt }
}

pub struct SourceEnergyMath {
    configuration: SourceEnergyConfiguration, entry: ExplicitExceptionState,
    state: ExplicitExceptionState, scalar: SourceScalarF32,
    latest: Option<SourceEnergyAttempt>, first_failure: Option<SourceEnergyMathFailure>,
}
impl SourceEnergyMath {
    pub fn from_explicit_parts(configuration: SourceEnergyConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceEnergyBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm {
                return Err(SourceEnergyBindingCause::Architecture);
            }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceEnergyBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
                ScalarF32Opcode::Mulss, ScalarF32Opcode::Addss]).map_err(SourceEnergyBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceEnergyBindingCause::Scalar)
        };
        match bind() {
            Ok(backend) => Ok(Self { configuration, entry, state: entry, scalar: backend,
                latest: None, first_failure: None }),
            Err(cause) => Err(SourceEnergyBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceEnergyConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn latest_attempt(&self) -> Option<&SourceEnergyAttempt> { self.latest.as_ref() }
    pub fn first_failure(&self) -> Option<&SourceEnergyMathFailure> { self.first_failure.as_ref() }
    fn perform(&mut self, pc: u32, index: usize, operation: SourceEnergyOperation, left: u32, right: u32)
        -> Result<u32, Stopped>
    {
        if self.first_failure.is_some() { return Err(Stopped); }
        let mut attempt = SourceEnergyAttempt { pc, sample_index: index, operation,
            left_bits: left, right_bits: right, entry: self.state, known_advanced: self.state,
            result_bits: None, scalar_reported: None, scalar_projection: None, divide_reported: None };
        self.latest = Some(attempt);
        let result = (|| {
            if operation == SourceEnergyOperation::Divide {
                let reported = scalar::raw_divss_with_explicit_state(left, right, self.state, self.configuration.architecture);

                attempt.divide_reported = Some(reported);
                self.latest = Some(attempt);
                return match reported {
                    Ok(value) => { self.state = value.exception_state; Ok(value.bits) }
                    Err(error) => {
                        self.state = divide_advanced(error, self.state);
                        Err(SourceEnergyMathCause::Divide(error))
                    }
                };
            }
            let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                .ok_or(SourceEnergyMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
            let instruction = match operation {
                SourceEnergyOperation::Multiply => ScalarF32Operation::Mulss { left, right },
                SourceEnergyOperation::Add => ScalarF32Operation::Addss { left, right },
                SourceEnergyOperation::Divide => unreachable!(),
            };
            let reported = self.scalar.execute(instruction, before);
            attempt.scalar_reported = Some(reported);
            self.latest = Some(attempt);
            let reported = reported.map_err(SourceEnergyMathCause::Scalar)?;
            let projection = project_scalar_f32_outcome(self.state, reported);
            attempt.scalar_projection = Some(projection);
            self.latest = Some(attempt);
            match projection {
                SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
                SourceScalarF32Projection::Trapped { advanced, .. } => {
                    self.state = advanced; Err(SourceEnergyMathCause::ReportedScalar(projection))
                }
                SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                    self.state = known_advanced; Err(SourceEnergyMathCause::ReportedScalar(projection))
                }
            }
        })();
        attempt.known_advanced = self.state;
        match result {
            Ok(bits) => { attempt.result_bits = Some(bits); self.latest = Some(attempt); Ok(bits) }
            Err(cause) => {
                self.latest = Some(attempt);
                self.first_failure = Some(SourceEnergyMathFailure { cause, attempt });
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

pub struct SourceEnergyProgress {
    sample_index: usize, stage: SourceEnergyStage, next_pc: u32,
    stored_energy_bits: u32, pending_square_bits: Option<u32>,
    accumulated_samples: usize, completed_stores: usize,
}
impl SourceEnergyProgress {
    fn new(seed: u32) -> Self {
        Self { sample_index: 0, stage: SourceEnergyStage::Multiply, next_pc: MULTIPLY_PCS[0],
            stored_energy_bits: seed, pending_square_bits: None, accumulated_samples: 0, completed_stores: 0 }
    }
    pub fn sample_index(&self) -> usize { self.sample_index }
    pub fn lane(&self) -> usize { self.sample_index % 8 }
    pub fn stage(&self) -> SourceEnergyStage { self.stage }
    pub fn next_pc(&self) -> u32 { self.next_pc }

    pub fn stored_energy_bits(&self) -> u32 { self.stored_energy_bits }
    pub fn pending_square_bits(&self) -> Option<u32> { self.pending_square_bits }
    pub fn accumulated_samples(&self) -> usize { self.accumulated_samples }
    pub fn completed_stores(&self) -> usize { self.completed_stores }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceEnergyFailureCause { WindowLength { actual: usize, required: usize }, Math }

pub struct SourceEnergyFailure {
    window: Vec<f32>, seed_bits: u32, math: SourceEnergyMath,
    progress: SourceEnergyProgress, cause: SourceEnergyFailureCause,
}
impl SourceEnergyFailure {
    pub fn window(&self) -> &[f32] { &self.window }
    pub fn seed_bits(&self) -> u32 { self.seed_bits }
    pub fn math(&self) -> &SourceEnergyMath { &self.math }
    pub fn progress(&self) -> &SourceEnergyProgress { &self.progress }
    pub fn cause(&self) -> SourceEnergyFailureCause { self.cause }
}
pub struct SourceEnergyCompleted {
    window: Vec<f32>, seed_bits: u32, math: SourceEnergyMath, progress: SourceEnergyProgress,
}
impl SourceEnergyCompleted {
    pub fn window(&self) -> &[f32] { &self.window }
    pub fn seed_bits(&self) -> u32 { self.seed_bits }
    pub fn energy_bits(&self) -> u32 { self.progress.stored_energy_bits }
    pub fn math(&self) -> &SourceEnergyMath { &self.math }
    pub fn progress(&self) -> &SourceEnergyProgress { &self.progress }
    pub fn into_parts(self) -> (Vec<f32>, u32, SourceEnergyMath, SourceEnergyProgress) {
        (self.window, self.progress.stored_energy_bits, self.math, self.progress)
    }
}

pub fn reduce_source_window(window: Vec<f32>, seed_bits: u32, mut math: SourceEnergyMath)
    -> Result<SourceEnergyCompleted, SourceEnergyFailure>
{
    let mut progress = SourceEnergyProgress::new(seed_bits);
    let cause = if window.len() != WINDOW_SAMPLES {
        Some(SourceEnergyFailureCause::WindowLength { actual: window.len(), required: WINDOW_SAMPLES })
    } else {
        let divisor = math.configuration.divisor_bits;
        match reduction_graph(&window, AddOrder::SquareLeft, divisor, &mut math, &mut progress) {
            Ok(()) => None, Err(Stopped) => Some(SourceEnergyFailureCause::Math),
        }
    };
    match cause {
        Some(cause) => Err(SourceEnergyFailure { window, seed_bits, math, progress, cause }),
        None => Ok(SourceEnergyCompleted { window, seed_bits, math, progress }),
    }
}

struct Stopped;
#[derive(Clone, Copy)]
enum AddOrder { SquareLeft, SumLeft }
trait EnergyOps {
    type Error;
    fn operation(&mut self, pc: u32, index: usize, operation: SourceEnergyOperation, left: u32, right: u32)
        -> Result<u32, Self::Error>;
}
impl EnergyOps for SourceEnergyMath {
    type Error = Stopped;
    fn operation(&mut self, pc: u32, index: usize, operation: SourceEnergyOperation, left: u32, right: u32)
        -> Result<u32, Stopped> { self.perform(pc, index, operation, left, right) }
}
struct HostSumLeft;
impl EnergyOps for HostSumLeft {
    type Error = Infallible;
    fn operation(&mut self, _: u32, _: usize, operation: SourceEnergyOperation, left: u32, right: u32)
        -> Result<u32, Infallible>
    {
        let left = f32::from_bits(left); let right = f32::from_bits(right);
        Ok(match operation {
            SourceEnergyOperation::Multiply => left * right,
            SourceEnergyOperation::Add => left + right,
            SourceEnergyOperation::Divide => left / right,
        }.to_bits())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct HostWindowLengthFailure { pub actual: usize, pub required: usize }

pub fn reduce_host_sum_left_seed_zero_divisor1920(window: &[f32]) -> Result<u32, HostWindowLengthFailure> {
    if window.len() != WINDOW_SAMPLES {
        return Err(HostWindowLengthFailure { actual: window.len(), required: WINDOW_SAMPLES });
    }
    let mut progress = SourceEnergyProgress::new(0);
    match reduction_graph(window, AddOrder::SumLeft, 1920.0_f32.to_bits(), &mut HostSumLeft, &mut progress) {
        Ok(()) => Ok(progress.stored_energy_bits), Err(error) => match error {},
    }
}
fn reduction_graph<O: EnergyOps>(window: &[f32], order: AddOrder, divisor: u32,
    ops: &mut O, progress: &mut SourceEnergyProgress) -> Result<(), O::Error>
{
    for base in (0..WINDOW_SAMPLES).step_by(8) {
        for lane in 0..8 {
            let index = base + lane;
            progress.sample_index = index;
            progress.stage = SourceEnergyStage::Multiply;
            progress.next_pc = MULTIPLY_PCS[lane];
            progress.pending_square_bits = None;
            let sample = window[index].to_bits();
            let square = ops.operation(MULTIPLY_PCS[lane], index, SourceEnergyOperation::Multiply, sample, sample)?;
            progress.pending_square_bits = Some(square);
            progress.stage = SourceEnergyStage::Add;
            progress.next_pc = ADD_PCS[lane];
            let (left, right) = match order {
                AddOrder::SquareLeft => (square, progress.stored_energy_bits),
                AddOrder::SumLeft => (progress.stored_energy_bits, square),
            };
            let sum = ops.operation(ADD_PCS[lane], index, SourceEnergyOperation::Add, left, right)?;
            progress.stage = SourceEnergyStage::Store;
            progress.next_pc = STORE_PCS[lane];
            progress.stored_energy_bits = sum;
            progress.completed_stores += 1;
            progress.accumulated_samples = index + 1;
            progress.pending_square_bits = None;
        }
    }
    progress.sample_index = WINDOW_SAMPLES;
    progress.stage = SourceEnergyStage::Divide;
    progress.next_pc = 0x552977;
    let divided = ops.operation(0x552977, WINDOW_SAMPLES, SourceEnergyOperation::Divide,
        progress.stored_energy_bits, divisor)?;
    progress.stage = SourceEnergyStage::FinalStore;
    progress.next_pc = 0x55297c;
    progress.stored_energy_bits = divided;
    progress.completed_stores += 1;
    progress.stage = SourceEnergyStage::Complete;
    progress.next_pc = 0x552982;
    Ok(())
}
