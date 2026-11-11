use std::{collections::TryReserveError, convert::Infallible};
use crate::source_scalar::{self as scalar, ExplicitExceptionState};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error, ScalarF32Outcome, ScalarF32SelectionOutcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, project_selection_f32_outcome,
    SourceScalarF32Projection, SourceSelectionF32Projection};

#[derive(Clone, Copy, Debug)]
pub struct SourceSlewConfiguration { pub step_bits: u32, pub architecture: ScalarF32Architecture }
#[derive(Clone, Copy, Debug)]
pub enum SourceSlewBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceSlewBindingFailure {
    configuration: SourceSlewConfiguration, entry: ExplicitExceptionState, cause: SourceSlewBindingCause,
}
impl SourceSlewBindingFailure {
    pub fn configuration(&self) -> SourceSlewConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceSlewBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceSlewOperation { Add, Maximum }
#[derive(Clone, Copy, Debug)]
pub enum SourceSlewDirection { Forward, ReverseRemainder, ReverseGroup, Snapshot, Complete }
#[derive(Clone, Copy, Debug)]
pub struct SourceSlewAttempt {
    pub round: usize, pub index: usize, pub pc: u32, pub operation: SourceSlewOperation,
    pub left_bits: u32, pub right_bits: u32,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub scalar_reported: Option<ScalarF32Outcome>, pub selection_reported: Option<ScalarF32SelectionOutcome>,
    pub result_bits: Option<u32>,
}
pub enum SourceSlewMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), ReportedSelection(SourceSelectionF32Projection),
    Snapshot { round: usize, error: TryReserveError },
}
pub struct SourceSlewMathFailure { cause: SourceSlewMathCause, attempt: Option<SourceSlewAttempt> }
impl SourceSlewMathFailure {
    pub fn cause(&self) -> &SourceSlewMathCause { &self.cause }
    pub fn attempt(&self) -> Option<&SourceSlewAttempt> { self.attempt.as_ref() }
}
pub struct SourceSlewMath {
    configuration: SourceSlewConfiguration, state: ExplicitExceptionState, scalar: SourceScalarF32,
    latest: Option<SourceSlewAttempt>, failure: Option<SourceSlewMathFailure>,
}
impl SourceSlewMath {
    pub fn from_explicit_parts(configuration: SourceSlewConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceSlewBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm { return Err(SourceSlewBindingCause::Architecture); }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceSlewBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Addss, ScalarF32Opcode::Maxss])
                .map_err(SourceSlewBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceSlewBindingCause::Scalar)
        };
        match bind() {
            Ok(scalar) => Ok(Self { configuration, state: entry, scalar, latest: None, failure: None }),
            Err(cause) => Err(SourceSlewBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceSlewConfiguration { self.configuration }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn latest_attempt(&self) -> Option<&SourceSlewAttempt> { self.latest.as_ref() }
    pub fn failure(&self) -> Option<&SourceSlewMathFailure> { self.failure.as_ref() }
    fn perform(&mut self, progress: &SourceSlewProgress, operation: SourceSlewOperation, left: u32, right: u32)
        -> Result<u32, Stopped>
    {
        if self.failure.is_some() { return Err(Stopped); }
        let mut attempt = SourceSlewAttempt { round: progress.round, index: progress.index, pc: progress.pc, operation,
            left_bits: left, right_bits: right, entry: self.state, known_advanced: self.state,
            scalar_reported: None, selection_reported: None, result_bits: None };
        let result = (|| {
            let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                .ok_or(SourceSlewMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
            match operation {
                SourceSlewOperation::Add => {
                    let reported = self.scalar.execute(ScalarF32Operation::Addss { left, right }, before)
                        .map_err(SourceSlewMathCause::Scalar)?;
                    attempt.scalar_reported = Some(reported);
                    let projection = project_scalar_f32_outcome(self.state, reported);
                    match projection {
                        SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
                        SourceScalarF32Projection::Trapped { advanced, .. } => {
                            self.state = advanced; Err(SourceSlewMathCause::ReportedScalar(projection))
                        }
                        SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                            self.state = known_advanced; Err(SourceSlewMathCause::ReportedScalar(projection))
                        }
                    }
                }
                SourceSlewOperation::Maximum => {
                    let reported = self.scalar.execute_maxss(left, right, before).map_err(SourceSlewMathCause::Scalar)?;
                    attempt.selection_reported = Some(reported);
                    let projection = project_selection_f32_outcome(self.state, reported);
                    match projection {
                        SourceSelectionF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
                        SourceSelectionF32Projection::Trapped { advanced, .. }
                        | SourceSelectionF32Projection::Required { advanced, .. } => {
                            self.state = advanced; Err(SourceSlewMathCause::ReportedSelection(projection))
                        }
                        SourceSelectionF32Projection::ReportedStateRequired { known_advanced, .. } => {
                            self.state = known_advanced; Err(SourceSlewMathCause::ReportedSelection(projection))
                        }
                    }
                }
            }
        })();
        attempt.known_advanced = self.state;
        match result {
            Ok(bits) => { attempt.result_bits = Some(bits); self.latest = Some(attempt); Ok(bits) }
            Err(cause) => {
                self.latest = Some(attempt); self.failure = Some(SourceSlewMathFailure { cause, attempt: Some(attempt) }); Err(Stopped)
            }
        }
    }
}
pub struct SourceSlewProgress {
    round: usize, index: usize, pc: u32, direction: SourceSlewDirection, pending_add: Option<u32>,
}
impl SourceSlewProgress {
    fn new() -> Self { Self { round: 0, index: 0, pc: 0x552cdb, direction: SourceSlewDirection::Forward, pending_add: None } }
    pub fn round(&self) -> usize { self.round }
    pub fn index(&self) -> usize { self.index }
    pub fn pc(&self) -> u32 { self.pc }
    pub fn direction(&self) -> SourceSlewDirection { self.direction }
    pub fn pending_add(&self) -> Option<u32> { self.pending_add }
}
pub enum SourceSlewCause { Input(String), CountRequired { count: usize }, Storage(TryReserveError), Math }
pub struct SourceSlewFailure {
    metric_log: Vec<f32>, target_db: f32, envelope: Vec<f32>, rounds: Vec<Vec<f32>>,
    math: SourceSlewMath, progress: SourceSlewProgress, cause: SourceSlewCause,
}
impl SourceSlewFailure {
    pub fn metric_log(&self) -> &[f32] { &self.metric_log }
    pub fn target_db(&self) -> f32 { self.target_db }
    pub fn envelope(&self) -> &[f32] { &self.envelope }
    pub fn rounds(&self) -> &[Vec<f32>] { &self.rounds }
    pub fn math(&self) -> &SourceSlewMath { &self.math }
    pub fn progress(&self) -> &SourceSlewProgress { &self.progress }
    pub fn cause(&self) -> &SourceSlewCause { &self.cause }
}
struct SlewPassesCompleted {
    metric_log: Vec<f32>, target_db: f32, envelope: Vec<f32>, rounds: Vec<Vec<f32>>,
    math: SourceSlewMath, progress: SourceSlewProgress,
}
enum PostLogAdmission { Legacy, SourceBits }
fn complete_slew_passes(metric_log: Vec<f32>, target_db: f32, math: SourceSlewMath)
    -> Result<SlewPassesCompleted, SourceSlewFailure>
{
    complete_slew_passes_with_admission(metric_log, target_db, math, PostLogAdmission::Legacy)
}
fn complete_slew_passes_with_admission(metric_log: Vec<f32>, target_db: f32, mut math: SourceSlewMath, admission: PostLogAdmission)
    -> Result<SlewPassesCompleted, SourceSlewFailure>
{
    let mut envelope = Vec::new(); let mut rounds = Vec::new(); let mut progress = SourceSlewProgress::new();
    let result = (|| {
        match admission {
            PostLogAdmission::Legacy => super::validate_slew_input(&metric_log, target_db),

            PostLogAdmission::SourceBits => if metric_log.is_empty() { Err("invalid gain envelope/target".into()) } else { Ok(()) },
        }.map_err(SourceSlewCause::Input)?;

        if metric_log.len() > i32::MAX as usize { return Err(SourceSlewCause::CountRequired { count: metric_log.len() }); }
        envelope.try_reserve_exact(metric_log.len()).map_err(SourceSlewCause::Storage)?;
        envelope.extend_from_slice(&metric_log);
        let step = math.configuration.step_bits;
        slew_graph(&mut envelope, &mut rounds, step, &mut math, &mut progress).map_err(|_| SourceSlewCause::Math)
    })();
    match result {
        Err(cause) => Err(SourceSlewFailure { metric_log, target_db, envelope, rounds, math, progress, cause }),
        Ok(()) => Ok(SlewPassesCompleted { metric_log, target_db, envelope, rounds, math, progress }),
    }
}
pub fn gain_slew_from_log_with_source_math(metric_log: Vec<f32>, target_db: f32, math: SourceSlewMath)
    -> Result<((Vec<Vec<f32>>, Vec<f32>), SourceSlewMath, Vec<f32>), SourceSlewFailure>
{
    let completed = complete_slew_passes(metric_log, target_db, math)?;
    let gain_db = super::host_gain_from_envelope(&completed.envelope, completed.target_db);
    Ok(((completed.rounds, gain_db), completed.math, completed.metric_log))
}
enum SlewFinalGainOwner {
    Passes { failure: SourceSlewFailure, final_math: super::SourceFinalGainMath },
    Final { metric_log: Vec<f32>, target_db: f32, rounds: Vec<Vec<f32>>, math: SourceSlewMath,
        progress: SourceSlewProgress, failure: super::SourceFinalGainFailure },
}
pub enum SourceSlewFinalGainError<'a> { Slew(&'a SourceSlewFailure), Final(&'a super::SourceFinalGainFailure) }
pub struct SourceSlewFinalGainFailure { owner: SlewFinalGainOwner }
impl SourceSlewFinalGainFailure {
    pub fn error(&self) -> SourceSlewFinalGainError<'_> {
        match &self.owner {
            SlewFinalGainOwner::Passes { failure, .. } => SourceSlewFinalGainError::Slew(failure),
            SlewFinalGainOwner::Final { failure, .. } => SourceSlewFinalGainError::Final(failure),
        }
    }
    pub fn metric_log(&self) -> &[f32] {
        match &self.owner { SlewFinalGainOwner::Passes { failure, .. } => failure.metric_log(),
            SlewFinalGainOwner::Final { metric_log, .. } => metric_log }
    }
    pub fn target_db(&self) -> f32 {
        match &self.owner { SlewFinalGainOwner::Passes { failure, .. } => failure.target_db(),
            SlewFinalGainOwner::Final { target_db, .. } => *target_db }
    }
    pub fn envelope(&self) -> &[f32] {
        match &self.owner { SlewFinalGainOwner::Passes { failure, .. } => failure.envelope(),
            SlewFinalGainOwner::Final { failure, .. } => failure.envelope() }
    }
    pub fn rounds(&self) -> &[Vec<f32>] {
        match &self.owner { SlewFinalGainOwner::Passes { failure, .. } => failure.rounds(),
            SlewFinalGainOwner::Final { rounds, .. } => rounds }
    }
    pub fn progress(&self) -> &SourceSlewProgress {
        match &self.owner { SlewFinalGainOwner::Passes { failure, .. } => failure.progress(),
            SlewFinalGainOwner::Final { progress, .. } => progress }
    }
    pub fn slew_math(&self) -> &SourceSlewMath {
        match &self.owner { SlewFinalGainOwner::Passes { failure, .. } => failure.math(),
            SlewFinalGainOwner::Final { math, .. } => math }
    }
    pub fn final_gain_math(&self) -> &super::SourceFinalGainMath {
        match &self.owner { SlewFinalGainOwner::Passes { final_math, .. } => final_math,
            SlewFinalGainOwner::Final { failure, .. } => failure.math() }
    }
}

