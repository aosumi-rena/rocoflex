use crate::Result;

fn maximum(left: f32, right: f32) -> f32 {
    if left > right { left } else { right }
}

fn horizontal_max(mut lanes: [f32; 8]) -> f32 {
    let old = lanes;
    for i in 0..8 { lanes[i] = maximum(old[i], old[i ^ 1]); }
    let old = lanes;
    for i in 0..8 { lanes[i] = maximum(old[i], old[i ^ 2]); }
    maximum(lanes[4], lanes[0])
}

fn horizontal_sum(mut lanes: [f32; 8]) -> f32 {
    let old = lanes;
    for i in 0..8 { lanes[i] = old[i] + old[i ^ 1]; }
    let old = lanes;
    for i in 0..8 { lanes[i] = old[i] + old[i ^ 2]; }
    lanes[4] + lanes[0]
}

pub(crate) fn normalize_auxiliary(
    small: &mut [f32; 15],
    large: &mut [f32; 321],
) -> Result<()> {
    if small.iter().chain(large.iter()).any(|value| !value.is_finite()) {
        return Err("Nonfinite auxiliary distribution logits".into());
    }
    let mut small_next = *small;
    let mut large_next = *large;
    normalize_auxiliary_row(&mut small_next)?;
    normalize_auxiliary_row(&mut large_next)?;
    *small = small_next;
    *large = large_next;
    Ok(())
}

fn normalize_auxiliary_row(logits: &mut [f32]) -> Result<()> {
    if !matches!(logits.len(), 15 | 321) {
        return Err("Unsupported auxiliary distribution width".into());
    }
    let sentinel = f32::from_bits(0xe0ad78ec);
    let vectors = (logits.len() + 7) / 8;
    let mut maxima = [[sentinel; 8]; 2];
    for block in 0..vectors {
        let partial = block * 8 + 8 > logits.len();
        for lane in 0..8 {
            let index = block * 8 + lane;
            let value = match logits.get(index) {

                Some(&value) if partial => value + 0.0,
                Some(&value) => value,
                None => sentinel,
            };
            maxima[block & 1][lane] = maximum(maxima[block & 1][lane], value);
        }
    }
    let mut merged = [0.0; 8];
    for lane in 0..8 { merged[lane] = maximum(maxima[0][lane], maxima[1][lane]); }
    let max = horizontal_max(merged);

    let mut weights = [0.0; 321];
    let mut sums = [[0.0; 8]; 2];
    for block in 0..vectors {
        for lane in 0..8 {
            let index = block * 8 + lane;
            let weight = if index < logits.len() {
                let shifted = max - logits[index];
                let z = if shifted < 12.0 { shifted } else { 12.0 };
                let product = f32::from_bits(0x3aec595a) * z;
                let linear = product + f32::from_bits(0xbd7f964e);
                let product = linear * z;
                let mut weight = product + 1.0;
                for _ in 0..4 { weight *= weight; }
                weights[index] = weight;
                weight
            } else {

                0.0
            };
            sums[block & 1][lane] += weight;
        }
    }
    for lane in 0..8 { merged[lane] = sums[0][lane] + sums[1][lane]; }
    let total = horizontal_sum(merged);
    if !total.is_finite() || total <= 0.0 {
        return Err("Invalid auxiliary polynomial distribution sum".into());
    }
    let reciprocal = crate::activation::rcpps(total);
    for weight in &mut weights[..logits.len()] { *weight *= reciprocal; }
    logits.copy_from_slice(&weights[..logits.len()]);
    Ok(())
}

pub(crate) fn normalize(logits: &mut [f32; 128]) -> Result<()> {
    if logits.iter().any(|value| !value.is_finite()) {
        return Err("Nonfinite categorical logits".into());
    }

    let mut maxima = [[f32::from_bits(0xe0ad78ec); 8]; 2];
    for (block, values) in logits.chunks_exact(8).enumerate() {
        for lane in 0..8 {
            maxima[block & 1][lane] = maximum(maxima[block & 1][lane], values[lane]);
        }
    }
    let mut merged = [0.0; 8];
    for lane in 0..8 { merged[lane] = maximum(maxima[0][lane], maxima[1][lane]); }
    let max = horizontal_max(merged);

    let mut weights = [0.0; 128];
    let mut sums = [[0.0; 8]; 2];
    for (block, values) in logits.chunks_exact(8).enumerate() {
        for lane in 0..8 {
            let shifted = max - values[lane];

            let z = if shifted < 12.0 { shifted } else { 12.0 };
            let product = f32::from_bits(0x3aec595a) * z;
            let linear = product + f32::from_bits(0xbd7f964e);
            let product = linear * z;
            let mut weight = product + 1.0;
            for _ in 0..4 { weight *= weight; }
            weights[block * 8 + lane] = weight;
            sums[block & 1][lane] += weight;
        }
    }
    for lane in 0..8 { merged[lane] = sums[0][lane] + sums[1][lane]; }
    let total = horizontal_sum(merged);
    if !total.is_finite() || total <= 0.0 {
        return Err("Invalid categorical polynomial sum".into());
    }
    let reciprocal = crate::activation::rcpps(total);
    for weight in &mut weights { *weight *= reciprocal; }
    logits.copy_from_slice(&weights);
    Ok(())
}
