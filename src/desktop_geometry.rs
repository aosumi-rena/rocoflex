#[path = "desktop_geometry_qr.rs"]
mod qr;

pub fn single_trajectory(points: &[[f32; 4]], incoming: [f32; 8]) -> Option<Vec<[f32; 2]>> {
    let n = points.len();

    if n < 4
        || n > 512
        || n % 4 != 0
        || points
            .iter()
            .flatten()
            .chain(incoming.iter())
            .any(|v| !v.is_finite())
    {
        return None;
    }
    let mut normalized = vec![0.0_f32; n * 4];
    for d in 0..4 {
        let column: Vec<_> = points.iter().map(|point| point[d]).collect();
        let offset = -packet_sum(&column) / n as f32;
        for i in 0..n {
            normalized[d * n + i] = points[i][d] + offset;
        }
    }
    let squares: Vec<_> = normalized.iter().map(|v| v * v).collect();
    let scale = 1.0 / (packet_sum(&squares) / n as f32 / 4.0 + f32::from_bits(0x3a83126f)).sqrt();
    for value in &mut normalized {
        *value *= scale;
    }
    let mut covariance = [0.0_f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let products: Vec<_> = (0..n)
                .map(|i| normalized[row * n + i] * normalized[col * n + i])
                .collect();
            let prefix = n / 8 * 8;
            let (mut even, mut odd) = (0.0_f32, 0.0_f32);
            for i in (0..prefix).step_by(2) {
                even = products[i] + even;
                odd = products[i + 1] + odd;
            }
            let mut total = even + odd;
            for product in &products[prefix..] {
                total += product;
            }
            covariance[col * 4 + row] = total / n as f32;
        }
    }
    let basis = basis_step(&covariance, incoming)?;
    let mut raw = vec![[0.0_f32; 2]; n];
    for i in 0..n {
        for col in 0..2 {
            raw[i][col] = (0..4)
                .map(|d| normalized[d * n + i] * basis[col * 4 + d])
                .reduce(|a, b| a + b)?
                * f32::from_bits(0x3e19999a);
        }
    }
    let mut parameters: Vec<_> = (0..n).map(|i| i as f32 * (1.0 / (n - 1) as f32)).collect();
    parameters[n - 1] = 1.0;
    let mut coefficients = [[0.0_f32; 4]; 2];
    for axis in 0..2 {
        let values: Vec<_> = raw.iter().map(|point| point[axis]).collect();
        let (matrix, rhs) = normal_system(&parameters, &values);
        coefficients[axis] = qr::solve(matrix, rhs);
    }
    let curve = Quintic {
        start: raw[0],
        end: raw[n - 1],
        coefficients,
    };
    let (resampled, arc_length) = arc_parameters(&curve, &parameters);
    if !arc_length.is_finite() || arc_length <= f32::MIN_POSITIVE {
        return None;
    }
    let mut base: Vec<_> = resampled.into_iter().map(|t| curve.evaluate(t)).collect();
    clamp_spacing(&mut base, arc_length);
    let (offsets, parent) = single_source_step(&base);
    let nodes: Vec<_> = base
        .into_iter()
        .zip(offsets)
        .map(|(point, offset)| {
            [
                (point[0] + offset[0]) + parent[0],
                (point[1] + offset[1]) + parent[1],
            ]
        })
        .collect();
    nodes
        .iter()
        .flatten()
        .all(|v| v.is_finite())
        .then_some(nodes)
}

fn packet_sum(values: &[f32]) -> f32 {
    if values.len() < 4 {
        return values.iter().copied().reduce(|a, b| a + b).unwrap_or(0.0);
    }
    let count = values.len() / 4 * 4;
    let mut a = [values[0], values[1], values[2], values[3]];
    if count >= 8 {
        let mut b = [values[4], values[5], values[6], values[7]];
        let mut i = 8;
        while i + 8 <= count {
            for lane in 0..4 {
                a[lane] += values[i + lane];
                b[lane] += values[i + 4 + lane];
            }
            i += 8;
        }
        for lane in 0..4 {
            a[lane] = b[lane] + a[lane];
        }
        if i < count {
            for lane in 0..4 {
                a[lane] += values[i + lane];
            }
        }
    }
    let mut sum = (a[1] + a[3]) + (a[0] + a[2]);
    for value in &values[count..] {
        sum += value;
    }
    sum
}

