use std::{collections::TryReserveError, convert::Infallible};
use crate::source_scalar::{self as scalar, ExplicitExceptionState, SourceF32MinMaxOpcode,
    SourceF32Selection, SourceF32SelectionError};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error, ScalarF32Outcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

#[derive(Clone, Copy, Debug)]
pub struct SourceFinalGainConfiguration { pub architecture: ScalarF32Architecture, pub cap_bits: u32 }
#[derive(Clone, Copy, Debug)]
pub enum SourceFinalGainBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceFinalGainBindingFailure {
    configuration: SourceFinalGainConfiguration, entry: ExplicitExceptionState, cause: SourceFinalGainBindingCause,
}
impl SourceFinalGainBindingFailure {
    pub fn configuration(&self) -> SourceFinalGainConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceFinalGainBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceFinalGainOperation { Subtract, Minimum }
#[derive(Clone, Copy, Debug)]
pub struct SourceFinalGainAttempt {
    pub index: usize, pub pc: u32, pub operation: SourceFinalGainOperation,
    pub left_bits: u32, pub right_bits: u32,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub scalar_reported: Option<Result<ScalarF32Outcome, ScalarF32Error>>,
    pub scalar_projection: Option<SourceScalarF32Projection>,
    pub minimum_reported: Option<Result<SourceF32Selection, SourceF32SelectionError>>,
    pub result_bits: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceFinalGainMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), Minimum(SourceF32SelectionError),
}
pub struct SourceFinalGainMathFailure { cause: SourceFinalGainMathCause, attempt: SourceFinalGainAttempt }
impl SourceFinalGainMathFailure {
    pub fn cause(&self) -> SourceFinalGainMathCause { self.cause }
    pub fn attempt(&self) -> &SourceFinalGainAttempt { &self.attempt }
}
pub struct SourceFinalGainMath {
    configuration: SourceFinalGainConfiguration, entry: ExplicitExceptionState, state: ExplicitExceptionState,
    scalar: SourceScalarF32, latest: Option<SourceFinalGainAttempt>, failure: Option<SourceFinalGainMathFailure>,
}
impl SourceFinalGainMath {
    pub fn from_explicit_parts(configuration: SourceFinalGainConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceFinalGainBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm { return Err(SourceFinalGainBindingCause::Architecture); }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceFinalGainBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Subss])
                .map_err(SourceFinalGainBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceFinalGainBindingCause::Scalar)
        };
        match bind() {
            Ok(scalar) => Ok(Self { configuration, entry, state: entry, scalar, latest: None, failure: None }),
            Err(cause) => Err(SourceFinalGainBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceFinalGainConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn latest_attempt(&self) -> Option<&SourceFinalGainAttempt> { self.latest.as_ref() }
    pub fn failure(&self) -> Option<&SourceFinalGainMathFailure> { self.failure.as_ref() }
    fn perform_source(&mut self, index: usize, operation: SourceFinalGainOperation, left: u32, right: u32)
        -> Result<u32, Stopped>
    {
        if self.failure.is_some() { return Err(Stopped); }
        let pc = match operation { SourceFinalGainOperation::Subtract => 0x552ec3, SourceFinalGainOperation::Minimum => 0x552ec8 };
        let mut attempt = SourceFinalGainAttempt { index, pc, operation, left_bits: left, right_bits: right,
            entry: self.state, known_advanced: self.state, scalar_reported: None, scalar_projection: None,
            minimum_reported: None, result_bits: None };
        self.latest = Some(attempt);
        let result = (|| {
            match operation {
                SourceFinalGainOperation::Subtract => {
                    let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                        .ok_or(SourceFinalGainMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
                    let reported = self.scalar.execute(ScalarF32Operation::Subss { left, right }, before);
                    attempt.scalar_reported = Some(reported);
                    self.latest = Some(attempt);
                    let reported = reported.map_err(SourceFinalGainMathCause::Scalar)?;
                    let projection = project_scalar_f32_outcome(self.state, reported);
                    attempt.scalar_projection = Some(projection);
                    self.latest = Some(attempt);
                    match projection {
                        SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
                        SourceScalarF32Projection::Trapped { advanced, .. } => {
                            self.state = advanced; Err(SourceFinalGainMathCause::ReportedScalar(projection))
                        }
                        SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                            self.state = known_advanced; Err(SourceFinalGainMathCause::ReportedScalar(projection))
                        }
                    }
                }
                SourceFinalGainOperation::Minimum => {
                    let reported = scalar::minmax_f32_with_explicit_state(SourceF32MinMaxOpcode::Minss,
                        left, right, self.state, self.configuration.architecture);

                    attempt.minimum_reported = Some(reported);
                    self.latest = Some(attempt);
                    match reported {
                        Ok(value) => { self.state = value.exception_state; Ok(value.bits) }
                        Err(error) => {
                            self.state = error.known_advanced_state(self.state);
                            Err(SourceFinalGainMathCause::Minimum(error))
                        }
                    }
                }
            }
        })();
        attempt.known_advanced = self.state;
        match result {
            Ok(bits) => { attempt.result_bits = Some(bits); self.latest = Some(attempt); Ok(bits) }
            Err(cause) => {
                self.latest = Some(attempt); self.failure = Some(SourceFinalGainMathFailure { cause, attempt }); Err(Stopped)
            }
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceFinalGainStage { Reserve, Subtract, Minimum, Store, Complete }
pub struct SourceFinalGainProgress {
    index: usize, stage: SourceFinalGainStage, pc: u32, pending_difference: Option<u32>, output: Vec<f32>,
}
impl SourceFinalGainProgress {
    fn new(output: Vec<f32>) -> Self {
        Self { index: 0, stage: SourceFinalGainStage::Reserve, pc: 0x552ec3, pending_difference: None, output }
    }
    pub fn index(&self) -> usize { self.index }
    pub fn stage(&self) -> SourceFinalGainStage { self.stage }
    pub fn pc(&self) -> u32 { self.pc }
    pub fn pending_difference(&self) -> Option<u32> { self.pending_difference }
    pub fn output(&self) -> &[f32] { &self.output }
}
pub enum SourceFinalGainCause { Storage(TryReserveError), Math }
pub struct SourceFinalGainFailure {
    envelope: Vec<f32>, target_bits: u32, math: SourceFinalGainMath,
    progress: SourceFinalGainProgress, cause: SourceFinalGainCause,
}
impl SourceFinalGainFailure {
    pub fn envelope(&self) -> &[f32] { &self.envelope }
    pub fn target_bits(&self) -> u32 { self.target_bits }
    pub fn math(&self) -> &SourceFinalGainMath { &self.math }
    pub fn progress(&self) -> &SourceFinalGainProgress { &self.progress }
    pub fn cause(&self) -> &SourceFinalGainCause { &self.cause }
}
pub(super) fn final_gain_with_source_math(envelope: Vec<f32>, target_bits: u32, mut math: SourceFinalGainMath)
    -> Result<(Vec<f32>, SourceFinalGainMath), SourceFinalGainFailure>
{
    let mut progress = SourceFinalGainProgress::new(Vec::new());
    if let Err(error) = progress.output.try_reserve_exact(envelope.len()) {
        return Err(SourceFinalGainFailure { envelope, target_bits, math, progress, cause: SourceFinalGainCause::Storage(error) });
    }
    let cap_bits = math.configuration.cap_bits;
    match final_graph(&envelope, target_bits, cap_bits, &mut math, &mut progress) {
        Ok(()) => Ok((progress.output, math)),
        Err(Stopped) => Err(SourceFinalGainFailure { envelope, target_bits, math, progress, cause: SourceFinalGainCause::Math }),
    }
}
pub(super) fn host_final_gain(envelope: &[f32], target: f32) -> Vec<f32> {
    let mut progress = SourceFinalGainProgress::new(Vec::with_capacity(envelope.len()));
    match final_graph(envelope, target.to_bits(), 24.0_f32.to_bits(), &mut Host24Minimum, &mut progress) {
        Ok(()) => progress.output, Err(never) => match never {},
    }
}
struct Stopped;
trait FinalMath {
    type Error;
    fn perform(&mut self, index: usize, operation: SourceFinalGainOperation, left: u32, right: u32) -> Result<u32, Self::Error>;
}
impl FinalMath for SourceFinalGainMath {
    type Error = Stopped;
    fn perform(&mut self, index: usize, operation: SourceFinalGainOperation, left: u32, right: u32) -> Result<u32, Stopped> {
        self.perform_source(index, operation, left, right)
    }
}
struct Host24Minimum;
impl FinalMath for Host24Minimum {
    type Error = Infallible;
    fn perform(&mut self, _: usize, operation: SourceFinalGainOperation, left: u32, right: u32) -> Result<u32, Infallible> {
        let (left, right) = (f32::from_bits(left), f32::from_bits(right));
        Ok(match operation { SourceFinalGainOperation::Subtract => left - right,
            SourceFinalGainOperation::Minimum => left.min(right) }.to_bits())
    }
}
fn final_graph<M: FinalMath>(envelope: &[f32], target: u32, cap: u32, math: &mut M, progress: &mut SourceFinalGainProgress)
    -> Result<(), M::Error>
{
    for (index, value) in envelope.iter().enumerate() {
        progress.index = index; progress.stage = SourceFinalGainStage::Subtract; progress.pc = 0x552ec3;
        let difference = math.perform(index, SourceFinalGainOperation::Subtract, target, value.to_bits())?;
        progress.pending_difference = Some(difference);
        progress.stage = SourceFinalGainStage::Minimum; progress.pc = 0x552ec8;
        let selected = math.perform(index, SourceFinalGainOperation::Minimum, difference, cap)?;
        progress.stage = SourceFinalGainStage::Store; progress.pc = 0x552ecc;
        progress.output.push(f32::from_bits(selected));
        progress.pending_difference = None;
        progress.index = index + 1;
    }
    progress.stage = SourceFinalGainStage::Complete; progress.pc = 0x552ee9;
    Ok(())
}
