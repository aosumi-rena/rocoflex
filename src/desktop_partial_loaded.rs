use super::{AfterProbe, CheckedProbeCompletion, Error, MetadataError, PartialController, PrepareFailure,
    Phase, Point, ProbeControls, ProbeRequest, Projection, RequestKind, RequestedRoot,
    PromotionState, RootState, Source, SourceField, View, Wrapper,
    checked_fields, identity4, moving, sample, valid_controls};
use super::layout::context::{CacheBinding, provider};
use super::layout::context::{ContextRequest, arithmetic::cache_blocking::Product};

#[path = "desktop_partial_loaded_event.rs"]
pub mod event_boundary;
#[path = "desktop_partial_loaded_release.rs"]
pub mod source_release;
#[path = "desktop_partial_loaded_append.rs"]
pub mod append_boundary;

#[derive(Clone, Copy, Debug)]
pub struct LoadGlobals { pub motion: f64, pub remaining: i32, pub reload: i32 }

pub struct FinalLoadInput {
    pub globals: LoadGlobals,

    pub cache: CacheBinding,
    pub parsed_projection: Projection,
    pub sources: Vec<Source>,
    pub controls: ProbeControls,
    pub completed_probe_geometry: Option<std::sync::Arc<super::layout::background_geometry::CompletedGeometry>>,
}
#[derive(Clone, Debug)]
pub enum LoadError {
    OutsideSmallPointDomain, NonfiniteReduced, InvalidControls, ParsedProjectionNotConstructor,
    FloatingPointControls(provider::Error),
}
pub struct LoadRefusal { pub input: FinalLoadInput, pub error: LoadError }

#[derive(Clone, Debug)]
pub struct LoadedPartialController { state: PartialController, reload: i32, cache: CacheBinding }

#[must_use = "Retain the loaded candidate through checked probe completion or refusal."]
pub struct PreparedLoadedPartial {
    candidate: LoadedPartialController, phase: Phase,
    after: Option<AfterProbe>, probe_attempted: bool,
}

pub struct CurrentSmallInput {
    pub projection: Projection,
    pub point: Option<Point>,

    pub sources: Vec<Source>,
    pub globals: LoadGlobals, pub controls: ProbeControls, pub cache: CacheBinding,
    pub completed_probe_geometry: Option<std::sync::Arc<super::layout::background_geometry::CompletedGeometry>>,
}
pub struct CurrentSmallRefusal { pub input: CurrentSmallInput, pub error: Error }
pub struct SourceStateRefusal {
    pub state: PromotionState, pub reload: i32, pub cache: CacheBinding, pub error: Error,
}
pub struct AppendConstructionRefusal {
    pub before: LoadedPartialController, pub source: Source, pub error: Error,
}

#[must_use = "Statistics/real services/checked probe and current reload suffix remain pending."]
pub struct AppendedSourceState {
    state: PromotionState, reload: i32, cache: CacheBinding, appended_index: usize,
}
impl AppendedSourceState {
    pub fn state(&self) -> &PromotionState { &self.state }
    pub fn cache_binding(&self) -> &CacheBinding { &self.cache }
    pub fn reload(&self) -> i32 { self.reload }
    pub fn appended_index(&self) -> usize { self.appended_index }

    pub fn statistics_context_request(&self) -> Option<ContextRequest> {
        let total = self.state.sources.iter().map(|source| match source {
            Source::Point { .. } => 1, Source::Trajectory { reduced, .. } => reduced.len(),
        }).sum::<usize>();
        (total >= 12 && self.cache.snapshot().is_none()).then_some(ContextRequest {
            product: Product::Global4d, depth: total, owner: None,
        })
    }
    pub fn into_parts(self) -> (PromotionState, i32, CacheBinding) { (self.state, self.reload, self.cache) }
}

