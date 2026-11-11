use crate::source_scalar::{ExplicitExceptionState, PowerError};
use crate::source_scalar::shared_scalar_f32::{
    ScalarF32Architecture, ScalarF32Error, ScalarF32Exceptions,
    ScalarF32InstructionSet, ScalarF32Mxcsr, SourceScalarF32, XmmF32Outcome,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceGenerationRsqrtpsRequest {
    pub input_bits: [u32; 4],
    pub entry: ExplicitExceptionState,
}

pub struct SourceGenerationRsqrtpsCallReport<R, E> {
    pub actual_request: SourceGenerationRsqrtpsRequest,
    pub known_advanced: ExplicitExceptionState,
    pub outcome: Result<R, E>,
}

pub struct SourceGenerationRsqrtpsProjectionReport<E> {
    pub actual_request: SourceGenerationRsqrtpsRequest,
    pub known_advanced: ExplicitExceptionState,
    pub outcome: Result<[u32; 4], E>,
}

pub trait SourceGenerationRsqrtpsService {
    type Reply;
    type Error;
    type ProjectionError;
    fn call(&mut self, request: SourceGenerationRsqrtpsRequest)
        -> SourceGenerationRsqrtpsCallReport<Self::Reply, Self::Error>;
    fn project_reply(&self, reply: &Self::Reply)
        -> SourceGenerationRsqrtpsProjectionReport<Self::ProjectionError>;
}

#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationPacketNormConfiguration {
    pub c0_prescale_bits: [u32; 4],
    pub c2_bits: [u32; 4],
    pub c3_bits: [u32; 4],
    pub c4_bits: [u32; 4],
    pub c5_bits: [u32; 4],
    pub inherited_c6_bits: [u32; 4],
    pub c7_bits: [u32; 4],
    pub c8_add_bits: [u32; 4],
    pub architecture: ScalarF32Architecture,
    pub instructions: ScalarF32InstructionSet,
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationPacketNormBindingCause {
    ArchitectureRequired,
    StrictEntryRequired,
    Backend(ScalarF32Error),
}
pub struct SourceGenerationPacketNormBindingError<S> {
    service: S,
    configuration: SourceGenerationPacketNormConfiguration,
    entry: ExplicitExceptionState,
    cause: SourceGenerationPacketNormBindingCause,
}
impl<S> SourceGenerationPacketNormBindingError<S> {
    pub fn service(&self) -> &S { &self.service }
    pub fn configuration(&self) -> &SourceGenerationPacketNormConfiguration { &self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceGenerationPacketNormBindingCause { self.cause }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceGenerationPacketNormTag { pub packet: usize }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGenerationPacketNormStage {
    Prescale, IntegerMask, RsqrtpsCall, RsqrtpsProjection, Square,
    PrescaledProduct, AddResource, Refinement, InputProduct, FinalMask, Complete,
}
impl SourceGenerationPacketNormStage {

    pub fn pc(self) -> Option<u32> {
        match self {
            Self::Prescale => Some(0x3e7774),
            Self::IntegerMask => Some(0x3e7778),
            Self::RsqrtpsCall => Some(0x3e77d6),
            Self::Square => Some(0x3e77de),
            Self::PrescaledProduct => Some(0x3e77e2),
            Self::AddResource => Some(0x3e77e6),
            Self::Refinement => Some(0x3e77ea),
            Self::InputProduct => Some(0x3e77ee),
            Self::FinalMask => Some(0x3e77f2),
            Self::RsqrtpsProjection | Self::Complete => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceGenerationPacketNormCurrent {
    pub tag: SourceGenerationPacketNormTag,
    pub stage: SourceGenerationPacketNormStage,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationPacketNormPendingPacked {
    pub slot: usize,
    pub pc: u32,
    pub left: [u32; 4],
    pub right: [u32; 4],
    pub entry: ExplicitExceptionState,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationPacketNormAttempt {
    pub pending: SourceGenerationPacketNormPendingPacked,
    pub reported: Result<XmmF32Outcome, ScalarF32Error>,
    pub admission: Option<Result<ExplicitExceptionState, PowerError>>,

    pub known_advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationPacketNormMask {
    pub negative: [u32; 4],
    pub a: [u32; 4],
    pub t: [u32; 4],
    pub f: [u32; 4],
    pub equal: [u32; 4],
    pub greater: [u32; 4],
    pub h: [u32; 4],
    pub d: [u32; 4],
    pub j: [u32; 4],
    pub bits: [u32; 4],
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationPacketNormPacket {
    pub tag: SourceGenerationPacketNormTag,
    pub input_bits: [u32; 4],
    pub entry: ExplicitExceptionState,
    pub attempts: [Option<SourceGenerationPacketNormAttempt>; 6],
    pub pending_packed: Option<SourceGenerationPacketNormPendingPacked>,
    pub prescaled: Option<[u32; 4]>,
    pub mask: Option<SourceGenerationPacketNormMask>,
    pub rsqrtps_request: Option<SourceGenerationRsqrtpsRequest>,
    pub q: Option<[u32; 4]>,
    pub squared_q: Option<[u32; 4]>,
    pub prescaled_product: Option<[u32; 4]>,
    pub factor: Option<[u32; 4]>,
    pub refined: Option<[u32; 4]>,
    pub final_product: Option<[u32; 4]>,
    pub completed_bits: Option<[u32; 4]>,
}
pub struct SourceGenerationPacketNormCallAttempt<R, E> {
    tag: SourceGenerationPacketNormTag,
    expected: SourceGenerationRsqrtpsRequest,
    report: SourceGenerationRsqrtpsCallReport<R, E>,
    admission: Option<Result<ExplicitExceptionState, PowerError>>,
}
impl<R, E> SourceGenerationPacketNormCallAttempt<R, E> {
    pub fn tag(&self) -> SourceGenerationPacketNormTag { self.tag }
    pub fn expected(&self) -> SourceGenerationRsqrtpsRequest { self.expected }
    pub fn report(&self) -> &SourceGenerationRsqrtpsCallReport<R, E> { &self.report }
    pub fn admission(&self) -> Option<Result<ExplicitExceptionState, PowerError>> { self.admission }
}
pub struct SourceGenerationPacketNormProjectionAttempt<E> {
    tag: SourceGenerationPacketNormTag,
    report: SourceGenerationRsqrtpsProjectionReport<E>,
}
impl<E> SourceGenerationPacketNormProjectionAttempt<E> {
    pub fn tag(&self) -> SourceGenerationPacketNormTag { self.tag }
    pub fn report(&self) -> &SourceGenerationRsqrtpsProjectionReport<E> { &self.report }
}

pub struct SourceGenerationPacketNormProgress<R, E, P> {
    current: Option<SourceGenerationPacketNormCurrent>,
    packet_reached: bool,
    packet: Option<SourceGenerationPacketNormPacket>,
    call_reached: bool,
    call: Option<SourceGenerationPacketNormCallAttempt<R, E>>,
    projection_reached: bool,
    projection: Option<SourceGenerationPacketNormProjectionAttempt<P>>,
}
impl<R, E, P> SourceGenerationPacketNormProgress<R, E, P> {
    pub fn current(&self) -> Option<SourceGenerationPacketNormCurrent> { self.current }
    pub fn packet_reached(&self) -> bool { self.packet_reached }
    pub fn packet(&self) -> Option<&SourceGenerationPacketNormPacket> { self.packet.as_ref() }
    pub fn call_reached(&self) -> bool { self.call_reached }
    pub fn call(&self) -> Option<&SourceGenerationPacketNormCallAttempt<R, E>> { self.call.as_ref() }
    pub fn projection_reached(&self) -> bool { self.projection_reached }
    pub fn projection(&self) -> Option<&SourceGenerationPacketNormProjectionAttempt<P>> { self.projection.as_ref() }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceGenerationPacketNormCause {
    Packed(ScalarF32Error),
    PackedTrap { unmasked: ScalarF32Exceptions },
    StateAdmission(PowerError),
    CallAssociation,
    CallErrno { expected: i32, reported: i32 },

    CallReturned,
    ProjectionAssociation,
    ProjectionState,

    ProjectionReturned,
}
#[derive(Clone, Copy, Debug)]
pub struct SourceGenerationPacketNormFailure {
    pub current: SourceGenerationPacketNormCurrent,
    pub cause: SourceGenerationPacketNormCause,
}
pub struct SourceGenerationPacketNormStopped { _private: () }

pub struct SourceGenerationPacketNormMath<S: SourceGenerationRsqrtpsService> {
    configuration: SourceGenerationPacketNormConfiguration,
    backend: SourceScalarF32,
    service: S,
    state: ExplicitExceptionState,
    active: bool,
    progress: SourceGenerationPacketNormProgress<S::Reply, S::Error, S::ProjectionError>,
    failure: Option<SourceGenerationPacketNormFailure>,
}
impl<S: SourceGenerationRsqrtpsService> SourceGenerationPacketNormMath<S> {
    pub fn from_explicit_parts(configuration: SourceGenerationPacketNormConfiguration,
        entry: ExplicitExceptionState, service: S) -> Result<Self, SourceGenerationPacketNormBindingError<S>>
    {
        if configuration.architecture != ScalarF32Architecture::IntelSdm {
            return Err(SourceGenerationPacketNormBindingError { service, configuration, entry,
                cause: SourceGenerationPacketNormBindingCause::ArchitectureRequired });
        }

        if entry.mxcsr() & !0x3f != 0x1f80 {
            return Err(SourceGenerationPacketNormBindingError { service, configuration, entry,
                cause: SourceGenerationPacketNormBindingCause::StrictEntryRequired });
        }
        let backend = match SourceScalarF32::from_explicit_architecture(configuration.architecture, configuration.instructions) {
            Ok(backend) => backend,
            Err(error) => return Err(SourceGenerationPacketNormBindingError { service, configuration, entry,
                cause: SourceGenerationPacketNormBindingCause::Backend(error) }),
        };
        Ok(Self { configuration, backend, service, state: entry, active: false, failure: None,
            progress: SourceGenerationPacketNormProgress { current: None, packet_reached: false,
                packet: None, call_reached: false, call: None, projection_reached: false, projection: None } })
    }
    pub fn configuration(&self) -> &SourceGenerationPacketNormConfiguration { &self.configuration }
    pub fn backend(&self) -> &SourceScalarF32 { &self.backend }
    pub fn service(&self) -> &S { &self.service }

    pub fn state(&self) -> ExplicitExceptionState { self.state }
    pub fn is_active(&self) -> bool { self.active }
    pub fn progress(&self) -> &SourceGenerationPacketNormProgress<S::Reply, S::Error, S::ProjectionError> { &self.progress }
    pub fn failure(&self) -> Option<&SourceGenerationPacketNormFailure> { self.failure.as_ref() }

    fn stage(&mut self, stage: SourceGenerationPacketNormStage) {
        self.progress.current.as_mut().unwrap().stage = stage;
    }
    fn stop(&mut self, cause: SourceGenerationPacketNormCause) -> SourceGenerationPacketNormStopped {
        if self.failure.is_none() {
            self.failure = Some(SourceGenerationPacketNormFailure { current: self.progress.current.unwrap(), cause });
        }
        self.active = false;
        SourceGenerationPacketNormStopped { _private: () }
    }
    fn packet_mut(&mut self) -> &mut SourceGenerationPacketNormPacket { self.progress.packet.as_mut().unwrap() }

    fn packed(&mut self, slot: usize, pc: u32, stage: SourceGenerationPacketNormStage,
        left: [u32; 4], right: [u32; 4], add: bool) -> Result<[u32; 4], SourceGenerationPacketNormStopped>
    {
        self.stage(stage);
        let entry = self.state;
        let pending = SourceGenerationPacketNormPendingPacked { slot, pc, left, right, entry };
        self.packet_mut().pending_packed = Some(pending);
        let raw_entry = ScalarF32Mxcsr::from_explicit_word(entry.mxcsr()).unwrap();
        let reported = if add { self.backend.execute_addps_xmm(left, right, raw_entry) }
            else { self.backend.execute_mulps_xmm(left, right, raw_entry) };
        self.packet_mut().attempts[slot] = Some(SourceGenerationPacketNormAttempt {
            pending, reported, admission: None, known_advanced: entry });

        let outcome = match reported {
            Ok(outcome) => outcome,
            Err(error) => return Err(self.stop(SourceGenerationPacketNormCause::Packed(error))),
        };
        let raw_state = match outcome {
            XmmF32Outcome::Completed { state, .. } | XmmF32Outcome::Trapped { state, .. } => state,
        };
        let admission = ExplicitExceptionState::from_explicit_parts(raw_state.word(), entry.errno());
        self.packet_mut().attempts[slot].as_mut().unwrap().admission = Some(admission);
        match admission {
            Ok(state) => {
                self.state = state;
                self.packet_mut().attempts[slot].as_mut().unwrap().known_advanced = state;
            }
            Err(error) => return Err(self.stop(SourceGenerationPacketNormCause::StateAdmission(error))),
        }
        match outcome {
            XmmF32Outcome::Completed { bits, .. } => {
                self.packet_mut().pending_packed = None;
                Ok(bits)
            }
            XmmF32Outcome::Trapped { unmasked, .. } => Err(self.stop(SourceGenerationPacketNormCause::PackedTrap { unmasked })),
        }
    }

    fn rsqrtps(&mut self, tag: SourceGenerationPacketNormTag, input_bits: [u32; 4])
        -> Result<[u32; 4], SourceGenerationPacketNormStopped>
    {
        let expected = SourceGenerationRsqrtpsRequest { input_bits, entry: self.state };
        self.packet_mut().rsqrtps_request = Some(expected);
        self.stage(SourceGenerationPacketNormStage::RsqrtpsCall);

        let report = self.service.call(expected);
        self.progress.call = Some(SourceGenerationPacketNormCallAttempt { tag, expected, report, admission: None });
        self.progress.call_reached = true;
        let (actual, reported_state) = {
            let call = self.progress.call.as_ref().unwrap();
            (call.report.actual_request, call.report.known_advanced)
        };
        if actual != expected { return Err(self.stop(SourceGenerationPacketNormCause::CallAssociation)); }
        if reported_state.errno() != expected.entry.errno() {
            return Err(self.stop(SourceGenerationPacketNormCause::CallErrno {
                expected: expected.entry.errno(), reported: reported_state.errno() }));
        }
        let admission = ExplicitExceptionState::from_explicit_parts(reported_state.mxcsr(), expected.entry.errno());
        self.progress.call.as_mut().unwrap().admission = Some(admission);
        match admission {
            Ok(state) => self.state = state,
            Err(error) => return Err(self.stop(SourceGenerationPacketNormCause::StateAdmission(error))),
        }
        if self.progress.call.as_ref().unwrap().report.outcome.is_err() {
            return Err(self.stop(SourceGenerationPacketNormCause::CallReturned));
        }
        self.stage(SourceGenerationPacketNormStage::RsqrtpsProjection);

        let report = {
            let reply = self.progress.call.as_ref().unwrap().report.outcome.as_ref().ok().unwrap();
            self.service.project_reply(reply)
        };
        self.progress.projection = Some(SourceGenerationPacketNormProjectionAttempt { tag, report });
        self.progress.projection_reached = true;
        let report = &self.progress.projection.as_ref().unwrap().report;
        if report.actual_request != actual {
            return Err(self.stop(SourceGenerationPacketNormCause::ProjectionAssociation));
        }
        if report.known_advanced != reported_state {
            return Err(self.stop(SourceGenerationPacketNormCause::ProjectionState));
        }

        match report.outcome.as_ref() {
            Ok(bits) => Ok(*bits),
            Err(_) => Err(self.stop(SourceGenerationPacketNormCause::ProjectionReturned)),
        }
    }

    pub(crate) fn evaluate(&mut self, tag: SourceGenerationPacketNormTag, input: [u32; 4])
        -> Result<[u32; 4], SourceGenerationPacketNormStopped>
    {
        if self.failure.is_some() || self.active {
            return Err(SourceGenerationPacketNormStopped { _private: () });
        }
        self.active = true;
        self.progress.packet_reached = false;
        self.progress.call_reached = false;
        self.progress.projection_reached = false;
        self.progress.current = Some(SourceGenerationPacketNormCurrent { tag, stage: SourceGenerationPacketNormStage::Prescale });
        self.progress.packet = Some(SourceGenerationPacketNormPacket { tag, input_bits: input, entry: self.state,
            attempts: [None; 6], pending_packed: None, prescaled: None, mask: None, rsqrtps_request: None,
            q: None, squared_q: None, prescaled_product: None, factor: None, refined: None,
            final_product: None, completed_bits: None });
        self.progress.packet_reached = true;
        let prescaled = self.packed(0, 0x3e7774, SourceGenerationPacketNormStage::Prescale,
            input, self.configuration.c0_prescale_bits, false)?;
        self.packet_mut().prescaled = Some(prescaled);
        self.stage(SourceGenerationPacketNormStage::IntegerMask);
        let mask = integer_mask(input, self.configuration);
        self.packet_mut().mask = Some(mask);

        let q = self.rsqrtps(tag, input)?;
        self.packet_mut().q = Some(q);
        let squared_q = self.packed(1, 0x3e77de, SourceGenerationPacketNormStage::Square, q, q, false)?;
        self.packet_mut().squared_q = Some(squared_q);
        let product = self.packed(2, 0x3e77e2, SourceGenerationPacketNormStage::PrescaledProduct, squared_q, prescaled, false)?;
        self.packet_mut().prescaled_product = Some(product);
        let factor = self.packed(3, 0x3e77e6, SourceGenerationPacketNormStage::AddResource,
            product, self.configuration.c8_add_bits, true)?;
        self.packet_mut().factor = Some(factor);
        let refined = self.packed(4, 0x3e77ea, SourceGenerationPacketNormStage::Refinement, factor, q, false)?;
        self.packet_mut().refined = Some(refined);
        let product = self.packed(5, 0x3e77ee, SourceGenerationPacketNormStage::InputProduct, refined, input, false)?;
        self.packet_mut().final_product = Some(product);
        self.stage(SourceGenerationPacketNormStage::FinalMask);
        let bits = std::array::from_fn(|i| product[i] & mask.bits[i]);
        self.packet_mut().completed_bits = Some(bits);
        self.stage(SourceGenerationPacketNormStage::Complete);
        self.active = false;
        Ok(bits)
    }
}

fn integer_mask(x: [u32; 4], c: SourceGenerationPacketNormConfiguration) -> SourceGenerationPacketNormMask {
    fn sgt(a: u32, b: u32) -> u32 { if (a as i32) > (b as i32) { u32::MAX } else { 0 } }

    let negative = std::array::from_fn(|i| sgt(0, x[i]));
    let a: [u32; 4] = std::array::from_fn(|i| x[i] & c.inherited_c6_bits[i]);
    let t: [u32; 4] = std::array::from_fn(|i| sgt(a[i].wrapping_add(u32::MAX) ^ c.c2_bits[i], c.c3_bits[i]));
    let f = std::array::from_fn(|i| (!t[i]) & negative[i]);
    let equal: [u32; 4] = std::array::from_fn(|i| if a[i] == c.c4_bits[i] { u32::MAX } else { 0 });
    let greater: [u32; 4] = std::array::from_fn(|i| sgt(a[i], c.c4_bits[i]));
    let h: [u32; 4] = std::array::from_fn(|i| greater[i] | equal[i]);
    let d: [u32; 4] = std::array::from_fn(|i| a[i].wrapping_sub(c.c5_bits[i]) ^ c.c2_bits[i]);
    let j: [u32; 4] = std::array::from_fn(|i| sgt(c.c7_bits[i], d[i]));
    let bits = std::array::from_fn(|i| j[i] | h[i] | f[i]);
    SourceGenerationPacketNormMask { negative, a, t, f, equal, greater, h, d, j, bits }
}
