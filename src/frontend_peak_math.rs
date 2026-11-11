use std::{collections::TryReserveError, convert::Infallible};
use crate::source_scalar::{self as scalar, ExplicitExceptionState, LogArithmeticError, SourceLogWithState};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error,
    ScalarF32Outcome, ScalarF32SelectionOutcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, project_selection_f32_outcome,
    SourceScalarF32Projection, SourceSelectionF32Projection};

#[derive(Clone, Copy, Debug)]
pub struct SourcePeakConfiguration {
    pub seed_bits: u32,
    pub absolute_mask_bits: u32,
    pub scale_bits: u32,
    pub architecture: ScalarF32Architecture,
}
#[derive(Clone, Copy, Debug)]
pub enum SourcePeakBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourcePeakBindingFailure {
    configuration: SourcePeakConfiguration, entry: ExplicitExceptionState, cause: SourcePeakBindingCause,
}
impl SourcePeakBindingFailure {
    pub fn configuration(&self) -> SourcePeakConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourcePeakBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub enum SourcePeakStage { Maximum, OutputStorage, Divide, Multiply, Complete }
#[derive(Clone, Copy, Debug)]
pub struct SourcePeakAttempt {
    pub pc: u32, pub index: usize, pub stage: SourcePeakStage,
    pub left_bits: u32, pub right_bits: u32,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub result_bits: Option<u32>,
    pub scalar_reported: Option<ScalarF32Outcome>,
    pub selection_reported: Option<ScalarF32SelectionOutcome>,
    pub divide_reported: Option<Result<SourceLogWithState, LogArithmeticError>>,
}
pub enum SourcePeakMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), ReportedSelection(SourceSelectionF32Projection),
    Divide(LogArithmeticError), Storage(TryReserveError),
}
pub struct SourcePeakMathFailure { cause: SourcePeakMathCause, attempt: Option<SourcePeakAttempt> }
impl SourcePeakMathFailure {
    pub fn cause(&self) -> &SourcePeakMathCause { &self.cause }
    pub fn attempt(&self) -> Option<&SourcePeakAttempt> { self.attempt.as_ref() }
}
pub struct SourcePeakMath {
    configuration: SourcePeakConfiguration, state: ExplicitExceptionState, scalar: SourceScalarF32,
    latest: Option<SourcePeakAttempt>, failure: Option<SourcePeakMathFailure>,
}
impl SourcePeakMath {
    pub fn from_explicit_parts(configuration: SourcePeakConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourcePeakBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm {
                return Err(SourcePeakBindingCause::Architecture);
            }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourcePeakBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
                ScalarF32Opcode::Maxss, ScalarF32Opcode::Mulss]).map_err(SourcePeakBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourcePeakBindingCause::Scalar)
        };
        match bind() {
            Ok(scalar) => Ok(Self { configuration, state: entry, scalar, latest: None, failure: None }),
            Err(cause) => Err(SourcePeakBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourcePeakConfiguration { self.configuration }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn latest_attempt(&self) -> Option<&SourcePeakAttempt> { self.latest.as_ref() }
    pub fn failure(&self) -> Option<&SourcePeakMathFailure> { self.failure.as_ref() }
    fn perform(&mut self, pc: u32, index: usize, stage: SourcePeakStage, left: u32, right: u32)
        -> Result<u32, Stopped>
    {
        if self.failure.is_some() { return Err(Stopped); }
        let mut attempt = SourcePeakAttempt { pc, index, stage, left_bits: left, right_bits: right,
            entry: self.state, known_advanced: self.state, result_bits: None,
            scalar_reported: None, selection_reported: None, divide_reported: None };
        let result = (|| {
            if matches!(stage, SourcePeakStage::Divide) {
                let reported = scalar::raw_divss_with_explicit_state(left, right, self.state, self.configuration.architecture);
                attempt.divide_reported = Some(reported);
                return match reported {
                    Ok(value) => { self.state = value.exception_state; Ok(value.bits) }
                    Err(error) => {
                        self.state = divide_advanced(error, self.state);
                        Err(SourcePeakMathCause::Divide(error))
                    }
                };
            }
            let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                .ok_or(SourcePeakMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
            if matches!(stage, SourcePeakStage::Maximum) {
                let reported = self.scalar.execute_maxss(left, right, before).map_err(SourcePeakMathCause::Scalar)?;
                attempt.selection_reported = Some(reported);
                let projection = project_selection_f32_outcome(self.state, reported);
                return match projection {
                    SourceSelectionF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
                    SourceSelectionF32Projection::Trapped { advanced, .. }
                    | SourceSelectionF32Projection::Required { advanced, .. } => {
                        self.state = advanced; Err(SourcePeakMathCause::ReportedSelection(projection))
                    }
                    SourceSelectionF32Projection::ReportedStateRequired { known_advanced, .. } => {
                        self.state = known_advanced; Err(SourcePeakMathCause::ReportedSelection(projection))
                    }
                };
            }
            let reported = self.scalar.execute(ScalarF32Operation::Mulss { left, right }, before)
                .map_err(SourcePeakMathCause::Scalar)?;
            attempt.scalar_reported = Some(reported);
            let projection = project_scalar_f32_outcome(self.state, reported);
            match projection {
                SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
                SourceScalarF32Projection::Trapped { advanced, .. } => {
                    self.state = advanced; Err(SourcePeakMathCause::ReportedScalar(projection))
                }
                SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                    self.state = known_advanced; Err(SourcePeakMathCause::ReportedScalar(projection))
                }
            }
        })();
        attempt.known_advanced = self.state;
        match result {
            Ok(bits) => { attempt.result_bits = Some(bits); self.latest = Some(attempt); Ok(bits) }
            Err(cause) => {
                self.latest = Some(attempt);
                self.failure = Some(SourcePeakMathFailure { cause, attempt: Some(attempt) });
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

pub struct SourcePeakInPlaceProgress {
    peak_bits: u32, index: usize, stage: SourcePeakStage, divided_bits: Option<u32>, graph_started: bool,
}
impl SourcePeakInPlaceProgress {
    fn new(seed: u32) -> Self {
        Self { peak_bits: seed, index: 0, stage: SourcePeakStage::Maximum, divided_bits: None, graph_started: false }
    }
    pub fn peak_bits(&self) -> u32 { self.peak_bits }
    pub fn index(&self) -> usize { self.index }
    pub fn stage(&self) -> SourcePeakStage { self.stage }
    pub fn divided_bits(&self) -> Option<u32> { self.divided_bits }

    pub fn graph_started(&self) -> bool { self.graph_started }
}
pub struct SourcePeakProgress {
    state: SourcePeakInPlaceProgress, output: Vec<f32>,
}
impl SourcePeakProgress {
    fn new(seed: u32) -> Self { Self { state: SourcePeakInPlaceProgress::new(seed), output: Vec::new() } }
    pub fn peak_bits(&self) -> u32 { self.state.peak_bits() }
    pub fn index(&self) -> usize { self.state.index() }
    pub fn stage(&self) -> SourcePeakStage { self.state.stage() }
    pub fn divided_bits(&self) -> Option<u32> { self.state.divided_bits() }
    pub fn output(&self) -> &[f32] { &self.output }
}
#[derive(Clone, Copy, Debug)]
pub enum SourcePeakFailureCause { NonFiniteInput, LengthRequired, Math }
pub struct SourcePeakFailure {
    input: Vec<f32>, math: SourcePeakMath, progress: SourcePeakProgress, cause: SourcePeakFailureCause,
}
impl SourcePeakFailure {
    pub fn input(&self) -> &[f32] { &self.input }
    pub fn math(&self) -> &SourcePeakMath { &self.math }
    pub fn progress(&self) -> &SourcePeakProgress { &self.progress }
    pub fn cause(&self) -> SourcePeakFailureCause { self.cause }
}
pub fn peak_normalize09_with_source_math(input: Vec<f32>, mut math: SourcePeakMath)
    -> Result<(Vec<f32>, SourcePeakMath), SourcePeakFailure>
{
    let configuration = math.configuration;
    let mut progress = SourcePeakProgress::new(configuration.seed_bits);
    let cause = if input.iter().any(|v| !v.is_finite()) { Some(SourcePeakFailureCause::NonFiniteInput) }
        else if input.len() > i32::MAX as usize { Some(SourcePeakFailureCause::LengthRequired) }
        else if peak_graph(PeakStorage::Separate { input: &input, output: &mut progress.output },
            configuration.absolute_mask_bits, configuration.scale_bits, &mut math, &mut progress.state).is_err() {
            Some(SourcePeakFailureCause::Math)
        } else { None };
    match cause {
        Some(cause) => Err(SourcePeakFailure { input, math, progress, cause }),
        None => Ok((progress.output, math)),
    }
}

pub struct SourcePeakInPlaceFailure {
    samples: Vec<f32>, math: SourcePeakMath, progress: SourcePeakInPlaceProgress, cause: SourcePeakFailureCause,
}
impl SourcePeakInPlaceFailure {
    pub fn samples(&self) -> &[f32] { &self.samples }
    pub fn math(&self) -> &SourcePeakMath { &self.math }
    pub fn progress(&self) -> &SourcePeakInPlaceProgress { &self.progress }
    pub fn cause(&self) -> SourcePeakFailureCause { self.cause }
}

pub fn peak_normalize09_in_place_with_source_math(samples: Vec<f32>, math: SourcePeakMath)
    -> Result<(Vec<f32>, SourcePeakMath), SourcePeakInPlaceFailure>
{
    peak_normalize09_in_place_with_admission(samples, math, PeakAdmission::Legacy)
}
pub(crate) fn peak_normalize09_in_place_with_source_bits(samples: Vec<f32>, math: SourcePeakMath)
    -> Result<(Vec<f32>, SourcePeakMath), SourcePeakInPlaceFailure>
{
    peak_normalize09_in_place_with_admission(samples, math, PeakAdmission::SourceBits)
}
enum PeakAdmission { Legacy, SourceBits }
fn peak_normalize09_in_place_with_admission(mut samples: Vec<f32>, mut math: SourcePeakMath, admission: PeakAdmission)
    -> Result<(Vec<f32>, SourcePeakMath), SourcePeakInPlaceFailure>
{
    let configuration = math.configuration;
    let mut progress = SourcePeakInPlaceProgress::new(configuration.seed_bits);
    let cause = if matches!(admission, PeakAdmission::Legacy) && samples.iter().any(|v| !v.is_finite()) { Some(SourcePeakFailureCause::NonFiniteInput) }
        else if samples.len() > i32::MAX as usize { Some(SourcePeakFailureCause::LengthRequired) }

        else if math.failure.is_some() { Some(SourcePeakFailureCause::Math) }
        else if peak_graph(PeakStorage::InPlace { samples: &mut samples }, configuration.absolute_mask_bits,
            configuration.scale_bits, &mut math, &mut progress).is_err() {
            Some(SourcePeakFailureCause::Math)
        } else { None };
    match cause {
        Some(cause) => Err(SourcePeakInPlaceFailure { samples, math, progress, cause }),
        None => Ok((samples, math)),
    }
}

struct Stopped;
trait PeakOps {
    type Error;
    fn operation(&mut self, pc: u32, index: usize, stage: SourcePeakStage, left: u32, right: u32)
        -> Result<u32, Self::Error>;
    fn reserve(&mut self, output: &mut Vec<f32>, count: usize) -> Result<(), Self::Error>;
}
impl PeakOps for SourcePeakMath {
    type Error = Stopped;
    fn operation(&mut self, pc: u32, index: usize, stage: SourcePeakStage, left: u32, right: u32)
        -> Result<u32, Stopped> { self.perform(pc, index, stage, left, right) }
    fn reserve(&mut self, output: &mut Vec<f32>, count: usize) -> Result<(), Stopped> {
        if self.failure.is_some() { return Err(Stopped); }
        output.try_reserve_exact(count).map_err(|error| {
            self.failure = Some(SourcePeakMathFailure { cause: SourcePeakMathCause::Storage(error), attempt: None });
            Stopped
        })
    }
}
struct HostPeak;
impl PeakOps for HostPeak {
    type Error = Infallible;
    fn operation(&mut self, _: u32, _: usize, stage: SourcePeakStage, left: u32, right: u32)
        -> Result<u32, Infallible>
    {
        let left = f32::from_bits(left); let right = f32::from_bits(right);
        Ok(match stage {
            SourcePeakStage::Maximum => right.max(left),
            SourcePeakStage::Divide => left / right,
            SourcePeakStage::Multiply => left * right,
            SourcePeakStage::OutputStorage | SourcePeakStage::Complete => unreachable!(),
        }.to_bits())
    }
    fn reserve(&mut self, output: &mut Vec<f32>, count: usize) -> Result<(), Infallible> {
        output.reserve_exact(count); Ok(())
    }
}
pub(super) fn host_peak(src: &[f32]) -> Vec<f32> {
    let mut progress = SourcePeakProgress::new(0.01_f32.to_bits());
    match peak_graph(PeakStorage::Separate { input: src, output: &mut progress.output },
        0x7fff_ffff, 0.9_f32.to_bits(), &mut HostPeak, &mut progress.state) {
        Ok(()) => progress.output, Err(error) => match error {},
    }
}
enum PeakStorage<'a> {
    Separate { input: &'a [f32], output: &'a mut Vec<f32> },
    InPlace { samples: &'a mut [f32] },
}
impl PeakStorage<'_> {
    fn len(&self) -> usize { match self { Self::Separate { input, .. } => input.len(), Self::InPlace { samples } => samples.len() } }
    fn read_bits(&self, index: usize) -> u32 { match self {
        Self::Separate { input, .. } => input[index].to_bits(), Self::InPlace { samples } => samples[index].to_bits(),
    } }
    fn reserve<O: PeakOps>(&mut self, ops: &mut O) -> Result<(), O::Error> { match self {
        Self::Separate { input, output } => ops.reserve(output, input.len()), Self::InPlace { .. } => Ok(()),
    } }
    fn store(&mut self, index: usize, bits: u32) { match self {
        Self::Separate { output, .. } => output.push(f32::from_bits(bits)),
        Self::InPlace { samples } => samples[index] = f32::from_bits(bits),
    } }
}
fn peak_graph<O: PeakOps>(mut storage: PeakStorage<'_>, absolute_mask: u32, scale: u32, ops: &mut O, progress: &mut SourcePeakInPlaceProgress)
    -> Result<(), O::Error>
{
    progress.graph_started = true;
    let count = storage.len();
    let grouped = count & !3;
    for base in (0..grouped).step_by(4) {
        let loaded = [storage.read_bits(base), storage.read_bits(base + 1), storage.read_bits(base + 2), storage.read_bits(base + 3)];
        for (lane, pc) in [0x415f1e, 0x415f25, 0x415f2c, 0x415f33].into_iter().enumerate() {
            progress.index = base + lane;
            progress.peak_bits = ops.operation(pc, progress.index, SourcePeakStage::Maximum,
                loaded[lane] & absolute_mask, progress.peak_bits)?;
        }
    }
    for index in grouped..count {
        progress.index = index;
        progress.peak_bits = ops.operation(0x415f68, index, SourcePeakStage::Maximum,
            storage.read_bits(index) & absolute_mask, progress.peak_bits)?;
    }
    progress.index = 0;
    progress.stage = SourcePeakStage::OutputStorage;
    storage.reserve(ops)?;
    for index in 0..count {
        progress.index = index;
        progress.stage = SourcePeakStage::Divide;
        progress.divided_bits = None;
        let divided = ops.operation(0x415f9c, index, SourcePeakStage::Divide, storage.read_bits(index), progress.peak_bits)?;
        progress.divided_bits = Some(divided);
        progress.stage = SourcePeakStage::Multiply;
        let result = ops.operation(0x415fa0, index, SourcePeakStage::Multiply, divided, scale)?;
        storage.store(index, result);
    }
    progress.index = count;
    progress.stage = SourcePeakStage::Complete;
    Ok(())
}
