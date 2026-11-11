use super::{CacheBinding, ContextController, ContextRequest, Error, GeometryContext, GeometryPlans, Phase};
use super::{arithmetic::cache_blocking::Product, provider};
use super::super::{geometry, Controller, Source};

#[derive(Clone, Copy)]
pub enum Origin { ExistingInspectionDisplay, HostWorkingDocument }
pub struct Input {
    pub origin: Origin, pub sources: Vec<Source>, pub incoming: [f32;8],
    pub probe: [f32;2], pub radius: f64, pub reload: i32, pub cache: CacheBinding,
}

fn legacy_display_admission(sources: &[Source], basis: [f32;8]) -> Result<usize, Error> {
    let total = sources.iter().try_fold(0usize, |n, source| n.checked_add(match source {
        Source::Point { .. } => 1, Source::Trajectory { reduced, .. } => reduced.len(),
    })).ok_or(Error::Domain)?;
    if !(2..48).contains(&total) || sources.iter().any(|source| matches!(source,
        Source::Trajectory { reduced, .. } if !(2..48).contains(&reduced.len()))) { return Err(Error::Domain); }
    super::domain(sources, basis)
}
#[must_use = "Own this stage with the accepted document before resolving context."]
pub struct PreparedDisplayContext {
    input: Input, phase: Phase, request: Option<ContextRequest>, issue: Option<Error>,
    prefix_started: bool, provider_attempted: bool,
    pending: Option<super::PreparedContextController>, ready: Option<ContextController>,
    plans: GeometryPlans,
}
impl PreparedDisplayContext {
    pub fn prepare(input: Input) -> Self {
        let admitted = legacy_display_admission(&input.sources, input.incoming);
        let mut value = Self { input, phase: Phase::Prefix, request: None, issue: None,
            prefix_started: false, provider_attempted: false, pending: None, ready: None, plans: GeometryPlans::default() };
        let total = match admitted {
            Ok(total) => total,
            Err(error) => { value.phase = Phase::GeometryFailed; value.issue = Some(error); return value; }
        };
        if total >= 12 && value.input.cache.snapshot().is_none() {
            value.request = Some(ContextRequest { product: Product::Global4d, depth: total, owner: None });
            value.phase = Phase::AwaitingContext;
        } else { value.start_prefix_once(); }
        value
    }
    pub fn sources(&self) -> &[Source] { &self.input.sources }
    pub fn cache_binding(&self) -> &CacheBinding { &self.input.cache }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn issue(&self) -> Option<&Error> { self.issue.as_ref() }
    pub fn geometry_changed(&self) -> bool { self.prefix_started }
    pub fn context_request(&self) -> Option<ContextRequest> { self.request }
    pub fn complete_context(&mut self, context: GeometryContext) -> Result<(), super::RejectedContext> {
        if self.phase != Phase::AwaitingContext || self.prefix_started || self.request.is_none()
            || self.input.cache.snapshot().is_some() {
            return Err(super::RejectedContext { context, error: Error::WrongPhase });
        }
        if let Err(error) = provider::check_current_thread() {
            return Err(super::RejectedContext { context, error: Error::Provider(error) });
        }
        self.input.cache = CacheBinding::Bound(context); self.request = None; self.issue = None;
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
        if self.prefix_started { return; }
        if let Err(error) = provider::check_current_thread() {
            self.phase = Phase::GeometryFailed; self.issue = Some(Error::Provider(error)); return;
        }
        self.prefix_started = true;
        let rows = geometry::flatten(&self.input.sources);
        let statistics = match super::produce_statistics(&rows, &self.input.cache, &mut self.plans) {
            Ok(statistics) => statistics,
            Err(error) => { self.phase = Phase::GeometryFailed; self.issue = Some(error); return; }
        };

        let controller = Controller {
            roots: self.input.sources.iter().map(Controller::new_root).collect(),
            scalars: self.input.sources.iter().map(|source| vec![[0.0;4]; super::count(source)]).collect(),
            sources: self.input.sources.clone(), statistics, basis: self.input.incoming,
            motion: 0.0, remaining: 0, probe: self.input.probe, radius: self.input.radius,
            completed_mix: None,
            completed_probe_geometry: None,
        };
        let pending = super::PreparedContextController::begin(ContextController {
            controller, cache: self.input.cache.clone(), reload: self.input.reload,
            plans: std::mem::take(&mut self.plans),
        }, match self.input.origin {
            Origin::ExistingInspectionDisplay => super::Work::Geometry,
            Origin::HostWorkingDocument => super::Work::RestoreGeometry,
        }, super::Suffix::Keep, true);
        self.phase = pending.phase();
        self.issue = pending.issue().cloned();
        self.pending = Some(pending);
    }
    pub fn finish_probe(&mut self) -> Result<(), Error> {
        match self.phase {
            Phase::Ready => return Ok(()),
            Phase::GeometryFailed | Phase::ProbeFailed => return Err(self.issue.clone().unwrap()),
            Phase::AwaitingProbe => {}, _ => return Err(Error::WrongPhase),
        }
        if let Err(error) = provider::check_current_thread() {
            let error = Error::Provider(error); self.phase = Phase::ProbeFailed; self.issue = Some(error.clone()); return Err(error);
        }
        if let Err(error) = self.pending.as_mut().unwrap().finish_probe() {
            self.phase = self.pending.as_ref().unwrap().phase(); self.issue = Some(error.clone()); return Err(error);
        }
        let pending = self.pending.take().unwrap();
        let controller = match pending.into_ready() {
            Ok(controller) => controller,
            Err(pending) => { self.pending = Some(pending); self.phase = Phase::ProbeFailed;
                self.issue = Some(Error::WrongPhase); return Err(Error::WrongPhase); }
        };
        self.ready = Some(controller); self.phase = Phase::Ready; Ok(())
    }
    pub fn into_ready(mut self) -> Result<ContextController, Self> {
        if self.phase == Phase::Ready { if let Some(controller) = self.ready.take() { return Ok(controller); } }
        Err(self)
    }
}
