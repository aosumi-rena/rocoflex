use super::{AppendedSourceState, LoadGlobals, event_boundary::SynchronizationSite};
use super::super::{AfterProbe, CheckedProbeCompletion, Error, ProbeRequest, PromotionState,
    RequestKind, Source, geometry, valid_controls};
use super::super::layout::context::{CacheBinding, ContextRequest, GeometryContext, GeometryPlans, provider};
use super::super::layout::context::arithmetic::cache_blocking::Product;

pub struct AppendState {
    state: PromotionState, reload: i32, cache: CacheBinding, plans: GeometryPlans,
    appended_index: usize, checked: Option<CheckedProbeCompletion>,
    probe_commit_open: bool, probe_commit_attempted: bool, probe_committed: bool,
}
impl AppendState {
    pub fn state(&self) -> &PromotionState { &self.state }
    pub fn cache_binding(&self) -> &CacheBinding { &self.cache }
    pub fn plans(&self) -> &GeometryPlans { &self.plans }
    pub fn appended_index(&self) -> usize { self.appended_index }
    pub fn checked_probe_completion(&self) -> Option<&CheckedProbeCompletion> { self.checked.as_ref() }
    pub fn globals(&self) -> LoadGlobals {
        LoadGlobals { motion: self.state.motion, remaining: self.state.remaining, reload: self.reload }
    }

    pub fn service_state_mut(&mut self) -> (&mut PromotionState, &mut i32) { (&mut self.state, &mut self.reload) }
    pub fn into_parts(self) -> (PromotionState, i32, CacheBinding, GeometryPlans, Option<CheckedProbeCompletion>) {
        (self.state, self.reload, self.cache, self.plans, self.checked)
    }

    pub fn bind_service_context(&mut self, context: GeometryContext) -> Result<(), ContextRefusal> {
        if let Some(bound) = self.cache.snapshot() {
            if !bound.same_snapshot(&context) { return Err(ContextRefusal { context, issue: ContextIssue::SnapshotChanged }); }
            return Ok(());
        }
        if let Err(error) = provider::check_current_thread() {
            return Err(ContextRefusal { context, issue: ContextIssue::Provider(error) });
        }
        self.cache = CacheBinding::Bound(context); Ok(())
    }
    fn current_shape(&self) -> Result<usize, Error> {
        if self.state.sources.len() != self.state.roots.len() { return Err(Error::GeometryShape); }
        valid_controls(self.state.controls)?;
        let mut total = 0usize;
        for (source, root) in self.state.sources.iter().zip(&self.state.roots) {
            let n = match source {
                Source::Point { reduced, .. } => {
                    if !root.children.is_empty() || root.covariance.is_some() || root.standalone_scalars.is_none() {
                        return Err(Error::GeometryShape);
                    }
                    if reduced.iter().any(|value| !value.is_finite()) { return Err(Error::NonfiniteInput); }
                    1
                }
                Source::Trajectory { reduced, .. } => {
                    if !(2..=2049).contains(&reduced.len()) || reduced.len() != root.children.len()
                        || root.covariance.is_none() || root.standalone_scalars.is_some() { return Err(Error::GeometryShape); }
                    for (row, child) in reduced.iter().zip(&root.children) {
                        if row.iter().any(|value| !value.is_finite()) { return Err(Error::NonfiniteInput); }
                        if row.map(f32::to_bits) != child.reduced.map(f32::to_bits) { return Err(Error::GeometryShape); }
                    }
                    reduced.len()
                }
            };
            total = total.checked_add(n).ok_or(Error::GeometryShape)?;
        }
        if total > 2049 { return Err(Error::ChildCountOutsideDomain); }
        Ok(total)
    }

    pub fn install_checked_probe_output(&mut self, checked: CheckedProbeCompletion) -> Result<(), Error> {
        if !self.probe_commit_open || self.probe_commit_attempted { return Err(Error::ProbeNotRequested); }
        self.probe_commit_attempted = true; self.checked = Some(checked);
        let expected = self.current_shape()?;
        let fields = match self.checked.as_ref().unwrap() {
            CheckedProbeCompletion::Literal(Ok(completed)) if expected > 0 => &completed.fields,
            CheckedProbeCompletion::Literal(Err(error)) => return Err(Error::ExternalProbe(error.clone())),
            CheckedProbeCompletion::EmptyGuard(Ok(())) if expected == 0 => { self.probe_committed = true; return Ok(()); }
            CheckedProbeCompletion::EmptyGuard(Err(error)) => return Err(Error::ExternalProbe(error.clone())),
            _ => return Err(Error::WrongCompletionKind),
        };
        if fields.len() != expected { return Err(Error::WrongScalarCount); }
        if fields.iter().flatten().any(|value| !value.is_finite()) { return Err(Error::NonfiniteProbe); }
        self.state.commit_probe_fields(fields);

        self.probe_committed = true; Ok(())
    }
}

