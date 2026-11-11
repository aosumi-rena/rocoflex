use super::{geometry, Controller, FullView, Source};
#[path = "desktop_layout_context_math.rs"]
pub mod arithmetic;
#[path = "desktop_layout_context_provider.rs"]
pub mod provider;
#[path = "desktop_layout_context_partial.rs"]
pub mod partial_gate;
#[path = "desktop_layout_context_display.rs"]
pub mod display_gate;
pub use arithmetic::GeometryContext;
use arithmetic::cache_blocking::{Blocking, Product};

#[derive(Clone, Debug)]
pub enum CacheBinding {

    Unbound,
    Bound(GeometryContext),
}
impl CacheBinding {
    pub fn snapshot(&self) -> Option<&GeometryContext> {
        match self { Self::Unbound => None, Self::Bound(context) => Some(context) }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextRequest { pub product: Product, pub depth: usize, pub owner: Option<usize> }
impl ContextRequest {
    pub fn consumes_cache_bytes(self) -> bool { self.depth >= 48 }
}
#[derive(Clone, Debug)]
pub struct CountPlan { pub request: ContextRequest, pub blocking: Option<Blocking> }
#[derive(Clone, Debug, Default)]
pub struct GeometryPlans {

    pub cached_statistics: Option<CountPlan>,
    pub statistics_attempt: Option<CountPlan>,
    pub parent_attempts: Vec<CountPlan>,
}
#[derive(Clone, Debug)]
pub enum Error {
    Shape(String), Domain, Input, RequiredPriorContext, WrongPhase, ProviderAlreadyAttempted, IdentityChanged,
    Arithmetic(arithmetic::context::Error), Provider(provider::Error), Geometry(String), Probe(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Prefix, AwaitingContext, AwaitingProbe, Ready, GeometryFailed, ProbeFailed }
#[derive(Clone, Copy)]
enum Work { Statistics, Geometry, RestoreGeometry, CallbackGeometry, CallbackGate, Probe, Done }
#[derive(Clone, Copy)]
enum Suffix { Keep, Reload, Decrement }

#[derive(Clone)]
pub struct ContextController {
    controller: Controller,
    cache: CacheBinding,

    reload: i32,
    plans: GeometryPlans,
}
pub struct OwnedRefusal {
    pub controller: Controller, pub cache: CacheBinding, pub reload: i32,
    pub plans: GeometryPlans, pub error: Error,
}
pub struct SourceRefusal { pub source: Source, pub error: Error }
pub struct RejectedContext { pub context: GeometryContext, pub error: Error }

#[must_use = "Retain the candidate and accepted document through context/probe failure."]
pub struct PreparedContextController {
    candidate: ContextController, phase: Phase, work: Work, suffix: Suffix,
    changed: bool, request: Option<ContextRequest>, issue: Option<Error>, provider_attempted: bool,
}

fn count(source: &Source) -> usize {
    match source { Source::Point { .. } => 1, Source::Trajectory { reduced, .. } => reduced.len() }
}
const MAX_FULL_POINTS: usize = 2049;

fn shape_domain(sources: &[Source]) -> Result<usize, super::InspectionDomain> {
    let total = sources.iter().try_fold(0usize, |n,s| n.checked_add(count(s)))
        .ok_or(super::InspectionDomain::ContextResourceLimit)?;
    if total < 2 { return Err(super::InspectionDomain::TooFewPoints); }
    if total > MAX_FULL_POINTS || sources.len() > u32::MAX as usize {
        return Err(super::InspectionDomain::ContextResourceLimit);
    }
    if sources.iter().any(|s| matches!(s, Source::Trajectory { reduced, .. }
        if !(2..=MAX_FULL_POINTS).contains(&reduced.len()))) {
        return Err(super::InspectionDomain::TrajectoryLength);
    }
    Ok(total)
}

pub fn inspection_domain(sources: &[Source]) -> Option<super::InspectionDomain> {
    shape_domain(sources).err()
}

fn domain(sources: &[Source], basis: [f32; 8]) -> Result<usize, Error> {
    let total = shape_domain(sources).map_err(|_| Error::Domain)?;
    if sources.iter().any(|s| match s {
        Source::Point { reduced, .. } => reduced.iter().any(|v| !v.is_finite()),
        Source::Trajectory { reduced, .. } => reduced.iter().flatten().any(|v| !v.is_finite()),
    }) || basis.iter().any(|v| !v.is_finite())
        || basis[..4].iter().all(|v| *v == 0.0) || basis[4..].iter().all(|v| *v == 0.0) { return Err(Error::Input); }
    Ok(total)
}

fn produce_statistics(
    rows: &[[f32; 4]], cache: &CacheBinding, plans: &mut GeometryPlans,
) -> Result<geometry::Statistics, Error> {
    let request = ContextRequest { product: Product::Global4d, depth: rows.len(), owner: None };
    plans.statistics_attempt = Some(CountPlan { request, blocking: None });
    if rows.len() >= 12 && cache.snapshot().is_none() { return Err(Error::RequiredPriorContext); }
    provider::check_current_thread().map_err(Error::Provider)?;
    let (statistics, blocking) = if rows.len() >= 12 {
        let (statistics, blocking) = arithmetic::statistics_with_context(rows, cache.snapshot().unwrap())
            .map_err(Error::Arithmetic)?;
        (statistics, Some(blocking))
    } else { (geometry::statistics(rows), None) };
    let plan = CountPlan { request, blocking };
    plans.statistics_attempt = Some(plan.clone());
    plans.cached_statistics = Some(plan);
    Ok(statistics)
}

impl ContextController {

    pub fn rebind_existing(controller: Controller, cache: CacheBinding, reload: i32) -> Result<Self, OwnedRefusal> {
        Self::rebind_existing_with_plans(controller, cache, reload, GeometryPlans::default())
    }

    pub fn rebind_existing_with_plans(
        controller: Controller, cache: CacheBinding, reload: i32, plans: GeometryPlans,
    ) -> Result<Self, OwnedRefusal> {
        let checked = controller.check_shape().map_err(Error::Shape)
            .and_then(|_| domain(&controller.sources, controller.basis));
        let error = match checked {
            Err(error) => Some(error),
            Ok(n) if n >= 12 && cache.snapshot().is_none() => Some(Error::RequiredPriorContext),
            Ok(_) => None,
        };
        if let Some(error) = error { return Err(OwnedRefusal { controller, cache, reload, plans, error }); }
        Ok(Self { controller, cache, reload, plans })
    }

    pub fn prepare_rebuild_owned(controller: Controller, cache: CacheBinding, reload: i32)
        -> Result<PreparedContextController, OwnedRefusal>
    {
        let error = controller.check_shape().map_err(Error::Shape)
            .and_then(|_| domain(&controller.sources, controller.basis)).err();
        if let Some(error) = error {
            return Err(OwnedRefusal { controller, cache, reload, plans: GeometryPlans::default(), error });
        }
        Ok(PreparedContextController::begin(Self { controller, cache, reload, plans: GeometryPlans::default() },
            Work::Statistics, Suffix::Keep, true))
    }
    pub fn controller(&self) -> &Controller { &self.controller }
    pub fn sources(&self) -> &[Source] { &self.controller.sources }
    pub fn cache_binding(&self) -> &CacheBinding { &self.cache }
    pub fn reload(&self) -> i32 { self.reload }
    pub fn plans(&self) -> &GeometryPlans { &self.plans }
    pub fn active(&self) -> bool { self.controller.active() }
    pub fn checked_view(&self) -> Result<FullView<'_>, String> { self.controller.checked_view() }
    pub fn working_uv_snapshot(&self) -> [f32; 8] { self.controller.basis }
    pub fn source_transformation_snapshot(&self) -> [f64; 8] { self.controller.basis.map(f64::from) }
    pub fn into_parts(self) -> (Controller, CacheBinding, i32, GeometryPlans) {
        (self.controller, self.cache, self.reload, self.plans)
    }
    pub fn fork_with_matching_metadata(&self, sources: &[Source]) -> Result<Self, Error> {
        self.controller.check_shape().map_err(Error::Shape)?;
        if !super::same_sources(self.sources(), sources) { return Err(Error::IdentityChanged); }
        let mut next = self.clone();
        for (old, new) in next.controller.sources.iter_mut().zip(sources) {
            match (old, new) {
                (Source::Point { enabled, .. }, Source::Point { enabled: value, .. })
                | (Source::Trajectory { enabled, .. }, Source::Trajectory { enabled: value, .. }) => *enabled = *value,
                _ => return Err(Error::IdentityChanged),
            }
        }
        Ok(next)
    }
    pub fn prepare_append(&self, source: Source) -> Result<PreparedContextController, SourceRefusal> {
        if let Err(error) = self.controller.check_shape() { return Err(SourceRefusal { source, error: Error::Shape(error) }); }
        let mut next = self.clone();
        next.controller.sources.push(source);
        if let Err(error) = domain(&next.controller.sources, next.controller.basis) {
            let source = next.controller.sources.pop().unwrap(); return Err(SourceRefusal { source, error });
        }
        let source = next.controller.sources.last().unwrap();
        next.controller.roots.push(Controller::new_root(source));
        next.controller.scalars.push(vec![[0.0;4]; count(source)]);
        Ok(PreparedContextController::begin(next, Work::Statistics, Suffix::Reload, true))
    }
    pub fn prepare_remove(&self, index: usize) -> Result<PreparedContextController, Error> {
        self.controller.check_shape().map_err(Error::Shape)?;
        if index >= self.sources().len() { return Err(Error::Domain); }
        let mut next = self.clone();
        next.controller.sources.remove(index); next.controller.roots.remove(index); next.controller.scalars.remove(index);
        domain(&next.controller.sources, next.controller.basis)?;
        Ok(PreparedContextController::begin(next, Work::Statistics, Suffix::Keep, true))
    }
    pub fn prepare_rebuild(&self) -> PreparedContextController {
        PreparedContextController::begin(self.clone(), Work::Statistics, Suffix::Keep, true)
    }
    pub fn prepare_callback(&self) -> PreparedContextController {
        let moving = self.controller.motion > f64::from_bits(0x3e7ad7f2a0000000);
        PreparedContextController::begin(self.clone(), if moving { Work::CallbackGeometry } else { Work::CallbackGate },
            Suffix::Decrement, moving)
    }

    pub fn probe_changed(&self, probe: [f32;2], radius: f64) -> Result<Self, Error> {
        if probe.iter().any(|v| !v.is_finite()) || !radius.is_finite() || radius <= 0.0 { return Err(Error::Input); }
        let mut next = self.clone(); next.controller.probe = probe; next.controller.radius = radius;
        next.controller.remaining = next.reload; Ok(next)
    }
}

impl PreparedContextController {
    fn begin(candidate: ContextController, work: Work, suffix: Suffix, changed: bool) -> Self {
        let mut value = Self { candidate, phase: Phase::Prefix, work, suffix, changed, request: None, issue: None,
            provider_attempted: false };
        value.advance_prefix(); value
    }
    pub fn candidate(&self) -> &ContextController { &self.candidate }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn issue(&self) -> Option<&Error> { self.issue.as_ref() }
    pub fn geometry_changed(&self) -> bool { self.changed }
    pub fn context_request(&self) -> Option<ContextRequest> { self.request }

    pub fn complete_context(&mut self, context: GeometryContext) -> Result<(), RejectedContext> {
        if self.phase != Phase::AwaitingContext || self.request.is_none() || self.candidate.cache.snapshot().is_some() {
            return Err(RejectedContext { context, error: Error::WrongPhase });
        }
        if let Err(error) = provider::check_current_thread() {
            return Err(RejectedContext { context, error: Error::Provider(error) });
        }
        self.candidate.cache = CacheBinding::Bound(context); self.request = None; self.phase = Phase::Prefix;
        self.issue = None;
        self.advance_prefix(); Ok(())
    }

    pub fn resolve_context_from_process(&mut self, separate_operations: bool) -> Result<(), Error> {
        if self.phase != Phase::AwaitingContext { return Err(Error::WrongPhase); }
        if self.provider_attempted { return Err(Error::ProviderAlreadyAttempted); }
        self.provider_attempted = true;
        let context = match provider::process_context(separate_operations) {
            Ok(context) => context,
            Err(error) => { let error = Error::Provider(error); self.issue = Some(error.clone()); return Err(error); }
        };
        self.complete_context(context).map_err(|rejected| rejected.error)
    }
    fn require_context(&mut self, request: ContextRequest) -> bool {
        if self.candidate.cache.snapshot().is_some() { true } else {
            self.request = Some(request); self.phase = Phase::AwaitingContext; false
        }
    }
    fn fail(&mut self, error: Error) { self.issue = Some(error); self.phase = Phase::GeometryFailed; }
    fn advance_prefix(&mut self) {
        if self.phase != Phase::Prefix { return; }
        if let Err(error) = self.candidate.controller.check_shape().map_err(Error::Shape)
            .and_then(|_| domain(self.candidate.sources(), self.candidate.controller.basis)) { self.fail(error); return; }
        loop { match self.work {
            Work::Statistics => {
                let rows = geometry::flatten(self.candidate.sources());
                let request = ContextRequest { product: Product::Global4d, depth: rows.len(), owner: None };
                self.candidate.plans.statistics_attempt = Some(CountPlan { request, blocking: None });

                if rows.len() >= 12 && !self.require_context(request) { return; }
                match produce_statistics(&rows, &self.candidate.cache, &mut self.candidate.plans) {
                    Ok(stats) => self.candidate.controller.statistics = stats,
                    Err(error) => { self.fail(error); return; }
                }
                self.work = Work::Geometry;
            }
            Work::Geometry | Work::RestoreGeometry | Work::CallbackGeometry => {
                let callback = matches!(self.work, Work::CallbackGeometry);

                let packed_parent = self.candidate.sources().iter().enumerate().find_map(|(owner,s)| match s {
                    Source::Trajectory { reduced, .. } if reduced.len() >= 16 => Some(ContextRequest {
                        product: Product::Parent2d, depth: reduced.len(), owner: Some(owner) }), _ => None,
                });
                if let Some(request) = packed_parent { if !self.require_context(request) { return; } }
                if let Err(error) = provider::check_current_thread() { self.fail(Error::Provider(error)); return; }
                self.candidate.plans.parent_attempts.clear();
                let owners: Vec<_> = self.candidate.sources().iter().enumerate().filter_map(|(owner,s)|
                    matches!(s,Source::Trajectory { .. }).then_some(owner)).collect();
                let mut parent_index = 0;
                let cache = &self.candidate.cache;
                let plans = &mut self.candidate.plans.parent_attempts;
                let build = |sources: &[Source], stats: &geometry::Statistics, basis| {
                    let bases = geometry::base_geometry_with_parent_summary(sources, stats, basis, |children| {
                        let request = ContextRequest { product: Product::Parent2d, depth: children.len(), owner: Some(owners[parent_index]) };
                        parent_index += 1; plans.push(CountPlan { request, blocking: None });
                        provider::check_current_thread().map_err(|error| format!("Parent FP controls: {error:?}"))?;
                        if children.len() < 16 { return Ok(geometry::summary(children)); }
                        match arithmetic::parent_summary_with_context(children, cache.snapshot().unwrap()) {
                            Ok((summary, blocking)) => { plans.last_mut().unwrap().blocking = Some(blocking); Ok(summary) }
                            Err(error) => Err(format!("Contextual parent arithmetic: {error:?}")),
                        }
                    })?;
                    provider::check_current_thread().map_err(|error| format!("Force FP controls: {error:?}"))?;
                    Ok(bases)
                };
                let result = if matches!(self.work, Work::RestoreGeometry) {
                    self.candidate.controller.restore_with_geometry(build)
                } else {
                    self.candidate.controller.refresh_with_geometry(build)
                };
                if let Err(error) = result { self.fail(Error::Geometry(error)); return; }
                self.work = if callback { self.candidate.controller.remaining = self.candidate.reload; Work::CallbackGate } else { Work::Probe };
            }
            Work::CallbackGate => {
                self.work = if self.candidate.controller.remaining > 0 { Work::Probe } else { Work::Done };
            }
            Work::Probe => { self.phase = Phase::AwaitingProbe; return; }
            Work::Done => { self.phase = Phase::Ready; return; }
        } }
    }
    pub fn finish_probe(&mut self) -> Result<(), Error> {
        match self.phase {
            Phase::Ready => return Ok(()),
            Phase::GeometryFailed | Phase::ProbeFailed => return Err(self.issue.clone().unwrap()),
            Phase::AwaitingProbe => {}, _ => return Err(Error::WrongPhase),
        }
        if let Err(error) = provider::check_current_thread() {
            let error = Error::Provider(error); self.issue = Some(error.clone()); self.phase = Phase::ProbeFailed; return Err(error);
        }
        if let Err(error) = self.candidate.controller.refresh_probe() {
            let error = Error::Probe(error); self.issue = Some(error.clone()); self.phase = Phase::ProbeFailed; return Err(error);
        }
        match self.suffix { Suffix::Keep => {}, Suffix::Reload => self.candidate.controller.remaining = self.candidate.reload,
            Suffix::Decrement => self.candidate.controller.remaining -= 1 }
        self.work = Work::Done; self.phase = Phase::Ready; Ok(())
    }
    pub fn into_ready(self) -> Result<ContextController, Self> {
        if self.phase == Phase::Ready { Ok(self.candidate) } else { Err(self) }
    }
}
