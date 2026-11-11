pub fn packet_sum(values: &[f32]) -> f32 {
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
    let mut result = [0.0; 4];
    for c in 0..4 {
        for r in 0..4 {
            result[r] = matrix[c * 4 + r] * vector[c] + result[r];
        }
    }
    result
}

pub fn basis_step(covariance: &[f32; 16], incoming: [f32; 8]) -> [f32; 8] {
    let mut result = incoming;
    let update = |old: &[f32], matrix: &[f32; 16]| {
        let product = multiply(matrix, old);
        let mut blended = [0.0; 4];
        for i in 0..4 {
            blended[i] = 0.5 * old[i] + 0.5 * product[i];
        }
        let squares: Vec<_> = blended.iter().map(|v| v * v).collect();
        let length = packet_sum(&squares).sqrt();
        for v in &mut blended {
            *v /= length;
        }
        blended
    };
    result[..4].copy_from_slice(&update(&incoming[..4], covariance));
    let product = multiply(covariance, &result[..4]);
    let mut deflated = *covariance;
    for c in 0..4 {
        for r in 0..4 {
            deflated[c * 4 + r] -= result[c] * product[r];
        }
    }
    result[4..].copy_from_slice(&update(&incoming[4..], &deflated));
    result
}
