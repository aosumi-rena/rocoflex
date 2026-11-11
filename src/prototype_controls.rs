use std::collections::BTreeMap;

use crate::{canvas::{Point, Probe}, preset::Preset};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Parameters {
    pub pregain_db: f32,
    pub pitch_semitones: f32,

    pub tone_db: f32,
}

impl Parameters {
    fn checked(self) -> Result<Self, String> {
        if ![self.pregain_db, self.pitch_semitones, self.tone_db].iter().all(|v| v.is_finite()) {
            return Err("Morph controls must be finite.".into());
        }
        Ok(Self {
            pregain_db: self.pregain_db.clamp(-36.0, 36.0),
            pitch_semitones: self.pitch_semitones.clamp(-24.0, 24.0),
            tone_db: self.tone_db.clamp(-12.0, 12.0),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetId {
    pub source: usize,
    pub point: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetWeight {
    pub target: TargetId,
    pub weight: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceWeight {
    pub source: usize,
    pub weight: f32,
}

#[derive(Clone, Debug)]
pub struct Snapshot {

    pub targets: Vec<TargetWeight>,
    pub embedding: [f32; 128],
    pub parameters: Parameters,
}

#[derive(Clone, Debug)]
pub struct ControllerMix {
    pub point_weights: Vec<[f32; 2]>,

    pub positive_scale: f64,

    pub negative_scale: f64,

    pub trajectory_blends: Vec<Option<f32>>,
}

pub(crate) fn trajectory_blend(weights: &[[f32; 2]]) -> Result<f32, String> {
    let count = i32::try_from(weights.len())
        .map_err(|_| "Controller trajectory exceeds the signed point-count range.".to_owned())?;
    if weights.iter().flatten().any(|value| !value.is_finite()) {
        return Err("Invalid captured trajectory audio weights.".into());
    }

    let mut sums = [1.0e-7_f32; 2];
    for point in weights {
        sums[0] += point[0];
        sums[1] += point[1];
    }
    let mut entropy = [0.0_f32; 2];
    for point in weights {
        for axis in 0..2 {
            let probability = (f64::from(point[axis]) + 1.0e-7_f64) / f64::from(sums[axis]);
            let argument = f64::from(count) * probability + 1.0e-7_f64;
            entropy[axis] = (argument.ln() * probability + f64::from(entropy[axis])) as f32;
        }
    }
    let weighted = entropy[0] * sums[0] + entropy[1] * sums[1];
    let blend = (-weighted / (sums[0] + sums[1])).exp();
    if !blend.is_finite() {
        return Err("Controller trajectory audio blend is non-finite.".into());
    }
    Ok(blend)
}

impl Snapshot {

    pub fn source_weights(&self) -> Vec<SourceWeight> {
        let mut weights = BTreeMap::<usize, f64>::new();
        for item in &self.targets {
            *weights.entry(item.target.source).or_default() += f64::from(item.weight);
        }
        weights.into_iter().map(|(source, weight)| SourceWeight { source, weight: weight as f32 }).collect()
    }

}

pub fn from_preset(preset: &Preset, probe: Probe, parameters: Parameters) -> Result<Snapshot, String> {
    from_canvas(preset, &preset.basis_points, &[], probe, parameters)
}

pub fn from_controller(
    _preset: &Preset,
    _scalars: &[Vec<[f32; 4]>],
    _parameters: Parameters,
) -> Result<Snapshot, String> {
    Err("Controller audio conditioning is unavailable: final display weights do not retain the original signed voice mix.".into())
}

pub fn from_controller_mix(
    preset: &Preset,
    mix: &ControllerMix,
    parameters: Parameters,
) -> Result<Snapshot, String> {
    let parameters = parameters.checked()?;
    let point_count: usize = preset.sources.iter().map(|source| source.points.len()).sum();
    if point_count == 0 { return Err("No completed controller voice mix is available.".into()); }
    if mix.point_weights.len() != point_count
        || mix.trajectory_blends.len() != preset.sources.len() {
        return Err("Controller audio mix and preset shapes do not agree.".into());
    }
    if !mix.positive_scale.is_finite() || !mix.negative_scale.is_finite()
        || mix.point_weights.iter().flatten().any(|value| !value.is_finite()) {
        return Err("Invalid captured controller audio weights.".into());
    }
    let mut embedding = [0.0_f32; 128];
    let mut targets = Vec::with_capacity(point_count);
    let mut cursor = 0;
    for (source_index, source) in preset.sources.iter().enumerate() {
        let blend = if source.kind == "trajectory" {
            let blend = mix.trajectory_blends[source_index]
                .ok_or_else(|| "Controller trajectory audio blend is unavailable.".to_owned())?;
            if !blend.is_finite() || source.data.len() != 128
                || source.data.iter().any(|value| !value.is_finite()) {
                return Err("Invalid captured trajectory audio blend or global embedding.".into());
            }
            Some(blend)
        } else {
            None
        };
        for (point_index, point) in source.points.iter().enumerate() {
            if point.data.len() != 128 || point.data.iter().any(|value| !value.is_finite()) {
                return Err("Invalid current point embedding.".into());
            }
            let [a, b] = mix.point_weights[cursor];
            cursor += 1;
            let weight = (f64::from(a) * mix.positive_scale
                - f64::from(b) * mix.negative_scale) as f32;
            if !weight.is_finite() { return Err("Controller audio contribution exceeds float range.".into()); }
            for lane in 0..128 {
                let value = match blend {
                    Some(t) => point.data[lane] * (1.0_f32 - t) + source.data[lane] * t,
                    None => point.data[lane],
                };
                embedding[lane] += value * weight;
            }
            targets.push(TargetWeight { target: TargetId { source: source_index, point: point_index }, weight });
        }
    }
    if embedding.iter().any(|value| !value.is_finite()) {
        return Err("Controller audio mix exceeds float range.".into());
    }
    Ok(Snapshot { targets, embedding, parameters })
}

#[derive(Clone, Copy)]
struct Location {
    source: usize,
    a: usize,
    b: usize,
    fraction: f64,
    distance_squared: f64,
}

fn distance_squared(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

pub fn from_canvas(
    preset: &Preset,
    positions: &[Vec<[f32; 2]>],
    offsets: &[Point],
    probe: Probe,
    parameters: Parameters,
) -> Result<Snapshot, String> {
    let parameters = parameters.checked()?;
    if !probe.position.x.is_finite() || !probe.position.y.is_finite()
        || !probe.radius.is_finite() || probe.radius < 0.0 {
        return Err("Invalid morph probe position or radius.".into());
    }
    if positions.len() != preset.sources.len()
        || (!offsets.is_empty() && offsets.len() != preset.sources.len()) {
        return Err("Canvas and preset target counts do not agree.".into());
    }

    let mut locations = Vec::new();
    for (source_index, source) in preset.sources.iter().enumerate() {
        let points = &positions[source_index];
        if points.len() != source.points.len() {
            return Err("Canvas and preset point counts do not agree.".into());
        }
        if !source.enabled { continue; }
        let offset = offsets.get(source_index).copied().unwrap_or_default();
        if !offset.x.is_finite() || !offset.y.is_finite() { return Err("Invalid source offset.".into()); }
        let position = |index: usize| Point::new(
            f64::from(points[index][0]) + offset.x,
            f64::from(points[index][1]) + offset.y,
        );
        let mut nearest: Option<Location> = None;
        for (index, point) in source.points.iter().enumerate() {
            if !point.enabled { continue; }
            if point.data.len() != 128 || point.data.iter().any(|v| !v.is_finite()) {
                return Err(format!("Invalid timbre embedding in {}.", source.name));
            }
            let at = position(index);
            if !at.x.is_finite() || !at.y.is_finite() { return Err("Invalid canvas target position.".into()); }
            let distance_squared = distance_squared(at, probe.position);
            if nearest.is_none_or(|prior| distance_squared < prior.distance_squared) {
                nearest = Some(Location { source: source_index, a: index, b: index,
                    fraction: 0.0, distance_squared });
            }
        }
        if source.kind == "trajectory" {
            for index in 0..source.points.len().saturating_sub(1) {
                if !source.points[index].enabled || !source.points[index + 1].enabled { continue; }
                let a = position(index);
                let b = position(index + 1);
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let extent = dx * dx + dy * dy;
                if extent <= 1e-20 { continue; }
                let fraction = (((probe.position.x - a.x) * dx + (probe.position.y - a.y) * dy) / extent).clamp(0.0, 1.0);
                let projected = Point::new(a.x + fraction * dx, a.y + fraction * dy);
                let distance_squared = distance_squared(projected, probe.position);
                if nearest.is_none_or(|prior| distance_squared < prior.distance_squared) {
                    nearest = Some(Location { source: source_index, a: index, b: index + 1, fraction, distance_squared });
                }
            }
        }
        if let Some(location) = nearest { locations.push(location); }
    }
    if locations.is_empty() { return Err("Enable at least one voice target.".into()); }

    let softening = (probe.radius * 0.25).max(0.005);
    let softening_squared = softening * softening;
    let log_weights: Vec<_> = locations.iter().map(|location|
        -2.0 * (location.distance_squared + softening_squared).ln()).collect();
    let largest = log_weights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let weights: Vec<_> = log_weights.iter().map(|weight| (weight - largest).exp()).collect();
    let total: f64 = weights.iter().sum();
    if !total.is_finite() || total <= 0.0 { return Err("Morph coordinates are outside the supported range.".into()); }

    let mut targets = Vec::with_capacity(locations.len() * 2);
    for (location, weight) in locations.iter().zip(weights) {
        let weight = weight / total;
        let a = weight * (1.0 - location.fraction);
        let b = weight * location.fraction;
        if a as f32 > 0.0 { targets.push(TargetWeight { target: TargetId { source: location.source, point: location.a }, weight: a as f32 }); }
        if b as f32 > 0.0 { targets.push(TargetWeight { target: TargetId { source: location.source, point: location.b }, weight: b as f32 }); }
    }
    assemble(preset, targets, parameters)
}

fn assemble(preset: &Preset, mut targets: Vec<TargetWeight>, parameters: Parameters) -> Result<Snapshot, String> {
    let normalization: f64 = targets.iter().map(|target| f64::from(target.weight)).sum();
    if !normalization.is_finite() || normalization <= 0.0 {
        return Err("Morph selection has no usable target weights.".into());
    }
    let mut embedding = [0.0_f64; 128];
    for target in &mut targets {
        target.weight = (f64::from(target.weight) / normalization) as f32;
        let data = &preset.sources[target.target.source].points[target.target.point].data;
        if data.len() != 128 || data.iter().any(|value| !value.is_finite()) {
            return Err("Invalid current voice embedding.".into());
        }
        for (output, value) in embedding.iter_mut().zip(data) { *output += f64::from(target.weight) * f64::from(*value); }
    }
    let embedding = embedding.map(|value| value as f32);
    if embedding.iter().any(|value| !value.is_finite()) { return Err("Morph timbre blend exceeds float range.".into()); }
    Ok(Snapshot { targets, embedding, parameters })
}
