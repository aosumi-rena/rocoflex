use std::{collections::TryReserveError, convert::Infallible};
use crate::{frontend_gain_interpolation_math as interpolation, source_scalar::{self as scalar, ExplicitExceptionState}};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error, ScalarF32Outcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceGainPowerAssociation {
    pub sample_index_dword: u32, pub pc: u32, pub base_bits: u32, pub exponent_bits: u32,
}

pub trait SourceGainPowerServices {
    type Reply;
    type Error;
    type ProjectionError;
    fn power(&mut self, request: interpolation::PowerRequest, association: SourceGainPowerAssociation)
        -> Result<Self::Reply, Self::Error>;
    fn reply_association(&self, reply: &Self::Reply) -> SourceGainPowerAssociation;
    fn project_reply(&mut self, reply: &Self::Reply) -> Result<u32, Self::ProjectionError>;
}
pub struct SourceGainPowerAttempt<P: SourceGainPowerServices> {
    pub request: interpolation::PowerRequest,
    pub association: SourceGainPowerAssociation,
    pub reported: Result<P::Reply, P::Error>,
    pub reported_association: Option<SourceGainPowerAssociation>,
    pub projection: Option<Result<u32, P::ProjectionError>>,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGainApplicationConfiguration { pub architecture: ScalarF32Architecture }
#[derive(Clone, Copy, Debug)]
pub enum SourceGainApplicationBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceGainApplicationBindingFailure {
    configuration: SourceGainApplicationConfiguration, entry: ExplicitExceptionState, cause: SourceGainApplicationBindingCause,
}
impl SourceGainApplicationBindingFailure {
    pub fn configuration(&self) -> SourceGainApplicationConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceGainApplicationBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGainApplicationAttempt {
    pub pc: u32, pub index: usize, pub left_bits: u32, pub right_bits: u32,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub reported: Option<Result<ScalarF32Outcome, ScalarF32Error>>,
    pub projection: Option<SourceScalarF32Projection>, pub result_bits: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainApplicationMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 }, Reported(SourceScalarF32Projection),
}
pub struct SourceGainApplicationMathFailure { cause: SourceGainApplicationMathCause, attempt: SourceGainApplicationAttempt }
impl SourceGainApplicationMathFailure {
    pub fn cause(&self) -> SourceGainApplicationMathCause { self.cause }
    pub fn attempt(&self) -> &SourceGainApplicationAttempt { &self.attempt }
}
pub struct SourceGainApplicationMath {
    configuration: SourceGainApplicationConfiguration, entry: ExplicitExceptionState, state: ExplicitExceptionState,
    scalar: SourceScalarF32, latest: Option<SourceGainApplicationAttempt>, failure: Option<SourceGainApplicationMathFailure>,
}
impl SourceGainApplicationMath {
    pub fn from_explicit_parts(configuration: SourceGainApplicationConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceGainApplicationBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm { return Err(SourceGainApplicationBindingCause::Architecture); }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceGainApplicationBindingCause::Controls); }
            let set = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Mulss])
                .map_err(SourceGainApplicationBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, set)
                .map_err(SourceGainApplicationBindingCause::Scalar)
        };
        match bind() {
            Ok(scalar) => Ok(Self { configuration, entry, state: entry, scalar, latest: None, failure: None }),
            Err(cause) => Err(SourceGainApplicationBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceGainApplicationConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn latest_attempt(&self) -> Option<&SourceGainApplicationAttempt> { self.latest.as_ref() }
    pub fn failure(&self) -> Option<&SourceGainApplicationMathFailure> { self.failure.as_ref() }
    fn multiply(&mut self, index: usize, left: u32, right: u32) -> Result<u32, Stopped> {
        if self.failure.is_some() { return Err(Stopped); }
        let mut attempt = SourceGainApplicationAttempt { pc: 0x552f8f, index, left_bits: left, right_bits: right,
            entry: self.state, known_advanced: self.state, reported: None, projection: None, result_bits: None };
        self.latest = Some(attempt);
        let result = (|| {
            let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                .ok_or(SourceGainApplicationMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
            let reported = self.scalar.execute(ScalarF32Operation::Mulss { left, right }, before);
            attempt.reported = Some(reported); self.latest = Some(attempt);
            let reported = reported.map_err(SourceGainApplicationMathCause::Scalar)?;
            let projection = project_scalar_f32_outcome(self.state, reported);
            attempt.projection = Some(projection); self.latest = Some(attempt);
            match projection {
                SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(bits) }
                SourceScalarF32Projection::Trapped { advanced, .. } => {
                    self.state = advanced; Err(SourceGainApplicationMathCause::Reported(projection))
                }
                SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                    self.state = known_advanced; Err(SourceGainApplicationMathCause::Reported(projection))
                }
            }
        })();
        attempt.known_advanced = self.state;
        match result {
            Ok(bits) => { attempt.result_bits = Some(bits); self.latest = Some(attempt); Ok(bits) }
            Err(cause) => {
                self.latest = Some(attempt); self.failure = Some(SourceGainApplicationMathFailure { cause, attempt }); Err(Stopped)
            }
        }
    }
}
struct Stopped;
pub struct SourceGainApplicationRequest<P> {
    pub output: Vec<f32>, pub interpolation_math: interpolation::SourceGainInterpolationMath,
    pub multiply_math: SourceGainApplicationMath, pub power: P,
}
pub struct SourceGainInPlaceApplicationRequest<P> {
    pub interpolation_math: interpolation::SourceGainInterpolationMath,
    pub multiply_math: SourceGainApplicationMath, pub power: P,
}
pub struct SourceGainApplicationInterpolation {
    pub request: interpolation::SourceGainInterpolationRequest,
    pub progress: interpolation::SourceGainInterpolationProgress,
    pub observation: interpolation::SourceGainInterpolationObservation,
    pub power: interpolation::PowerRequest,
    pub reports: [Option<interpolation::SourceGainInterpolationAttempt>; 9],
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainApplicationStage { Admission, HistoryReserve, Interpolation, PowerCall, ReplyAssociation, Projection, Multiply, Store, Complete }
pub struct SourceGainApplicationCurrent<P: SourceGainPowerServices> {
    pub stage: SourceGainApplicationStage,
    pub interpolation: SourceGainApplicationInterpolation,
    pub power: Option<SourceGainPowerAttempt<P>>,
    pub pending_amplitude: Option<u32>, pub multiply: Option<SourceGainApplicationAttempt>,
}
pub struct SourceGainApplicationRecord<P: SourceGainPowerServices> {
    pub source_bits: u32, pub output_bits: u32,
    pub interpolation: SourceGainApplicationInterpolation,
    pub power: SourceGainPowerAttempt<P>, pub multiply: SourceGainApplicationAttempt,
}
pub struct SourceGainApplicationContext<P: SourceGainPowerServices> {
    interpolation: interpolation::SourceGainInterpolationMath, multiply: SourceGainApplicationMath,
    power: P, history: Vec<SourceGainApplicationRecord<P>>,
}
impl<P: SourceGainPowerServices> SourceGainApplicationContext<P> {
    pub fn interpolation_math(&self) -> &interpolation::SourceGainInterpolationMath { &self.interpolation }
    pub fn multiply_math(&self) -> &SourceGainApplicationMath { &self.multiply }
    pub fn power_services(&self) -> &P { &self.power }
    pub fn history(&self) -> &[SourceGainApplicationRecord<P>] { &self.history }
    pub fn into_parts(self) -> (interpolation::SourceGainInterpolationMath, SourceGainApplicationMath, P, Vec<SourceGainApplicationRecord<P>>) {
        (self.interpolation, self.multiply, self.power, self.history)
    }
}
pub struct SourceGainApplicationParts<P: SourceGainPowerServices> {
    pub input: Vec<f32>, pub gain: Vec<f32>, pub output: Vec<f32>, pub context: SourceGainApplicationContext<P>,
}
pub struct SourceGainApplicationCompleted<P: SourceGainPowerServices> { parts: SourceGainApplicationParts<P> }
impl<P: SourceGainPowerServices> SourceGainApplicationCompleted<P> {
    pub fn input(&self) -> &[f32] { &self.parts.input }
    pub fn gain(&self) -> &[f32] { &self.parts.gain }
    pub fn output(&self) -> &[f32] { &self.parts.output }
    pub fn context(&self) -> &SourceGainApplicationContext<P> { &self.parts.context }
    pub fn into_parts(self) -> SourceGainApplicationParts<P> { self.parts }
}
pub struct SourceGainInPlaceApplicationParts<P: SourceGainPowerServices> {
    pub samples: Vec<f32>, pub gain: Vec<f32>, pub context: SourceGainApplicationContext<P>,
}
pub struct SourceGainInPlaceApplicationCompleted<P: SourceGainPowerServices> {
    parts: SourceGainInPlaceApplicationParts<P>,
}
impl<P: SourceGainPowerServices> SourceGainInPlaceApplicationCompleted<P> {
    pub fn samples(&self) -> &[f32] { &self.parts.samples }
    pub fn gain(&self) -> &[f32] { &self.parts.gain }
    pub fn context(&self) -> &SourceGainApplicationContext<P> { &self.parts.context }
    pub fn into_parts(self) -> SourceGainInPlaceApplicationParts<P> { self.parts }
}
pub enum SourceGainApplicationCause {
    Input(String), OutputExtent { actual: usize, required: usize }, Storage(TryReserveError),
    Interpolation, Power, Association, Projection, Multiply,
}
enum SourceOwner<P: SourceGainPowerServices> {
    Ready { gain: Vec<f32>, interpolation: interpolation::SourceGainInterpolationMath, multiply: SourceGainApplicationMath,
        power: P, current: Option<SourceGainApplicationCurrent<P>> },
    Interpolation { failure: interpolation::SourceGainInterpolationFailure, multiply: SourceGainApplicationMath, power: P },
}
struct SourceStop<P: SourceGainPowerServices> {
    owner: SourceOwner<P>, history: Vec<SourceGainApplicationRecord<P>>, cause: SourceGainApplicationCause, stage: SourceGainApplicationStage,
}
pub struct SourceGainApplicationFailure<P: SourceGainPowerServices> {
    input: Vec<f32>, output: Vec<f32>, index: usize, stop: SourceStop<P>,
}
impl<P: SourceGainPowerServices> SourceGainApplicationFailure<P> {
    pub fn input(&self) -> &[f32] { &self.input }
    pub fn output(&self) -> &[f32] { &self.output }
    pub fn index(&self) -> usize { self.index }
    pub fn stage(&self) -> SourceGainApplicationStage { self.stop.stage }
    pub fn cause(&self) -> &SourceGainApplicationCause { &self.stop.cause }
    pub fn gain(&self) -> &[f32] { match &self.stop.owner {
        SourceOwner::Ready { gain, .. } => gain, SourceOwner::Interpolation { failure, .. } => failure.gain() } }
    pub fn interpolation_math(&self) -> &interpolation::SourceGainInterpolationMath { match &self.stop.owner {
        SourceOwner::Ready { interpolation, .. } => interpolation, SourceOwner::Interpolation { failure, .. } => failure.math() } }
    pub fn multiply_math(&self) -> &SourceGainApplicationMath { match &self.stop.owner {
        SourceOwner::Ready { multiply, .. } | SourceOwner::Interpolation { multiply, .. } => multiply } }
    pub fn power_services(&self) -> &P { match &self.stop.owner {
        SourceOwner::Ready { power, .. } | SourceOwner::Interpolation { power, .. } => power } }
    pub fn history(&self) -> &[SourceGainApplicationRecord<P>] { &self.stop.history }
    pub fn current(&self) -> Option<&SourceGainApplicationCurrent<P>> { match &self.stop.owner {
        SourceOwner::Ready { current, .. } => current.as_ref(), _ => None } }
    pub fn interpolation_failure(&self) -> Option<&interpolation::SourceGainInterpolationFailure> { match &self.stop.owner {
        SourceOwner::Interpolation { failure, .. } => Some(failure), _ => None } }
}

pub struct SourceGainInPlaceApplicationFailure<P: SourceGainPowerServices> {
    samples: Vec<f32>, index: usize, stop: SourceStop<P>,
}
impl<P: SourceGainPowerServices> SourceGainInPlaceApplicationFailure<P> {
    pub fn samples(&self) -> &[f32] { &self.samples }
    pub fn index(&self) -> usize { self.index }
    pub fn stage(&self) -> SourceGainApplicationStage { self.stop.stage }
    pub fn cause(&self) -> &SourceGainApplicationCause { &self.stop.cause }
    pub fn gain(&self) -> &[f32] { match &self.stop.owner {
        SourceOwner::Ready { gain, .. } => gain, SourceOwner::Interpolation { failure, .. } => failure.gain() } }
    pub fn interpolation_math(&self) -> &interpolation::SourceGainInterpolationMath { match &self.stop.owner {
        SourceOwner::Ready { interpolation, .. } => interpolation, SourceOwner::Interpolation { failure, .. } => failure.math() } }
    pub fn multiply_math(&self) -> &SourceGainApplicationMath { match &self.stop.owner {
        SourceOwner::Ready { multiply, .. } | SourceOwner::Interpolation { multiply, .. } => multiply } }
    pub fn power_services(&self) -> &P { match &self.stop.owner {
        SourceOwner::Ready { power, .. } | SourceOwner::Interpolation { power, .. } => power } }
    pub fn history(&self) -> &[SourceGainApplicationRecord<P>] { &self.stop.history }
    pub fn current(&self) -> Option<&SourceGainApplicationCurrent<P>> { match &self.stop.owner {
        SourceOwner::Ready { current, .. } => current.as_ref(), _ => None } }
    pub fn interpolation_failure(&self) -> Option<&interpolation::SourceGainInterpolationFailure> { match &self.stop.owner {
        SourceOwner::Interpolation { failure, .. } => Some(failure), _ => None } }
}

pub fn apply_gain_windows_with_source_services<P: SourceGainPowerServices>(input: Vec<f32>, gain: Vec<f32>,
    request: SourceGainApplicationRequest<P>) -> Result<SourceGainApplicationCompleted<P>, SourceGainApplicationFailure<P>>
{
    let SourceGainApplicationRequest { output, interpolation_math, multiply_math, power } = request;
    let choice = SourceChoice { gain, interpolation: interpolation_math, multiply: multiply_math, power, history: Vec::new() };
    let refusal = match super::validate_gain_application(&input, &choice.gain) {
        Err(error) => Some(SourceGainApplicationCause::Input(error)),
        Ok(()) if output.len() != input.len() => Some(SourceGainApplicationCause::OutputExtent { actual: output.len(), required: input.len() }),
        Ok(()) => None,
    };
    if let Some(cause) = refusal {
        return Err(SourceGainApplicationFailure { input, output, index: 0,
            stop: choice.stop(None, cause, SourceGainApplicationStage::Admission) });
    }
    match application_loop(ApplicationStorage::Separate { input: &input, output, store: OutputStore::Replace }, choice) {
        Ok((ApplicationStorage::Separate { output, .. }, choice)) => Ok(SourceGainApplicationCompleted { parts: SourceGainApplicationParts {
            input, gain: choice.gain, output, context: SourceGainApplicationContext {
                interpolation: choice.interpolation, multiply: choice.multiply, power: choice.power, history: choice.history } } }),
        Err(LoopFailure { storage: ApplicationStorage::Separate { output, .. }, index, failure }) =>
            Err(SourceGainApplicationFailure { input, output, index, stop: failure }),
        Ok((ApplicationStorage::InPlace { .. }, _)) | Err(LoopFailure { storage: ApplicationStorage::InPlace { .. }, .. }) => unreachable!(),
    }
}

pub fn apply_gain_windows_in_place_with_source_services<P: SourceGainPowerServices>(samples: Vec<f32>, gain: Vec<f32>,
    request: SourceGainInPlaceApplicationRequest<P>)
    -> Result<SourceGainInPlaceApplicationCompleted<P>, SourceGainInPlaceApplicationFailure<P>>
{
    apply_gain_windows_in_place_with_admission(samples, gain, request, AppAdmission::Legacy)
}
pub(crate) fn apply_gain_windows_in_place_with_source_bits<P: SourceGainPowerServices>(samples: Vec<f32>, gain: Vec<f32>,
    request: SourceGainInPlaceApplicationRequest<P>)
    -> Result<SourceGainInPlaceApplicationCompleted<P>, SourceGainInPlaceApplicationFailure<P>>
{
    apply_gain_windows_in_place_with_admission(samples, gain, request, AppAdmission::SourceBits)
}
enum AppAdmission { Legacy, SourceBits }
fn apply_gain_windows_in_place_with_admission<P: SourceGainPowerServices>(samples: Vec<f32>, gain: Vec<f32>,
    request: SourceGainInPlaceApplicationRequest<P>, admission: AppAdmission)
    -> Result<SourceGainInPlaceApplicationCompleted<P>, SourceGainInPlaceApplicationFailure<P>>
{
    let SourceGainInPlaceApplicationRequest { interpolation_math, multiply_math, power } = request;
    let choice = SourceChoice { gain, interpolation: interpolation_math, multiply: multiply_math, power, history: Vec::new() };
    let admitted = match admission {
        AppAdmission::Legacy => super::validate_gain_application(&samples, &choice.gain),
        AppAdmission::SourceBits => if choice.gain.is_empty() || samples.len() > i32::MAX as usize {
            Err("gain application requires finite source/nonempty gain".into())
        } else { Ok(()) },
    };
    if let Err(error) = admitted {
        return Err(SourceGainInPlaceApplicationFailure { samples, index: 0,
            stop: choice.stop(None, SourceGainApplicationCause::Input(error), SourceGainApplicationStage::Admission) });
    }
    match application_loop(ApplicationStorage::InPlace { samples }, choice) {
        Ok((ApplicationStorage::InPlace { samples }, choice)) => Ok(SourceGainInPlaceApplicationCompleted {
            parts: SourceGainInPlaceApplicationParts { samples, gain: choice.gain, context: SourceGainApplicationContext {
                interpolation: choice.interpolation, multiply: choice.multiply, power: choice.power, history: choice.history } } }),
        Err(LoopFailure { storage: ApplicationStorage::InPlace { samples }, index, failure }) =>
            Err(SourceGainInPlaceApplicationFailure { samples, index, stop: failure }),
        Ok((ApplicationStorage::Separate { .. }, _)) | Err(LoopFailure { storage: ApplicationStorage::Separate { .. }, .. }) => unreachable!(),
    }
}
struct SourceChoice<P: SourceGainPowerServices> {
    gain: Vec<f32>, interpolation: interpolation::SourceGainInterpolationMath, multiply: SourceGainApplicationMath,
    power: P, history: Vec<SourceGainApplicationRecord<P>>,
}
impl<P: SourceGainPowerServices> SourceChoice<P> {
    fn stop(self, current: Option<SourceGainApplicationCurrent<P>>, cause: SourceGainApplicationCause,
        stage: SourceGainApplicationStage) -> SourceStop<P>
    {
        SourceStop { owner: SourceOwner::Ready { gain: self.gain, interpolation: self.interpolation, multiply: self.multiply,
            power: self.power, current }, history: self.history, cause, stage }
    }
}
trait ApplicationChoice: Sized {
    type Record;
    type ReserveError;
    type Error;
    fn reserve(&mut self) -> Result<(), Self::ReserveError>;
    fn reserve_failure(self, error: Self::ReserveError) -> Self::Error;
    fn sample(self, index: usize, source_bits: u32) -> Result<(u32, Self, Self::Record), Self::Error>;
    fn commit(&mut self, record: Self::Record);
}
impl<P: SourceGainPowerServices> ApplicationChoice for SourceChoice<P> {
    type Record = SourceGainApplicationRecord<P>;
    type ReserveError = TryReserveError;
    type Error = SourceStop<P>;
    fn reserve(&mut self) -> Result<(), TryReserveError> { self.history.try_reserve(1) }
    fn reserve_failure(self, error: TryReserveError) -> SourceStop<P> {
        self.stop(None, SourceGainApplicationCause::Storage(error), SourceGainApplicationStage::HistoryReserve)
    }
    fn sample(self, index: usize, source_bits: u32) -> Result<(u32, Self, Self::Record), SourceStop<P>> {
        let Self { gain, interpolation, multiply, power, history } = self;
        let request = interpolation::SourceGainInterpolationRequest { sample_index_dword: index as u32 };
        let completed = match interpolation::prepare_gain_power_request(gain, request, interpolation) {
            Ok(completed) => completed,
            Err(failure) => return Err(SourceStop { owner: SourceOwner::Interpolation { failure, multiply, power }, history,
                cause: SourceGainApplicationCause::Interpolation, stage: SourceGainApplicationStage::Interpolation }),
        };
        let interpolation::SourceGainInterpolationParts { gain, request, math: interpolation, progress, observation, power: power_request } = completed.into_parts();
        let reports = *interpolation.reports();
        let mut choice = Self { gain, interpolation, multiply, power, history };
        let mut current: SourceGainApplicationCurrent<P> = SourceGainApplicationCurrent { stage: SourceGainApplicationStage::PowerCall,
            interpolation: SourceGainApplicationInterpolation { request, progress, observation, power: power_request, reports },
            power: None, pending_amplitude: None, multiply: None };
        let association = SourceGainPowerAssociation { sample_index_dword: request.sample_index_dword, pc: power_request.pc,
            base_bits: power_request.base_bits, exponent_bits: power_request.exponent_bits };

        let reported = choice.power.power(power_request, association);
        current.power = Some(SourceGainPowerAttempt { request: power_request, association, reported,
            reported_association: None, projection: None });
        if matches!(&current.power.as_ref().unwrap().reported, Err(_)) {
            return Err(choice.stop(Some(current), SourceGainApplicationCause::Power, SourceGainApplicationStage::PowerCall));
        }
        current.stage = SourceGainApplicationStage::ReplyAssociation;
        let actual_association = match &current.power.as_ref().unwrap().reported {
            Ok(reply) => choice.power.reply_association(reply), Err(_) => unreachable!(),
        };
        current.power.as_mut().unwrap().reported_association = Some(actual_association);
        if actual_association != association {
            return Err(choice.stop(Some(current), SourceGainApplicationCause::Association, SourceGainApplicationStage::ReplyAssociation));
        }
        current.stage = SourceGainApplicationStage::Projection;
        let projection = match &current.power.as_ref().unwrap().reported {
            Ok(reply) => choice.power.project_reply(reply), Err(_) => unreachable!(),
        };
        current.power.as_mut().unwrap().projection = Some(projection);
        let amplitude = match &current.power.as_ref().unwrap().projection {
            Some(Ok(bits)) => *bits,
            Some(Err(_)) => return Err(choice.stop(Some(current), SourceGainApplicationCause::Projection, SourceGainApplicationStage::Projection)),
            None => unreachable!(),
        };
        current.pending_amplitude = Some(amplitude);
        current.stage = SourceGainApplicationStage::Multiply;
        let product = choice.multiply.multiply(index, amplitude, source_bits);
        current.multiply = choice.multiply.latest;
        let product = match product { Ok(bits) => bits,
            Err(Stopped) => return Err(choice.stop(Some(current), SourceGainApplicationCause::Multiply, SourceGainApplicationStage::Multiply)) };
        current.stage = SourceGainApplicationStage::Store;
        let record = SourceGainApplicationRecord { source_bits, output_bits: product, interpolation: current.interpolation,
            power: current.power.unwrap(), multiply: current.multiply.unwrap() };
        Ok((product, choice, record))
    }
    fn commit(&mut self, record: Self::Record) { self.history.push(record); }
}
enum OutputStore { Append, Replace }
enum ApplicationStorage<'a> {
    Separate { input: &'a [f32], output: Vec<f32>, store: OutputStore },
    InPlace { samples: Vec<f32> },
}
impl ApplicationStorage<'_> {
    fn len(&self) -> usize { match self {
        Self::Separate { input, .. } => input.len(), Self::InPlace { samples } => samples.len(),
    } }
    fn read(&self, index: usize) -> u32 { match self {
        Self::Separate { input, .. } => input[index].to_bits(), Self::InPlace { samples } => samples[index].to_bits(),
    } }
    fn store(&mut self, index: usize, bits: u32) { match self {
        Self::Separate { output, store, .. } => match store {
            OutputStore::Append => output.push(f32::from_bits(bits)), OutputStore::Replace => output[index] = f32::from_bits(bits),
        },
        Self::InPlace { samples } => samples[index] = f32::from_bits(bits),
    } }
}
struct LoopFailure<'a, E> { storage: ApplicationStorage<'a>, index: usize, failure: E }
fn application_loop<'a, C: ApplicationChoice>(mut storage: ApplicationStorage<'a>, mut choice: C)
    -> Result<(ApplicationStorage<'a>, C), LoopFailure<'a, C::Error>>
{
    for index in 0..storage.len() {
        if let Err(error) = choice.reserve() {
            return Err(LoopFailure { storage, index, failure: choice.reserve_failure(error) });
        }
        let source_bits = storage.read(index);
        let (bits, next, record) = match choice.sample(index, source_bits) {
            Ok(completed) => completed, Err(failure) => return Err(LoopFailure { storage, index, failure }),
        };
        choice = next;
        storage.store(index, bits);
        choice.commit(record);
    }
    Ok((storage, choice))
}
struct HostApplication<'a> { gain: &'a [f32] }
impl ApplicationChoice for HostApplication<'_> {
    type Record = ();
    type ReserveError = Infallible;
    type Error = Infallible;
    fn reserve(&mut self) -> Result<(), Infallible> { Ok(()) }
    fn reserve_failure(self, never: Infallible) -> Infallible { match never {} }
    fn sample(self, i: usize, source_bits: u32) -> Result<(u32, Self, ()), Infallible> {
        let index = i / 480;
        let position = ((i as f32) * 100.0_f32) / 48000.0_f32;
        let fraction = position - index as f32;
        let a = self.gain[index.min(self.gain.len() - 1)];
        let b = self.gain[(index + 1).min(self.gain.len() - 1)];
        let diff = b - a;
        let interp = diff * fraction + a;
        let amplitude = 10.0_f32.powf(interp / 20.0_f32);
        Ok(((amplitude * f32::from_bits(source_bits)).to_bits(), self, ()))
    }
    fn commit(&mut self, (): ()) {}
}
pub(super) fn host_application(input: &[f32], gain: &[f32]) -> Vec<f32> {
    match application_loop(ApplicationStorage::Separate { input, output: Vec::with_capacity(input.len()), store: OutputStore::Append }, HostApplication { gain }) {
        Ok((ApplicationStorage::Separate { output, .. }, _)) => output,
        Err(LoopFailure { failure, .. }) => match failure {},
        Ok((ApplicationStorage::InPlace { .. }, _)) => unreachable!(),
    }
}
