use super::{CacheBinding, ContextController, ContextRequest, Error as FullError, GeometryContext, GeometryPlans, Phase, Source};
use super::{arithmetic::cache_blocking::Product, provider};
use super::super::super::partial as source_partial;

#[derive(Clone, Copy)]
pub enum Route { FirstTrajectory, Promotion(source_partial::GeometryHook) }
#[derive(Clone, Debug)]
pub enum Error {
    WrongPhase, ProviderAlreadyAttempted, ExistingSmallDomainOnly, ContextDomain, Partial(source_partial::Error),
    FirstTrajectory(source_partial::FirstTrajectoryError), Provider(provider::Error), Full(FullError),
}
pub struct RejectedContext { pub context: GeometryContext, pub error: Error }
pub struct CompletionFailure { pub pending: PreparedPartialContext, pub error: Error }
pub struct PromotedContext { pub controller: ContextController, pub binding: source_partial::DispatchBinding }

#[must_use = "Keep accepted Source/document and old partial state while context/probe is pending."]
pub struct PreparedPartialContext {
    before: source_partial::PartialController,
    source: Option<Source>, route: Route, cache: CacheBinding,
    request: Option<ContextRequest>, phase: Phase, issue: Option<Error>,
    prefix_started: bool,
    provider_attempted: bool,
    contextual: bool,
    plans: GeometryPlans,
    promotion: Option<source_partial::PendingPromotion>,

    advanced_full: Option<super::OwnedRefusal>,
}
impl PreparedPartialContext {

    #[allow(dead_code)]
    pub fn prepare(before: source_partial::PartialController, source: Source, route: Route, cache: CacheBinding) -> Self {
        Self::prepare_using(before, source, route, cache, false)
    }

    pub fn prepare_contextual(before: source_partial::PartialController, source: Source, route: Route, cache: CacheBinding) -> Self {
        Self::prepare_using(before, source, route, cache, true)
    }
    fn prepare_using(before: source_partial::PartialController, source: Source, route: Route, cache: CacheBinding, contextual: bool) -> Self {
        let mut value = Self { before, source: Some(source), route, cache, request: None,
            phase: Phase::Prefix, issue: None, prefix_started: false, provider_attempted: false,
            contextual, plans: GeometryPlans::default(),
            promotion: None, advanced_full: None };
        let total = match admission(&value.before, value.source.as_ref().unwrap(), value.route, contextual) {
            Ok(total) => total,
            Err(error) => {
                value.phase = Phase::GeometryFailed; value.issue = Some(error); return value;
            }
        };
        if total >= 12 && value.cache.snapshot().is_none() {
            value.request = Some(ContextRequest { product: Product::Global4d, depth: total, owner: None });
            value.phase = Phase::AwaitingContext;
        } else { value.start_prefix_once(); }
        value
    }
    pub fn before(&self) -> &source_partial::PartialController { &self.before }
    pub fn accepted_source(&self) -> Option<&Source> { self.source.as_ref() }
    pub fn pending_promotion(&self) -> Option<&source_partial::PendingPromotion> { self.promotion.as_ref() }
    pub fn advanced_full(&self) -> Option<&super::OwnedRefusal> { self.advanced_full.as_ref() }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn issue(&self) -> Option<&Error> { self.issue.as_ref() }
    pub fn cache_binding(&self) -> &CacheBinding { &self.cache }
    pub fn plans(&self) -> &GeometryPlans { &self.plans }
    pub fn context_request(&self) -> Option<ContextRequest> { self.request }
    pub fn prefix_started(&self) -> bool { self.prefix_started }
    pub fn probe_request(&self) -> Option<source_partial::ProbeRequest> {
        if self.phase != Phase::AwaitingProbe { return None; }
        self.promotion.as_ref()?.request()
    }
    pub fn complete_context(&mut self, context: GeometryContext) -> Result<(), RejectedContext> {
        if self.phase != Phase::AwaitingContext || self.prefix_started || self.request.is_none()
            || self.cache.snapshot().is_some() { return Err(RejectedContext { context, error: Error::WrongPhase }); }
        if let Err(error) = provider::check_current_thread() {
            return Err(RejectedContext { context, error: Error::Provider(error) });
        }
        self.cache = CacheBinding::Bound(context); self.request = None; self.issue = None;
        self.start_prefix_once(); Ok(())
    }
    pub fn resolve_context_from_process(&mut self, separate_operations: bool) -> Result<(), Error> {
        if self.phase != Phase::AwaitingContext || self.prefix_started { return Err(Error::WrongPhase); }
        if self.provider_attempted { return Err(Error::ProviderAlreadyAttempted); }
        self.provider_attempted = true;
        let context = match provider::process_context(separate_operations) {
            Ok(context) => context,
            Err(error) => { let error = Error::Provider(error); self.issue = Some(error.clone()); return Err(error); }
        };
        self.complete_context(context).map_err(|rejected| rejected.error)
    }
    fn start_prefix_once(&mut self) {
        if self.prefix_started || self.promotion.is_some() || self.advanced_full.is_some() { return; }
        if let Err(error) = provider::check_current_thread() {
            self.phase = Phase::GeometryFailed; self.issue = Some(Error::Provider(error)); return;
        }
        self.prefix_started = true;
        let source = self.source.take().unwrap();
        let prepared = match self.route {
            Route::FirstTrajectory => {
                let result = if self.contextual {
                    self.before.prepare_first_trajectory_with_context(source, self.cache.snapshot(), &mut self.plans)
                } else { self.before.prepare_first_trajectory(source) };
                match result {
                    Ok(pending) => Ok(pending),
                    Err(failure) => { self.source = Some(failure.source); Err(Error::FirstTrajectory(failure.error)) }
                }
            }
            Route::Promotion(hook) => {
                let result = if self.contextual {
                    self.before.prepare_promotion_with_context(source, hook, self.cache.snapshot(), &mut self.plans)
                } else { self.before.prepare_promotion(source, hook) };
                match result {
                    Ok(pending) => Ok(pending),
                    Err(failure) => { self.source = Some(failure.source); Err(Error::Partial(failure.error)) }
                }
            }
        };
        match prepared {
            Err(error) => { self.phase = Phase::GeometryFailed; self.issue = Some(error); }
            Ok(pending) => {
                self.phase = if pending.phase() == source_partial::Phase::AwaitingProbe { Phase::AwaitingProbe } else { Phase::GeometryFailed };
                self.issue = pending.issue().cloned().map(Error::Partial);
                self.promotion = Some(pending);
            }
        }
    }
    pub fn complete_probe(mut self, checked: source_partial::CheckedProbeCompletion) -> Result<PromotedContext, CompletionFailure> {
        if self.phase != Phase::AwaitingProbe { return Err(CompletionFailure { pending: self, error: Error::WrongPhase }); }
        if let Err(error) = provider::check_current_thread() {
            let error = Error::Provider(error); self.phase = Phase::ProbeFailed; self.issue = Some(error.clone());
            return Err(CompletionFailure { pending: self, error });
        }
        let pending = self.promotion.take().unwrap();
        let ready = match pending.complete_probe(checked) {
            Ok(ready) => ready,
            Err(failure) => {
                let error = Error::Partial(failure.error); self.promotion = Some(failure.pending);
                self.phase = Phase::ProbeFailed; self.issue = Some(error.clone());
                return Err(CompletionFailure { pending: self, error });
            }
        };

        let (controller, binding) = ready.adopt().into_parts();
        match ContextController::rebind_existing_with_plans(controller, self.cache.clone(), binding.source_reload, self.plans.clone()) {
            Ok(controller) => Ok(PromotedContext { controller, binding }),
            Err(failure) => {
                let error = Error::Full(failure.error.clone()); self.advanced_full = Some(failure);
                self.phase = Phase::GeometryFailed; self.issue = Some(error.clone());
                Err(CompletionFailure { pending: self, error })
            }
        }
    }
}