pub fn gain_slew_from_log_with_source_final_gain(metric_log: Vec<f32>, target_db: f32,
    math: SourceSlewMath, final_math: super::SourceFinalGainMath)
    -> Result<((Vec<Vec<f32>>, Vec<f32>), SourceSlewMath, super::SourceFinalGainMath, Vec<f32>), SourceSlewFinalGainFailure>
{
    gain_slew_final_with_admission(metric_log, target_db, math, final_math, PostLogAdmission::Legacy)
}
pub(super) fn gain_slew_from_log_with_source_final_gain_bits(metric_log: Vec<f32>, target_db: f32,
    math: SourceSlewMath, final_math: super::SourceFinalGainMath)
    -> Result<((Vec<Vec<f32>>, Vec<f32>), SourceSlewMath, super::SourceFinalGainMath, Vec<f32>), SourceSlewFinalGainFailure>
{
    gain_slew_final_with_admission(metric_log, target_db, math, final_math, PostLogAdmission::SourceBits)
}
fn gain_slew_final_with_admission(metric_log: Vec<f32>, target_db: f32,
    math: SourceSlewMath, final_math: super::SourceFinalGainMath, admission: PostLogAdmission)
    -> Result<((Vec<Vec<f32>>, Vec<f32>), SourceSlewMath, super::SourceFinalGainMath, Vec<f32>), SourceSlewFinalGainFailure>
{
    let completed = match complete_slew_passes_with_admission(metric_log, target_db, math, admission) {
        Ok(completed) => completed,
        Err(failure) => return Err(SourceSlewFinalGainFailure { owner: SlewFinalGainOwner::Passes { failure, final_math } }),
    };
    let SlewPassesCompleted { metric_log, target_db, envelope, rounds, math, progress } = completed;
    match super::final_gain_math::final_gain_with_source_math(envelope, target_db.to_bits(), final_math) {
        Ok((gain_db, final_math)) => Ok(((rounds, gain_db), math, final_math, metric_log)),
        Err(failure) => Err(SourceSlewFinalGainFailure { owner: SlewFinalGainOwner::Final {
            metric_log, target_db, rounds, math, progress, failure } }),
    }
}

