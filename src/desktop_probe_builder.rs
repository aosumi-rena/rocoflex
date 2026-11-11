#[path = "desktop_probe_query.rs"]
pub mod spatial_query;
pub use spatial_query::{Node, Segment};
fn minss(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
fn maxss(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}
fn neg(x: f32) -> f32 {
    f32::from_bits(x.to_bits() ^ 0x80000000)
}
fn near_zero(x: f32) -> bool {
    if !x.is_finite() {
        return false;
    }
    let a = f32::from_bits(x.to_bits() & 0x7fffffff);
    a <= f32::MIN_POSITIVE || a * f32::EPSILON >= a
}
fn between(t: f32) -> bool {
    0. <= t && t <= 1.
}

pub fn intersects(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    if b[0] == c[0] && b[1] == c[1] {
        return true;
    }
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let ex = d[0] - c[0];
    let ey = d[1] - c[1];
    let den = dx * ey - dy * ex;
    if near_zero(den) {
        if (dx == 0. && dy == 0.) || (ex == 0. && ey == 0.) {
            return false;
        }
        if near_zero(dy) && !near_zero(ey) {
            return between((a[1] - c[1]) / ey);
        }
        if near_zero(ey) && !near_zero(dy) {
            return between((c[1] - a[1]) / dy);
        }
        if near_zero(dx) && !near_zero(ex) {
            return between((a[0] - c[0]) / ex);
        }
        if near_zero(ex) && !near_zero(dx) {
            return between((c[0] - a[0]) / dx);
        }
        return false;
    }
    let ox = a[0] - c[0];
    let oy = a[1] - c[1];
    let t = (ex * oy - ey * ox) / den;
    if !between(t) {
        return false;
    }
    let u = (oy * dx + dy * neg(ox)) / den;
    between(u)
}

pub fn rectangle_hit(b: [f32; 4], s: [f32; 4]) -> bool {
    let right = b[0] + b[2];
    let bottom = b[1] + b[3];
    let contains = |x, y| x >= b[0] && y >= b[1] && right > x && bottom > y;
    if contains(s[0], s[1]) || contains(s[2], s[3]) {
        return true;
    }
    let a = [s[0], s[1]];
    let z = [s[2], s[3]];
    intersects(a, z, [b[0], b[1]], [right, b[1]])
        || intersects(a, z, [right, b[1]], [right, bottom])
        || intersects(a, z, [right, bottom], [b[0], bottom])
        || intersects(a, z, [b[0], bottom], [b[0], b[1]])
}

pub fn bounds(v: &[Segment]) -> [f32; 4] {
    let Some(first) = v.first() else {
        return [0.; 4];
    };
    let individual = |s: &Segment| {
        let [ax, ay, bx, by] = s.xy;
        let dx = ax - bx;
        let dy = ay - by;
        [
            minss(bx, ax),
            minss(by, ay),
            maxss(neg(dx), dx),
            maxss(neg(dy), dy),
        ]
    };
    let mut b = individual(first);
    for segment in &v[1..] {
        let n = individual(segment);
        if 0. >= n[2] || 0. >= n[3] {
            continue;
        }
        if 0. >= b[2] || 0. >= b[3] {
            b = n;
            continue;
        }
        let x = minss(n[0], b[0]);
        let y = minss(n[1], b[1]);
        let w = maxss(n[2] + n[0], b[2] + b[0]) - x;
        let h = maxss(n[3] + n[1], b[3] + b[1]) - y;
        b = [x, y, w, h];
    }
    b
}
#[derive(Debug, PartialEq, Eq)]
pub enum BuildError {
    SourceRecursiveSameInput,
}

pub fn build(v: &[Segment]) -> Result<Node, BuildError> {
    let leaf = || Node {
        segments: v.to_vec(),
        boxes: [[0.; 4]; 2],
        children: [None, None],
    };
    if v.len() <= 3 {
        return Ok(leaf());
    }
    let mut split = bounds(v);
    if split[2] > split[3] {
        split[2] = maxss(split[2] * -0.5 + split[2], 0.)
    } else {
        split[3] = maxss(split[3] * -0.5 + split[3], 0.)
    }
    let mut left = vec![];
    let mut right = vec![];
    for s in v {
        if rectangle_hit(split, s.xy) {
            left.push(s.clone())
        } else {
            right.push(s.clone())
        }
    }
    if left.len() == v.len() {
        return Ok(leaf());
    }
    if right.len() == v.len() {
        return Err(BuildError::SourceRecursiveSameInput);
    }
    let boxes = [bounds(&left), bounds(&right)];
    let children = [
        Some(Box::new(build(&left)?)),
        Some(Box::new(build(&right)?)),
    ];
    Ok(Node {
        segments: vec![],
        boxes,
        children,
    })
}
