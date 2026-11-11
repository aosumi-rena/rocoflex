use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, b: Self) -> f64 {
        (self.x - b.x).hypot(self.y - b.y)
    }
    fn toward(self, b: Self, t: f64) -> Self {
        Self::new(self.x + (b.x - self.x) * t, self.y + (b.y - self.y) * t)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Curve {
    pub nodes: Vec<Point>,
    pub path: Vec<Point>,
    pub color: [u8; 3],
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Anchor {
    pub position: Point,
    pub color: [u8; 3],
    pub curve: Option<usize>,
    pub index: usize,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Probe {
    pub position: Point,
    pub radius: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct Ray {
    pub source: usize,
    pub distance: f64,
    pub edge_distance: f64,
    pub strength: f64,
    pub shadow_strength: f64,
    pub shadow_depth: f64,
    pub blocked: bool,
}
#[derive(Clone, Copy)]
struct Wall {
    a: Point,
    b: Point,
    curve: usize,
}
#[derive(Default)]
pub struct Scene {
    pub curves: Vec<Curve>,
    pub anchors: Vec<Anchor>,
    walls: Vec<Wall>,
    shadow_walls: Vec<Wall>,
}
pub fn connection_reach(radius: f64) -> f64 {
    0.25 + radius * 6.4
}
fn cross(a: Point, b: Point) -> f64 {
    a.x * b.y - a.y * b.x
}
fn projection(a: Point, b: Point, p: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let d = dx * dx + dy * dy;
    if d <= 1e-9 {
        0.0
    } else {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / d
    }
}
pub fn point_segment(p: Point, a: Point, b: Point) -> f64 {
    p.distance(a.toward(b, projection(a, b, p).clamp(0.0, 1.0)))
}
fn orientation(a: Point, b: Point, c: Point) -> i8 {
    let v = (b.y - a.y) * (c.x - b.x) - (b.x - a.x) * (c.y - b.y);
    if v.abs() <= 1e-9 {
        0
    } else if v > 0.0 {
        1
    } else {
        -1
    }
}
fn on_segment(p: Point, a: Point, b: Point) -> bool {
    p.x <= a.x.max(b.x) + 1e-9
        && p.x >= a.x.min(b.x) - 1e-9
        && p.y <= a.y.max(b.y) + 1e-9
        && p.y >= a.y.min(b.y) - 1e-9
}
pub fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (o1, o2, o3, o4) = (
        orientation(a, b, c),
        orientation(a, b, d),
        orientation(c, d, a),
        orientation(c, d, b),
    );
    (o1 != o2 && o3 != o4)
        || (o1 == 0 && on_segment(c, a, b))
        || (o2 == 0 && on_segment(d, a, b))
        || (o3 == 0 && on_segment(a, c, d))
        || (o4 == 0 && on_segment(b, c, d))
}
fn segment_distance(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if segments_intersect(a, b, c, d) {
        0.0
    } else {
        point_segment(a, c, d)
            .min(point_segment(b, c, d))
            .min(point_segment(c, a, b))
            .min(point_segment(d, a, b))
    }
}
fn ray_segment(origin: Point, dir: Point, a: Point, b: Point) -> Option<f64> {
    let s = Point::new(b.x - a.x, b.y - a.y);
    let denominator = cross(dir, s);
    if denominator.abs() <= 1e-9 {
        return None;
    }
    let q = Point::new(a.x - origin.x, a.y - origin.y);
    let t = cross(q, s) / denominator;
    let u = cross(q, dir) / denominator;
    if t >= 0.0 && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}
fn ray_circle(origin: Point, dir: Point, center: Point, radius: f64) -> Option<f64> {
    let o = Point::new(origin.x - center.x, origin.y - center.y);
    let b = 2.0 * (o.x * dir.x + o.y * dir.y);
    let c = o.x * o.x + o.y * o.y - radius * radius;
    let d = b * b - 4.0 * c;
    if d < 0.0 {
        return None;
    }
    let a = (-b - d.sqrt()) / 2.0;
    let z = (-b + d.sqrt()) / 2.0;
    if a >= 0.0 {
        Some(a)
    } else if z >= 0.0 {
        Some(z)
    } else {
        None
    }
}
impl Scene {
    pub fn new(curves: Vec<Curve>, points: Vec<Anchor>) -> Self {
        let mut scene = Self {
            curves,
            ..Default::default()
        };
        for (i, curve) in scene.curves.iter().enumerate() {
            scene.anchors.extend(
                curve
                    .nodes
                    .iter()
                    .enumerate()
                    .map(|(index, &position)| Anchor {
                        position,
                        color: curve.color,
                        curve: Some(i),
                        index,
                    }),
            );
            scene.walls.extend(curve.path.windows(2).map(|v| Wall {
                a: v[0],
                b: v[1],
                curve: i,
            }));
            let mut coarse = Vec::new();
            if curve.path.len() <= 36 {
                coarse.extend_from_slice(&curve.path);
            } else {
                let step = (curve.path.len() - 1).div_ceil(35);
                coarse.extend(curve.path.iter().step_by(step).copied());
                if coarse.last() != curve.path.last() {
                    coarse.push(*curve.path.last().unwrap());
                }
            }
            scene.shadow_walls.extend(coarse.windows(2).map(|v| Wall {
                a: v[0],
                b: v[1],
                curve: i,
            }));
        }
        scene.anchors.extend(points);
        scene
    }
    fn clearance(&self, source: &Anchor) -> f64 {
        let Some(i) = source.curve else {
            return 0.018;
        };
        let Some(nodes) = self.curves.get(i).map(|v| &v.nodes) else {
            return 0.036;
        };
        let Some(&at) = nodes.get(source.index) else {
            return 0.036;
        };
        let a = source
            .index
            .checked_sub(1)
            .and_then(|j| nodes.get(j))
            .map_or(f64::INFINITY, |&v| at.distance(v));
        let b = nodes
            .get(source.index + 1)
            .map_or(f64::INFINITY, |&v| at.distance(v));
        let gap = a.min(b);
        if gap.is_finite() {
            (gap * 0.68).clamp(0.034, 0.085)
        } else {
            0.042
        }
    }
    fn blocked(&self, source_id: usize, position: Point, origin: Point, probe: Probe) -> bool {
        let source = &self.anchors[source_id];
        let clearance = self.clearance(source);
        for wall in &self.walls {
            let own = source.curve == Some(wall.curve);
            if point_segment(position, wall.a, wall.b) <= if own { clearance } else { 0.018 }
                || point_segment(probe.position, wall.a, wall.b) <= probe.radius + 0.014
            {
                continue;
            }
            let mid = wall.a.toward(wall.b, 0.5);
            let t = projection(origin, position, mid);
            if t <= 0.015 || t >= 0.985 {
                continue;
            }

            let pad = 0.020000001;
            if origin.x.max(position.x) + pad < wall.a.x.min(wall.b.x)
                || origin.x.min(position.x) - pad > wall.a.x.max(wall.b.x)
                || origin.y.max(position.y) + pad < wall.a.y.min(wall.b.y)
                || origin.y.min(position.y) - pad > wall.a.y.max(wall.b.y)
            {
                continue;
            }
            if segment_distance(origin, position, wall.a, wall.b) <= 0.02
                || segments_intersect(origin, position, wall.a, wall.b)
            {
                return true;
            }
        }
        for (i, blocker) in self.anchors.iter().enumerate() {
            if i == source_id {
                continue;
            }
            if source.curve.is_some()
                && source.curve == blocker.curve
                && source.index.abs_diff(blocker.index) <= 1
            {
                continue;
            }
            if blocker.position.distance(probe.position) <= probe.radius + 0.034 {
                continue;
            }
            let t = projection(origin, position, blocker.position);
            if t <= 0.04 || t >= 0.945 {
                continue;
            }
            if point_segment(blocker.position, origin, position) <= 0.034 {
                return true;
            }
        }
        false
    }
    pub fn visible(&self, source: usize, position: Point, probe: Probe) -> bool {
        let distance = position.distance(probe.position);
        if distance <= probe.radius + 1e-6 {
            return true;
        }
        let (dx, dy) = if distance <= 1e-6 {
            (1.0, 0.0)
        } else {
            (
                (position.x - probe.position.x) / distance,
                (position.y - probe.position.y) / distance,
            )
        };
        for offset in [0.0_f64, -0.42, 0.42, -0.78, 0.78] {
            let forward = (1.0 - offset * offset).max(0.0).sqrt();
            let origin = Point::new(
                probe.position.x + (dx * forward - dy * offset) * probe.radius,
                probe.position.y + (dy * forward + dx * offset) * probe.radius,
            );
            if !self.blocked(source, position, origin, probe) {
                return true;
            }
        }
        false
    }
    pub fn shadow_depth(&self, source: usize, probe: Probe) -> f64 {
        let position = self.anchors[source].position;
        let distance = position.distance(probe.position);
        let edge = (distance - probe.radius).max(0.0);
        if edge <= 1e-6 {
            return 0.0;
        }
        let count = ((edge / 0.032).ceil() as usize).clamp(8, 28);
        let mut previous = 0.0;
        for i in 1..=count {
            let d = edge * i as f64 / count as f64;
            if self.visible(source, position.toward(probe.position, d / distance), probe) {
                let (mut low, mut high) = (previous, d);
                for _ in 0..4 {
                    let mid = (low + high) / 2.0;
                    if self.visible(
                        source,
                        position.toward(probe.position, mid / distance),
                        probe,
                    ) {
                        high = mid;
                    } else {
                        low = mid;
                    }
                }
                return high;
            }
            previous = d;
        }
        edge
    }
    pub fn rays(&self, probe: Probe) -> Vec<Ray> {
        let mut rays = Vec::new();
        let reach = connection_reach(probe.radius);
        for (i, source) in self.anchors.iter().enumerate() {
            let distance = source.position.distance(probe.position);
            let edge = (distance - probe.radius).max(0.0);
            let blocked = !self.visible(i, source.position, probe);
            let strength = if blocked || edge > reach {
                0.0
            } else if source.curve.is_some() {
                0.92
            } else {
                1.0
            };
            let depth = if blocked {
                self.shadow_depth(i, probe)
            } else {
                0.0
            };
            let shadow = if blocked && depth >= 0.11 { 1.0 } else { 0.0 };
            if strength < 0.0025 && shadow < 0.004 {
                continue;
            }
            rays.push(Ray {
                source: i,
                distance,
                edge_distance: edge,
                strength,
                shadow_strength: shadow,
                shadow_depth: depth,
                blocked,
            });
        }
        rays.sort_by(|a, b| b.strength.total_cmp(&a.strength));
        rays.truncate(320);
        rays
    }
    pub fn shadow_fan(&self, probe: Probe, scale: f64) -> Vec<[Point; 4]> {
        let length = (4.0 / scale).hypot(2.6 / scale) * 1.45;
        let mut samples = Vec::with_capacity(129);
        for i in 0..=128 {
            let angle = i as f64 / 128.0 * std::f64::consts::TAU;
            let dir = Point::new(angle.cos(), angle.sin());
            let origin = Point::new(
                probe.position.x + dir.x * probe.radius,
                probe.position.y + dir.y * probe.radius,
            );
            let mut nearest = length + 1.0;
            for wall in &self.shadow_walls {
                if point_segment(probe.position, wall.a, wall.b) <= probe.radius + 0.014 {
                    continue;
                }
                if let Some(d) = ray_segment(origin, dir, wall.a, wall.b) {
                    if d > 0.003 && d < nearest {
                        nearest = d;
                    }
                }
            }
            for point in self.anchors.iter().filter(|a| a.curve.is_none()) {
                if point.position.distance(probe.position) <= probe.radius + 0.034 {
                    continue;
                }
                if let Some(d) = ray_circle(origin, dir, point.position, 0.038) {
                    if d > 0.003 && d < nearest {
                        nearest = d;
                    }
                }
            }
            let at = |d: f64| Point::new(origin.x + dir.x * d, origin.y + dir.y * d);
            samples.push((
                if nearest <= length {
                    Some(at(nearest))
                } else {
                    None
                },
                at(length),
            ));
        }
        samples
            .windows(2)
            .filter_map(|v| Some([v[0].0?, v[1].0?, v[1].1, v[0].1]))
            .collect()
    }
}
