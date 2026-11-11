#[path = "desktop_layout_force.rs"]
pub mod force;
#[path = "desktop_layout_geometry.rs"]
pub mod geometry;
#[path = "desktop_probe.rs"]
pub(super) mod probe_math;
#[path = "desktop_background_geometry.rs"]
pub mod background_geometry;
pub use geometry::Source;
#[path = "desktop_layout_context.rs"]
pub mod context;

#[derive(Clone)]
pub struct Controller {
    pub sources: Vec<Source>,
    pub statistics: geometry::Statistics,
    pub basis: [f32; 8],
    pub roots: Vec<force::Root>,
    pub motion: f64,
    pub remaining: i32,
    pub probe: [f32; 2],
    pub radius: f64,
    pub scalars: Vec<Vec<[f32; 4]>>,
    pub(super) completed_mix: Option<std::sync::Arc<crate::prototype_controls::ControllerMix>>,
    pub(super) completed_probe_geometry: Option<std::sync::Arc<background_geometry::CompletedGeometry>>,
}

pub fn same_sources(a: &[Source], b: &[Source]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| match (a, b) {
            (Source::Point { reduced: a, .. }, Source::Point { reduced: b, .. }) => {
                same_rows(&[*a], &[*b])
            }
            (Source::Trajectory { reduced: a, .. }, Source::Trajectory { reduced: b, .. }) => {
                same_rows(a, b)
            }
            _ => false,
        })
}

