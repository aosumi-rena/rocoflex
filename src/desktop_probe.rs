#![allow(dead_code, non_upper_case_globals)]
include!("desktop_probe_scalar.rs");
#[derive(Clone, Debug)]
pub struct Sample {
    pub xy: [f32; 2],
    pub old: [f32; 3],
}
#[derive(Clone, Debug)]
pub enum Root {
    Standalone { owner: u32, point: Sample },
    Trajectory { owner: u32, children: Vec<Sample> },
}
#[derive(Debug)]
pub enum ProbeError {
    Empty,
    InvalidTrajectory,
    DuplicateOwner,
    NonFiniteInput,
    Builder(spatial_builder::BuildError),
    NonFiniteOutput,
    TopologyChanged,
}

pub struct CompletedProbe {
    pub fields: Vec<[f32; 4]>,
    pub hits: Vec<(u32, f32)>,
    pub segments: Vec<Segment>,
    pub mix: std::sync::Arc<crate::prototype_controls::ControllerMix>,
}
impl Root {
    pub fn owner(&self) -> u32 {
        match self {
            Self::Standalone { owner, .. } | Self::Trajectory { owner, .. } => *owner,
        }
    }
    pub fn samples(&self) -> &[Sample] {
        match self {
            Self::Standalone { point, .. } => std::slice::from_ref(point),
            Self::Trajectory { children, .. } => children,
        }
    }
    fn samples_mut(&mut self) -> &mut [Sample] {
        match self {
            Self::Standalone { point, .. } => std::slice::from_mut(point),
            Self::Trajectory { children, .. } => children,
        }
    }
    pub fn trajectory(&self) -> bool {
        matches!(self, Self::Trajectory { .. })
    }
}
fn flatten_checked(roots: &[Root]) -> Result<Vec<Point>, ProbeError> {
    if roots.is_empty() {
        return Err(ProbeError::Empty);
    }
    let mut owners = std::collections::BTreeSet::new();
    let mut result = vec![];
    for root in roots {
        if !owners.insert(root.owner()) {
            return Err(ProbeError::DuplicateOwner);
        }
        if root.trajectory() && root.samples().len() < 2 {
            return Err(ProbeError::InvalidTrajectory);
        }
        for p in root.samples() {
            if p.xy.iter().chain(&p.old).any(|x| !x.is_finite()) {
                return Err(ProbeError::NonFiniteInput);
            }
            result.push(Point {
                xy: p.xy,
                old: p.old,
                owner: root.owner(),
                trajectory: root.trajectory(),
            })
        }
    }
    Ok(result)
}
pub fn segments_checked(roots: &[Root]) -> Result<Vec<Segment>, ProbeError> {
    let points = flatten_checked(roots)?;
    let mut total = 0.00001f32;
    for p in &points {
        total += p.old[0]
    }
    let mut segments = vec![];
    for root in roots {
        let mut numerator = 0.0f32;
        let vertices = match root {
            Root::Standalone { point, .. } => {
                numerator = point.old[0];
                octagon::vertices(point.xy).to_vec()
            }
            Root::Trajectory { children, .. } => {
                for p in children {
                    numerator += p.old[0]
                }
                children.iter().map(|p| p.xy).collect()
            }
        };
        let tag = numerator / total;
        for edge in vertices.windows(2) {
            segments.push(Segment {
                xy: [edge[0][0], edge[0][1], edge[1][0], edge[1][1]],
                tag,
                owner: root.owner() as u64,
            })
        }
    }
    Ok(segments)
}
pub fn refresh_checked(
    roots: &mut [Root],
    center: [f32; 2],
    radius: f64,
) -> Result<CompletedProbe, ProbeError> {
    if center.iter().any(|x| !x.is_finite()) || !radius.is_finite() || radius <= 0.0 {
        return Err(ProbeError::NonFiniteInput);
    }
    let points = flatten_checked(roots)?;
    let segments = segments_checked(roots)?;
    let tree = spatial_builder::build(&segments).map_err(ProbeError::Builder)?;
    let (v, s, hits) = visible(&points, &tree, center, radius as f32);
    let (a, b, mix) = weights(&points, &v, &s, center, radius as f32, roots.len())?;
    let result: Vec<_> = (0..points.len())
        .map(|i| [a[i], b[i], v[i], s[i]])
        .collect();
    if result.iter().flatten().any(|x| !x.is_finite()) {
        return Err(ProbeError::NonFiniteOutput);
    }
    let mut at = 0;
    for root in roots {
        for p in root.samples_mut() {
            p.old = [a[at], v[at], s[at]];
            at += 1
        }
    }
    Ok(CompletedProbe { fields: result, hits, segments, mix: std::sync::Arc::new(mix) })
}

pub fn replace_geometry_checked(roots: &mut [Root], next: &[Root]) -> Result<(), ProbeError> {
    flatten_checked(next)?;
    if roots.len() != next.len()
        || roots.iter().zip(next).any(|(a, b)| {
            a.owner() != b.owner()
                || a.trajectory() != b.trajectory()
                || a.samples().len() != b.samples().len()
        })
    {
        return Err(ProbeError::TopologyChanged);
    }
    for (root, input) in roots.iter_mut().zip(next) {
        for (p, q) in root.samples_mut().iter_mut().zip(input.samples()) {
            p.xy = q.xy
        }
    }
    Ok(())
}