pub fn construct_appended_source_state(before: LoadedPartialController, source: Source)
    -> Result<AppendedSourceState, AppendConstructionRefusal>
{
    let maximum_children = if before.point().is_some() { 2048 } else { 2049 };
    let error = match &source {
        Source::Point { reduced, .. } if reduced.iter().any(|v| !v.is_finite()) => Some(Error::NonfiniteInput),
        Source::Trajectory { reduced, .. } if !(2..=maximum_children).contains(&reduced.len()) => Some(Error::ChildCountOutsideDomain),
        Source::Trajectory { reduced, .. } if reduced.iter().flatten().any(|v| !v.is_finite()) => Some(Error::NonfiniteInput),
        _ => None,
    };
    if let Some(error) = error { return Err(AppendConstructionRefusal { before, source, error }); }
    let (mut state, reload, cache) = before.into_source_state();
    let appended_index = state.sources.len();
    let root = RootState::constructor(&source);
    state.sources.push(source); state.roots.push(root);
    Ok(AppendedSourceState { state, reload, cache, appended_index })
}
pub enum ExistingStatisticsRefusal {
    Input(CurrentSmallRefusal),
    FloatingPointControls { candidate: LoadedPartialController, error: provider::Error },
}

pub fn prepare_after_wrapper_removal(input: CurrentSmallInput)
    -> Result<PreparedLoadedPartial, ExistingStatisticsRefusal>
{
    let mut candidate = LoadedPartialController::from_current_boundary(input)
        .map_err(ExistingStatisticsRefusal::Input)?;
    if let Err(error) = provider::check_current_thread() {
        return Err(ExistingStatisticsRefusal::FloatingPointControls { candidate, error });
    }
    candidate.state.small_statistics();
    Ok(PreparedLoadedPartial::ready(candidate))
}

pub fn prepare_final_load(input: FinalLoadInput) -> Result<PreparedLoadedPartial, LoadRefusal> {
    let error = if !matches!(input.sources.as_slice(), [] | [Source::Point { .. }]) {
        Some(LoadError::OutsideSmallPointDomain)
    } else if input.sources.iter().any(|source| matches!(source, Source::Point { reduced, .. }
        if reduced.iter().any(|v| !v.is_finite()))) {
        Some(LoadError::NonfiniteReduced)
    } else if valid_controls(input.controls).is_err() {
        Some(LoadError::InvalidControls)
    } else if !matches!(input.parsed_projection.negative_mean, SourceField::Unwritten)
        || !matches!(input.parsed_projection.scale, SourceField::Unwritten)
        || !matches!(input.parsed_projection.projected_basis, SourceField::Unwritten)
        || input.parsed_projection.covariance.map(f32::to_bits) != identity4().map(f32::to_bits) {
        Some(LoadError::ParsedProjectionNotConstructor)
    } else { None };
    if let Some(error) = error { return Err(LoadRefusal { input, error }); }
    if let Err(error) = provider::check_current_thread() {
        return Err(LoadRefusal { input, error: LoadError::FloatingPointControls(error) });
    }
    let FinalLoadInput { globals, cache, parsed_projection, sources, controls, completed_probe_geometry } = input;
    let point = match sources.as_slice() {
        [Source::Point { reduced, .. }] => Some(Point::constructor(*reduced)), _ => None,
    };
    let mut state = PartialController { projection: parsed_projection, point, sources,
        motion: globals.motion, remaining: globals.remaining, controls, completed_probe_geometry };
    state.small_statistics();
    Ok(PreparedLoadedPartial::ready(LoadedPartialController { state, reload: globals.reload, cache }))
}

impl LoadedPartialController {