pub trait AppendServices<C, T> {
    type Error;

    fn check_binding(&mut self, receiver: &C, candidate: &AppendState, staged_source: &Option<T>) -> Result<(), Self::Error>;

    fn statistics_before_geometry_cleanup(&mut self, receiver: &mut C, candidate: &mut AppendState,
        staged_source: &mut Option<T>) -> Result<(), Self::Error>;
    fn synchronization(&mut self, site: SynchronizationSite, receiver: &mut C,
        candidate: &mut AppendState, staged_source: &mut Option<T>) -> Result<(), Self::Error>;

    fn geometry_cleanup(&mut self, receiver: &mut C, candidate: &mut AppendState,
        staged_source: &mut Option<T>) -> Result<(), Self::Error>;

    fn checked_probe_body(&mut self, receiver: &mut C, candidate: &mut AppendState,
        request: &ProbeRequest, staged_source: &mut Option<T>) -> Result<(), Self::Error>;
    fn probe_cleanup(&mut self, receiver: &mut C, candidate: &mut AppendState,
        staged_source: &mut Option<T>) -> Result<(), Self::Error>;

    fn statistics_cleanup(&mut self, receiver: &mut C, candidate: &mut AppendState,
        staged_source: &mut Option<T>) -> Result<(), Self::Error>;
    fn handler_cleanup(&mut self, receiver: &mut C, candidate: &mut AppendState,
        staged_source: &mut Option<T>) -> Result<(), Self::Error>;

    fn remaining_dispatch(&mut self, receiver: &mut C, candidate: &mut AppendState,
        staged_source: &mut Option<T>) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Binding, StatisticsContext, Statistics, StatisticsBeforeGeometryCleanup, GeometryEnter, GeometryBody, GeometryCleanup,
    GeometryLeave, ProbeEnter, ProbeBody, ProbeCleanup, ProbeLeave, StatisticsCleanup,
    Reload, HandlerCleanup, RemainingDispatch, AwaitingContext, Returned,
}
pub enum Issue<E> {
    AlreadyHalted, WrongPhase, RequiredService(E), Geometry(Error), Probe(Error),
    Provider(provider::Error), ProviderAlreadyAttempted, Context(ContextRefusal),
}
pub struct Stopped<C, T, S, E> { pub pending: OwnedAppend<C, T, S>, pub issue: Issue<E> }
pub enum ContextIssue { WrongPhase, SnapshotChanged, Provider(provider::Error) }
pub struct ContextRefusal { pub context: GeometryContext, pub issue: ContextIssue }
pub struct ContextStopped<C, T, S> { pub pending: OwnedAppend<C, T, S>, pub refusal: ContextRefusal }

#[must_use = "Retain receiver/source/services and advanced state through context or probe refusal."]
pub struct OwnedAppend<C, T, S> {
    receiver: C, candidate: AppendState, staged_source: Option<T>, services: S,
    phase: Phase, halted: bool, resume: Option<Phase>, request: Option<ContextRequest>,
    provider_attempted: bool, probe_request: Option<ProbeRequest>,
}
pub struct OwnedParts<C, T, S> {
    pub receiver: C, pub candidate: AppendState, pub staged_source: Option<T>, pub services: S,
    pub phase: Phase, pub halted: bool, pub resume: Option<Phase>, pub request: Option<ContextRequest>,
    pub provider_attempted: bool, pub probe_request: Option<ProbeRequest>,
}
impl<C, T, S: AppendServices<C, T>> OwnedAppend<C, T, S> {

    pub fn from_constructed(receiver: C, appended: AppendedSourceState, staged_source: Option<T>, services: S) -> Self {
        let appended_index = appended.appended_index();
        let (state, reload, cache) = appended.into_parts();
        Self { receiver, candidate: AppendState { state, reload, cache, plans: GeometryPlans::default(), appended_index,
                checked: None, probe_commit_open: false, probe_commit_attempted: false, probe_committed: false },
            staged_source, services, phase: Phase::Binding, halted: false, resume: None,
            request: None, provider_attempted: false, probe_request: None }
    }
    pub fn receiver(&self) -> &C { &self.receiver }
    pub fn candidate(&self) -> &AppendState { &self.candidate }
    pub fn staged_source(&self) -> Option<&T> { self.staged_source.as_ref() }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn context_request(&self) -> Option<ContextRequest> { self.request }
    pub fn probe_request(&self) -> Option<&ProbeRequest> { self.probe_request.as_ref() }
    fn wait_for_context(&mut self, request: ContextRequest, resume: Phase) {
        self.request = Some(request); self.resume = Some(resume); self.phase = Phase::AwaitingContext;
    }

