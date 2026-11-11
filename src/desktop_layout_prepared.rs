#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparedPhase {
    AwaitingProbe,
    Ready,
    GeometryFailed,
    ProbeFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeSuffix {
    Keep,
    Reload,
    Decrement,
}

#[must_use = "Keep the prepared candidate until publication or explicit user resolution."]
pub struct PreparedFullController {
    candidate: Controller,
    phase: PreparedPhase,
    suffix: ProbeSuffix,
    changed: bool,
    issue: Option<String>,
}

pub struct FullView<'a> {
    pub positions: Vec<Vec<[f32; 2]>>,
    pub scalars: &'a [Vec<[f32; 4]>],
}

pub enum InspectionDomain {
    TooFewPoints,
    TooManyPoints,
    ContextResourceLimit,
    TrajectoryLength,
    IncomingProjection,
    FirstTrajectory,
}

impl InspectionDomain {
    pub fn message(&self) -> &'static str {
        match self {
            Self::TooFewPoints => "Canvas layout: fewer than two points.",
            Self::TooManyPoints => {
                "Canvas layout: this preset exceeds the supported point limit."
            }
            Self::ContextResourceLimit => {
                "Canvas layout: this canvas exceeds the supported point limit."
            }
            Self::TrajectoryLength => {
                "Canvas layout: this trajectory length is not supported."
            }
            Self::IncomingProjection => {
                "Canvas layout: the incoming projection cannot be used."
            }
            Self::FirstTrajectory => {
                "Canvas layout: this first trajectory length is not supported."
            }
        }
    }
}

pub fn inspection_domain(sources: &[Source]) -> Option<InspectionDomain> {
    let total: usize = sources.iter().map(Controller::point_count).sum();
    if total < 2 {
        Some(InspectionDomain::TooFewPoints)
    } else if total >= 48 {
        Some(InspectionDomain::TooManyPoints)
    } else if sources.iter().any(|source| {
        matches!(source,
        Source::Trajectory { reduced, .. } if !(2..48).contains(&reduced.len()))
    }) {
        Some(InspectionDomain::TrajectoryLength)
    } else {
        None
    }
}

impl PreparedFullController {
    fn geometry(mut candidate: Controller, suffix: ProbeSuffix) -> Self {
        let issue = candidate.refresh().err();
        let phase = if issue.is_some() {
            PreparedPhase::GeometryFailed
        } else {
            PreparedPhase::AwaitingProbe
        };
        Self {
            candidate,
            phase,
            suffix,
            changed: true,
            issue,
        }
    }

    #[allow(dead_code)]
    pub fn candidate(&self) -> &Controller {
        &self.candidate
    }
    #[allow(dead_code)]
    pub fn phase(&self) -> PreparedPhase {
        self.phase
    }
    #[allow(dead_code)]
    pub fn issue(&self) -> Option<&str> {
        self.issue.as_deref()
    }
    #[allow(dead_code)]
    pub fn geometry_changed(&self) -> bool {
        self.changed
    }

    #[allow(dead_code)]
    pub fn finish_probe(&mut self) -> Result<(), String> {
        match self.phase {
            PreparedPhase::Ready => return Ok(()),
            PreparedPhase::GeometryFailed | PreparedPhase::ProbeFailed => {
                return Err(self.issue.clone().expect("failed stage records its error"));
            }
            PreparedPhase::AwaitingProbe => {}
        }
        if let Err(error) = self.candidate.refresh_probe() {
            self.phase = PreparedPhase::ProbeFailed;
            self.issue = Some(error.clone());
            return Err(error);
        }
        match self.suffix {
            ProbeSuffix::Keep => {}
            ProbeSuffix::Reload => self.candidate.remaining = 10,
            ProbeSuffix::Decrement => self.candidate.remaining -= 1,
        }
        self.phase = PreparedPhase::Ready;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn into_ready(self) -> Result<Controller, Self> {
        if self.phase == PreparedPhase::Ready {
            Ok(self.candidate)
        } else {
            Err(self)
        }
    }
}

impl Controller {
    fn check_shape(&self) -> Result<(), String> {
        if self.sources.len() != self.roots.len() || self.sources.len() != self.scalars.len() {
            return Err("Invalid layout owner state.".into());
        }
        for ((source, root), scalars) in self.sources.iter().zip(&self.roots).zip(&self.scalars) {
            let trajectory = matches!(source, Source::Trajectory { .. });
            let expected = Self::point_count(source);
            if root.trajectory != trajectory
                || scalars.len() != expected
                || root.children.len() != if trajectory { expected } else { 0 }
            {
                return Err("Invalid layout point state.".into());
            }
        }
        Ok(())
    }