    pub fn from_source_state(state: PromotionState, reload: i32, cache: CacheBinding)
        -> Result<Self, SourceStateRefusal>
    {
        let shape = match (state.sources.as_slice(), state.roots.as_slice()) {
            ([], []) => true,
            ([Source::Point { reduced, .. }], [root]) => root.children.is_empty()
                && root.covariance.is_none() && root.standalone_scalars.is_some()
                && reduced.iter().all(|v| v.is_finite()),
            _ => false,
        };
        let error = if !shape { Some(Error::GeometryShape) } else { valid_controls(state.controls).err() };
        if let Some(error) = error { return Err(SourceStateRefusal { state, reload, cache, error }); }
        let PromotionState { projection, sources, mut roots, motion, remaining, controls, completed_probe_geometry } = state;
        let point = if let Some(root) = roots.pop() {
            let Source::Point { reduced, .. } = &sources[0] else { unreachable!() };
            let Wrapper { base, offset, force } = root.point;
            Some(Point { reduced: *reduced, base, offset, force, scalars: root.standalone_scalars.unwrap() })
        } else { None };
        Ok(Self { state: PartialController { projection, point, sources, motion, remaining, controls, completed_probe_geometry }, reload, cache })
    }

    pub fn into_source_state(self) -> (PromotionState, i32, CacheBinding) {
        let PartialController { projection, point, sources, motion, remaining, controls, completed_probe_geometry } = self.state;
        let roots = point.into_iter().map(|point| RootState {
            point: Wrapper { base: point.base, offset: point.offset, force: point.force },
            standalone_scalars: Some(point.scalars), covariance: None, children: vec![],
        }).collect();
        (PromotionState { projection, sources, roots, motion, remaining, controls, completed_probe_geometry }, self.reload, self.cache)
    }
    pub fn from_current_boundary(input: CurrentSmallInput) -> Result<Self, CurrentSmallRefusal> {
        if let Err(error) = valid_controls(input.controls) { return Err(CurrentSmallRefusal { input, error }); }
        let valid = match (&input.point, input.sources.as_slice()) {
            (None, []) => true,
            (Some(point), [Source::Point { reduced, .. }]) =>
                point.reduced.map(f32::to_bits) == reduced.map(f32::to_bits)
                    && reduced.iter().all(|v| v.is_finite()),
            _ => false,
        };
        if !valid { return Err(CurrentSmallRefusal { input, error: Error::GeometryShape }); }
        let CurrentSmallInput { projection, point, sources, globals, controls, cache, completed_probe_geometry } = input;
        Ok(Self { state: PartialController { projection, point, sources,
            motion: globals.motion, remaining: globals.remaining, controls, completed_probe_geometry }, reload: globals.reload, cache })
    }
    pub fn sources(&self) -> &[Source] { self.state.sources() }
    pub fn projection(&self) -> &Projection { self.state.projection() }
    pub fn point(&self) -> Option<&Point> { self.state.point() }
    pub fn controls(&self) -> ProbeControls { self.state.controls }
    pub fn cache_binding(&self) -> &CacheBinding { &self.cache }
    pub fn globals(&self) -> LoadGlobals {
        LoadGlobals { motion: self.state.motion, remaining: self.state.remaining, reload: self.reload }
    }

    pub fn into_current_boundary(self) -> CurrentSmallInput {
        let PartialController { projection, point, sources, motion, remaining, controls, completed_probe_geometry } = self.state;
        CurrentSmallInput { projection, point, sources, globals: LoadGlobals {
            motion, remaining, reload: self.reload,
        }, controls, cache: self.cache, completed_probe_geometry }
    }

    pub fn install_service_globals(&mut self, globals: LoadGlobals) {
        self.state.motion = globals.motion;
        self.state.remaining = globals.remaining;
        self.reload = globals.reload;
    }