fn same_rows(a: &[[f32; 4]], b: &[[f32; 4]]) -> bool {
    a.len() == b.len()
        && a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn supported(sources: &[Source], basis: [f32; 8]) -> Result<(), String> {
    let total: usize = sources
        .iter()
        .map(|s| match s {
            Source::Point { .. } => 1,
            Source::Trajectory { reduced, .. } => reduced.len(),
        })
        .sum();
    if total < 2 {
        return Err("Canvas layout: fewer than two points.".into());
    }
    if total >= 48 {
        return Err(
            "Canvas layout: this preset exceeds the supported point limit."
                .into(),
        );
    }
    if sources.iter().any(
        |s| matches!(s, Source::Trajectory { reduced, .. } if !(2..48).contains(&reduced.len())),
    ) {
        return Err("Canvas layout: this trajectory length is not supported.".into());
    }
    if geometry::flatten(sources)
        .iter()
        .flatten()
        .chain(basis.iter())
        .any(|v| !v.is_finite())
        || basis[..4].iter().all(|v| *v == 0.0)
        || basis[4..].iter().all(|v| *v == 0.0)
    {
        return Err("Canvas layout: the incoming projection cannot be used.".into());
    }
    Ok(())
}

impl Controller {
    #[allow(dead_code)]
    pub fn new(
        sources: Vec<Source>,
        incoming: [f32; 8],
        probe: [f32; 2],
        radius: f64,
    ) -> Result<Self, String> {
        supported(&sources, incoming)?;
        let statistics = geometry::statistics(&geometry::flatten(&sources));
        let roots = sources.iter().map(Self::new_root).collect();
        let scalars = sources
            .iter()
            .map(|source| vec![[0.0; 4]; Self::point_count(source)])
            .collect();
        let mut state = Self {
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
        };
        state.refresh()?;
        state.refresh_probe()?;
        Ok(state)
    }

    fn new_root(source: &Source) -> force::Root {
        let (trajectory, count) = match source {
            Source::Point { .. } => (false, 0),
            Source::Trajectory { reduced, .. } => (true, reduced.len()),
        };
        force::Root {
            point: force::Point::default(),
            covariance: [0.0; 4],
            trajectory,
            children: vec![force::Point::default(); count],
        }
    }

    fn point_count(source: &Source) -> usize {
        match source {
            Source::Point { .. } => 1,
            Source::Trajectory { reduced, .. } => reduced.len(),
        }
    }

    #[allow(dead_code)]
    pub fn append(&mut self, source: Source) -> Result<(), String> {
        let mut next = self.clone();
        next.roots.push(Self::new_root(&source));
        next.scalars
            .push(vec![[0.0; 4]; Self::point_count(&source)]);
        next.sources.push(source);
        supported(&next.sources, next.basis)?;
        next.statistics = geometry::statistics(&geometry::flatten(&next.sources));
        next.refresh()?;
        next.refresh_probe()?;
        next.remaining = 10;
        *self = next;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, index: usize) -> Result<(), String> {
        if index >= self.sources.len() {
            return Err("Invalid layout source.".into());
        }
        let mut next = self.clone();
        next.sources.remove(index);
        next.roots.remove(index);
        next.scalars.remove(index);
        supported(&next.sources, next.basis)?;
        next.statistics = geometry::statistics(&geometry::flatten(&next.sources));
        next.refresh()?;
        next.refresh_probe()?;
        *self = next;
        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        self.refresh_with_geometry(|sources, statistics, basis| {
            Ok(geometry::base_geometry(sources, statistics, basis))
        })
    }

    pub(super) fn refresh_with_geometry(
        &mut self,
        build: impl FnOnce(&[Source], &geometry::Statistics, [f32; 8])
            -> Result<Vec<geometry::Geometry>, String>,
    ) -> Result<(), String> {
        let basis = geometry::refresh_basis(&self.statistics, self.basis);
        self.rebuild_with_basis(basis, build)
    }

    pub(super) fn restore_with_geometry(
        &mut self,
        build: impl FnOnce(&[Source], &geometry::Statistics, [f32; 8])
            -> Result<Vec<geometry::Geometry>, String>,
    ) -> Result<(), String> {
        self.rebuild_with_basis(self.basis, build)
    }

    fn rebuild_with_basis(
        &mut self,
        basis: [f32; 8],
        build: impl FnOnce(&[Source], &geometry::Statistics, [f32; 8])
            -> Result<Vec<geometry::Geometry>, String>,
    ) -> Result<(), String> {
        if basis.iter().any(|v| !v.is_finite()) {
            return Err("Canvas layout: degenerate projection.".into());
        }
        let bases = build(&self.sources, &self.statistics, basis)?;
        if bases.iter().any(|b| {
            b.base
                .iter()
                .chain(b.children.iter().flatten())
                .chain(b.covariance.iter().flatten())
                .any(|v| !v.is_finite())
        }) {
            return Err("Canvas layout: degenerate trajectory.".into());
        }
        self.basis = basis;
        for (root, base) in self.roots.iter_mut().zip(bases) {
            root.point.base = base.base;
            if let Some(covariance) = base.covariance {
                root.covariance = covariance;
            }
            for (child, position) in root.children.iter_mut().zip(base.children) {
                child.base = position;
            }
        }
        self.motion = force::step(&mut self.roots);
        if !self.motion.is_finite()
            || self.roots.iter().any(|r| {
                std::iter::once(&r.point)
                    .chain(&r.children)
                    .any(|p| p.offset.iter().chain(&p.force).any(|v| !v.is_finite()))
            })
        {
            return Err("Canvas layout: this configuration cannot be relaxed.".into());
        }
        Ok(())
    }

    pub fn active(&self) -> bool {
        self.motion > f64::from_bits(0x3e7ad7f2a0000000) || self.remaining > 0
    }

    #[allow(dead_code)]
    pub fn callback(&mut self) -> Result<bool, String> {
        let moving = self.motion > f64::from_bits(0x3e7ad7f2a0000000);
        if moving {
            self.refresh()?;
            self.remaining = 10;
        }
        if self.remaining > 0 {
            self.refresh_probe()?;
            self.remaining -= 1;
        }
        Ok(moving)
    }

    #[allow(dead_code)]
    pub fn probe_changed(&mut self, probe: [f32; 2], radius: f64) {
        self.probe = probe;
        self.radius = radius;
        self.remaining = 10;
    }

    pub fn refresh_probe(&mut self) -> Result<(), String> {
        self.check_shape()?;
        if self.probe.iter().any(|v| !v.is_finite())
            || !self.radius.is_finite()
            || self.radius <= 0.0
        {
            return Err("Invalid probe state.".into());
        }
        let positions = self.positions();
        let mut roots = Vec::with_capacity(self.sources.len());
        for (owner, ((positions, scalars), source)) in positions
            .iter()
            .zip(&self.scalars)
            .zip(&self.sources)
            .enumerate()
        {
            let samples: Vec<_> = positions
                .iter()
                .zip(scalars)
                .map(|(xy, fields)| probe_math::Sample {
                    xy: *xy,
                    old: [fields[0], fields[2], fields[3]],
                })
                .collect();
            roots.push(match source {
                Source::Point { .. } => probe_math::Root::Standalone {
                    owner: owner as u32,
                    point: samples[0].clone(),
                },
                Source::Trajectory { .. } => probe_math::Root::Trajectory {
                    owner: owner as u32,
                    children: samples,
                },
            });
        }
        let completed = probe_math::refresh_checked(&mut roots, self.probe, self.radius)
            .map_err(|_| "This canvas configuration cannot be updated yet.".to_owned())?;
        let probe_math::CompletedProbe { fields, segments, mix, .. } = completed;
        let expected: usize = self.scalars.iter().map(Vec::len).sum();
        if fields.len() != expected {
            return Err("Invalid probe result shape.".into());
        }
        let mut cursor = 0;
        for owner in &mut self.scalars {
            for point in owner {
                *point = fields[cursor];
                cursor += 1;
            }
        }
        self.completed_mix = Some(mix);
        self.completed_probe_geometry = Some(std::sync::Arc::new(background_geometry::CompletedGeometry {
            sources: self.sources.clone(), segments, getters: positions,
            root_positions: self.roots.iter().map(|root| [
                root.point.base[0] + root.point.offset[0],
                root.point.base[1] + root.point.offset[1],
            ]).collect(),
        }));
        Ok(())
    }

    pub fn completed_probe_geometry(&self)
        -> Option<&std::sync::Arc<background_geometry::CompletedGeometry>>
    {
        self.completed_probe_geometry.as_ref()
    }

    pub fn completed_mix(&self) -> Option<&std::sync::Arc<crate::prototype_controls::ControllerMix>> {
        self.completed_mix.as_ref()
    }

    pub fn positions(&self) -> Vec<Vec<[f32; 2]>> {
        self.roots
            .iter()
            .map(|root| {
                if root.trajectory {
                    root.children
                        .iter()
                        .map(|p| {
                            [
                                (p.base[0] + p.offset[0]) + root.point.offset[0],
                                (p.base[1] + p.offset[1]) + root.point.offset[1],
                            ]
                        })
                        .collect()
                } else {
                    vec![[
                        root.point.base[0] + root.point.offset[0],
                        root.point.base[1] + root.point.offset[1],
                    ]]
                }
            })
            .collect()
    }
}

include!("desktop_layout_prepared.rs");