enum GainOwner {
    Input { error: String, math: SourceSlewMath },
    Suffix(super::GainSuffixChoiceFailure<SourceSlewMath, SourceSlewFailure>),
}
pub struct SourceSlewGainFailure { input: Vec<f32>, target_db: f32, owner: GainOwner }
impl SourceSlewGainFailure {
    pub fn input(&self) -> &[f32] { &self.input }
    pub fn target_db(&self) -> f32 { self.target_db }
    pub fn math(&self) -> &SourceSlewMath {
        match &self.owner {
            GainOwner::Input { math, .. } => math,
            GainOwner::Suffix(super::GainSuffixChoiceFailure::Before { owner, .. }) => owner,
            GainOwner::Suffix(super::GainSuffixChoiceFailure::Slew { failure, .. }) => failure.math(),
        }
    }
    pub fn input_error(&self) -> Option<&str> { match &self.owner { GainOwner::Input { error, .. } => Some(error), _ => None } }
    pub fn before_slew_error(&self) -> Option<&str> {
        match &self.owner { GainOwner::Suffix(super::GainSuffixChoiceFailure::Before { failure, .. }) => Some(&failure.error), _ => None }
    }
    pub fn slew_failure(&self) -> Option<&SourceSlewFailure> {
        match &self.owner { GainOwner::Suffix(super::GainSuffixChoiceFailure::Slew { failure, .. }) => Some(failure), _ => None }
    }
    pub fn first_filter(&self) -> Option<&[f32]> {
        match &self.owner { GainOwner::Input { .. } => None,
            GainOwner::Suffix(super::GainSuffixChoiceFailure::Before { failure, .. }) => Some(&failure.first_filter),
            GainOwner::Suffix(super::GainSuffixChoiceFailure::Slew { first_filter, .. }) => Some(first_filter) }
    }
    pub fn second_filter(&self) -> Option<&[f32]> {
        match &self.owner { GainOwner::Input { .. } => None,
            GainOwner::Suffix(super::GainSuffixChoiceFailure::Before { failure, .. }) => Some(&failure.second_filter),
            GainOwner::Suffix(super::GainSuffixChoiceFailure::Slew { second_filter, .. }) => Some(second_filter) }
    }
    pub fn energy(&self) -> Option<&[f32]> {
        match &self.owner { GainOwner::Input { .. } => None,
            GainOwner::Suffix(super::GainSuffixChoiceFailure::Before { failure, .. }) => Some(&failure.energy),
            GainOwner::Suffix(super::GainSuffixChoiceFailure::Slew { energy, .. }) => Some(energy) }
    }
}
pub fn frontend_gain_table_with_source_slew(input: Vec<f32>, target_db: f32, math: SourceSlewMath)
    -> Result<(super::FrontendGainStages, SourceSlewMath, Vec<f32>), SourceSlewGainFailure>
{
    if let Err(error) = super::validate_gain_input(&input) {
        return Err(SourceSlewGainFailure { input, target_db, owner: GainOwner::Input { error, math } });
    }
    let (first_filter, second_filter) = super::host_gain_filters(&input);
    match super::gain_table_suffix_with(input.len(), target_db, first_filter, second_filter, math, |metric_log, target, owner| {
        gain_slew_from_log_with_source_math(metric_log, target, owner)
            .map(|((rounds, gain_db), owner, metric_log)| super::SlewCompleted { rounds, gain_db, owner, metric_log })
    }) {
        Ok((stages, math)) => Ok((stages, math, input)),
        Err(failure) => Err(SourceSlewGainFailure { input, target_db, owner: GainOwner::Suffix(failure) }),
    }
}