    pub fn complete_context(mut self, context: GeometryContext) -> Result<Self, ContextStopped<C, T, S>> {
        if self.halted || self.phase != Phase::AwaitingContext || self.resume.is_none()
            || self.candidate.cache.snapshot().is_some() {
            return Err(ContextStopped { pending: self, refusal: ContextRefusal { context, issue: ContextIssue::WrongPhase } });
        }
        if let Err(refusal) = self.candidate.bind_service_context(context) {
            return Err(ContextStopped { pending: self, refusal });
        }
        self.phase = self.resume.take().unwrap(); self.request = None; Ok(self)
    }

    pub fn resolve_context_from_process(mut self, separate_operations: bool) -> Result<Self, Stopped<C, T, S, S::Error>> {
        if self.halted || self.phase != Phase::AwaitingContext {
            return Err(Stopped { pending: self, issue: Issue::WrongPhase });
        }
        if self.provider_attempted { return Err(Stopped { pending: self, issue: Issue::ProviderAlreadyAttempted }); }
        self.provider_attempted = true;
        let context = match provider::process_context(separate_operations) {
            Ok(context) => context,
            Err(error) => return Err(Stopped { pending: self, issue: Issue::Provider(error) }),
        };
        match self.complete_context(context) {
            Ok(pending) => Ok(pending),
            Err(failure) => Err(Stopped { pending: failure.pending, issue: Issue::Context(failure.refusal) }),
        }
    }

