use super::{FinalLoadInput, LoadError, LoadGlobals, LoadedPartialController,
    CurrentSmallInput, ExistingStatisticsRefusal, ProbeRequest,
    SmallGeometryGuardService, SmallProbeService, prepare_after_wrapper_removal, prepare_final_load};
use super::super::layout::context::{CacheBinding, provider};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OuterEvent { First, Second, Third }

pub trait FinalLoadServices<C> {
    type Error;

    fn reach_controller_action(&mut self, receiver: &mut C) -> Result<FinalLoadInput, Self::Error>;

    fn install_controller_action(&mut self, receiver: &mut C, action: LoadedPartialController)
        -> Result<(), (LoadedPartialController, Self::Error)>;

    fn finish_final_dispatch(&mut self, receiver: &mut C) -> Result<(), Self::Error>;
    fn dispatch_outer_event(&mut self, event: OuterEvent, receiver: &mut C) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    ControllerBoundary, ControllerPrefix, ControllerInstall, FinalDispatchSuffix,
    DirectApplyReturned, OuterFirst, OuterSecond, OuterThird,
    OuterApplyReturned,
}
pub enum Issue<E> { AlreadyHalted, RequiredService(E), SmallPrefix(LoadError), WrongPhase }
pub struct Stopped<C, E> { pub pending: FinalLoadContinuation<C>, pub issue: Issue<E> }

#[must_use = "Keep the advanced same-controller receiver and all intermediate ownership."]
pub struct FinalLoadContinuation<C> {
    receiver: C,
    phase: Phase,
    halted: bool,
    input: Option<FinalLoadInput>,
    uninstalled: Option<LoadedPartialController>,
    entry_globals: Option<LoadGlobals>,
}
pub struct OwnedParts<C> {
    pub receiver: C,
    pub phase: Phase,
    pub halted: bool,
    pub input: Option<FinalLoadInput>,
    pub uninstalled: Option<LoadedPartialController>,
    pub entry_globals: Option<LoadGlobals>,
}

impl<C> FinalLoadContinuation<C> {

    pub fn at_final_event(receiver: C) -> Self {
        Self { receiver, phase: Phase::ControllerBoundary, halted: false,
            input: None, uninstalled: None, entry_globals: None }
    }
    pub fn receiver(&self) -> &C { &self.receiver }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn is_halted(&self) -> bool { self.halted }
    pub fn entry_globals(&self) -> Option<LoadGlobals> { self.entry_globals }
    pub fn uninstalled(&self) -> Option<&LoadedPartialController> { self.uninstalled.as_ref() }
    pub fn input(&self) -> Option<&FinalLoadInput> { self.input.as_ref() }