fn multiply(matrix: &[f32; 16], vector: &[f32]) -> [f32; 4] {
    let mut result = [0.0_f32; 4];
    for c in 0..4 {
        for r in 0..4 {
            result[r] = matrix[c * 4 + r] * vector[c] + result[r];
        }
    }
    result
}

fn basis_step(covariance: &[f32; 16], incoming: [f32; 8]) -> Option<[f32; 8]> {
    let update = |old: &[f32], matrix: &[f32; 16]| -> Option<[f32; 4]> {
        let product = multiply(matrix, old);
        let mut blended = [0.0_f32; 4];
        for i in 0..4 {
            blended[i] = 0.5 * old[i] + 0.5 * product[i];
        }
        let squares: Vec<_> = blended.iter().map(|v| v * v).collect();
        let length = packet_sum(&squares).sqrt();
        if !length.is_finite() || length <= f32::MIN_POSITIVE {
            return None;
        }
        for v in &mut blended {
            *v /= length;
        }
        Some(blended)
    };
    let mut result = incoming;
    result[..4].copy_from_slice(&update(&incoming[..4], covariance)?);
    let product = multiply(covariance, &result[..4]);
    let mut deflated = *covariance;
    for c in 0..4 {
        for r in 0..4 {
            deflated[c * 4 + r] -= result[c] * product[r];
        }
    }
    result[4..].copy_from_slice(&update(&incoming[4..], &deflated)?);
    Some(result)
}

fn normal_system(parameters: &[f32], values: &[f32]) -> ([[f32; 4]; 4], [f32; 4]) {
    let start = values[0];
    let end = values[values.len() - 1];
    let mut matrix = [[0.0_f32; 4]; 4];
    let mut rhs = [0.0_f32; 4];
    for i in 0..4 {
        matrix[i][i] = f32::from_bits(0x3cf5c28f);
    }
    for (&t, &y) in parameters.iter().zip(values) {
        for r in 0..4 {
            let row = t.powf((r + 2) as f32) - t;
            let mut power = t;
            for c in 0..4 {
                power *= t;
                matrix[c][r] = (power - t) * row + matrix[c][r];
            }
            let residual = -(end - start) * t + (y - start);
            rhs[r] = residual * row + rhs[r];
        }
    }
    (matrix, rhs)
}

struct Quintic {
    start: [f32; 2],
    end: [f32; 2],
    coefficients: [[f32; 4]; 2],
}

impl Quintic {
    fn evaluate(&self, t: f32) -> [f32; 2] {
        let mut result = self.start;
        for d in 0..2 {
            let c = self.coefficients[d];
            let sum = (c[1] + c[3]) + (c[0] + c[2]);
            let slope = (self.end[d] - self.start[d]) - sum;
            result[d] += slope * t;
            let mut power = t;
            for coefficient in c {
                power = t * power;
                result[d] = coefficient * power + result[d];
            }
        }
        result
    }
}

fn arc_parameters(curve: &Quintic, requested: &[f32]) -> (Vec<f32>, f32) {
    const COUNT: usize = 500;
    let mut parameters = [0.0_f32; COUNT];
    for (i, value) in parameters.iter_mut().enumerate() {
        *value = i as f32 * (1.0 / (COUNT - 1) as f32);
    }
    parameters[COUNT - 1] = 1.0;
    let mut cumulative = [0.0_f32; COUNT];
    let mut previous = curve.evaluate(parameters[0]);
    for i in 1..COUNT {
        let current = curve.evaluate(parameters[i]);
        let dx = current[0] - previous[0];
        let dy = current[1] - previous[1];
        cumulative[i] = cumulative[i - 1] + (dx * dx + dy * dy).sqrt();
        previous = current;
    }
    let total = cumulative[COUNT - 1];
    let mut j = 0;
    let mut result = Vec::with_capacity(requested.len());
    for &t in requested {
        let distance = t * total;
        while j < COUNT - 2 && distance > cumulative[j + 1] {
            j += 1;
        }

        let fraction =
            ((distance - cumulative[j]) / cumulative[j + 1] - cumulative[j]).clamp(0.0, 1.0);
        result.push((parameters[j + 1] - parameters[j]) * fraction + parameters[j]);
    }
    (result, total)
}

