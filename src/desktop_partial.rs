use super::layout;
use layout::{Source, force, geometry};

#[path = "desktop_partial_loaded.rs"]
#[allow(dead_code)]
pub mod loaded;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SourceField<T> {
    Unwritten,
    Defined(T),
}
impl<T> SourceField<T> {
    pub fn defined(&self) -> Option<&T> {
        match self {
            Self::Unwritten => None,
            Self::Defined(v) => Some(v),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Projection {
    pub negative_mean: SourceField<[f32; 4]>,
    pub scale: SourceField<[f32; 4]>,
    pub projected_basis: SourceField<[f32; 8]>,
    pub covariance: [f32; 16],
    pub working_uv: [f32; 8],
}
impl Projection {
    fn constructor() -> Self {
        Self {
            negative_mean: SourceField::Unwritten,
            scale: SourceField::Unwritten,
            projected_basis: SourceField::Unwritten,
            covariance: identity4(),
            working_uv: [1.0; 8],
        }
    }
    fn small_statistics(&mut self, rows: &[[f32; 4]]) {
        let n = rows.len();
        self.negative_mean = SourceField::Defined(std::array::from_fn(|d| {
            let col: Vec<_> = rows.iter().map(|row| row[d]).collect();
            -geometry::aligned_sum(&col, d * n) / n as f32
        }));
        self.covariance = identity4();
        self.working_uv = [1.0; 8];
    }
}
#[derive(Clone, Debug)]
pub struct Point {
    pub reduced: [f32; 4],
    pub base: [f32; 2],
    pub offset: [f32; 2],
    pub force: SourceField<[f32; 2]>,
    pub scalars: [f32; 4],
}
impl Point {
    fn constructor(reduced: [f32; 4]) -> Self {
        Self {
            reduced,
            base: [0.0; 2],
            offset: [0.0; 2],
            force: SourceField::Unwritten,
            scalars: [0.0; 4],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupOrigin {
    ActualNoFileStartup,
}
#[derive(Clone, Copy, Debug)]
pub struct ProbeControls {
    pub xy: [f32; 2],
    pub radius: f64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidProbe,
    NeedFirstPoint,
    NeedOneStandalone,
    UnsupportedSecondSource,
    ChildCountOutsideDomain,
    NonfiniteInput,
    NonfiniteGeometry,
    GeometryShape,
    ProbeNotRequested,
    WrongCompletionKind,
    WrongScalarCount,
    NonfiniteProbe,
    ExternalProbe(String),
    ContextRequired,
    ExternalGeometry(String),
    GeometryNotReady,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    AwaitingProbe,
    ProbeFailed,
    GeometryFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterProbe {
    DecrementRemaining,
    ReloadRemaining,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestKind {
    CheckedLiteral,
    EmptyGuard,
}
#[derive(Clone, Debug)]
pub struct ProbeSample {
    pub xy: [f32; 2],
    pub old: [f32; 3],
}
#[derive(Clone, Debug)]
pub struct RequestedRoot {
    pub owner: u32,
    pub trajectory: bool,
    pub points: Vec<ProbeSample>,
}
#[derive(Clone, Debug)]
pub struct ProbeRequest {
    pub kind: RequestKind,
    pub roots: Vec<RequestedRoot>,
    pub controls: ProbeControls,
    pub after: AfterProbe,
}

pub enum CheckedProbeCompletion {
    Literal(Result<layout::probe_math::CompletedProbe, String>),

    EmptyGuard(Result<(), String>),
}
pub struct CompletionFailure<T> {
    pub pending: T,
    pub error: Error,
}
pub struct PrepareFailure {
    pub source: Source,
    pub error: Error,
}
#[derive(Clone, Debug)]
pub struct View {
    pub positions: Vec<Vec<[f32; 2]>>,
    pub scalars: Vec<Vec<[f32; 4]>>,
}

#[derive(Clone, Debug)]
pub struct PartialController {
    projection: Projection,
    point: Option<Point>,
    sources: Vec<Source>,
    motion: f64,
    remaining: i32,
    controls: ProbeControls,
    completed_probe_geometry: Option<std::sync::Arc<layout::background_geometry::CompletedGeometry>>,
}
pub struct PreparedPartial {
    candidate: PartialController,
    phase: Phase,
    after: Option<AfterProbe>,
    issue: Option<Error>,
}
impl PartialController {

    pub fn startup(_origin: StartupOrigin, controls: ProbeControls) -> Result<Self, Error> {
        valid_controls(controls)?;
        let mut state = Self {
            projection: Projection::constructor(),
            point: None,
            sources: vec![],
            motion: 0.0,
            remaining: 0,
            controls,
            completed_probe_geometry: None,
        };
        state.small_statistics();
        Ok(state)
    }

    fn small_statistics(&mut self) {
        let rows: Vec<_> = self.point.iter().map(|point| point.reduced).collect();
        self.projection.small_statistics(&rows);
    }
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }
    pub fn completed_probe_geometry(&self)
        -> Option<&std::sync::Arc<layout::background_geometry::CompletedGeometry>>
    {
        self.completed_probe_geometry.as_ref()
    }
    pub fn projection(&self) -> &Projection {
        &self.projection
    }
    pub fn point(&self) -> Option<&Point> {
        self.point.as_ref()
    }
    pub fn motion(&self) -> f64 {
        self.motion
    }
    pub fn remaining(&self) -> i32 {
        self.remaining
    }
    pub fn controls(&self) -> ProbeControls {
        self.controls
    }
    pub fn active(&self) -> bool {
        moving(self.motion) || self.remaining > 0
    }

    pub fn working_uv_snapshot(&self) -> [f32; 8] {
        self.projection.working_uv
    }
    pub fn source_transformation_snapshot(&self) -> [f64; 8] {
        self.projection.working_uv.map(f64::from)
    }
    pub fn checked_view(&self) -> Result<View, Error> {
        match (&self.point, self.sources.as_slice()) {
            (None, []) => Ok(View {
                positions: vec![],
                scalars: vec![],
            }),
            (Some(p), [Source::Point { reduced, .. }]) if bits4(*reduced) == bits4(p.reduced) => {
                let xy = getter(p, None);
                if xy.iter().chain(&p.scalars).any(|v| !v.is_finite()) {
                    return Err(Error::NonfiniteInput);
                }
                Ok(View {
                    positions: vec![vec![xy]],
                    scalars: vec![vec![p.scalars]],
                })
            }
            _ => Err(Error::GeometryShape),
        }
    }
    pub fn prepare_point(&self, source: Source) -> Result<PreparedPartial, PrepareFailure> {
        let Source::Point { reduced, .. } = &source else {
            return Err(PrepareFailure {
                source,
                error: Error::NeedFirstPoint,
            });
        };
        if self.point.is_some() || !self.sources.is_empty() {
            return Err(PrepareFailure {
                source,
                error: Error::NeedFirstPoint,
            });
        }
        if reduced.iter().any(|v| !v.is_finite()) {
            return Err(PrepareFailure {
                source,
                error: Error::NonfiniteInput,
            });
        }
        let mut candidate = self.clone();
        candidate.point = Some(Point::constructor(*reduced));
        candidate.sources.push(source);
        candidate.small_statistics();
        candidate.remaining = 10;
        Ok(PreparedPartial {
            candidate,
            phase: Phase::Ready,
            after: None,
            issue: None,
        })
    }

    pub fn prepare_probe_changed(&self, controls: ProbeControls) -> Result<PreparedPartial, Error> {
        valid_controls(controls)?;
        let mut candidate = self.clone();
        candidate.controls = controls;
        candidate.remaining = 10;
        Ok(PreparedPartial {
            candidate,
            phase: Phase::Ready,
            after: None,
            issue: None,
        })
    }
    pub fn prepare_callback(&self) -> PreparedPartial {
        let mut candidate = self.clone();

        if moving(candidate.motion) {
            candidate.remaining = 10;
        }
        let after = (candidate.remaining > 0).then_some(AfterProbe::DecrementRemaining);
        PreparedPartial {
            candidate,
            phase: if after.is_some() {
                Phase::AwaitingProbe
            } else {
                Phase::Ready
            },
            after,
            issue: None,
        }
    }

    pub fn prepare_promotion(
        &self,
        source: Source,
        hook: GeometryHook,
    ) -> Result<PendingPromotion, PrepareFailure> {
        self.prepare_promotion_using(source, hook, 46, |state| state.geometry_prefix())
    }

    pub fn prepare_promotion_with_context(
        &self, source: Source, hook: GeometryHook,
        context: Option<&layout::context::GeometryContext>,
        plans: &mut layout::context::GeometryPlans,
    ) -> Result<PendingPromotion, PrepareFailure> {
        self.prepare_promotion_using(source, hook, 2048, |state| state.geometry_prefix_with_context(context, plans))
    }

    fn prepare_promotion_using(
        &self, source: Source, hook: GeometryHook, maximum_children: usize,
        prefix: impl FnOnce(&mut PromotionState) -> Result<(), Error>,
    ) -> Result<PendingPromotion, PrepareFailure> {
        if self.point.is_none() || !matches!(self.sources.as_slice(), [Source::Point { .. }]) {
            return Err(PrepareFailure {
                source,
                error: Error::NeedOneStandalone,
            });
        }
        if let Err(error) = self.checked_view() {
            return Err(PrepareFailure { source, error });
        }
        match &source {
            Source::Point { reduced, .. } => {
                if hook != GeometryHook::ExactTwoStandaloneFirstProductInstalled {
                    return Err(PrepareFailure {
                        source,
                        error: Error::UnsupportedSecondSource,
                    });
                }
                if reduced.iter().any(|v| !v.is_finite()) {
                    return Err(PrepareFailure {
                        source,
                        error: Error::NonfiniteInput,
                    });
                }
            }
            Source::Trajectory { reduced, .. } => {
                if !(2..=maximum_children).contains(&reduced.len()) {
                    return Err(PrepareFailure {
                        source,
                        error: Error::ChildCountOutsideDomain,
                    });
                }
                if reduced.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(PrepareFailure {
                        source,
                        error: Error::NonfiniteInput,
                    });
                }
            }
        }
        let mut sources = self.sources.clone();
        sources.push(source);
        let old = self.point.as_ref().unwrap();
        let first = RootState {
            point: Wrapper {
                base: old.base,
                offset: old.offset,
                force: old.force,
            },
            standalone_scalars: Some(old.scalars),
            covariance: None,
            children: vec![],
        };
        let second = RootState::constructor(&sources[1]);
        let exact_two = matches!(
            sources.as_slice(),
            [Source::Point { .. }, Source::Point { .. }]
        );
        let mut pending = PendingPromotion {
            state: PromotionState {
                projection: self.projection.clone(),
                sources,
                roots: vec![first, second],
                motion: self.motion,
                remaining: self.remaining,
                controls: self.controls,
                completed_probe_geometry: self.completed_probe_geometry.clone(),
            },
            phase: Phase::GeometryFailed,
            issue: None,
            binding: DispatchBinding {
                checked_literal_required: true,
                source_reload: 10,
                exact_two_standalone_first_product_required: exact_two,
            },
        };

        match prefix(&mut pending.state) {
            Ok(()) => pending.phase = Phase::AwaitingProbe,
            Err(error) => pending.issue = Some(error),
        }
        Ok(pending)
    }
}
impl PreparedPartial {
    pub fn candidate(&self) -> &PartialController {
        &self.candidate
    }
    pub fn sources(&self) -> &[Source] {
        self.candidate.sources()
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn issue(&self) -> Option<&Error> {
        self.issue.as_ref()
    }
    pub fn checked_view(&self) -> Result<View, Error> {
        self.candidate.checked_view()
    }
    pub fn request(&self) -> Option<ProbeRequest> {
        let after = self.after?;
        if self.phase != Phase::AwaitingProbe && self.phase != Phase::ProbeFailed {
            return None;
        }
        let roots = self
            .candidate
            .point
            .iter()
            .map(|p| RequestedRoot {
                owner: 0,
                trajectory: false,
                points: vec![sample(p, None)],
            })
            .collect();
        Some(ProbeRequest {
            kind: if self.candidate.point.is_some() {
                RequestKind::CheckedLiteral
            } else {
                RequestKind::EmptyGuard
            },
            roots,
            controls: self.candidate.controls,
            after,
        })
    }
    pub fn complete_probe(
        mut self,
        checked: CheckedProbeCompletion,
    ) -> Result<Self, CompletionFailure<Self>> {
        let request = self.request();
        let result = match request.as_ref() {
            None => Err(Error::ProbeNotRequested),
            Some(request) => checked_fields(
                checked,
                request.kind,
                usize::from(self.candidate.point.is_some()),
            ),
        };
        match result {
            Err(error) => {
                self.issue = Some(error.clone());
                if self.after.is_some() {
                    self.phase = Phase::ProbeFailed;
                }
                Err(CompletionFailure {
                    pending: self,
                    error,
                })
            }
            Ok(completed) => {
                if let Some(p) = &mut self.candidate.point {
                    p.scalars = completed.fields[0];
                }
                if completed.segments.is_some() {
                    let root_positions = self.candidate.point.iter().map(|point| getter(point, None)).collect();
                    self.candidate.completed_probe_geometry = completed.geometry(
                        &self.candidate.sources, request.as_ref().unwrap(), root_positions);
                }
                match self.after.take().unwrap() {
                    AfterProbe::DecrementRemaining => self.candidate.remaining -= 1,
                    AfterProbe::ReloadRemaining => self.candidate.remaining = 10,
                }
                self.phase = Phase::Ready;
                self.issue = None;
                Ok(self)
            }
        }
    }
    pub fn into_ready(self) -> Result<PartialController, Self> {
        if self.phase == Phase::Ready {
            Ok(self.candidate)
        } else {
            Err(self)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryHook {
    CurrentGeneric,
    ExactTwoStandaloneFirstProductInstalled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchBinding {
    pub checked_literal_required: bool,
    pub source_reload: i32,
    pub exact_two_standalone_first_product_required: bool,
}
#[derive(Clone, Debug)]
pub struct Wrapper {
    pub base: [f32; 2],
    pub offset: [f32; 2],
    pub force: SourceField<[f32; 2]>,
}
#[derive(Clone, Debug)]
pub struct RootState {
    pub point: Wrapper,

    pub standalone_scalars: Option<[f32; 4]>,
    pub covariance: Option<[f32; 4]>,
    pub children: Vec<Point>,
}
impl RootState {
    fn constructor(source: &Source) -> Self {
        let point = Wrapper {
            base: [0.0; 2],
            offset: [0.0; 2],
            force: SourceField::Unwritten,
        };
        match source {
            Source::Point { .. } => Self {
                point,
                standalone_scalars: Some([0.0; 4]),
                covariance: None,
                children: vec![],
            },
            Source::Trajectory { reduced, .. } => Self {
                point,
                standalone_scalars: None,
                covariance: Some([1.0, 0.0, 0.0, 1.0]),
                children: reduced.iter().copied().map(Point::constructor).collect(),
            },
        }
    }
}
#[derive(Clone, Debug)]
pub struct PromotionState {
    pub projection: Projection,
    pub sources: Vec<Source>,
    pub roots: Vec<RootState>,
    pub motion: f64,
    pub remaining: i32,
    pub controls: ProbeControls,
    pub completed_probe_geometry: Option<std::sync::Arc<layout::background_geometry::CompletedGeometry>>,
}
impl PromotionState {
    fn geometry_prefix(&mut self) -> Result<(), Error> {
        self.geometry_prefix_using(
            |rows| Ok(geometry::statistics(rows)),
            |sources, stats, basis| Ok(geometry::base_geometry(sources, stats, basis)),
            || Ok(()),
        )
    }

    fn geometry_prefix_using(
        &mut self,
        statistics: impl FnOnce(&[[f32; 4]]) -> Result<geometry::Statistics, Error>,
        bases: impl FnOnce(&[Source], &geometry::Statistics, [f32; 8]) -> Result<Vec<geometry::Geometry>, Error>,
        before_force: impl FnOnce() -> Result<(), Error>,
    ) -> Result<(), Error> {
        let stats = statistics(&geometry::flatten(&self.sources))?;
        self.projection.negative_mean = SourceField::Defined(stats.offset);
        self.projection.scale = SourceField::Defined(stats.scale);
        self.projection.covariance = stats.covariance;
        self.geometry_from_statistics_using(&stats, bases, before_force)
    }

    fn geometry_from_statistics_using(
        &mut self,
        stats: &geometry::Statistics,
        bases: impl FnOnce(&[Source], &geometry::Statistics, [f32; 8]) -> Result<Vec<geometry::Geometry>, Error>,
        before_force: impl FnOnce() -> Result<(), Error>,
    ) -> Result<(), Error> {
        let basis = geometry::refresh_basis(stats, self.projection.working_uv);
        self.projection.working_uv = basis;
        self.projection.projected_basis = SourceField::Defined(basis);
        if basis.iter().any(|v| !v.is_finite()) {
            return Err(Error::NonfiniteGeometry);
        }
        let bases = bases(&self.sources, stats, basis)?;
        if bases.len() != self.roots.len()
            || bases
                .iter()
                .zip(&self.roots)
                .any(|(b, r)| b.children.len() != r.children.len())
        {
            return Err(Error::GeometryShape);
        }
        for (root, base) in self.roots.iter_mut().zip(bases) {
            root.point.base = base.base;
            if let Some(covariance) = base.covariance {
                root.covariance = Some(covariance);
            }
            for (p, xy) in root.children.iter_mut().zip(base.children) {
                p.base = xy;
            }
        }
        if stats
            .offset
            .iter()
            .chain(&stats.scale)
            .chain(&stats.covariance)
            .any(|v| !v.is_finite())
            || self.roots.iter().any(|r| {
                r.point
                    .base
                    .iter()
                    .chain(r.covariance.iter().flatten())
                    .chain(r.children.iter().flat_map(|p| p.base.iter()))
                    .any(|v| !v.is_finite())
            })
        {
            return Err(Error::NonfiniteGeometry);
        }

        before_force()?;
        for root in &mut self.roots {
            root.point.force = SourceField::Defined([0.0; 2]);
            for p in &mut root.children {
                p.force = SourceField::Defined([0.0; 2]);
            }
        }
        let mut numeric: Vec<_> = self.roots.iter().map(numeric_root).collect();
        self.motion = force::step(&mut numeric);
        for (root, computed) in self.roots.iter_mut().zip(numeric) {
            root.point.offset = computed.point.offset;
            root.point.force = SourceField::Defined(computed.point.force);
            for (p, c) in root.children.iter_mut().zip(computed.children) {
                p.offset = c.offset;
                p.force = SourceField::Defined(c.force);
            }
        }
        if !self.motion.is_finite()
            || self.roots.iter().any(|r| {
                r.point
                    .offset
                    .iter()
                    .chain(r.point.force.defined().unwrap())
                    .any(|v| !v.is_finite())
                    || r.children.iter().any(|p| {
                        p.offset
                            .iter()
                            .chain(p.force.defined().unwrap())
                            .any(|v| !v.is_finite())
                    })
            })
        {
            return Err(Error::NonfiniteGeometry);
        }
        Ok(())
    }

    fn geometry_prefix_with_context(
        &mut self, context: Option<&layout::context::GeometryContext>, plans: &mut layout::context::GeometryPlans,
    ) -> Result<(), Error> {
        self.statistics_prefix_with_context(context, plans)?;
        self.geometry_from_current_statistics_with_context(context, plans)
    }

    fn statistics_prefix_with_context(
        &mut self, context: Option<&layout::context::GeometryContext>, plans: &mut layout::context::GeometryPlans,
    ) -> Result<(), Error> {
        use layout::context::{arithmetic, provider, ContextRequest, CountPlan};
        use arithmetic::cache_blocking::Product;
        let total = self.sources.iter().try_fold(0usize, |n, source| n.checked_add(match source {
            Source::Point { .. } => 1, Source::Trajectory { reduced, .. } => reduced.len(),
        })).ok_or(Error::GeometryShape)?;
        if !(2..=2049).contains(&total) { return Err(Error::ChildCountOutsideDomain); }
        if total >= 12 && context.is_none() { return Err(Error::ContextRequired); }
        plans.parent_attempts.clear();
        let rows = geometry::flatten(&self.sources);
        let request = ContextRequest { product: Product::Global4d, depth: rows.len(), owner: None };
        plans.statistics_attempt = Some(CountPlan { request, blocking: None });
        provider::check_current_thread().map_err(|error| Error::ExternalGeometry(format!("Floating-point controls: {error:?}")))?;
        let (stats, blocking) = if rows.len() >= 12 {
            let (stats, blocking) = arithmetic::statistics_with_context(&rows, context.ok_or(Error::ContextRequired)?)
                .map_err(|error| Error::ExternalGeometry(format!("Context statistics: {error:?}")))?;
            (stats, Some(blocking))
        } else { (geometry::statistics(&rows), None) };
        let plan = CountPlan { request, blocking };
        plans.statistics_attempt = Some(plan.clone()); plans.cached_statistics = Some(plan);
        self.projection.negative_mean = SourceField::Defined(stats.offset);
        self.projection.scale = SourceField::Defined(stats.scale);
        self.projection.covariance = stats.covariance;
        Ok(())
    }

    fn geometry_from_current_statistics_with_context(
        &mut self, context: Option<&layout::context::GeometryContext>, plans: &mut layout::context::GeometryPlans,
    ) -> Result<(), Error> {
        use layout::context::{arithmetic, provider, ContextRequest, CountPlan};
        use arithmetic::cache_blocking::Product;
        let controls = || provider::check_current_thread().map(|_| ())
            .map_err(|error| Error::ExternalGeometry(format!("Floating-point controls: {error:?}")));
        let SourceField::Defined(offset) = self.projection.negative_mean else { return Err(Error::GeometryNotReady); };
        let SourceField::Defined(scale) = self.projection.scale else { return Err(Error::GeometryNotReady); };
        let stats = geometry::Statistics { offset, scale, covariance: self.projection.covariance };
        if self.sources.iter().any(|source| matches!(source, Source::Trajectory { reduced, .. } if reduced.len() >= 16))
            && context.is_none() { return Err(Error::ContextRequired); }
        controls()?;
        plans.parent_attempts.clear();
        let parent_attempts = &mut plans.parent_attempts;
        let owners: Vec<_> = self.sources.iter().enumerate().filter_map(|(owner, source)|
            matches!(source, Source::Trajectory { .. }).then_some(owner)).collect();
        let mut parent = 0;
        self.geometry_from_statistics_using(&stats,
            |sources, stats, basis| {
                controls()?;
                geometry::base_geometry_with_parent_summary(sources, stats, basis, |children| {
                    let request = ContextRequest { product: Product::Parent2d, depth: children.len(), owner: Some(owners[parent]) };
                    parent += 1; parent_attempts.push(CountPlan { request, blocking: None });
                    controls()?;
                    if children.len() < 16 { return Ok(geometry::summary(children)); }
                    let (summary, blocking) = arithmetic::parent_summary_with_context(children, context.ok_or(Error::ContextRequired)?)
                        .map_err(|error| Error::ExternalGeometry(format!("Context parent summary: {error:?}")))?;
                    parent_attempts.last_mut().unwrap().blocking = Some(blocking);
                    Ok(summary)
                })
            },
            || controls(),
        )
    }
    pub fn checked_view(&self) -> Result<View, Error> {
        if self.sources.len() != self.roots.len() {
            return Err(Error::GeometryShape);
        }
        let mut positions = vec![];
        let mut scalars = vec![];
        for (source, r) in self.sources.iter().zip(&self.roots) {
            let (xy, values): (Vec<_>, Vec<_>) = match source {
                Source::Point { .. }
                    if r.children.is_empty()
                        && r.covariance.is_none()
                        && r.standalone_scalars.is_some() =>
                {
                    (
                        vec![wrapper_getter(&r.point)],
                        vec![r.standalone_scalars.unwrap()],
                    )
                }
                Source::Trajectory { reduced, .. }
                    if r.children.len() == reduced.len()
                        && r.covariance.is_some()
                        && r.standalone_scalars.is_none() =>
                {
                    (
                        r.children
                            .iter()
                            .map(|p| getter(p, Some(r.point.offset)))
                            .collect(),
                        r.children.iter().map(|p| p.scalars).collect(),
                    )
                }
                _ => return Err(Error::GeometryShape),
            };
            if xy
                .iter()
                .flatten()
                .chain(values.iter().flatten())
                .any(|v| !v.is_finite())
            {
                return Err(Error::NonfiniteGeometry);
            }
            positions.push(xy);
            scalars.push(values);
        }
        Ok(View { positions, scalars })
    }
    pub fn working_uv_snapshot(&self) -> [f32; 8] {
        self.projection.working_uv
    }
    fn literal_probe_request(&self, after: AfterProbe) -> ProbeRequest {
        ProbeRequest {
            kind: RequestKind::CheckedLiteral, controls: self.controls, after,
            roots: self.roots.iter().enumerate().map(|(owner, root)| {
                let trajectory = root.covariance.is_some();
                let points = if trajectory {
                    root.children.iter().map(|point| sample(point, Some(root.point.offset))).collect()
                } else {
                    let fields = root.standalone_scalars.unwrap();
                    vec![ProbeSample { xy: wrapper_getter(&root.point), old: [fields[0], fields[2], fields[3]] }]
                };
                RequestedRoot { owner: owner as u32, trajectory, points }
            }).collect(),
        }
    }
    fn commit_probe_fields(&mut self, fields: &[[f32; 4]]) {
        let mut cursor = 0;
        for root in &mut self.roots {
            if root.covariance.is_some() {
                for point in &mut root.children { point.scalars = fields[cursor]; cursor += 1; }
            } else { root.standalone_scalars = Some(fields[cursor]); cursor += 1; }
        }
    }
}
pub struct PendingPromotion {
    state: PromotionState,
    phase: Phase,
    issue: Option<Error>,
    binding: DispatchBinding,
}
impl PendingPromotion {
    pub fn state(&self) -> &PromotionState {
        &self.state
    }
    pub fn sources(&self) -> &[Source] {
        &self.state.sources
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn issue(&self) -> Option<&Error> {
        self.issue.as_ref()
    }
    pub fn checked_view(&self) -> Result<View, Error> {
        self.state.checked_view()
    }
    pub fn request(&self) -> Option<ProbeRequest> {
        if self.phase != Phase::AwaitingProbe && self.phase != Phase::ProbeFailed {
            return None;
        }
        Some(self.state.literal_probe_request(AfterProbe::ReloadRemaining))
    }
    pub fn complete_probe(
        mut self,
        checked: CheckedProbeCompletion,
    ) -> Result<ReadyFull, CompletionFailure<Self>> {
        if self.request().is_none() {
            return Err(CompletionFailure {
                pending: self,
                error: Error::GeometryNotReady,
            });
        }
        let expected = self
            .state
            .roots
            .iter()
            .map(|r| {
                if r.covariance.is_some() {
                    r.children.len()
                } else {
                    1
                }
            })
            .sum();
        let request = self.request().unwrap();
        let completed = match checked_fields(checked, RequestKind::CheckedLiteral, expected) {
            Ok(completed) => completed,
            Err(error) => {
                self.phase = Phase::ProbeFailed;
                self.issue = Some(error.clone());
                return Err(CompletionFailure {
                    pending: self,
                    error,
                });
            }
        };
        self.state.commit_probe_fields(&completed.fields);
        let root_positions = self.state.roots.iter().map(|root| wrapper_getter(&root.point)).collect();
        self.state.completed_probe_geometry = completed.geometry(&self.state.sources, &request, root_positions);
        self.state.remaining = 10;
        Ok(ready_from_completed(self.state, self.binding))
    }
}
pub struct ReadyFull {
    controller: layout::Controller,
    binding: DispatchBinding,
}
pub struct AdoptedFull {
    controller: layout::Controller,
    binding: DispatchBinding,
}
impl ReadyFull {
    pub fn controller_for_review(&self) -> &layout::Controller {
        &self.controller
    }

    pub fn adopt(self) -> AdoptedFull {
        AdoptedFull {
            controller: self.controller,
            binding: self.binding,
        }
    }
}
impl AdoptedFull {
    pub fn into_parts(self) -> (layout::Controller, DispatchBinding) {
        (self.controller, self.binding)
    }
}
fn ready_from_completed(state: PromotionState, binding: DispatchBinding) -> ReadyFull {
    let scalars = state
        .roots
        .iter()
        .map(|r| {
            if r.covariance.is_some() {
                r.children.iter().map(|p| p.scalars).collect()
            } else {
                vec![r.standalone_scalars.unwrap()]
            }
        })
        .collect();
    let roots = state.roots.iter().map(numeric_root).collect();
    let SourceField::Defined(offset) = state.projection.negative_mean else {
        unreachable!("Successful prefix defines mean")
    };
    let SourceField::Defined(scale) = state.projection.scale else {
        unreachable!("Successful prefix defines scale")
    };
    let SourceField::Defined(basis) = state.projection.projected_basis else {
        unreachable!("Successful prefix defines projection")
    };
    ReadyFull {
        controller: layout::Controller {
            sources: state.sources,
            statistics: geometry::Statistics {
                offset,
                scale,
                covariance: state.projection.covariance,
            },
            basis,
            roots,
            motion: state.motion,
            remaining: state.remaining,
            probe: state.controls.xy,
            radius: state.controls.radius,
            scalars,
            completed_mix: None,
            completed_probe_geometry: state.completed_probe_geometry,
        },
        binding,
    }
}
fn numeric_point(p: &Point) -> force::Point {
    let SourceField::Defined(force) = p.force else {
        unreachable!("Source clear precedes numeric conversion")
    };
    force::Point {
        base: p.base,
        offset: p.offset,
        force,
    }
}
fn numeric_root(r: &RootState) -> force::Root {
    let SourceField::Defined(force) = r.point.force else {
        unreachable!("Source clear precedes wrapper conversion")
    };
    force::Root {
        point: force::Point {
            base: r.point.base,
            offset: r.point.offset,
            force,
        },
        covariance: r.covariance.unwrap_or([0.0; 4]),
        children: r.children.iter().map(numeric_point).collect(),
        trajectory: r.covariance.is_some(),
    }
}
fn identity4() -> [f32; 16] {
    std::array::from_fn(|i| if i % 5 == 0 { 1.0 } else { 0.0 })
}
fn bits4(v: [f32; 4]) -> [u32; 4] {
    v.map(f32::to_bits)
}
fn moving(motion: f64) -> bool {
    motion > f64::from_bits(0x3e7ad7f2a0000000)
}
fn valid_controls(c: ProbeControls) -> Result<(), Error> {
    if c.xy.iter().any(|v| !v.is_finite()) || !c.radius.is_finite() || c.radius <= 0.0 {
        Err(Error::InvalidProbe)
    } else {
        Ok(())
    }
}
fn getter(p: &Point, parent: Option<[f32; 2]>) -> [f32; 2] {
    let mut xy = [p.base[0] + p.offset[0], p.base[1] + p.offset[1]];
    if let Some(parent) = parent {
        xy[0] += parent[0];
        xy[1] += parent[1];
    }
    xy
}
fn sample(p: &Point, parent: Option<[f32; 2]>) -> ProbeSample {
    ProbeSample {
        xy: getter(p, parent),
        old: [p.scalars[0], p.scalars[2], p.scalars[3]],
    }
}
fn wrapper_getter(p: &Wrapper) -> [f32; 2] {
    [p.base[0] + p.offset[0], p.base[1] + p.offset[1]]
}
struct CheckedFields {
    fields: Vec<[f32; 4]>,
    segments: Option<Vec<layout::probe_math::spatial_builder::Segment>>,
}
impl CheckedFields {
    fn geometry(self, sources: &[Source], request: &ProbeRequest, root_positions: Vec<[f32; 2]>)
        -> Option<std::sync::Arc<layout::background_geometry::CompletedGeometry>>
    {
        Some(std::sync::Arc::new(layout::background_geometry::CompletedGeometry {
            sources: sources.to_vec(), segments: self.segments?, root_positions,
            getters: request.roots.iter().map(|root| root.points.iter().map(|point| point.xy).collect()).collect(),
        }))
    }
}
fn checked_fields(
    completion: CheckedProbeCompletion,
    kind: RequestKind,
    expected: usize,
) -> Result<CheckedFields, Error> {
    let (fields, segments) = match (kind, completion) {
        (RequestKind::CheckedLiteral, CheckedProbeCompletion::Literal(result)) => {
            let completed = result.map_err(Error::ExternalProbe)?;
            (completed.fields, Some(completed.segments))
        }
        (RequestKind::EmptyGuard, CheckedProbeCompletion::EmptyGuard(result)) => {
            result.map_err(Error::ExternalProbe)?;
            (vec![], None)
        }
        _ => return Err(Error::WrongCompletionKind),
    };
    if fields.len() != expected {
        return Err(Error::WrongScalarCount);
    }
    if fields.iter().flatten().any(|v| !v.is_finite()) {
        return Err(Error::NonfiniteProbe);
    }
    Ok(CheckedFields { fields, segments })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataError {
    CurrentShapeMismatch,
    CountChanged,
    ClassChanged,
    IdentityChanged,
}
impl PartialController {

    pub fn fork_with_matching_metadata(
        &self,
        sources: &[layout::Source],
    ) -> Result<Self, MetadataError> {
        match (&self.point, self.sources.as_slice()) {
            (None, []) => {}
            (Some(point), [Source::Point { reduced, .. }])
                if bits4(point.reduced) == bits4(*reduced) => {}
            _ => return Err(MetadataError::CurrentShapeMismatch),
        }
        if sources.len() != self.sources.len() {
            return Err(MetadataError::CountChanged);
        }
        if sources
            .iter()
            .any(|source| !matches!(source, Source::Point { .. }))
        {
            return Err(MetadataError::ClassChanged);
        }
        if !layout::same_sources(&self.sources, sources) {
            return Err(MetadataError::IdentityChanged);
        }
        let mut next = self.clone();
        for (current, incoming) in next.sources.iter_mut().zip(sources) {
            if let (
                Source::Point { enabled, .. },
                Source::Point {
                    enabled: new_enabled,
                    ..
                },
            ) = (current, incoming)
            {
                *enabled = *new_enabled;
            }
        }
        Ok(next)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstTrajectoryError {
    NeedEmptyStartup,
    NeedTrajectory,
    ChildCountOutsideDomain,
    NonfiniteInput,
}
pub struct FirstTrajectoryPrepareFailure {
    pub source: layout::Source,
    pub error: FirstTrajectoryError,
}
impl PartialController {
    pub fn prepare_first_trajectory(
        &self,
        source: layout::Source,
    ) -> Result<PendingPromotion, FirstTrajectoryPrepareFailure> {
        self.prepare_first_trajectory_using(source, 46, |state| state.geometry_prefix())
    }

    pub fn prepare_first_trajectory_with_context(
        &self, source: layout::Source,
        context: Option<&layout::context::GeometryContext>, plans: &mut layout::context::GeometryPlans,
    ) -> Result<PendingPromotion, FirstTrajectoryPrepareFailure> {
        self.prepare_first_trajectory_using(source, 2049, |state| state.geometry_prefix_with_context(context, plans))
    }

    fn prepare_first_trajectory_using(
        &self, source: layout::Source, maximum_children: usize,
        prefix: impl FnOnce(&mut PromotionState) -> Result<(), Error>,
    ) -> Result<PendingPromotion, FirstTrajectoryPrepareFailure> {
        if self.point.is_some() || !self.sources.is_empty() {
            return Err(FirstTrajectoryPrepareFailure {
                source,
                error: FirstTrajectoryError::NeedEmptyStartup,
            });
        }
        let Source::Trajectory { reduced, .. } = &source else {
            return Err(FirstTrajectoryPrepareFailure {
                source,
                error: FirstTrajectoryError::NeedTrajectory,
            });
        };
        if !(2..=maximum_children).contains(&reduced.len()) {
            return Err(FirstTrajectoryPrepareFailure {
                source,
                error: FirstTrajectoryError::ChildCountOutsideDomain,
            });
        }
        if reduced.iter().flatten().any(|v| !v.is_finite()) {
            return Err(FirstTrajectoryPrepareFailure {
                source,
                error: FirstTrajectoryError::NonfiniteInput,
            });
        }
        let root = RootState::constructor(&source);
        let mut pending = PendingPromotion {
            state: PromotionState {
                projection: self.projection.clone(),
                sources: vec![source],
                roots: vec![root],
                motion: self.motion,
                remaining: self.remaining,
                controls: self.controls,
                completed_probe_geometry: self.completed_probe_geometry.clone(),
            },
            phase: Phase::GeometryFailed,
            issue: None,
            binding: DispatchBinding {
                checked_literal_required: true,
                source_reload: 10,
                exact_two_standalone_first_product_required: false,
            },
        };
        match prefix(&mut pending.state) {
            Ok(()) => pending.phase = Phase::AwaitingProbe,
            Err(error) => pending.issue = Some(error),
        }
        Ok(pending)
    }
}
