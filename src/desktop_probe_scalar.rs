include!("desktop_probe_constants.rs");
#[path = "desktop_probe_octagon.rs"]
pub mod octagon;
#[path = "desktop_probe_builder.rs"]
pub mod spatial_builder;
use spatial_builder::{Node, Segment};
#[link(name = "ucrt")]
unsafe extern "C" {
    fn expf(x: f32) -> f32;
    fn powf(x: f32, y: f32) -> f32;
}

#[derive(Clone, Debug)]
pub struct Point {
    pub xy: [f32; 2],
    pub old: [f32; 3],
    pub owner: u32,
    pub trajectory: bool,
}
fn sum(v: &[f32]) -> f32 {
    if v.len() < 4 {
        return v.iter().copied().reduce(|a, b| a + b).unwrap_or(0.0);
    }
    let mut a = [v[0], v[1], v[2], v[3]];
    let mut pos = 4;
    if v.len() >= 8 {
        let mut b = [v[4], v[5], v[6], v[7]];
        pos = 8;
        while pos + 8 <= v.len() {
            for k in 0..4 {
                a[k] += v[pos + k];
                b[k] += v[pos + 4 + k]
            }
            pos += 8
        }
        for k in 0..4 {
            a[k] = b[k] + a[k]
        }
    }
    if pos + 4 <= v.len() {
        for k in 0..4 {
            a[k] += v[pos + k]
        }
        pos += 4
    }
    let mut s = (a[1] + a[3]) + (a[0] + a[2]);
    for x in &v[pos..] {
        s += x
    }
    s
}
fn exp_packet(x: f32) -> f32 {
    let z = x.min(K28).max(K29);
    let t = z * K11 + K10;
    let q = t.trunc();
    let n = q - if t < q { 1.0 } else { 0.0 };
    let r = n * K31 + (n * K30 + z);
    let p = (((((r * K32 + K33) * r + K34) * r + K35) * r + K36) * r + K10) * (r * r) + r + 1.0;
    x.max(f32::from_bits(((n + K37) as i32 as u32) << 23) * p)
}
fn origins(center: [f32; 2], r: f32) -> [[f32; 2]; 8] {
    let [x, y] = center;
    [
        [x + r, y],
        [x + K04 * r, y + K04 * r],
        [x + K15 * r, y + r],
        [x - K04 * r, y + K04 * r],
        [x - r, y + K16 * r],
        [x + K25 * r, y + K24 * r],
        [x + K17 * r, y - r],
        [x + K27 * r, y + K26 * r],
    ]
}
fn visible(
    points: &[Point],
    tree: &Node,
    center: [f32; 2],
    r: f32,
) -> (Vec<f32>, Vec<f32>, Vec<(u32, f32)>) {
    let mut vis = vec![];
    let mut signed = vec![];
    let mut hits = vec![];
    for p in points {
        let mut blocked = 0u32;
        let mut tag_sum = 0.0f32;
        for o in origins(center, r) {
            let dx = p.xy[0] - o[0];
            let dy = p.xy[1] - o[1];
            let len = (dx * dx + dy * dy).sqrt();
            let direction = [dx / len, dy / len];
            let mut out = [len, len, 0.];
            spatial_builder::spatial_query::query(
                tree,
                [
                    o[0],
                    o[1],
                    direction[0],
                    direction[1],
                    1. / direction[0],
                    1. / direction[1],
                    len,
                ],
                p.owner as u64,
                &mut out,
            );
            let [any, other, tag] = out;
            let end = len * K05;
            if end > if p.trajectory { any } else { other } {
                blocked += 1
            }
            tag_sum += if other < end { tag } else { -0.0 };
        }
        vis.push((1.0 - blocked as f32 / 8.0) * K23 + p.old[1] * K14);
        signed.push((tag_sum / 8.0) * K23 + p.old[2] * K14);
        hits.push((blocked, tag_sum));
    }
    (vis, signed, hits)
}
fn centroid(points: &[Point], w: &[f32]) -> [f32; 2] {
    let mut result = [0.0; 2];
    for axis in 0..2 {
        let mut a = [0.0f32; 4];
        let mut at = 0;
        while at + 4 <= points.len() {
            for k in 0..4 {
                a[k] += points[at + k].xy[axis] * w[at + k]
            }
            at += 4
        }
        let mut s = (a[1] + a[3]) + (a[0] + a[2]);
        for i in at..points.len() {
            s += points[i].xy[axis] * w[i]
        }
        result[axis] = s;
    }
    result
}
fn weights(
    points: &[Point],
    v: &[f32],
    s: &[f32],
    probe: [f32; 2],
    r: f32,
    source_count: usize,
) -> Result<(Vec<f32>, Vec<f32>, crate::prototype_controls::ControllerMix), ProbeError> {
    let d: Vec<_> = points
        .iter()
        .map(|p| {
            let x = p.xy[0] - probe[0];
            let y = p.xy[1] - probe[1];
            unsafe { powf(x * x + y * y, K18) }
        })
        .collect();
    let lo = d.iter().copied().reduce(f32::min).unwrap();
    let hi = d.iter().copied().reduce(f32::max).unwrap();
    let mut a: Vec<_> = d
        .iter()
        .zip(v)
        .enumerate()
        .map(|(i, (d, v))| {
            let x = (lo - d) / r;
            (if i / 4 < points.len() / 4 {
                exp_packet(x)
            } else {
                unsafe { expf(x) }
            }) * v
        })
        .collect();
    let mut b: Vec<_> = d
        .iter()
        .zip(s)
        .map(|(d, s)| unsafe { expf(((d - hi) / r) * K08) } * (s * s))
        .collect();
    let sa = sum(&a) + 0.00001f32;
    let sb = sum(&b) + 1.0;
    for (a, b) in a.iter_mut().zip(&mut b) {
        *a /= sa;
        *b /= sb
    }
    let ma = a.iter().copied().reduce(f32::max).unwrap();
    let mb = b.iter().copied().reduce(f32::max).unwrap();
    for x in &mut a {
        *x = (((((*x / ma) - 0.05f32) / 0.35f32).max(0.0).min(1.0) as f64 + 0.00001f64).sqrt()
            * ma as f64) as f32;
    }
    let sa = sum(&a) + 0.00001f32;
    for x in &mut a {
        *x /= sa
    }
    let final_ma = a.iter().copied().reduce(f32::max).unwrap();
    let ca = centroid(points, &a);
    let cb = centroid(points, &b);
    let ax = ca[0] - cb[0];
    let ay = ca[1] - cb[1];
    let px = probe[0] - cb[0];
    let py = probe[1] - cb[1];
    let k = (((px * ax + py * ay + 0.1f32) / (ax * ax + ay * ay + 0.1f32)) - 1.0)
        .max(0.0)
        .min(1.0);
    let invisible: Vec<_> = v.iter().map(|v| 1.0 - v).collect();
    let sv = sum(&invisible);
    let strength = ((k * k * sv) / (sv + 4.0)) * sum(&b);
    let scale = strength as f64 / (mb as f64 + 0.00001f64);
    let point_weights: Vec<_> = a.iter().zip(&b).map(|(&a, &b)| [a, b]).collect();
    let mut trajectory_blends = Vec::with_capacity(source_count);
    let mut first = 0;
    while first < points.len() {
        let owner = points[first].owner;
        let mut end = first + 1;
        while end < points.len() && points[end].owner == owner { end += 1; }
        trajectory_blends.push(if points[first].trajectory {
            Some(crate::prototype_controls::trajectory_blend(&point_weights[first..end])
                .map_err(|_| ProbeError::NonFiniteOutput)?)
        } else { None });
        first = end;
    }
    let mix = crate::prototype_controls::ControllerMix {
        point_weights,
        positive_scale: f64::from(strength + 1.0_f32),
        negative_scale: f64::from(strength),
        trajectory_blends,
    };
    for (a, b) in a.iter_mut().zip(&mut b) {
        *a /= final_ma;
        *b = (*b as f64 * scale) as f32
    }
    Ok((a, b, mix))
}