pub fn contextual_admission(before: &source_partial::PartialController, source: &Source, route: Route) -> Result<usize, Error> {
    admission(before, source, route, true)
}

fn admission(before: &source_partial::PartialController, source: &Source, route: Route, contextual: bool) -> Result<usize, Error> {
        match route {
            Route::FirstTrajectory => {
                if before.point().is_some() || !before.sources().is_empty() {
                    return Err(Error::FirstTrajectory(source_partial::FirstTrajectoryError::NeedEmptyStartup));
                }
                let Source::Trajectory { reduced, .. } = source else {
                    return Err(Error::FirstTrajectory(source_partial::FirstTrajectoryError::NeedTrajectory));
                };
                if !(2..=if contextual { 2049 } else { 46 }).contains(&reduced.len()) {
                    return Err(Error::FirstTrajectory(source_partial::FirstTrajectoryError::ChildCountOutsideDomain));
                }
                if reduced.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(Error::FirstTrajectory(source_partial::FirstTrajectoryError::NonfiniteInput));
                }
            }
            Route::Promotion(hook) => {
                if before.point().is_none() || !matches!(before.sources(), [Source::Point { .. }]) {
                    return Err(Error::Partial(source_partial::Error::NeedOneStandalone));
                }
                before.checked_view().map_err(Error::Partial)?;
                match source {
                    Source::Point { reduced, .. } => {
                        if hook != source_partial::GeometryHook::ExactTwoStandaloneFirstProductInstalled {
                            return Err(Error::Partial(source_partial::Error::UnsupportedSecondSource));
                        }
                        if reduced.iter().any(|v| !v.is_finite()) { return Err(Error::Partial(source_partial::Error::NonfiniteInput)); }
                    }
                    Source::Trajectory { reduced, .. } => {
                        if !(2..=if contextual { 2048 } else { 46 }).contains(&reduced.len()) { return Err(Error::Partial(source_partial::Error::ChildCountOutsideDomain)); }
                        if reduced.iter().flatten().any(|v| !v.is_finite()) { return Err(Error::Partial(source_partial::Error::NonfiniteInput)); }
                    }
                }
            }
        }
        let total = before.sources().iter().chain(std::iter::once(source)).try_fold(0usize, |n,s| {
            n.checked_add(match s { Source::Point { .. } => 1, Source::Trajectory { reduced, .. } => reduced.len() })
        });
        match total {
            Some(n) if (2..=if contextual { 2049 } else { 47 }).contains(&n) => Ok(n),
            _ => Err(if contextual { Error::ContextDomain } else { Error::ExistingSmallDomainOnly }),
        }
}