fn clamp_spacing(points: &mut [[f32; 2]], arc_length: f32) {
    let spacing = arc_length / points.len() as f32;
    let scale = if spacing < 0.06 {
        0.06 / spacing
    } else if spacing > 0.1 {
        0.1 / spacing
    } else {
        return;
    };
    let mut mean = [0.0_f32; 2];
    for point in points.iter() {
        mean[0] += point[0];
        mean[1] += point[1];
    }
    for value in &mut mean {
        *value /= points.len() as f32;
    }
    for point in points {
        for d in 0..2 {
            point[d] = (point[d] - mean[d]) * scale + mean[d];
        }
    }
}

fn single_source_step(base: &[[f32; 2]]) -> (Vec<[f32; 2]>, [f32; 2]) {
    let mut force = vec![[0.0_f32; 2]; base.len()];
    let mut parent_force = [0.0_f32; 2];
    for i in 0..base.len() {
        for j in i + 1..base.len() {
            let dx = base[i][0] - base[j][0];
            let dy = base[i][1] - base[j][1];
            let strength = 0.001 / ((dx * dx + dy * dy) + 1.0e-5);
            let fx = dx * strength;
            let fy = strength * dy;
            let weighted = [0.3 * fx, 0.3 * fy];
            for axis in 0..2 {
                force[i][axis] += weighted[axis];
                force[j][axis] -= weighted[axis];
            }
            parent_force[0] = fx * 0.1 + parent_force[0];
            parent_force[1] = fy * 0.1 + parent_force[1];
            parent_force[0] -= fx * 0.1;
            parent_force[1] -= fy * 0.1;
        }
    }
    let mut mean = [0.0_f32; 2];
    for point in &mut force {
        for axis in 0..2 {
            point[axis] *= 0.8;
            mean[axis] += point[axis];
        }
    }
    for axis in 0..2 {
        mean[axis] /= base.len() as f32;
    }
    for point in &mut force {
        for axis in 0..2 {
            point[axis] -= mean[axis];
        }
    }
    (
        force,
        [
            (parent_force[0] + mean[0]) * 0.8,
            (parent_force[1] + mean[1]) * 0.8,
        ],
    )
}

pub fn camera_scale(probe: [f64; 2]) -> f64 {

    f64::from(super::empty_background::literal::public_numeric::source_zoom(
        probe.map(|value| Some(value as f32)),
    ))
}

pub fn probe_from_pixel(pixel: [f64; 2], size: [f64; 2], current: [f64; 2]) -> [f64; 2] {
    let half = size[0].min(size[1]) * 0.5;
    let input = [
        (pixel[0].clamp(0.0, size[0]) - size[0] * 0.5) / half,
        (size[1] * 0.5 - pixel[1].clamp(0.0, size[1])) / half,
    ];
    let guess = probe_from_normalized(input, current);
    [guess[0].clamp(-2.15, 2.15), guess[1].clamp(-1.4, 1.4)]
}

pub fn probe_from_normalized(input: [f64; 2], current: [f64; 2]) -> [f64; 2] {

    let zoom = super::empty_background::literal::public_numeric::source_zoom_f64(
        current.map(|value| Some(value as f32)),
    );
    let y = input[1] / zoom;
    let x = input[0] / zoom;
    let mut guess = [x, y];
    for _ in 0..10 {
        let zoom = super::empty_background::literal::public_numeric::source_zoom_from_unclamped_pair(guess);
        let y = input[1] / zoom;
        let x = input[0] / zoom;
        guess = [x, y];
    }
    guess
}