    pub fn install_checked_probe_output(&mut self, checked: CheckedProbeCompletion) -> Result<(), Error> {
        let kind = if self.state.point.is_some() { RequestKind::CheckedLiteral } else { RequestKind::EmptyGuard };
        let completed = checked_fields(checked, kind, usize::from(self.state.point.is_some()))?;
        if let Some(point) = &mut self.state.point { point.scalars = completed.fields[0]; }

        Ok(())
    }
    pub(super) fn current_probe_request(&self, after: AfterProbe) -> ProbeRequest {
        let roots = self.state.point.iter().map(|point| RequestedRoot {
            owner: 0, trajectory: false, points: vec![sample(point, None)],
        }).collect();
        ProbeRequest { kind: if self.state.point.is_some() {
            RequestKind::CheckedLiteral
        } else { RequestKind::EmptyGuard }, roots, controls: self.state.controls, after }
    }
    pub fn checked_view(&self) -> Result<View, Error> { self.state.checked_view() }
    pub fn working_uv_snapshot(&self) -> [f32;8] { self.state.working_uv_snapshot() }
    pub fn source_transformation_snapshot(&self) -> [f64;8] { self.state.source_transformation_snapshot() }
    pub fn active(&self) -> bool { moving(self.state.motion) || self.state.remaining > 0 }

    pub fn prepare_first_point(&self, source: Source) -> Result<PreparedLoadedPartial, PrepareFailure> {
        let Source::Point { reduced, .. } = &source else {
            return Err(PrepareFailure { source, error: Error::NeedFirstPoint });
        };
        if self.state.point.is_some() || !self.state.sources.is_empty() {
            return Err(PrepareFailure { source, error: Error::NeedFirstPoint });
        }
        if reduced.iter().any(|v| !v.is_finite()) {
            return Err(PrepareFailure { source, error: Error::NonfiniteInput });
        }
        if let Err(error) = provider::check_current_thread() {
            return Err(PrepareFailure { source, error: Error::ExternalGeometry(format!("Floating-point controls: {error:?}")) });
        }
        let mut candidate = self.clone();
        candidate.state.point = Some(Point::constructor(*reduced));
        candidate.state.sources.push(source);
        candidate.state.small_statistics();
        candidate.state.remaining = candidate.reload;
        Ok(PreparedLoadedPartial::ready(candidate))
    }

    pub fn fork_with_matching_metadata(&self, sources: &[Source]) -> Result<Self, MetadataError> {
        let state = self.state.fork_with_matching_metadata(sources)?;
        Ok(Self { state, reload: self.reload, cache: self.cache.clone() })
    }

    pub fn prepare_parameter_event(&self, controls: ProbeControls) -> Result<PreparedLoadedPartial, Error> {
        valid_controls(controls)?;
        let mut candidate = self.clone();
        candidate.state.controls = controls;
        candidate.state.remaining = candidate.reload;
        Ok(PreparedLoadedPartial::ready(candidate))
    }

    pub fn prepare_callback(&self) -> CallbackPreparation {
        if moving(self.state.motion) {
            CallbackPreparation::GeometryGuard(RequiredSmallGeometryGuard {
                candidate: self.clone(), attempted: false,
            })
        } else { CallbackPreparation::ProbeGate(PreparedLoadedPartial::probe_gate(self.clone())) }
    }
}

pub enum CallbackPreparation {
    GeometryGuard(RequiredSmallGeometryGuard),
    ProbeGate(PreparedLoadedPartial),
}

pub trait SmallGeometryGuardService {
    type Error;
    fn execute(&mut self, candidate: &mut LoadedPartialController) -> Result<(), Self::Error>;
}

#[must_use = "Keep the advanced receiver and service failure; the guard cannot be retried."]
pub struct RequiredSmallGeometryGuard { candidate: LoadedPartialController, attempted: bool }
pub enum GuardError<E> { AlreadyAttempted, Service(E) }
pub struct GuardFailure<E> { pub pending: RequiredSmallGeometryGuard, pub error: GuardError<E> }
pub struct OwnedGuardFailure<S, E> {
    pub pending: RequiredSmallGeometryGuard, pub service: S, pub error: GuardError<E>,
}

pub trait SmallProbeService {
    type Error;
    fn execute(&mut self, candidate: &mut LoadedPartialController, request: ProbeRequest) -> Result<(), Self::Error>;
}
pub enum ProbeServiceError<E> { WrongPhase, AlreadyAttempted, Service(E) }
pub struct ProbeServiceFailure<E> { pub pending: PreparedLoadedPartial, pub error: ProbeServiceError<E> }
pub struct OwnedProbeServiceFailure<S, E> {
    pub pending: PreparedLoadedPartial, pub service: S, pub error: ProbeServiceError<E>,
}

