use super::atan2_table::{ANGLE_HIGH, ANGLE_LOW};

fn signed(value: f64, negative: bool) -> f64 {
    if negative { f64::from_bits(value.to_bits() ^ 0x8000_0000_0000_0000) } else { value }
}

fn scale_small_magnitude(value: f64) -> f64 {
    if value.to_bits() & 0x7ff0_0000_0000_0000 == 0 {
        f64::from_bits(value.to_bits() | 0x4010_0000_0000_0000) - 4.0
    } else {
        f64::from_bits(value.to_bits() + 0x4000_0000_0000_0000)
    }
}

fn underflow_ratio(numerator: f64, denominator: f64) -> f64 {
    let mut value = f64::from_bits(0x4630000000000000) * numerator;
    value /= denominator;
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as u32;
    if exponent > 100 {
        return f64::from_bits((u64::from(exponent - 100) << 52)
            | (bits & 0x000f_ffff_ffff_ffff));
    }
    let mantissa = (bits & 0x001f_ffff_ffff_ffff) | 0x0010_0000_0000_0000;
    if 101 - exponent > 54 { return 0.0; }
    let shifted = mantissa >> (100 - exponent);
    f64::from_bits((shifted >> 1) + (shifted & 1))
}

pub fn finite_atan2(y: f64, x: f64) -> Result<f64, &'static str> {
    if !y.is_finite() || !x.is_finite() {
        return Err("finite angle scalar inputs required");
    }
    let negative_y = y.is_sign_negative();
    let negative_x = x.is_sign_negative();
    let mut numerator = y.abs();
    let mut denominator = x.abs();
    if numerator == 0.0 {
        return Ok(if negative_x { signed(f64::from_bits(0x400921fb54442d18), negative_y) }
            else { y });
    }
    if denominator == 0.0 {
        return Ok(signed(f64::from_bits(0x3ff921fb54442d18), negative_y));
    }
    if numerator < 0.25 && denominator < 0.25 {
        numerator = scale_small_magnitude(numerator);
        denominator = scale_small_magnitude(denominator);
    }
    let difference = ((numerator.to_bits() >> 52) & 0x7ff) as i32
        - ((denominator.to_bits() >> 52) & 0x7ff) as i32;
    if difference > 56 {
        return Ok(signed(f64::from_bits(0x3ff921fb54442d18), negative_y));
    }
    if difference < -28 && !negative_x {
        if difference < -1074 { return Ok(signed(0.0, negative_y)); }
        if difference < -1022 {
            return Ok(signed(underflow_ratio(numerator, denominator), negative_y));
        }
        return Ok(signed(numerator / denominator, negative_y));
    }
    if difference < -56 && negative_x {
        return Ok(signed(f64::from_bits(0x400921fb54442d18), negative_y));
    }
    let swapped = numerator > denominator;
    if swapped { std::mem::swap(&mut numerator, &mut denominator); }
    let mut ratio = numerator / denominator;
    let (mut angle_high, mut angle_low) = if ratio > f64::from_bits(0x3fb0000000000000) {
        let mut grid = ratio * 256.0;
        grid += 0.5;
        let grid = grid.trunc() as usize;
        if !(16..=256).contains(&grid) {
            return Err("angle scalar grid index outside source domain");
        }
        let grid_fraction = grid as f64 * f64::from_bits(0x3f70000000000000);
        let scale = 1023 - ((denominator.to_bits() >> 52) & 0x7ff) as i32;
        let half = scale / 2;
        let remainder = scale - half;
        let first = f64::from_bits(((half + 1023) as u64) << 52);
        let second = f64::from_bits(((remainder + 1023) as u64) << 52);
        let mut scaled_denominator = first * denominator;
        scaled_denominator *= second;
        let mut scaled_numerator = first * numerator;
        scaled_numerator *= second;
        let high = f64::from_bits(scaled_denominator.to_bits() & 0xffff_ffff_f800_0000);
        let low = scaled_denominator - high;
        let high_product = high * grid_fraction;
        let low_product = low * grid_fraction;
        let mut reduced = scaled_numerator - high_product;
        let mut divisor = grid_fraction * scaled_numerator;
        divisor += scaled_denominator;
        reduced -= low_product;
        reduced /= divisor;
        let square = reduced * reduced;
        let mut angle = reduced + f64::from_bits(ANGLE_LOW[grid - 16]);
        let product = square * f64::from_bits(0x3fc99992b9802d7d);
        let mut polynomial = f64::from_bits(0x3fd5555555550877) - product;
        polynomial *= square;
        polynomial *= reduced;
        angle -= polynomial;
        (f64::from_bits(ANGLE_HIGH[grid - 16]), angle)
    } else {
        if !(f64::from_bits(0x3e45798ee2308c3a) > ratio) {
            let square = ratio * ratio;
            let high_denominator = f64::from_bits(denominator.to_bits() & 0xffff_ffff_0000_0000);
            let low_denominator = denominator - high_denominator;
            let high_ratio = f64::from_bits(ratio.to_bits() & 0xffff_ffff_0000_0000);
            let low_product = low_denominator * high_ratio;
            let high_product = high_ratio * high_denominator;
            let mut residual = numerator - high_product;
            let low_ratio = ratio - high_ratio;
            residual -= low_product;
            let product = low_ratio * denominator;
            residual -= product;
            residual /= denominator;
            let product = square * f64::from_bits(0x3fb70c3192d751c2);
            let mut polynomial = f64::from_bits(0x3fbc718839b8c4eb) - product;
            polynomial *= square;
            polynomial = f64::from_bits(0x3fc2492482bd6be1) - polynomial;
            polynomial *= square;
            polynomial = f64::from_bits(0x3fc99999999643a3) - polynomial;
            polynomial *= square;
            polynomial = f64::from_bits(0x3fd5555555555538) - polynomial;
            let cube = square * ratio;
            polynomial *= cube;
            residual -= polynomial;
            ratio += residual;
        }
        (0.0, ratio)
    };
    if swapped {
        angle_high = f64::from_bits(0x3ff921fb54442d18) - angle_high;
        angle_low = f64::from_bits(0x3c91a62633145c07) - angle_low;
    }
    if negative_x {
        angle_high = f64::from_bits(0x400921fb50000000) - angle_high;
        angle_low = f64::from_bits(0x3e6110b4611a6263) - angle_low;
    }
    Ok(signed(angle_high + angle_low, negative_y))
}
