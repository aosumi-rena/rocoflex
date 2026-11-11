use crate::source_scalar::{self as scalar, ExplicitExceptionState,
    SourceF32I32Truncation, SourceScalarTruncationError};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error, ScalarF32Outcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

pub const WINDOW_SAMPLES: usize = 1920;
const LOAD_PCS: [u32; 4] = [0x55281f, 0x55283c, 0x552860, 0x552886];
const STORE_PCS: [u32; 4] = [0x552825, 0x552842, 0x552866, 0x5527f0];

#[derive(Clone, Copy, Debug)]
pub struct SourceWindowConfiguration {
    pub architecture: ScalarF32Architecture,
    pub xmm7_bits: u32,
    pub xmm8_bits: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceWindowRequest {

    pub row_dword: u32,

    pub source_count: usize,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceWindowBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceWindowBindingFailure {
    configuration: SourceWindowConfiguration, entry: ExplicitExceptionState, cause: SourceWindowBindingCause,
}
impl SourceWindowBindingFailure {
    pub fn configuration(&self) -> SourceWindowConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceWindowBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceWindowOperands {
    SignedDword { value: i32 }, Multiply { left: u32, right: u32 }, Truncate { bits: u32 },
}
#[derive(Clone, Copy, Debug)]
pub struct SourceWindowAttempt {
    pub pc: u32, pub operands: SourceWindowOperands,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub result_float_bits: Option<u32>, pub result_center_dword: Option<u32>,
    pub scalar_reported: Option<Result<ScalarF32Outcome, ScalarF32Error>>,
    pub scalar_projection: Option<SourceScalarF32Projection>,
    pub truncate_reported: Option<Result<SourceF32I32Truncation, SourceScalarTruncationError>>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceWindowMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), Truncate(SourceScalarTruncationError),
}
pub struct SourceWindowMathFailure { cause: SourceWindowMathCause, attempt: SourceWindowAttempt }
impl SourceWindowMathFailure {
    pub fn cause(&self) -> SourceWindowMathCause { self.cause }
    pub fn attempt(&self) -> &SourceWindowAttempt { &self.attempt }
}
pub struct SourceWindowMath {
    configuration: SourceWindowConfiguration, entry: ExplicitExceptionState,
    state: ExplicitExceptionState, scalar: SourceScalarF32,
    reports: [Option<SourceWindowAttempt>; 4], first_failure: Option<SourceWindowMathFailure>,
}
impl SourceWindowMath {
    pub fn from_explicit_parts(configuration: SourceWindowConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceWindowBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm {
                return Err(SourceWindowBindingCause::Architecture);
            }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceWindowBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
                ScalarF32Opcode::Cvtsi2ssSigned32, ScalarF32Opcode::Mulss])
                .map_err(SourceWindowBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceWindowBindingCause::Scalar)
        };
        match bind() {
            Ok(backend) => Ok(Self { configuration, entry, state: entry, scalar: backend,
                reports: [None; 4], first_failure: None }),
            Err(cause) => Err(SourceWindowBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceWindowConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }

    pub fn reports(&self) -> &[Option<SourceWindowAttempt>; 4] { &self.reports }
    pub fn first_failure(&self) -> Option<&SourceWindowMathFailure> { self.first_failure.as_ref() }
    fn attempt(&self, pc: u32, operands: SourceWindowOperands) -> SourceWindowAttempt {
        SourceWindowAttempt { pc, operands, entry: self.state, known_advanced: self.state,
            result_float_bits: None, result_center_dword: None,
            scalar_reported: None, scalar_projection: None, truncate_reported: None }
    }
    fn fail(&mut self, slot: usize, mut attempt: SourceWindowAttempt, cause: SourceWindowMathCause) -> Stopped {
        attempt.known_advanced = self.state;
        self.reports[slot] = Some(attempt);
        self.first_failure = Some(SourceWindowMathFailure { cause, attempt });
        Stopped
    }
    fn scalar(&mut self, slot: usize, pc: u32, operands: SourceWindowOperands, instruction: ScalarF32Operation)
        -> Result<u32, Stopped>
    {
        if self.first_failure.is_some() { return Err(Stopped); }
        let mut attempt = self.attempt(pc, operands);
        self.reports[slot] = Some(attempt);
        let before = match ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr()) {
            Some(before) => before,
            None => return Err(self.fail(slot, attempt, SourceWindowMathCause::LogicalState { mxcsr: self.state.mxcsr() })),
        };
        let reported = self.scalar.execute(instruction, before);

        attempt.scalar_reported = Some(reported);
        self.reports[slot] = Some(attempt);
        let reported = match reported {
            Ok(reported) => reported,
            Err(error) => return Err(self.fail(slot, attempt, SourceWindowMathCause::Scalar(error))),
        };
        let projection = project_scalar_f32_outcome(self.state, reported);
        attempt.scalar_projection = Some(projection);
        self.reports[slot] = Some(attempt);
        let bits = match projection {
            SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; bits }
            SourceScalarF32Projection::Trapped { advanced, .. } => {
                self.state = advanced;
                return Err(self.fail(slot, attempt, SourceWindowMathCause::ReportedScalar(projection)));
            }
            SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                self.state = known_advanced;
                return Err(self.fail(slot, attempt, SourceWindowMathCause::ReportedScalar(projection)));
            }
        };
        attempt.known_advanced = self.state;
        attempt.result_float_bits = Some(bits);
        self.reports[slot] = Some(attempt);
        Ok(bits)
    }
    fn truncate(&mut self, bits: u32) -> Result<SourceF32I32Truncation, Stopped> {
        if self.first_failure.is_some() { return Err(Stopped); }
        let mut attempt = self.attempt(0x5527a7, SourceWindowOperands::Truncate { bits });
        self.reports[3] = Some(attempt);
        let reported = scalar::truncate_f32_to_i32_with_explicit_state(bits, self.state, self.configuration.architecture);
        attempt.truncate_reported = Some(reported);
        self.reports[3] = Some(attempt);
        match reported {
            Ok(value) => {

                self.state = value.exception_state;
                attempt.known_advanced = self.state;
                attempt.result_center_dword = Some(value.low_dword);
                self.reports[3] = Some(attempt);
                Ok(value)
            }
            Err(error) => {
                self.state = error.known_advanced_state(self.state);
                Err(self.fail(3, attempt, SourceWindowMathCause::Truncate(error)))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceWindowStage { Admission, Convert, FirstMultiply, SecondMultiply, Truncate, Populate, Complete }
#[derive(Clone, Copy, Debug)]
pub struct SourceWindowAccess {
    pub output_index: usize, pub source_index_dword: u32, pub signed_source_index: i32,

    pub selected_source_index: Option<usize>, pub selected_bits: u32,
    pub source_load_pc: Option<u32>, pub store_pc: u32,
}
pub struct SourceWindowProgress {
    stage: SourceWindowStage, next_pc: u32, row_plus_two_dword: Option<u32>,
    converted_bits: Option<u32>, first_product_bits: Option<u32>, second_product_bits: Option<u32>,
    center: Option<SourceF32I32Truncation>, completed_stores: usize, last_store: Option<SourceWindowAccess>,
}
impl SourceWindowProgress {
    fn new() -> Self {
        Self { stage: SourceWindowStage::Admission, next_pc: 0x552794, row_plus_two_dword: None,
            converted_bits: None, first_product_bits: None, second_product_bits: None,
            center: None, completed_stores: 0, last_store: None }
    }
    pub fn stage(&self) -> SourceWindowStage { self.stage }
    pub fn next_pc(&self) -> u32 { self.next_pc }
    pub fn row_plus_two_dword(&self) -> Option<u32> { self.row_plus_two_dword }
    pub fn converted_bits(&self) -> Option<u32> { self.converted_bits }
    pub fn first_product_bits(&self) -> Option<u32> { self.first_product_bits }
    pub fn second_product_bits(&self) -> Option<u32> { self.second_product_bits }
    pub fn center(&self) -> Option<SourceF32I32Truncation> { self.center }
    pub fn completed_stores(&self) -> usize { self.completed_stores }
    pub fn last_store(&self) -> Option<SourceWindowAccess> { self.last_store }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceWindowFailureCause {

    SourceCount { requested: usize, backing: usize },
    WindowLength { actual: usize, required: usize }, Math,
}

pub struct SourceWindowFailure {
    source: Vec<f32>, window: Vec<f32>, request: SourceWindowRequest,
    math: SourceWindowMath, progress: SourceWindowProgress, cause: SourceWindowFailureCause,
}
impl SourceWindowFailure {
    pub fn source(&self) -> &[f32] { &self.source }
    pub fn window(&self) -> &[f32] { &self.window }
    pub fn request(&self) -> SourceWindowRequest { self.request }
    pub fn math(&self) -> &SourceWindowMath { &self.math }
    pub fn progress(&self) -> &SourceWindowProgress { &self.progress }
    pub fn cause(&self) -> SourceWindowFailureCause { self.cause }
}
pub struct SourceWindowCompleted {
    source: Vec<f32>, window: Vec<f32>, request: SourceWindowRequest,
    math: SourceWindowMath, progress: SourceWindowProgress,
}
pub struct SourceWindowParts {
    pub source: Vec<f32>, pub window: Vec<f32>, pub request: SourceWindowRequest,
    pub math: SourceWindowMath, pub progress: SourceWindowProgress,
}
impl SourceWindowCompleted {
    pub fn source(&self) -> &[f32] { &self.source }
    pub fn window(&self) -> &[f32] { &self.window }
    pub fn request(&self) -> SourceWindowRequest { self.request }
    pub fn math(&self) -> &SourceWindowMath { &self.math }
    pub fn progress(&self) -> &SourceWindowProgress { &self.progress }
    pub fn into_parts(self) -> SourceWindowParts {
        SourceWindowParts { source: self.source, window: self.window, request: self.request,
            math: self.math, progress: self.progress }
    }
}

pub fn prepare_source_window(source: Vec<f32>, request: SourceWindowRequest, mut window: Vec<f32>, mut math: SourceWindowMath)
    -> Result<SourceWindowCompleted, SourceWindowFailure>
{
    let mut progress = SourceWindowProgress::new();
    let cause = if request.source_count > source.len() || request.source_count > i32::MAX as usize {
        Some(SourceWindowFailureCause::SourceCount { requested: request.source_count, backing: source.len() })
    } else if window.len() != WINDOW_SAMPLES {
        Some(SourceWindowFailureCause::WindowLength { actual: window.len(), required: WINDOW_SAMPLES })
    } else {

        if math.first_failure.is_none() { math.reports = [None; 4]; }
        match window_graph(&source, request, &mut window, &mut math, &mut progress) {
            Ok(()) => None, Err(Stopped) => Some(SourceWindowFailureCause::Math),
        }
    };
    match cause {
        Some(cause) => Err(SourceWindowFailure { source, window, request, math, progress, cause }),
        None => Ok(SourceWindowCompleted { source, window, request, math, progress }),
    }
}
struct Stopped;
fn window_graph(source: &[f32], request: SourceWindowRequest, window: &mut [f32],
    math: &mut SourceWindowMath, progress: &mut SourceWindowProgress) -> Result<(), Stopped>
{
    let row_plus_two = request.row_dword.wrapping_add(2);
    progress.row_plus_two_dword = Some(row_plus_two);
    progress.stage = SourceWindowStage::Convert;
    progress.next_pc = 0x55279a;
    let signed = row_plus_two as i32;
    let converted = math.scalar(0, 0x55279a, SourceWindowOperands::SignedDword { value: signed },
        ScalarF32Operation::Cvtsi2ssSigned32 { value: signed })?;
    progress.converted_bits = Some(converted);
    progress.stage = SourceWindowStage::FirstMultiply;
    progress.next_pc = 0x55279e;
    let xmm7 = math.configuration.xmm7_bits;
    let first = math.scalar(1, 0x55279e, SourceWindowOperands::Multiply { left: converted, right: xmm7 },
        ScalarF32Operation::Mulss { left: converted, right: xmm7 })?;
    progress.first_product_bits = Some(first);
    progress.stage = SourceWindowStage::SecondMultiply;
    progress.next_pc = 0x5527a2;
    let xmm8 = math.configuration.xmm8_bits;
    let second = math.scalar(2, 0x5527a2, SourceWindowOperands::Multiply { left: first, right: xmm8 },
        ScalarF32Operation::Mulss { left: first, right: xmm8 })?;
    progress.second_product_bits = Some(second);
    progress.stage = SourceWindowStage::Truncate;
    progress.next_pc = 0x5527a7;
    let center = math.truncate(second)?;
    progress.center = Some(center);
    progress.stage = SourceWindowStage::Populate;
    let offsets = [960u32, 959, 958, 957];
    let count = request.source_count as i32;
    for base in (0..WINDOW_SAMPLES).step_by(4) {
        for lane in 0..4 {
            let index_word = center.low_dword.wrapping_sub(offsets[lane]).wrapping_add(base as u32);
            let signed_index = index_word as i32;
            let selected_source_index = if signed_index >= 0 && signed_index < count {
                Some(index_word as usize)
            } else { None };
            let bits = match selected_source_index { Some(index) => source[index].to_bits(), None => 0 };
            let output_index = base + lane;
            progress.next_pc = STORE_PCS[lane];
            window[output_index] = f32::from_bits(bits);
            progress.completed_stores += 1;
            progress.last_store = Some(SourceWindowAccess { output_index,
                source_index_dword: index_word, signed_source_index: signed_index,
                selected_source_index, selected_bits: bits,
                source_load_pc: selected_source_index.map(|_| LOAD_PCS[lane]), store_pc: STORE_PCS[lane] });
        }
    }
    progress.stage = SourceWindowStage::Complete;
    progress.next_pc = 0x5528a0;
    Ok(())
}