    pub fn advance<S: FinalLoadServices<C>>(mut self, services: &mut S)
        -> Result<Self, Stopped<C, S::Error>>
    {
        if self.halted { return Err(Stopped { pending: self, issue: Issue::AlreadyHalted }); }
        let result = match self.phase {
            Phase::ControllerBoundary => match services.reach_controller_action(&mut self.receiver) {
                Ok(input) => {
                    self.entry_globals = Some(input.globals); self.input = Some(input);
                    self.phase = Phase::ControllerPrefix; Ok(())
                }
                Err(error) => Err(Issue::RequiredService(error)),
            },
            Phase::ControllerPrefix => match prepare_final_load(self.input.take().unwrap()) {
                Ok(prepared) => match prepared.into_ready() {
                    Ok(action) => { self.uninstalled = Some(action); self.phase = Phase::ControllerInstall; Ok(()) }

                    Err(_) => unreachable!("Final small load is ready without probe"),
                },
                Err(failure) => { self.input = Some(failure.input); Err(Issue::SmallPrefix(failure.error)) }
            },
            Phase::ControllerInstall => {
                match services.install_controller_action(&mut self.receiver, self.uninstalled.take().unwrap()) {
                    Ok(()) => { self.phase = Phase::FinalDispatchSuffix; Ok(()) }
                    Err((action, error)) => { self.uninstalled = Some(action); Err(Issue::RequiredService(error)) }
                }
            }
            Phase::FinalDispatchSuffix => match services.finish_final_dispatch(&mut self.receiver) {
                Ok(()) => { self.phase = Phase::DirectApplyReturned; Ok(()) }
                Err(error) => Err(Issue::RequiredService(error)),
            },
            Phase::OuterFirst | Phase::OuterSecond | Phase::OuterThird => {
                let (event, next) = match self.phase {
                    Phase::OuterFirst => (OuterEvent::First, Phase::OuterSecond),
                    Phase::OuterSecond => (OuterEvent::Second, Phase::OuterThird),
                    Phase::OuterThird => (OuterEvent::Third, Phase::OuterApplyReturned),
                    _ => unreachable!(),
                };
                match services.dispatch_outer_event(event, &mut self.receiver) {
                    Ok(()) => { self.phase = next; Ok(()) }
                    Err(error) => Err(Issue::RequiredService(error)),
                }
            }
            Phase::DirectApplyReturned | Phase::OuterApplyReturned => Err(Issue::WrongPhase),
        };
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(Stopped { pending: self, issue }) }
        }
    }

    pub fn begin_ordinary_outer_suffix(mut self) -> Result<Self, Self> {
        if self.halted || self.phase != Phase::DirectApplyReturned { return Err(self); }
        self.phase = Phase::OuterFirst; Ok(self)
    }
    pub fn into_direct_apply_return(self) -> Result<C, Self> {
        if !self.halted && self.phase == Phase::DirectApplyReturned { Ok(self.receiver) } else { Err(self) }
    }
    pub fn into_outer_apply_return(self) -> Result<C, Self> {
        if !self.halted && self.phase == Phase::OuterApplyReturned { Ok(self.receiver) } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> OwnedParts<C> {
        OwnedParts { receiver: self.receiver, phase: self.phase, halted: self.halted,
            input: self.input, uninstalled: self.uninstalled, entry_globals: self.entry_globals }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynchronizationSite { BasisEnter, BasisLeave, ProbeEnter, ProbeLeave }

pub trait Synchronization<C> {
    type Error;
    fn invoke(&mut self, site: SynchronizationSite, receiver: &mut C) -> Result<(), Self::Error>;
}

pub trait SmallGeometryBody<C> {
    type Error;

    fn check_binding(&mut self, receiver: &C, candidate: &LoadedPartialController) -> Result<(), Self::Error>;

    fn flatten_guard_cleanup(&mut self, receiver: &mut C) -> Result<(), Self::Error>;

    fn current_small_state(&mut self, receiver: &mut C) -> Result<LoadedPartialController, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardServicePhase { Binding, Enter, Body, Leave, CurrentState, Complete }
pub enum GuardServiceError<Y, B> { AlreadyAttempted, Synchronization(Y), Body(B) }

pub struct BoundSmallGeometryGuard<C, Y, B> {
    receiver: C, synchronization: Y, body: B,
    phase: GuardServicePhase, attempted: bool,
}
pub struct GuardOwnedParts<C, Y, B> {
    pub receiver: C, pub synchronization: Y, pub body: B,
    pub phase: GuardServicePhase, pub attempted: bool,
}
impl<C, Y, B> BoundSmallGeometryGuard<C, Y, B> {
    pub fn new(receiver: C, synchronization: Y, body: B) -> Self {
        Self { receiver, synchronization, body, phase: GuardServicePhase::Binding, attempted: false }
    }
    pub fn receiver(&self) -> &C { &self.receiver }
    pub fn phase(&self) -> GuardServicePhase { self.phase }
    pub fn into_owned_parts(self) -> GuardOwnedParts<C, Y, B> {
        GuardOwnedParts { receiver: self.receiver, synchronization: self.synchronization,
            body: self.body, phase: self.phase, attempted: self.attempted }
    }
}
impl<C, Y: Synchronization<C>, B: SmallGeometryBody<C>> SmallGeometryGuardService
    for BoundSmallGeometryGuard<C, Y, B>
{
    type Error = GuardServiceError<Y::Error, B::Error>;
    fn execute(&mut self, candidate: &mut LoadedPartialController) -> Result<(), Self::Error> {
        if self.attempted { return Err(GuardServiceError::AlreadyAttempted); }
        self.attempted = true;
        self.body.check_binding(&self.receiver, candidate).map_err(GuardServiceError::Body)?;
        self.phase = GuardServicePhase::Enter;
        self.synchronization.invoke(SynchronizationSite::BasisEnter, &mut self.receiver)
            .map_err(GuardServiceError::Synchronization)?;
        self.phase = GuardServicePhase::Body;
        self.body.flatten_guard_cleanup(&mut self.receiver).map_err(GuardServiceError::Body)?;
        self.phase = GuardServicePhase::Leave;
        self.synchronization.invoke(SynchronizationSite::BasisLeave, &mut self.receiver)
            .map_err(GuardServiceError::Synchronization)?;
        self.phase = GuardServicePhase::CurrentState;
        let current = self.body.current_small_state(&mut self.receiver).map_err(GuardServiceError::Body)?;
        *candidate = current;
        self.phase = GuardServicePhase::Complete;
        Ok(())
    }
}

pub trait SmallProbeBody<C> {
    type Error;

    fn check_binding(&mut self, receiver: &C, candidate: &LoadedPartialController,
        request: &ProbeRequest) -> Result<(), Self::Error>;

    fn checked_body_cleanup(&mut self, receiver: &mut C) -> Result<(), Self::Error>;
    fn current_small_state(&mut self, receiver: &mut C) -> Result<LoadedPartialController, Self::Error>;
}

pub struct BoundSmallProbe<C, Y, B>(BoundSmallGeometryGuard<C, Y, B>);
impl<C, Y, B> BoundSmallProbe<C, Y, B> {
    pub fn new(receiver: C, synchronization: Y, body: B) -> Self {
        Self(BoundSmallGeometryGuard::new(receiver, synchronization, body))
    }
    pub fn receiver(&self) -> &C { self.0.receiver() }
    pub fn phase(&self) -> GuardServicePhase { self.0.phase() }
    pub fn into_owned_parts(self) -> GuardOwnedParts<C, Y, B> { self.0.into_owned_parts() }
}
impl<C, Y: Synchronization<C>, B: SmallProbeBody<C>> SmallProbeService for BoundSmallProbe<C, Y, B> {
    type Error = GuardServiceError<Y::Error, B::Error>;
    fn execute(&mut self, candidate: &mut LoadedPartialController, request: ProbeRequest) -> Result<(), Self::Error> {
        let state = &mut self.0;
        if state.attempted { return Err(GuardServiceError::AlreadyAttempted); }
        state.attempted = true;
        state.body.check_binding(&state.receiver, candidate, &request).map_err(GuardServiceError::Body)?;
        state.phase = GuardServicePhase::Enter;
        state.synchronization.invoke(SynchronizationSite::ProbeEnter, &mut state.receiver)
            .map_err(GuardServiceError::Synchronization)?;
        state.phase = GuardServicePhase::Body;
        state.body.checked_body_cleanup(&mut state.receiver).map_err(GuardServiceError::Body)?;
        state.phase = GuardServicePhase::Leave;
        state.synchronization.invoke(SynchronizationSite::ProbeLeave, &mut state.receiver)
            .map_err(GuardServiceError::Synchronization)?;
        state.phase = GuardServicePhase::CurrentState;
        let current = state.body.current_small_state(&mut state.receiver).map_err(GuardServiceError::Body)?;
        *candidate = current;
        state.phase = GuardServicePhase::Complete;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterAxis { X, Y }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParameterValueBits(pub u32);
pub struct ParameterInput<Q> { pub caller_arguments: Q, pub radius_bits: u64 }

#[derive(Clone, Debug)]
pub enum ParameterArithmeticError {
    NonfiniteInput { bits: u64 },
    FloatingPointControls(super::provider::Error),
}

pub fn normalized_parameter_value(coordinate: f64) -> Result<ParameterValueBits, ParameterArithmeticError> {
    if !coordinate.is_finite() {
        return Err(ParameterArithmeticError::NonfiniteInput { bits: coordinate.to_bits() });
    }
    super::provider::check_current_thread().map_err(ParameterArithmeticError::FloatingPointControls)?;
    let scaled = coordinate * f64::from_bits(0x3fe0000000000000);
    let shifted = scaled + f64::from_bits(0x3fe0000000000000);
    Ok(ParameterValueBits((shifted as f32).to_bits()))
}

pub fn clamped_radius_bits(bits: u64) -> Result<u64, ParameterArithmeticError> {
    let radius = f64::from_bits(bits);
    if !radius.is_finite() { return Err(ParameterArithmeticError::NonfiniteInput { bits }); }
    super::provider::check_current_thread().map_err(ParameterArithmeticError::FloatingPointControls)?;
    let low = f64::from_bits(0x3fa47ae147ae147b);
    let high = f64::from_bits(0x3fc3333333333333);
    Ok(if radius < low { low.to_bits() } else if radius > high { high.to_bits() } else { bits })
}

pub trait ParameterServices<C, Q> {
    type Parameter: Copy;
    type Error;

    fn current_parameter(&mut self, receiver: &C, axis: ParameterAxis)
        -> Result<Option<Self::Parameter>, Self::Error>;

    fn set_parameter(&mut self, receiver: &mut C, axis: ParameterAxis,
        parameter: Self::Parameter, arguments: &Q) -> Result<ParameterValueBits, Self::Error>;
    fn notify_parameter(&mut self, receiver: &mut C, parameter: Self::Parameter,
        value: ParameterValueBits) -> Result<(), Self::Error>;
    fn clamp_store_radius(&mut self, receiver: &mut C, radius_bits: u64) -> Result<(), Self::Error>;
    fn dispatch_parameter_event(&mut self, receiver: &mut C) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterPhase { ReadX, SetX, NotifyX, ReadY, SetY, NotifyY, Radius, Event, Returned }
pub enum ParameterIssue<E> { AlreadyHalted, AlreadyReturned, RequiredService(E) }
pub struct ParameterStopped<C, Q, P, E> {
    pub pending: ParameterContinuation<C, Q, P>, pub issue: ParameterIssue<E>,
}

#[must_use = "Keep advanced receiver, caller arguments and held parameter identity on refusal."]
pub struct ParameterContinuation<C, Q, P> {
    receiver: C, input: ParameterInput<Q>, held: Option<P>, phase: ParameterPhase, halted: bool,
    value: Option<ParameterValueBits>,
}
pub struct ParameterOwnedParts<C, Q, P> {
    pub receiver: C, pub input: ParameterInput<Q>, pub held: Option<P>,
    pub value: Option<ParameterValueBits>, pub phase: ParameterPhase, pub halted: bool,
}
impl<C, Q, P: Copy> ParameterContinuation<C, Q, P> {
    pub fn new(receiver: C, input: ParameterInput<Q>) -> Self {
        Self { receiver, input, held: None, value: None, phase: ParameterPhase::ReadX, halted: false }
    }
    pub fn receiver(&self) -> &C { &self.receiver }
    pub fn phase(&self) -> ParameterPhase { self.phase }
    pub fn advance<S: ParameterServices<C, Q, Parameter = P>>(mut self, services: &mut S)
        -> Result<Self, ParameterStopped<C, Q, P, S::Error>>
    {
        if self.halted { return Err(ParameterStopped { pending: self, issue: ParameterIssue::AlreadyHalted }); }
        let result = match self.phase {
            ParameterPhase::ReadX | ParameterPhase::ReadY => {
                let x = self.phase == ParameterPhase::ReadX;
                let axis = if x { ParameterAxis::X } else { ParameterAxis::Y };
                match services.current_parameter(&self.receiver, axis) {
                    Ok(parameter) => {
                        self.held = parameter;
                        self.phase = match (x, parameter.is_some()) {
                            (true, true) => ParameterPhase::SetX, (true, false) => ParameterPhase::ReadY,
                            (false, true) => ParameterPhase::SetY, (false, false) => ParameterPhase::Radius,
                        }; Ok(())
                    }
                    Err(error) => Err(ParameterIssue::RequiredService(error)),
                }
            }
            ParameterPhase::SetX | ParameterPhase::SetY => {
                let x = self.phase == ParameterPhase::SetX;
                let axis = if x { ParameterAxis::X } else { ParameterAxis::Y };
                match services.set_parameter(&mut self.receiver, axis, self.held.unwrap(), &self.input.caller_arguments) {
                    Ok(value) => { self.value = Some(value); self.phase = if x { ParameterPhase::NotifyX } else { ParameterPhase::NotifyY }; Ok(()) }
                    Err(error) => Err(ParameterIssue::RequiredService(error)),
                }
            }
            ParameterPhase::NotifyX | ParameterPhase::NotifyY => {
                let x = self.phase == ParameterPhase::NotifyX;

                match services.notify_parameter(&mut self.receiver, self.held.unwrap(), self.value.unwrap()) {
                    Ok(()) => { self.held = None; self.value = None; self.phase = if x { ParameterPhase::ReadY } else { ParameterPhase::Radius }; Ok(()) }
                    Err(error) => Err(ParameterIssue::RequiredService(error)),
                }
            }
            ParameterPhase::Radius => match services.clamp_store_radius(&mut self.receiver, self.input.radius_bits) {
                Ok(()) => { self.phase = ParameterPhase::Event; Ok(()) }
                Err(error) => Err(ParameterIssue::RequiredService(error)),
            },
            ParameterPhase::Event => match services.dispatch_parameter_event(&mut self.receiver) {
                Ok(()) => { self.phase = ParameterPhase::Returned; Ok(()) }
                Err(error) => Err(ParameterIssue::RequiredService(error)),
            },
            ParameterPhase::Returned => Err(ParameterIssue::AlreadyReturned),
        };
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(ParameterStopped { pending: self, issue }) }
        }
    }
    pub fn into_returned(self) -> Result<C, Self> {
        if !self.halted && self.phase == ParameterPhase::Returned { Ok(self.receiver) } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> ParameterOwnedParts<C, Q, P> {
        ParameterOwnedParts { receiver: self.receiver, input: self.input, held: self.held,
            value: self.value, phase: self.phase, halted: self.halted }
    }
}

pub trait SmallCallbackServices<C> {
    type Error;

    fn check_binding(&mut self, receiver: &C, small: &LoadedPartialController) -> Result<(), Self::Error>;
    fn synchronization(&mut self, site: SynchronizationSite, receiver: &mut C,
        small: &mut LoadedPartialController) -> Result<(), Self::Error>;
    fn geometry_guard_cleanup(&mut self, receiver: &mut C,
        small: &mut LoadedPartialController) -> Result<(), Self::Error>;

    fn checked_probe_cleanup(&mut self, receiver: &mut C, small: &mut LoadedPartialController,
        request: ProbeRequest) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackPhase {
    Binding, IncomingMotion, GeometryEnter, GeometryBody, GeometryLeave, Reload,
    ProbeGate, ProbeEnter, ProbeBody, ProbeLeave, Decrement, Returned,
}
pub enum CallbackIssue<E> {
    AlreadyHalted, AlreadyReturned, RequiredService(E),
    FloatingPointControls(provider::Error), CacheBindingChanged,
}
pub struct CallbackStopped<C, S, E> {
    pub pending: OwnedSmallCallback<C, S>, pub issue: CallbackIssue<E>,
}

#[must_use = "Keep this same receiver/state/services owner until returned or explicitly abandoned."]
pub struct OwnedSmallCallback<C, S> {
    receiver: C, small: LoadedPartialController, services: S,
    phase: CallbackPhase, halted: bool, incoming_motion_bits: Option<u64>,
    cache: CacheBinding,
}
pub struct CallbackOwnedParts<C, S> {
    pub receiver: C, pub small: LoadedPartialController, pub services: S,
    pub phase: CallbackPhase, pub halted: bool, pub incoming_motion_bits: Option<u64>,
    pub cache: CacheBinding,
}
impl<C, S: SmallCallbackServices<C>> OwnedSmallCallback<C, S> {

    pub fn new(receiver: C, small: LoadedPartialController, services: S) -> Self {
        let cache = small.cache_binding().clone();
        Self { receiver, small, services, phase: CallbackPhase::Binding,
            halted: false, incoming_motion_bits: None, cache }
    }
    pub fn receiver(&self) -> &C { &self.receiver }
    pub fn candidate(&self) -> &LoadedPartialController { &self.small }
    pub fn phase(&self) -> CallbackPhase { self.phase }
    pub fn incoming_motion_bits(&self) -> Option<u64> { self.incoming_motion_bits }

    fn retain_cache_binding(&mut self) -> bool {
        match (&self.cache, self.small.cache_binding()) {
            (CacheBinding::Bound(before), CacheBinding::Bound(after)) => before.same_snapshot(after),
            (CacheBinding::Bound(_), CacheBinding::Unbound) => false,
            (CacheBinding::Unbound, CacheBinding::Bound(context)) => {

                self.cache = CacheBinding::Bound(context.clone()); true
            }
            (CacheBinding::Unbound, CacheBinding::Unbound) => true,
        }
    }

    pub fn advance(mut self) -> Result<Self, CallbackStopped<C, S, S::Error>> {
        if self.halted { return Err(CallbackStopped { pending: self, issue: CallbackIssue::AlreadyHalted }); }
        let result = match self.phase {
            CallbackPhase::Binding => self.services.check_binding(&self.receiver, &self.small)
                .map(|()| { self.phase = CallbackPhase::IncomingMotion; })
                .map_err(CallbackIssue::RequiredService),
            CallbackPhase::IncomingMotion => match provider::check_current_thread() {
                Err(error) => Err(CallbackIssue::FloatingPointControls(error)),
                Ok(_) => {
                    let motion = self.small.globals().motion;
                    self.incoming_motion_bits = Some(motion.to_bits());
                    self.phase = if super::moving(motion) { CallbackPhase::GeometryEnter } else { CallbackPhase::ProbeGate };
                    Ok(())
                }
            },
            CallbackPhase::GeometryEnter | CallbackPhase::GeometryLeave
                | CallbackPhase::ProbeEnter | CallbackPhase::ProbeLeave => {
                let (site, next) = match self.phase {
                    CallbackPhase::GeometryEnter => (SynchronizationSite::BasisEnter, CallbackPhase::GeometryBody),
                    CallbackPhase::GeometryLeave => (SynchronizationSite::BasisLeave, CallbackPhase::Reload),
                    CallbackPhase::ProbeEnter => (SynchronizationSite::ProbeEnter, CallbackPhase::ProbeBody),
                    CallbackPhase::ProbeLeave => (SynchronizationSite::ProbeLeave, CallbackPhase::Decrement),
                    _ => unreachable!(),
                };
                self.services.synchronization(site, &mut self.receiver, &mut self.small)
                    .map(|()| { self.phase = next; }).map_err(CallbackIssue::RequiredService)
            }
            CallbackPhase::GeometryBody => self.services.geometry_guard_cleanup(&mut self.receiver, &mut self.small)
                .map(|()| { self.phase = CallbackPhase::GeometryLeave; }).map_err(CallbackIssue::RequiredService),
            CallbackPhase::Reload => {
                self.small.state.remaining = self.small.reload;
                self.phase = CallbackPhase::ProbeGate; Ok(())
            }
            CallbackPhase::ProbeGate => {
                self.phase = if self.small.globals().remaining > 0 { CallbackPhase::ProbeEnter } else { CallbackPhase::Returned };
                Ok(())
            }
            CallbackPhase::ProbeBody => match provider::check_current_thread() {
                Err(error) => Err(CallbackIssue::FloatingPointControls(error)),
                Ok(_) => {
                    let request = self.small.current_probe_request(super::AfterProbe::DecrementRemaining);
                    self.services.checked_probe_cleanup(&mut self.receiver, &mut self.small, request)
                        .map(|()| { self.phase = CallbackPhase::ProbeLeave; }).map_err(CallbackIssue::RequiredService)
                }
            },
            CallbackPhase::Decrement => {
                self.small.state.remaining = self.small.state.remaining.wrapping_sub(1);
                self.phase = CallbackPhase::Returned; Ok(())
            }
            CallbackPhase::Returned => Err(CallbackIssue::AlreadyReturned),
        };
        let result = result.and_then(|()| if self.retain_cache_binding() { Ok(()) } else { Err(CallbackIssue::CacheBindingChanged) });
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(CallbackStopped { pending: self, issue }) }
        }
    }
    pub fn into_returned(self) -> Result<(C, LoadedPartialController, S), Self> {
        if !self.halted && self.phase == CallbackPhase::Returned { Ok((self.receiver, self.small, self.services)) } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> CallbackOwnedParts<C, S> {
        CallbackOwnedParts { receiver: self.receiver, small: self.small, services: self.services,
            phase: self.phase, halted: self.halted, incoming_motion_bits: self.incoming_motion_bits, cache: self.cache }
    }
}

pub trait SmallRemovalServices<C, W> {
    type Error;

    fn remove_wrapper_resize(&mut self, receiver: &mut C, index: u32)
        -> Result<Option<W>, (Option<W>, Self::Error)>;

    fn release_wrapper(&mut self, receiver: &mut C, wrapper: W)
        -> Result<(), (Option<W>, Self::Error)>;

    fn current_small_input(&mut self, receiver: &mut C) -> Result<CurrentSmallInput, Self::Error>;
    fn install_statistics_action(&mut self, receiver: &mut C, action: LoadedPartialController)
        -> Result<(), (LoadedPartialController, Self::Error)>;
    fn finish_handler_path_cleanup(&mut self, receiver: &mut C) -> Result<(), Self::Error>;

    fn finish_remaining_dispatch(&mut self, receiver: &mut C) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemovalPhase { RemoveResize, Release, CurrentInput, Statistics, Install, HandlerCleanup, RemainingDispatch, Returned }
pub enum RemovalIssue<E> { AlreadyHalted, AlreadyReturned, RequiredService(E), Statistics(ExistingStatisticsRefusal) }
pub struct RemovalStopped<C, W, S, E> {
    pub pending: OwnedSmallRemoval<C, W, S>, pub issue: RemovalIssue<E>,
}
#[must_use = "Retain the advanced receiver/services/detached wrapper through removal failure."]
pub struct OwnedSmallRemoval<C, W, S> {
    receiver: C, services: S, index: u32, phase: RemovalPhase, halted: bool,
    wrapper: Option<W>, input: Option<CurrentSmallInput>, uninstalled: Option<LoadedPartialController>,
    statistics_entry: Option<LoadGlobals>,
}
pub struct RemovalOwnedParts<C, W, S> {
    pub receiver: C, pub services: S, pub index: u32, pub phase: RemovalPhase, pub halted: bool,
    pub wrapper: Option<W>, pub input: Option<CurrentSmallInput>,
    pub uninstalled: Option<LoadedPartialController>, pub statistics_entry: Option<LoadGlobals>,
}
impl<C, W, S: SmallRemovalServices<C, W>> OwnedSmallRemoval<C, W, S> {

    pub fn new(receiver: C, index: u32, services: S) -> Self {
        Self { receiver, services, index, phase: RemovalPhase::RemoveResize, halted: false,
            wrapper: None, input: None, uninstalled: None, statistics_entry: None }
    }
    pub fn receiver(&self) -> &C { &self.receiver }
    pub fn phase(&self) -> RemovalPhase { self.phase }
    pub fn statistics_entry(&self) -> Option<LoadGlobals> { self.statistics_entry }
    pub fn advance(mut self) -> Result<Self, RemovalStopped<C, W, S, S::Error>> {
        if self.halted { return Err(RemovalStopped { pending: self, issue: RemovalIssue::AlreadyHalted }); }
        let result = match self.phase {
            RemovalPhase::RemoveResize => match self.services.remove_wrapper_resize(&mut self.receiver, self.index) {
                Ok(wrapper) => { self.wrapper = wrapper; self.phase = RemovalPhase::Release; Ok(()) }
                Err((wrapper, error)) => { self.wrapper = wrapper; Err(RemovalIssue::RequiredService(error)) }
            },
            RemovalPhase::Release => match self.wrapper.take() {
                None => { self.phase = RemovalPhase::CurrentInput; Ok(()) }
                Some(wrapper) => match self.services.release_wrapper(&mut self.receiver, wrapper) {
                    Ok(()) => { self.phase = RemovalPhase::CurrentInput; Ok(()) }
                    Err((remaining, error)) => { self.wrapper = remaining; Err(RemovalIssue::RequiredService(error)) }
                }
            },
            RemovalPhase::CurrentInput => match self.services.current_small_input(&mut self.receiver) {
                Ok(input) => { self.statistics_entry = Some(input.globals); self.input = Some(input); self.phase = RemovalPhase::Statistics; Ok(()) }
                Err(error) => Err(RemovalIssue::RequiredService(error)),
            },
            RemovalPhase::Statistics => match prepare_after_wrapper_removal(self.input.take().unwrap()) {
                Ok(prepared) => match prepared.into_ready() {
                    Ok(action) => { self.uninstalled = Some(action); self.phase = RemovalPhase::Install; Ok(()) }
                    Err(_) => unreachable!("Small removal statistics never request probe"),
                },
                Err(failure) => Err(RemovalIssue::Statistics(failure)),
            },
            RemovalPhase::Install => match self.services.install_statistics_action(&mut self.receiver, self.uninstalled.take().unwrap()) {
                Ok(()) => { self.phase = RemovalPhase::HandlerCleanup; Ok(()) }
                Err((action, error)) => { self.uninstalled = Some(action); Err(RemovalIssue::RequiredService(error)) }
            },
            RemovalPhase::HandlerCleanup => match self.services.finish_handler_path_cleanup(&mut self.receiver) {
                Ok(()) => { self.phase = RemovalPhase::RemainingDispatch; Ok(()) }
                Err(error) => Err(RemovalIssue::RequiredService(error)),
            },
            RemovalPhase::RemainingDispatch => match self.services.finish_remaining_dispatch(&mut self.receiver) {
                Ok(()) => { self.phase = RemovalPhase::Returned; Ok(()) }
                Err(error) => Err(RemovalIssue::RequiredService(error)),
            },
            RemovalPhase::Returned => Err(RemovalIssue::AlreadyReturned),
        };
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(RemovalStopped { pending: self, issue }) }
        }
    }
    pub fn into_returned(self) -> Result<(C, S), Self> {
        if !self.halted && self.phase == RemovalPhase::Returned { Ok((self.receiver, self.services)) } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> RemovalOwnedParts<C, W, S> {
        RemovalOwnedParts { receiver: self.receiver, services: self.services, index: self.index,
            phase: self.phase, halted: self.halted, wrapper: self.wrapper, input: self.input,
            uninstalled: self.uninstalled, statistics_entry: self.statistics_entry }
    }
}

pub struct RemovedStateEntry<N> { pub detached: Option<N>, pub count_after: u32 }

pub trait StateRemovalServices<C, N> {
    type Error;
    fn dispatch_remove(&mut self, receiver: &mut C, index: u32) -> Result<(), Self::Error>;
    fn current_state_count(&mut self, receiver: &C) -> Result<u32, Self::Error>;

    fn remove_current_state_entry(&mut self, receiver: &mut C, index: u32)
        -> Result<RemovedStateEntry<N>, (Option<N>, Self::Error)>;

    fn conditional_state_resize(&mut self, receiver: &mut C, count: u32) -> Result<(), Self::Error>;
    fn release_state_source(&mut self, receiver: &mut C, source: N)
        -> Result<(), (Option<N>, Self::Error)>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateRemovalPhase { Dispatch, ReadCount, RemoveEntry, Resize, Release, Returned }
pub enum StateRemovalIssue<E> { AlreadyHalted, AlreadyReturned, CountOutsideSignedDomain, RequiredService(E) }
pub struct StateRemovalStopped<C, N, S, E> {
    pub pending: OwnedStateRemoval<C, N, S>, pub issue: StateRemovalIssue<E>,
}
#[must_use = "Retain actual State/services/held source after callback or release refusal."]
pub struct OwnedStateRemoval<C, N, S> {
    receiver: C, services: S, index: u32, phase: StateRemovalPhase, halted: bool,
    count: Option<u32>, source: Option<N>,
}
pub struct StateRemovalOwnedParts<C, N, S> {
    pub receiver: C, pub services: S, pub index: u32, pub phase: StateRemovalPhase,
    pub halted: bool, pub count: Option<u32>, pub source: Option<N>,
}
impl<C, N, S: StateRemovalServices<C, N>> OwnedStateRemoval<C, N, S> {
    pub fn new(receiver: C, index: u32, services: S) -> Self {
        Self { receiver, services, index, phase: StateRemovalPhase::Dispatch,
            halted: false, count: None, source: None }
    }
    pub fn receiver(&self) -> &C { &self.receiver }
    pub fn phase(&self) -> StateRemovalPhase { self.phase }
    pub fn advance(mut self) -> Result<Self, StateRemovalStopped<C, N, S, S::Error>> {
        if self.halted { return Err(StateRemovalStopped { pending: self, issue: StateRemovalIssue::AlreadyHalted }); }
        let result = match self.phase {
            StateRemovalPhase::Dispatch => match self.services.dispatch_remove(&mut self.receiver, self.index) {
                Ok(()) => { self.phase = StateRemovalPhase::ReadCount; Ok(()) }
                Err(error) => Err(StateRemovalIssue::RequiredService(error)),
            },
            StateRemovalPhase::ReadCount => match self.services.current_state_count(&self.receiver) {
                Ok(count) => {
                    self.count = Some(count);
                    if count > i32::MAX as u32 { Err(StateRemovalIssue::CountOutsideSignedDomain) }
                    else {
                        self.phase = if self.index < count { StateRemovalPhase::RemoveEntry } else { StateRemovalPhase::Resize };
                        Ok(())
                    }
                }
                Err(error) => Err(StateRemovalIssue::RequiredService(error)),
            },
            StateRemovalPhase::RemoveEntry => match self.services.remove_current_state_entry(&mut self.receiver, self.index) {
                Ok(entry) => {
                    self.source = entry.detached; self.count = Some(entry.count_after);
                    if entry.count_after > i32::MAX as u32 { Err(StateRemovalIssue::CountOutsideSignedDomain) }
                    else { self.phase = StateRemovalPhase::Resize; Ok(()) }
                }
                Err((source, error)) => { self.source = source; Err(StateRemovalIssue::RequiredService(error)) }
            },
            StateRemovalPhase::Resize => match self.services.conditional_state_resize(&mut self.receiver, self.count.unwrap()) {
                Ok(()) => { self.phase = StateRemovalPhase::Release; Ok(()) }
                Err(error) => Err(StateRemovalIssue::RequiredService(error)),
            },
            StateRemovalPhase::Release => match self.source.take() {
                None => { self.phase = StateRemovalPhase::Returned; Ok(()) }
                Some(source) => match self.services.release_state_source(&mut self.receiver, source) {
                    Ok(()) => { self.phase = StateRemovalPhase::Returned; Ok(()) }
                    Err((remaining, error)) => { self.source = remaining; Err(StateRemovalIssue::RequiredService(error)) }
                }
            },
            StateRemovalPhase::Returned => Err(StateRemovalIssue::AlreadyReturned),
        };
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(StateRemovalStopped { pending: self, issue }) }
        }
    }

    pub fn into_returned(self) -> Result<(C, S), Self> {
        if !self.halted && self.phase == StateRemovalPhase::Returned { Ok((self.receiver, self.services)) } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> StateRemovalOwnedParts<C, N, S> {
        StateRemovalOwnedParts { receiver: self.receiver, services: self.services, index: self.index,
            phase: self.phase, halted: self.halted, count: self.count, source: self.source }
    }
}