impl RequiredSmallGeometryGuard {
    pub fn candidate(&self) -> &LoadedPartialController { &self.candidate }
    pub fn attempted(&self) -> bool { self.attempted }
    pub fn execute<S: SmallGeometryGuardService>(mut self, service: &mut S)
        -> Result<PreparedLoadedPartial, GuardFailure<S::Error>>
    {
        if self.attempted { return Err(GuardFailure { pending: self, error: GuardError::AlreadyAttempted }); }
        self.attempted = true;
        if let Err(error) = service.execute(&mut self.candidate) {
            return Err(GuardFailure { pending: self, error: GuardError::Service(error) });
        }

        self.candidate.state.remaining = self.candidate.reload;
        Ok(PreparedLoadedPartial::probe_gate(self.candidate))
    }
    pub fn into_advanced(self) -> LoadedPartialController { self.candidate }

    pub fn execute_owned<S: SmallGeometryGuardService>(self, mut service: S)
        -> Result<(PreparedLoadedPartial, S), OwnedGuardFailure<S, S::Error>>
    {
        match self.execute(&mut service) {
            Ok(prepared) => Ok((prepared, service)),
            Err(failure) => Err(OwnedGuardFailure { pending: failure.pending, service, error: failure.error }),
        }
    }
}

impl PreparedLoadedPartial {
    fn ready(candidate: LoadedPartialController) -> Self {
        Self { candidate, phase: Phase::Ready, after: None, probe_attempted: false }
    }
    fn probe_gate(candidate: LoadedPartialController) -> Self {
        let after = (candidate.state.remaining > 0).then_some(AfterProbe::DecrementRemaining);
        Self { candidate, phase: if after.is_some() { Phase::AwaitingProbe } else { Phase::Ready },
            after, probe_attempted: false }
    }
    pub fn candidate(&self) -> &LoadedPartialController { &self.candidate }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn request(&self) -> Option<ProbeRequest> {
        let after = self.after?;
        if self.phase != Phase::AwaitingProbe { return None; }
        Some(self.candidate.current_probe_request(after))
    }
    pub fn probe_attempted(&self) -> bool { self.probe_attempted }

    pub fn finish_probe<S: SmallProbeService>(mut self, service: &mut S)
        -> Result<Self, ProbeServiceFailure<S::Error>>
    {
        if self.probe_attempted {
            return Err(ProbeServiceFailure { pending: self, error: ProbeServiceError::AlreadyAttempted });
        }
        let Some(request) = self.request() else {
            return Err(ProbeServiceFailure { pending: self, error: ProbeServiceError::WrongPhase });
        };
        self.probe_attempted = true;
        if let Err(error) = service.execute(&mut self.candidate, request) {
            self.phase = Phase::ProbeFailed;
            return Err(ProbeServiceFailure { pending: self, error: ProbeServiceError::Service(error) });
        }
        match self.after.take().unwrap() {
            AfterProbe::DecrementRemaining => self.candidate.state.remaining = self.candidate.state.remaining.wrapping_sub(1),
            AfterProbe::ReloadRemaining => self.candidate.state.remaining = self.candidate.reload,
        }
        self.phase = Phase::Ready;
        Ok(self)
    }
    pub fn into_ready(self) -> Result<LoadedPartialController, Self> {
        if self.phase == Phase::Ready { Ok(self.candidate) } else { Err(self) }
    }
    pub fn finish_probe_owned<S: SmallProbeService>(self, mut service: S)
        -> Result<(Self, S), OwnedProbeServiceFailure<S, S::Error>>
    {
        match self.finish_probe(&mut service) {
            Ok(prepared) => Ok((prepared, service)),
            Err(failure) => Err(OwnedProbeServiceFailure { pending: failure.pending, service, error: failure.error }),
        }
    }
}
