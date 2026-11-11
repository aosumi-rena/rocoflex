use crate::source_scalar::{self as scalar, ExplicitExceptionState, LogArithmeticError, SourceLogWithState};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32CompareOperation, ScalarF32Mxcsr, ScalarF32Error,
    ScalarF32Outcome, ScalarF32ComparisonOutcome, ScalarF32ComparisonFlags};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, project_comparison_f32_outcome,
    SourceScalarF32Projection, SourceComparisonF32Projection};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass { First, Second }
#[derive(Clone, Copy, Debug)]
pub struct SourceGatedEnergyRequest {
    pub pass: Pass,

    pub threshold_bits: u32,

    pub empty_fallback_bits: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGatedEnergyConfiguration { pub architecture: ScalarF32Architecture }
#[derive(Clone, Copy, Debug)]
pub enum SourceGatedEnergyBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceGatedEnergyBindingFailure {
    configuration: SourceGatedEnergyConfiguration, entry: ExplicitExceptionState, cause: SourceGatedEnergyBindingCause,
}
impl SourceGatedEnergyBindingFailure {
    pub fn configuration(&self) -> SourceGatedEnergyConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceGatedEnergyBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGatedEnergyOperation {
    Compare { metric_bits: u32, threshold_bits: u32 },
    Add { sum_bits: u32, energy_bits: u32 },
    ConvertCount { value: i32 },
    Divide { sum_bits: u32, count_bits: u32 },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGatedEnergyValue { Flags(ScalarF32ComparisonFlags), Word(u32) }
#[derive(Clone, Copy, Debug)]
pub struct SourceGatedEnergyAttempt {
    pub pc: u32, pub index: usize, pub operation: SourceGatedEnergyOperation,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub result: Option<SourceGatedEnergyValue>,
    pub comparison_reported: Option<Result<ScalarF32ComparisonOutcome, ScalarF32Error>>,
    pub comparison_projection: Option<SourceComparisonF32Projection>,
    pub scalar_reported: Option<Result<ScalarF32Outcome, ScalarF32Error>>,
    pub scalar_projection: Option<SourceScalarF32Projection>,
    pub divide_reported: Option<Result<SourceLogWithState, LogArithmeticError>>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGatedEnergyMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedComparison(SourceComparisonF32Projection), ReportedScalar(SourceScalarF32Projection),
    Divide(LogArithmeticError),
}
pub struct SourceGatedEnergyMathFailure { cause: SourceGatedEnergyMathCause, attempt: SourceGatedEnergyAttempt }
impl SourceGatedEnergyMathFailure {
    pub fn cause(&self) -> SourceGatedEnergyMathCause { self.cause }
    pub fn attempt(&self) -> &SourceGatedEnergyAttempt { &self.attempt }
}
pub struct SourceGatedEnergyMath {
    configuration: SourceGatedEnergyConfiguration, entry: ExplicitExceptionState,
    state: ExplicitExceptionState, scalar: SourceScalarF32,
    latest: Option<SourceGatedEnergyAttempt>, first_failure: Option<SourceGatedEnergyMathFailure>,
}
impl SourceGatedEnergyMath {
    pub fn from_explicit_parts(configuration: SourceGatedEnergyConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceGatedEnergyBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm {
                return Err(SourceGatedEnergyBindingCause::Architecture);
            }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceGatedEnergyBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
                ScalarF32Opcode::Ucomiss, ScalarF32Opcode::Addss, ScalarF32Opcode::Cvtsi2ssSigned32])
                .map_err(SourceGatedEnergyBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceGatedEnergyBindingCause::Scalar)
        };
        match bind() {
            Ok(backend) => Ok(Self { configuration, entry, state: entry, scalar: backend,
                latest: None, first_failure: None }),
            Err(cause) => Err(SourceGatedEnergyBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceGatedEnergyConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }

    pub fn latest_attempt(&self) -> Option<&SourceGatedEnergyAttempt> { self.latest.as_ref() }
    pub fn first_failure(&self) -> Option<&SourceGatedEnergyMathFailure> { self.first_failure.as_ref() }
    fn perform(&mut self, pc: u32, index: usize, operation: SourceGatedEnergyOperation)
        -> Result<SourceGatedEnergyValue, Stopped>
    {
        if self.first_failure.is_some() { return Err(Stopped); }
        let mut attempt = SourceGatedEnergyAttempt { pc, index, operation,
            entry: self.state, known_advanced: self.state, result: None,
            comparison_reported: None, comparison_projection: None,
            scalar_reported: None, scalar_projection: None, divide_reported: None };
        self.latest = Some(attempt);
        let result = (|| {
            if let SourceGatedEnergyOperation::Divide { sum_bits, count_bits } = operation {
                let reported = scalar::raw_divss_with_explicit_state(sum_bits, count_bits,
                    self.state, self.configuration.architecture);
                attempt.divide_reported = Some(reported);
                self.latest = Some(attempt);
                return match reported {
                    Ok(value) => { self.state = value.exception_state; Ok(SourceGatedEnergyValue::Word(value.bits)) }
                    Err(error) => {
                        self.state = divide_advanced(error, self.state);
                        Err(SourceGatedEnergyMathCause::Divide(error))
                    }
                };
            }
            let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                .ok_or(SourceGatedEnergyMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
            if let SourceGatedEnergyOperation::Compare { metric_bits, threshold_bits } = operation {
                let reported = self.scalar.execute_comparison(ScalarF32CompareOperation::Ucomiss {
                    left: metric_bits, right: threshold_bits }, before);

                attempt.comparison_reported = Some(reported);
                self.latest = Some(attempt);
                let reported = reported.map_err(SourceGatedEnergyMathCause::Scalar)?;
                let projection = project_comparison_f32_outcome(self.state, reported);
                attempt.comparison_projection = Some(projection);
                self.latest = Some(attempt);
                return match projection {
                    SourceComparisonF32Projection::Completed { flags, advanced } => {
                        self.state = advanced; Ok(SourceGatedEnergyValue::Flags(flags))
                    }
                    SourceComparisonF32Projection::Trapped { advanced, .. } => {
                        self.state = advanced; Err(SourceGatedEnergyMathCause::ReportedComparison(projection))
                    }
                    SourceComparisonF32Projection::ReportedStateRequired { known_advanced, .. } => {
                        self.state = known_advanced; Err(SourceGatedEnergyMathCause::ReportedComparison(projection))
                    }
                };
            }
            let instruction = match operation {
                SourceGatedEnergyOperation::Add { sum_bits, energy_bits } => ScalarF32Operation::Addss {
                    left: sum_bits, right: energy_bits },
                SourceGatedEnergyOperation::ConvertCount { value } => ScalarF32Operation::Cvtsi2ssSigned32 { value },
                _ => unreachable!(),
            };
            let reported = self.scalar.execute(instruction, before);
            attempt.scalar_reported = Some(reported);
            self.latest = Some(attempt);
            let reported = reported.map_err(SourceGatedEnergyMathCause::Scalar)?;
            let projection = project_scalar_f32_outcome(self.state, reported);
            attempt.scalar_projection = Some(projection);
            self.latest = Some(attempt);
            match projection {
                SourceScalarF32Projection::Completed { bits, advanced } => {
                    self.state = advanced; Ok(SourceGatedEnergyValue::Word(bits))
                }
                SourceScalarF32Projection::Trapped { advanced, .. } => {
                    self.state = advanced; Err(SourceGatedEnergyMathCause::ReportedScalar(projection))
                }
                SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                    self.state = known_advanced; Err(SourceGatedEnergyMathCause::ReportedScalar(projection))
                }
            }
        })();
        attempt.known_advanced = self.state;
        match result {
            Ok(value) => { attempt.result = Some(value); self.latest = Some(attempt); Ok(value) }
            Err(cause) => {
                self.latest = Some(attempt);
                self.first_failure = Some(SourceGatedEnergyMathFailure { cause, attempt });
                Err(Stopped)
            }
        }
    }
    fn comparison(&mut self, pc: u32, index: usize, metric_bits: u32, threshold_bits: u32)
        -> Result<ScalarF32ComparisonFlags, Stopped>
    {
        match self.perform(pc, index, SourceGatedEnergyOperation::Compare { metric_bits, threshold_bits })? {
            SourceGatedEnergyValue::Flags(flags) => Ok(flags), _ => unreachable!(),
        }
    }
    fn word(&mut self, pc: u32, index: usize, operation: SourceGatedEnergyOperation) -> Result<u32, Stopped> {
        match self.perform(pc, index, operation)? { SourceGatedEnergyValue::Word(bits) => Ok(bits), _ => unreachable!() }
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
pub enum SourceGatedEnergyStage { Admission, Compare, Branch, Add, Increment, ConvertCount, Divide, EmptyFallback, Complete }
pub struct SourceGatedEnergyProgress {
    stage: SourceGatedEnergyStage, next_pc: u32, index: usize, completed_rows: usize,
    sum_bits: u32, accepted_count: u32, current_flags: Option<ScalarF32ComparisonFlags>,
    pending_metric_bits: Option<u32>, pending_energy_bits: Option<u32>, count_float_bits: Option<u32>,
}
impl SourceGatedEnergyProgress {
    fn new(pass: Pass) -> Self {

        Self { stage: SourceGatedEnergyStage::Admission, next_pc: match pass { Pass::First => 0x5529e5, Pass::Second => 0x552b09 },
            index: 0, completed_rows: 0, sum_bits: 0, accepted_count: 0, current_flags: None,
            pending_metric_bits: None, pending_energy_bits: None, count_float_bits: None }
    }
    pub fn stage(&self) -> SourceGatedEnergyStage { self.stage }
    pub fn next_pc(&self) -> u32 { self.next_pc }
    pub fn index(&self) -> usize { self.index }
    pub fn completed_rows(&self) -> usize { self.completed_rows }
    pub fn sum_bits(&self) -> u32 { self.sum_bits }
    pub fn accepted_count(&self) -> u32 { self.accepted_count }
    pub fn current_flags(&self) -> Option<ScalarF32ComparisonFlags> { self.current_flags }
    pub fn pending_metric_bits(&self) -> Option<u32> { self.pending_metric_bits }
    pub fn pending_energy_bits(&self) -> Option<u32> { self.pending_energy_bits }
    pub fn count_float_bits(&self) -> Option<u32> { self.count_float_bits }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGatedEnergyOutcome {
    EmptyFallback { bits: u32 },

    MeanEnergy { bits: u32, count: u32 },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGatedEnergyFailureCause {

    Extent { energy: usize, metric_log: usize }, Math,
}

pub struct SourceGatedEnergyFailure {
    energy: Vec<f32>, metric_log: Vec<f32>, request: SourceGatedEnergyRequest,
    math: SourceGatedEnergyMath, progress: SourceGatedEnergyProgress, cause: SourceGatedEnergyFailureCause,
}
impl SourceGatedEnergyFailure {
    pub fn energy(&self) -> &[f32] { &self.energy }
    pub fn metric_log(&self) -> &[f32] { &self.metric_log }
    pub fn request(&self) -> SourceGatedEnergyRequest { self.request }
    pub fn math(&self) -> &SourceGatedEnergyMath { &self.math }
    pub fn progress(&self) -> &SourceGatedEnergyProgress { &self.progress }
    pub fn cause(&self) -> SourceGatedEnergyFailureCause { self.cause }
}
pub struct SourceGatedEnergyCompleted {
    energy: Vec<f32>, metric_log: Vec<f32>, request: SourceGatedEnergyRequest,
    math: SourceGatedEnergyMath, progress: SourceGatedEnergyProgress, outcome: SourceGatedEnergyOutcome,
}
pub struct SourceGatedEnergyParts {
    pub energy: Vec<f32>, pub metric_log: Vec<f32>, pub request: SourceGatedEnergyRequest,
    pub math: SourceGatedEnergyMath, pub progress: SourceGatedEnergyProgress, pub outcome: SourceGatedEnergyOutcome,
}
impl SourceGatedEnergyCompleted {
    pub fn energy(&self) -> &[f32] { &self.energy }
    pub fn metric_log(&self) -> &[f32] { &self.metric_log }
    pub fn request(&self) -> SourceGatedEnergyRequest { self.request }
    pub fn math(&self) -> &SourceGatedEnergyMath { &self.math }
    pub fn progress(&self) -> &SourceGatedEnergyProgress { &self.progress }
    pub fn outcome(&self) -> SourceGatedEnergyOutcome { self.outcome }
    pub fn into_parts(self) -> SourceGatedEnergyParts {
        SourceGatedEnergyParts { energy: self.energy, metric_log: self.metric_log, request: self.request,
            math: self.math, progress: self.progress, outcome: self.outcome }
    }
}
pub fn reduce_gated_energy(energy: Vec<f32>, metric_log: Vec<f32>, request: SourceGatedEnergyRequest, mut math: SourceGatedEnergyMath)
    -> Result<SourceGatedEnergyCompleted, SourceGatedEnergyFailure>
{
    let mut progress = SourceGatedEnergyProgress::new(request.pass);
    if energy.is_empty() || energy.len() != metric_log.len() || energy.len() > i32::MAX as usize {
        let cause = SourceGatedEnergyFailureCause::Extent { energy: energy.len(), metric_log: metric_log.len() };
        return Err(SourceGatedEnergyFailure { energy, metric_log, request, math, progress, cause });
    }
    match pass_graph(&energy, &metric_log, request, &mut math, &mut progress) {
        Ok(outcome) => Ok(SourceGatedEnergyCompleted { energy, metric_log, request, math, progress, outcome }),
        Err(Stopped) => Err(SourceGatedEnergyFailure { energy, metric_log, request, math, progress,
            cause: SourceGatedEnergyFailureCause::Math }),
    }
}

struct Stopped;
struct PassPcs {
    compare: [u32; 4], add: [u32; 4], increment: [u32; 4],
    tail_compare: u32, tail_add: u32, tail_increment: u32,
    convert: u32, divide: u32, fallback: u32, after_fallback: u32, after_mean: u32,
}
impl Pass {
    fn pcs(self) -> PassPcs {
        match self {
            Pass::First => PassPcs {
                compare: [0x552ab3, 0x552ac7, 0x552adc, 0x552af1],
                add: [0x552ab8, 0x552acc, 0x552ae1, 0x552af6],
                increment: [0x552abe, 0x552ad3, 0x552ae8, 0x552afd],
                tail_compare: 0x552a2e, tail_add: 0x552a33, tail_increment: 0x552a39,
                convert: 0x552a48, divide: 0x552a4c, fallback: 0x552b01,
                after_fallback: 0x552b09, after_mean: 0x552a50,
            },
            Pass::Second => PassPcs {
                compare: [0x552bd3, 0x552be7, 0x552bfc, 0x552c11],
                add: [0x552bd8, 0x552bec, 0x552c01, 0x552c16],
                increment: [0x552bde, 0x552bf3, 0x552c08, 0x552c1d],
                tail_compare: 0x552b4e, tail_add: 0x552b53, tail_increment: 0x552b59,
                convert: 0x552b7f, divide: 0x552b83, fallback: 0x552c21,

                after_fallback: 0x552c29, after_mean: 0x552b87,
            },
        }
    }
}
fn visit(energy: &[f32], metric_log: &[f32], request: SourceGatedEnergyRequest,
    math: &mut SourceGatedEnergyMath, progress: &mut SourceGatedEnergyProgress,
    index: usize, compare_pc: u32, add_pc: u32, increment_pc: u32) -> Result<(), Stopped>
{
    progress.index = index;
    progress.stage = SourceGatedEnergyStage::Compare;
    progress.next_pc = compare_pc;
    progress.current_flags = None;
    progress.pending_energy_bits = None;
    let metric_bits = metric_log[index].to_bits();
    progress.pending_metric_bits = Some(metric_bits);
    let flags = math.comparison(compare_pc, index, metric_bits, request.threshold_bits)?;
    progress.current_flags = Some(flags);
    progress.stage = SourceGatedEnergyStage::Branch;
    progress.next_pc = compare_pc + 3;

    if !flags.carry && !flags.zero {
        let energy_bits = energy[index].to_bits();
        progress.pending_energy_bits = Some(energy_bits);
        progress.stage = SourceGatedEnergyStage::Add;
        progress.next_pc = add_pc;
        let sum = math.word(add_pc, index, SourceGatedEnergyOperation::Add {
            sum_bits: progress.sum_bits, energy_bits })?;
        progress.sum_bits = sum;
        progress.stage = SourceGatedEnergyStage::Increment;
        progress.next_pc = increment_pc;
        progress.accepted_count += 1;
        progress.pending_energy_bits = None;
    }
    progress.completed_rows = index + 1;
    Ok(())
}
fn pass_graph(energy: &[f32], metric_log: &[f32], request: SourceGatedEnergyRequest,
    math: &mut SourceGatedEnergyMath, progress: &mut SourceGatedEnergyProgress)
    -> Result<SourceGatedEnergyOutcome, Stopped>
{
    let pcs = request.pass.pcs();

    let grouped = energy.len() & !3;
    for base in (0..grouped).step_by(4) {
        for lane in 0..4 {
            visit(energy, metric_log, request, math, progress, base + lane,
                pcs.compare[lane], pcs.add[lane], pcs.increment[lane])?;
        }
    }
    for index in grouped..energy.len() {
        visit(energy, metric_log, request, math, progress, index, pcs.tail_compare, pcs.tail_add, pcs.tail_increment)?;
    }
    progress.index = energy.len();
    if progress.accepted_count == 0 {
        progress.stage = SourceGatedEnergyStage::EmptyFallback;
        progress.next_pc = pcs.fallback;
        let outcome = SourceGatedEnergyOutcome::EmptyFallback { bits: request.empty_fallback_bits };
        progress.stage = SourceGatedEnergyStage::Complete;
        progress.next_pc = pcs.after_fallback;
        return Ok(outcome);
    }
    progress.stage = SourceGatedEnergyStage::ConvertCount;
    progress.next_pc = pcs.convert;

    let count_bits = math.word(pcs.convert, energy.len(), SourceGatedEnergyOperation::ConvertCount {
        value: progress.accepted_count as i32 })?;
    progress.count_float_bits = Some(count_bits);
    progress.stage = SourceGatedEnergyStage::Divide;
    progress.next_pc = pcs.divide;
    let bits = math.word(pcs.divide, energy.len(), SourceGatedEnergyOperation::Divide {
        sum_bits: progress.sum_bits, count_bits })?;
    progress.stage = SourceGatedEnergyStage::Complete;
    progress.next_pc = pcs.after_mean;
    Ok(SourceGatedEnergyOutcome::MeanEnergy { bits, count: progress.accepted_count })
}
