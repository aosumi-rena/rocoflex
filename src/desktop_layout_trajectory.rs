#[derive(Clone, Copy, Debug)]
pub struct Quintic {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub coefficients: [[f32; 4]; 2],
}

impl Quintic {
    pub fn evaluate(&self, t: f32) -> [f32; 2] {
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

pub fn normal_system(parameters: &[f32], values: &[f32]) -> ([[f32; 4]; 4], [f32; 4]) {
    assert_eq!(parameters.len(), values.len());
    assert!(!values.is_empty());
    let start = values[0];
    let end = values[values.len() - 1];
    let mut matrix = [[0.0; 4]; 4];
    let mut rhs = [0.0; 4];
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

pub fn arc_parameters(curve: &Quintic, requested: &[f32]) -> (Vec<f32>, f32) {
    const COUNT: usize = 500;
    let mut parameters = [0.0_f32; COUNT];
    for (i, value) in parameters.iter_mut().enumerate() {
        *value = i as f32 * (1.0 / (COUNT - 1) as f32);
    }
    parameters[COUNT - 1] = 1.0;
    let mut cumulative = [0.0_f32; COUNT];
    let evaluated: Vec<_> = parameters.iter().map(|&t| curve.evaluate(t)).collect();
    let mut squared = [0.0_f32; COUNT - 1];
    for i in 0..COUNT - 1 {
        let dx = evaluated[i + 1][0] - evaluated[i][0];
        let dy = evaluated[i + 1][1] - evaluated[i][1];
        squared[i] = dx * dx + dy * dy;
    }
    let mut distances = [0.0_f32; COUNT - 1];
    use std::arch::x86_64::*;
    unsafe {
        for i in (0..496).step_by(4) {
            let x = _mm_loadu_ps(squared.as_ptr().add(i));
            let q = _mm_rsqrt_ps(x);
            let negative_half_x = _mm_mul_ps(x, _mm_set1_ps(-0.5));
            let a = _mm_add_ps(
                _mm_mul_ps(_mm_mul_ps(q, q), negative_half_x),
                _mm_set1_ps(1.5),
            );
            let result = _mm_mul_ps(_mm_mul_ps(a, q), x);
            _mm_storeu_ps(distances.as_mut_ptr().add(i), result);
        }
    }
    for i in 496..COUNT - 1 {
        distances[i] = squared[i].sqrt();
    }
    for i in 1..COUNT {
        cumulative[i] = cumulative[i - 1] + distances[i - 1];
    }
    let total = cumulative[COUNT - 1];
    let mut j = 0;
    let mut result = Vec::with_capacity(requested.len());
    for &t in requested {
        let distance = t * total;
        while j < COUNT - 1 && distance > cumulative[j + 1] {
            j += 1;
        }
        let fraction =
            ((distance - cumulative[j]) / cumulative[j + 1] - cumulative[j]).clamp(0.0, 1.0);
        result.push((parameters[j + 1] - parameters[j]) * fraction + parameters[j]);
    }
    (result, total)
}

pub fn clamp_spacing(points: &mut [[f32; 2]], arc_length: f32) {
    let spacing = arc_length / points.len() as f32;
    let scale = if spacing < 0.06 {
        0.06 / spacing
    } else if spacing > 0.1 {
        0.1 / spacing
    } else {
        return;
    };
    let mut mean = [0.0; 2];
    for d in 0..2 {
        let col: Vec<_> = points.iter().map(|p| p[d]).collect();
        mean[d] = super::aligned_sum(&col, d * points.len()) / points.len() as f32;
    }
    for point in points {
        for d in 0..2 {
            point[d] = (point[d] - mean[d]) * scale + mean[d];
        }
    }
}