struct Stopped;
trait SlewOps {
    type Error;
    fn operation(&mut self, progress: &SourceSlewProgress, operation: SourceSlewOperation, left: u32, right: u32) -> Result<u32, Self::Error>;
    fn snapshot(&mut self, round: usize, envelope: &[f32], rounds: &mut Vec<Vec<f32>>) -> Result<(), Self::Error>;
}
impl SlewOps for SourceSlewMath {
    type Error = Stopped;
    fn operation(&mut self, progress: &SourceSlewProgress, operation: SourceSlewOperation, left: u32, right: u32) -> Result<u32, Stopped> {
        self.perform(progress, operation, left, right)
    }
    fn snapshot(&mut self, round: usize, envelope: &[f32], rounds: &mut Vec<Vec<f32>>) -> Result<(), Stopped> {
        if self.failure.is_some() { return Err(Stopped); }
        let result = (|| {
            rounds.try_reserve(1)?;
            let mut copy = Vec::new(); copy.try_reserve_exact(envelope.len())?;
            copy.extend_from_slice(envelope); rounds.push(copy); Ok::<(), TryReserveError>(())
        })();
        result.map_err(|error| { self.failure = Some(SourceSlewMathFailure {
            cause: SourceSlewMathCause::Snapshot { round, error }, attempt: None }); Stopped })
    }
}
struct HostSlew;
impl SlewOps for HostSlew {
    type Error = Infallible;
    fn operation(&mut self, _: &SourceSlewProgress, operation: SourceSlewOperation, left: u32, right: u32) -> Result<u32, Infallible> {
        let left = f32::from_bits(left); let right = f32::from_bits(right);
        Ok(match operation { SourceSlewOperation::Add => left + right, SourceSlewOperation::Maximum => left.max(right) }.to_bits())
    }
    fn snapshot(&mut self, _: usize, envelope: &[f32], rounds: &mut Vec<Vec<f32>>) -> Result<(), Infallible> {
        rounds.push(envelope.to_vec()); Ok(())
    }
}
pub(super) fn host_slew(log: &[f32]) -> (Vec<Vec<f32>>, Vec<f32>) {
    let mut envelope = log.to_vec(); let mut rounds = Vec::new(); let mut progress = SourceSlewProgress::new();
    match slew_graph(&mut envelope, &mut rounds, (-0.03_f32).to_bits(), &mut HostSlew, &mut progress) {
        Ok(()) => (rounds, envelope), Err(error) => match error {},
    }
}
fn apply<O: SlewOps>(envelope: &mut [f32], index: usize, neighbor: usize, step: u32, pcs: (u32, u32),
    ops: &mut O, progress: &mut SourceSlewProgress) -> Result<(), O::Error>
{
    progress.index = index; progress.pc = pcs.0; progress.pending_add = None;
    let added = ops.operation(progress, SourceSlewOperation::Add, envelope[neighbor].to_bits(), step)?;
    progress.pending_add = Some(added); progress.pc = pcs.1;
    let selected = ops.operation(progress, SourceSlewOperation::Maximum, envelope[index].to_bits(), added)?;
    envelope[index] = f32::from_bits(selected);
    progress.pending_add = None;
    Ok(())
}
fn slew_graph<O: SlewOps>(envelope: &mut [f32], rounds: &mut Vec<Vec<f32>>, step: u32,
    ops: &mut O, progress: &mut SourceSlewProgress) -> Result<(), O::Error>
{
    for round in 0..5 {
        progress.round = round; progress.direction = SourceSlewDirection::Forward;
        for index in 1..envelope.len() { apply(envelope, index, index - 1, step, (0x552d2d, 0x552d31), ops, progress)?; }
        if envelope.len() >= 2 {
            let count = envelope.len() - 1;
            let remainder = count & 3;
            let mut remaining = count;
            progress.direction = SourceSlewDirection::ReverseRemainder;
            for _ in 0..remainder {
                let index = remaining - 1;
                apply(envelope, index, index + 1, step, (0x552d81, 0x552d85), ops, progress)?;
                remaining -= 1;
            }
            progress.direction = SourceSlewDirection::ReverseGroup;
            while remaining != 0 {
                for pcs in [(0x552daa, 0x552db3), (0x552dc5, 0x552dcf), (0x552de3, 0x552ded), (0x552e01, 0x552e0b)] {
                    let index = remaining - 1;
                    apply(envelope, index, index + 1, step, pcs, ops, progress)?;
                    remaining -= 1;
                }
            }
        }
        progress.direction = SourceSlewDirection::Snapshot; progress.pc = 0x552cf0;
        ops.snapshot(round, envelope, rounds)?;
    }
    progress.direction = SourceSlewDirection::Complete; progress.pc = 0x552e24;
    Ok(())
}
