#![allow(dead_code)]
#[path = "desktop_layout_qr.rs"]
mod fit_qr;
#[path = "desktop_layout_projection.rs"]
mod projection;
#[path = "desktop_layout_trajectory.rs"]
mod trajectory;

#[derive(Clone, Debug)]
pub enum Source {
    Point {
        reduced: [f32; 4],
        enabled: bool,
    },
    Trajectory {
        reduced: Vec<[f32; 4]>,
        enabled: bool,
    },
}
#[derive(Clone, Debug)]
pub struct Statistics {
    pub offset: [f32; 4],
    pub scale: [f32; 4],
    pub covariance: [f32; 16],
}
#[derive(Clone, Debug)]
pub struct Geometry {
    pub base: [f32; 2],
    pub covariance: Option<[f32; 4]>,
    pub children: Vec<[f32; 2]>,
}

pub fn aligned_sum(values: &[f32], word_offset: usize) -> f32 {
    let prefix = ((4 - word_offset % 4) % 4).min(values.len());
    if values.len() - prefix < 4 {
        return values.iter().copied().reduce(|a, b| a + b).unwrap_or(0.0);
    }
    let packet_end = prefix + (values.len() - prefix) / 4 * 4;
    let mut a = [0.0; 4];
    a.copy_from_slice(&values[prefix..prefix + 4]);
    let mut i = prefix + 4;
    if packet_end - prefix >= 8 {
        let mut b = [0.0; 4];
        b.copy_from_slice(&values[i..i + 4]);
        i += 4;
        while i + 8 <= packet_end {
            for k in 0..4 {
                a[k] += values[i + k];
                b[k] += values[i + 4 + k];
            }
            i += 8;
        }
        for k in 0..4 {
            a[k] = b[k] + a[k];
        }
    }
    if i < packet_end {
        for k in 0..4 {
            a[k] += values[i + k];
        }
    }
    let mut sum = (a[1] + a[3]) + (a[0] + a[2]);
    for &v in &values[..prefix] {
        sum += v;
    }
    for &v in &values[packet_end..] {
        sum += v;
    }
    sum
}

pub fn flatten(sources: &[Source]) -> Vec<[f32; 4]> {
    sources
        .iter()
        .flat_map(|s| match s {
            Source::Point { reduced, .. } => std::slice::from_ref(reduced).iter(),
            Source::Trajectory { reduced, .. } => reduced.iter(),
        })
        .copied()
        .collect()
}

pub fn statistics(points: &[[f32; 4]]) -> Statistics {
    let n = points.len();
    assert!(n > 1, "Layout requires at least two points");
    let mut offset = [0.0; 4];
    let mut centered = vec![0.0; n * 4];
    for d in 0..4 {
        let column: Vec<_> = points.iter().map(|p| p[d]).collect();
        offset[d] = -aligned_sum(&column, d * n) / n as f32;
        for i in 0..n {
            centered[d * n + i] = offset[d] + points[i][d];
        }
    }
    let squares: Vec<_> = centered.iter().map(|v| v * v).collect();
    let scale =
        1.0 / (aligned_sum(&squares, 0) / n as f32 / 4.0 + f32::from_bits(0x3a83126f)).sqrt();
    for v in &mut centered {
        *v *= scale;
    }
    let mut covariance = [0.0; 16];
    for c in 0..4 {
        for r in 0..4 {
            if n + 4 + 4 <= 19 {
                let products: Vec<_> = (0..n)
                    .map(|i| centered[r * n + i] * centered[c * n + i])
                    .collect();
                covariance[c * 4 + r] = aligned_sum(&products, 0) / n as f32;
                continue;
            }
            let mut even = 0.0;
            let mut odd = 0.0;
            let end = n / 8 * 8;
            for i in (0..end).step_by(2) {
                even = centered[r * n + i] * centered[c * n + i] + even;
                odd = centered[r * n + i + 1] * centered[c * n + i + 1] + odd;
            }
            let mut sum = even + odd;
            for i in end..n {
                sum += centered[r * n + i] * centered[c * n + i];
            }
            covariance[c * 4 + r] = sum / n as f32;
        }
    }
    Statistics {
        offset,
        scale: [scale; 4],
        covariance,
    }
}

