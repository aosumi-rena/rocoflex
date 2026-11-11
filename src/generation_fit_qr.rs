pub fn solve(columns: [[f32; 4]; 4], rhs: [f32; 4]) -> [f32; 4] {
    let mut qr = [[0.0_f32; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            qr[row][col] = columns[col][row];
        }
    }
    let mut permutation = [0, 1, 2, 3];
    let mut norms = [0.0_f32; 4];
    let mut original_norms = [0.0_f32; 4];
    for col in 0..4 {
        norms[col] = ((qr[1][col] * qr[1][col] + qr[3][col] * qr[3][col])
            + (qr[0][col] * qr[0][col] + qr[2][col] * qr[2][col]))
            .sqrt();
        original_norms[col] = norms[col];
    }
    let mut tau = [0.0_f32; 4];
    for k in 0..4 {
        let mut pivot = k;
        for col in k + 1..4 {
            if norms[col] > norms[pivot] {
                pivot = col;
            }
        }
        if pivot != k {
            for row in &mut qr {
                row.swap(k, pivot);
            }
            norms.swap(k, pivot);
            original_norms.swap(k, pivot);
            permutation.swap(k, pivot);
        }
        let mut tail_squared = 0.0_f32;
        for row in k + 1..4 {
            tail_squared += qr[row][k] * qr[row][k];
        }
        let leading = qr[k][k];
        if tail_squared <= f32::MIN_POSITIVE {
            tau[k] = 0.0;
        } else {
            let length = (tail_squared + leading * leading).sqrt();
            let beta = if leading >= 0.0 { -length } else { length };
            let divisor = leading - beta;
            for row in k + 1..4 {
                qr[row][k] /= divisor;
            }
            tau[k] = (beta - leading) / beta;
            qr[k][k] = beta;
        }
        for col in k + 1..4 {
            let mut dot = 0.0_f32;
            for row in k + 1..4 {
                dot += qr[row][k] * qr[row][col];
            }
            let projection = dot + qr[k][col];
            let product = tau[k] * projection;
            qr[k][col] -= product;
            for row in k + 1..4 {
                qr[row][col] -= (tau[k] * qr[row][k]) * projection;
            }
            if norms[col] != 0.0 {
                let ratio = qr[k][col].abs() / norms[col];
                let update = ((1.0 + ratio) * (1.0 - ratio)).max(0.0);
                let reliability = norms[col] / original_norms[col];
                if update * (reliability * reliability) <= f32::EPSILON.sqrt() {
                    let mut squared = 0.0_f32;
                    for row in k + 1..4 {
                        squared += qr[row][col] * qr[row][col];
                    }
                    norms[col] = squared.sqrt();
                    original_norms[col] = norms[col];
                } else {
                    norms[col] *= update.sqrt();
                }
            }
        }
    }
    let mut transformed = rhs;
    for k in 0..4 {
        let mut dot = 0.0_f32;
        for row in k + 1..4 {
            dot += qr[row][k] * transformed[row];
        }
        let projection = dot + transformed[k];
        let product = tau[k] * projection;
        transformed[k] -= product;
        for row in k + 1..4 {
            transformed[row] -= (tau[k] * qr[row][k]) * projection;
        }
    }
    for k in (0..4).rev() {
        transformed[k] /= qr[k][k];
        for row in 0..k {
            transformed[row] -= transformed[k] * qr[row][k];
        }
    }
    let mut result = [0.0; 4];
    for k in 0..4 {
        result[permutation[k]] = transformed[k];
    }
    result
}