    pub fn advance(mut self) -> Result<Self, Stopped<C, T, S, S::Error>> {
        if self.halted { return Err(Stopped { pending: self, issue: Issue::AlreadyHalted }); }
        if matches!(self.phase, Phase::AwaitingContext | Phase::Returned) {
            return Err(Stopped { pending: self, issue: Issue::WrongPhase });
        }
        let result = match self.phase {
            Phase::Binding => self.services.check_binding(&self.receiver, &self.candidate, &self.staged_source)
                .map(|()| { self.phase = Phase::StatisticsContext; }).map_err(Issue::RequiredService),
            Phase::StatisticsContext => match self.candidate.current_shape() {
                Err(error) => Err(Issue::Geometry(error)),
                Ok(total) => {
                    if total >= 12 && self.candidate.cache.snapshot().is_none() {
                        self.wait_for_context(ContextRequest { product: Product::Global4d, depth: total, owner: None }, Phase::Statistics);
                    } else { self.phase = Phase::Statistics; }
                    Ok(())
                }
            },
            Phase::Statistics => match self.candidate.current_shape() {
                Err(error) => Err(Issue::Geometry(error)),
                Ok(total) if total < 2 => match provider::check_current_thread() {
                    Err(error) => Err(Issue::Provider(error)),
                    Ok(_) => {
                        let rows = geometry::flatten(&self.candidate.state.sources);
                        self.candidate.state.projection.small_statistics(&rows);
                        let plan = super::super::layout::context::CountPlan {
                            request: ContextRequest { product: Product::Global4d, depth: total, owner: None }, blocking: None,
                        };
                        self.candidate.plans.statistics_attempt = Some(plan.clone());
                        self.candidate.plans.cached_statistics = Some(plan);

                        self.phase = Phase::StatisticsCleanup; Ok(())
                    }
                },
                Ok(_) => self.candidate.state.statistics_prefix_with_context(
                    self.candidate.cache.snapshot(), &mut self.candidate.plans)
                    .map(|()| { self.phase = Phase::StatisticsBeforeGeometryCleanup; }).map_err(Issue::Geometry),
            },
            Phase::GeometryEnter | Phase::GeometryLeave | Phase::ProbeEnter | Phase::ProbeLeave => {
                let (site, next) = match self.phase {
                    Phase::GeometryEnter => (SynchronizationSite::BasisEnter, Phase::GeometryBody),
                    Phase::GeometryLeave => (SynchronizationSite::BasisLeave, Phase::ProbeEnter),
                    Phase::ProbeEnter => (SynchronizationSite::ProbeEnter, Phase::ProbeBody),
                    Phase::ProbeLeave => (SynchronizationSite::ProbeLeave, Phase::StatisticsCleanup),
                    _ => unreachable!(),
                };
                self.services.synchronization(site, &mut self.receiver, &mut self.candidate, &mut self.staged_source)
                    .map(|()| { self.phase = next; }).map_err(Issue::RequiredService)
            }
            Phase::GeometryBody => match self.candidate.current_shape() {
                Err(error) => Err(Issue::Geometry(error)),
                Ok(total) if total < 2 => { self.phase = Phase::GeometryCleanup; Ok(()) },
                Ok(_) => {
                    let packed_parent = self.candidate.state.sources.iter().enumerate().find_map(|(owner, source)| match source {
                        Source::Trajectory { reduced, .. } if reduced.len() >= 16 => Some(ContextRequest {
                            product: Product::Parent2d, depth: reduced.len(), owner: Some(owner) }), _ => None,
                    });
                    if let Some(request) = packed_parent.filter(|_| self.candidate.cache.snapshot().is_none()) {
                        self.wait_for_context(request, Phase::GeometryBody); Ok(())
                    } else {
                        self.candidate.state.geometry_from_current_statistics_with_context(self.candidate.cache.snapshot(), &mut self.candidate.plans)
                            .map(|()| { self.phase = Phase::GeometryCleanup; }).map_err(Issue::Geometry)
                    }
                }
            },
            Phase::ProbeBody => {
                let ready = self.candidate.current_shape().map_err(Issue::Probe)
                    .and_then(|_| provider::check_current_thread().map_err(Issue::Provider));
                match ready {
                    Err(issue) => Err(issue),
                    Ok(_) => {
                        let mut request = self.candidate.state.literal_probe_request(AfterProbe::ReloadRemaining);
                        if request.roots.is_empty() { request.kind = RequestKind::EmptyGuard; }
                        self.probe_request = Some(request);
                        self.candidate.probe_commit_open = true;
                        let result = self.services.checked_probe_body(&mut self.receiver, &mut self.candidate,
                            self.probe_request.as_ref().unwrap(), &mut self.staged_source);
                        self.candidate.probe_commit_open = false;
                        match result {
                            Err(error) => Err(Issue::RequiredService(error)),
                            Ok(()) if self.candidate.probe_committed => { self.phase = Phase::ProbeCleanup; Ok(()) },
                            Ok(()) => Err(Issue::Probe(Error::GeometryNotReady)),
                        }
                    }
                }
            }
            Phase::StatisticsBeforeGeometryCleanup | Phase::GeometryCleanup | Phase::ProbeCleanup | Phase::StatisticsCleanup
                | Phase::HandlerCleanup | Phase::RemainingDispatch => {
                let (result, next) = match self.phase {
                    Phase::StatisticsBeforeGeometryCleanup => (self.services.statistics_before_geometry_cleanup(&mut self.receiver, &mut self.candidate, &mut self.staged_source), Phase::GeometryEnter),
                    Phase::GeometryCleanup => (self.services.geometry_cleanup(&mut self.receiver, &mut self.candidate, &mut self.staged_source), Phase::GeometryLeave),
                    Phase::ProbeCleanup => (self.services.probe_cleanup(&mut self.receiver, &mut self.candidate, &mut self.staged_source), Phase::ProbeLeave),
                    Phase::StatisticsCleanup => (self.services.statistics_cleanup(&mut self.receiver, &mut self.candidate, &mut self.staged_source), Phase::Reload),
                    Phase::HandlerCleanup => (self.services.handler_cleanup(&mut self.receiver, &mut self.candidate, &mut self.staged_source), Phase::RemainingDispatch),
                    Phase::RemainingDispatch => (self.services.remaining_dispatch(&mut self.receiver, &mut self.candidate, &mut self.staged_source), Phase::Returned),
                    _ => unreachable!(),
                };
                result.map(|()| { self.phase = next; }).map_err(Issue::RequiredService)
            }
            Phase::Reload => {

                self.candidate.state.remaining = self.candidate.reload;
                self.phase = Phase::HandlerCleanup; Ok(())
            }
            Phase::AwaitingContext | Phase::Returned => unreachable!(),
        };
        match result {
            Ok(()) => Ok(self),
            Err(issue) => { self.halted = true; Err(Stopped { pending: self, issue }) }
        }
    }

    pub fn into_returned(self) -> Result<(C, AppendState, Option<T>, S), Self> {
        if !self.halted && self.phase == Phase::Returned {
            Ok((self.receiver, self.candidate, self.staged_source, self.services))
        } else { Err(self) }
    }
    pub fn into_owned_parts(self) -> OwnedParts<C, T, S> {
        OwnedParts { receiver: self.receiver, candidate: self.candidate, staged_source: self.staged_source,
            services: self.services, phase: self.phase, halted: self.halted, resume: self.resume,
            request: self.request, provider_attempted: self.provider_attempted,
            probe_request: self.probe_request }
    }
}