pub fn refresh_basis(stats: &Statistics, incoming: [f32; 8]) -> [f32; 8] {
    projection::basis_step(&stats.covariance, incoming)
}

pub fn project(points: &[[f32; 4]], stats: &Statistics, basis: [f32; 8]) -> Vec<[f32; 2]> {
    points
        .iter()
        .map(|p| {
            let mut out = [0.0; 2];
            for c in 0..2 {
                let mut sum = 0.0;
                for d in 0..4 {
                    sum = ((stats.offset[d] + p[d]) * stats.scale[d]) * basis[c * 4 + d] + sum;
                }
                out[c] = sum * f32::from_bits(0x3e19999a);
            }
            out
        })
        .collect()
}

pub fn summary(points: &[[f32; 2]]) -> ([f32; 2], [f32; 4]) {
    let n = points.len();
    assert!(n > 0 && n < 48);
    let mut mean = [0.0; 2];
    for d in 0..2 {
        let col: Vec<_> = points.iter().map(|p| p[d]).collect();
        mean[d] = aligned_sum(&col, d * n) / n as f32;
    }
    let mut cov = [0.0; 4];
    for c in 0..2 {
        for r in 0..2 {
            let accumulated = if n + 4 <= 19 {
                let products: Vec<_> = points
                    .iter()
                    .map(|p| (p[r] - mean[r]) * (p[c] - mean[c]))
                    .collect();
                aligned_sum(&products, 0)
            } else {

                let mut partial = 0.0_f32;
                for p in points {
                    partial = (p[r] - mean[r]) * (p[c] - mean[c]) + partial;
                }
                partial * 1.0 + 0.0
            };
            cov[c * 2 + r] = accumulated / n as f32;
        }
    }
    (mean, cov)
}

pub fn base_geometry(sources: &[Source], stats: &Statistics, basis: [f32; 8]) -> Vec<Geometry> {
    match base_geometry_with_parent_summary(sources, stats, basis, |children| {
        Ok::<_, std::convert::Infallible>(summary(children))
    }) {
        Ok(result) => result,
        Err(never) => match never {},
    }
}

pub fn base_geometry_with_parent_summary<E>(
    sources: &[Source], stats: &Statistics, basis: [f32; 8],
    mut summarize: impl FnMut(&[[f32; 2]]) -> Result<([f32; 2], [f32; 4]), E>,
) -> Result<Vec<Geometry>, E> {
    let scalar_two_points = matches!(sources, [Source::Point { .. }, Source::Point { .. }]);
    sources
        .iter()
        .map(|s| -> Result<Geometry, E> { Ok(match s {
            Source::Point { reduced, .. } => Geometry {
                base: if scalar_two_points {
                    std::array::from_fn(|c| {
                        let mut sum =
                            ((stats.offset[0] + reduced[0]) * stats.scale[0]) * basis[c * 4];
                        for d in 1..4 {
                            sum += ((stats.offset[d] + reduced[d]) * stats.scale[d])
                                * basis[c * 4 + d];
                        }
                        sum * f32::from_bits(0x3e19999a)
                    })
                } else {
                    project(&[*reduced], stats, basis)[0]
                },
                covariance: None,
                children: vec![],
            },
            Source::Trajectory { reduced, .. } => {
                let raw = project(reduced, stats, basis);
                let n = raw.len();
                assert!(n > 1);
                let mut t: Vec<_> = (0..n).map(|i| i as f32 * (1.0 / (n - 1) as f32)).collect();
                t[n - 1] = 1.0;
                let mut coefficients = [[0.0; 4]; 2];
                for d in 0..2 {
                    let y: Vec<_> = raw.iter().map(|p| p[d]).collect();
                    let (a, b) = trajectory::normal_system(&t, &y);
                    coefficients[d] = fit_qr::solve(a, b);
                }
                let curve = trajectory::Quintic {
                    start: raw[0],
                    end: raw[n - 1],
                    coefficients,
                };
                let (parameters, arc) = trajectory::arc_parameters(&curve, &t);
                let mut children: Vec<_> = parameters.iter().map(|&t| curve.evaluate(t)).collect();
                trajectory::clamp_spacing(&mut children, arc);
                let (base, cov) = summarize(&children)?;
                Geometry {
                    base,
                    covariance: Some(cov),
                    children,
                }
            }
        }) })
        .collect()
}