    pub fn checked_view(&self) -> Result<FullView<'_>, String> {
        self.check_shape()?;
        let positions = self.positions();
        if positions
            .iter()
            .flatten()
            .flatten()
            .chain(self.scalars.iter().flatten().flatten())
            .any(|v| !v.is_finite())
        {
            return Err("Layout view is unavailable.".into());
        }
        Ok(FullView {
            positions,
            scalars: &self.scalars,
        })
    }

    #[allow(dead_code)]
    pub fn prepare_new(
        sources: Vec<Source>,
        incoming: [f32; 8],
        probe: [f32; 2],
        radius: f64,
    ) -> Result<PreparedFullController, String> {
        supported(&sources, incoming)?;
        let statistics = geometry::statistics(&geometry::flatten(&sources));
        let roots = sources.iter().map(Self::new_root).collect();
        let scalars = sources
            .iter()
            .map(|s| vec![[0.0; 4]; Self::point_count(s)])
            .collect();
        Ok(PreparedFullController::geometry(
            Self {
                sources,
                statistics,
                basis: incoming,
                roots,
                motion: 0.0,
                remaining: 0,
                probe,
                radius,
                scalars,
                completed_mix: None,
                completed_probe_geometry: None,
            },
            ProbeSuffix::Keep,
        ))
    }

    #[allow(dead_code)]
    pub fn prepare_append(&self, source: Source) -> Result<PreparedFullController, String> {
        self.check_shape()?;
        let mut next = self.clone();
        next.roots.push(Self::new_root(&source));
        next.scalars
            .push(vec![[0.0; 4]; Self::point_count(&source)]);
        next.sources.push(source);
        supported(&next.sources, next.basis)?;
        next.statistics = geometry::statistics(&geometry::flatten(&next.sources));
        Ok(PreparedFullController::geometry(next, ProbeSuffix::Reload))
    }

    #[allow(dead_code)]
    pub fn prepare_remove(&self, index: usize) -> Result<PreparedFullController, String> {
        self.check_shape()?;
        if index >= self.sources.len() {
            return Err("Invalid layout source.".into());
        }
        let mut next = self.clone();
        next.sources.remove(index);
        next.roots.remove(index);
        next.scalars.remove(index);
        supported(&next.sources, next.basis)?;
        next.statistics = geometry::statistics(&geometry::flatten(&next.sources));
        Ok(PreparedFullController::geometry(next, ProbeSuffix::Keep))
    }

    #[allow(dead_code)]
    pub fn prepare_callback(&self) -> PreparedFullController {
        let moving = self.motion > f64::from_bits(0x3e7ad7f2a0000000);
        let mut next = self.clone();
        let issue = next
            .check_shape()
            .err()
            .or_else(|| if moving { next.refresh().err() } else { None });
        if let Some(error) = issue {
            return PreparedFullController {
                candidate: next,
                phase: PreparedPhase::GeometryFailed,
                suffix: ProbeSuffix::Keep,
                changed: moving,
                issue: Some(error),
            };
        }
        if moving {
            next.remaining = 10;
        }
        let phase = if next.remaining > 0 {
            PreparedPhase::AwaitingProbe
        } else {
            PreparedPhase::Ready
        };
        PreparedFullController {
            candidate: next,
            phase,
            suffix: ProbeSuffix::Decrement,
            changed: moving,
            issue: None,
        }
    }
}
