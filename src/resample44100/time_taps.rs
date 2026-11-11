use super::finite_rounding::floor_finite;
use super::tap_calibration::{calibrate,Calibration,CalibrationMath,TransitionBand};
use super::time_tap_data::*;

pub trait DesignMath: CalibrationMath {
    fn log(&mut self, value: f64) -> f64;
    fn sin(&mut self, value: f64) -> f64;
    fn pow(&mut self, base: f64, exponent: f64) -> f64;
}

pub trait WideDesignMath: DesignMath {
    fn tan(&mut self, value: f64) -> f64;
    fn cosh(&mut self, value: f64) -> f64;
}

#[derive(Debug)]
pub struct TimeTaps {
    pub calibration: Calibration,
    pub width_factor: f64,
    pub width_adjustment: f64,
    pub radius: f64,
    pub carrier: f64,
    pub taps: Vec<f64>,
}

fn finite(value: f64) -> Result<f64, &'static str> {
    if value.is_finite() { Ok(value) } else { Err("finite time-tap source scope required") }
}

fn window<M: DesignMath>(radial: f64, math: &mut M) -> Result<f64, &'static str> {
    if radial < 0.0 { return Ok(0.0); }
    let x = radial.sqrt() * WINDOW_COEFFICIENT_011;
    let absolute = f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff);
    let value = if WINDOW_COEFFICIENT_012 > absolute {
        let mut y = x / WINDOW_COEFFICIENT_012;
        y *= y;
        let mut value = WINDOW_COEFFICIENT_022 * y;
        for (index,coefficient) in [WINDOW_COEFFICIENT_023,WINDOW_COEFFICIENT_024,WINDOW_COEFFICIENT_025,WINDOW_COEFFICIENT_026,WINDOW_COEFFICIENT_027,SCALAR_COEFFICIENT_008].into_iter().enumerate() {
            value += coefficient;
            if index < 5 { value *= y; }
        }
        value
    } else {
        let y = WINDOW_COEFFICIENT_012 / absolute;
        let exp = finite(math.exp(absolute))?;
        let scaled = exp / absolute.sqrt();
        let mut value = WINDOW_COEFFICIENT_013 * y;
        for (index,coefficient) in [WINDOW_COEFFICIENT_014,WINDOW_COEFFICIENT_015,WINDOW_COEFFICIENT_016,WINDOW_COEFFICIENT_017,WINDOW_COEFFICIENT_018,WINDOW_COEFFICIENT_019,WINDOW_COEFFICIENT_020,WINDOW_COEFFICIENT_021].into_iter().enumerate() {
            value += coefficient;
            if index < 7 { value *= y; }
        }
        value * scaled
    };
    finite(value / WINDOW_COEFFICIENT_028)
}

fn signed_power<M: DesignMath>(value: f64, exponent: f64, math: &mut M) -> Result<f64, &'static str> {
    if value < 0.0 {
        let absolute = f64::from_bits(value.to_bits() ^ 0x8000_0000_0000_0000);
        let value = finite(math.pow(absolute,exponent))?;
        Ok(f64::from_bits(value.to_bits() ^ 0x8000_0000_0000_0000))
    } else { finite(math.pow(value,exponent)) }
}

pub fn phase0_narrow_taps<M: DesignMath>(
    cutoff: f64, transition_percent: f64, attenuation: f64, math: &mut M,
) -> Result<TimeTaps, &'static str> {
    finite(cutoff)?;
    if !(cutoff > 0.0 && cutoff <= 1.0) { return Err("cutoff candidate scope (0,1] required"); }
    let calibration = calibrate(transition_percent,attenuation,math)?;
    if calibration.band != TransitionBand::BelowTenth || !calibration.above_shape_threshold {
        return Err("unsupported source width branch; no fallback");
    }
    narrow_taps_from_calibration(cutoff, calibration, math)
}

fn narrow_taps_from_calibration<M: DesignMath>(
    cutoff: f64,
    calibration: Calibration,
    math: &mut M,
) -> Result<TimeTaps, &'static str> {
    let shape = calibration.shape_coordinate;
    let relative = calibration.relative_transition;
    let square = shape * shape;
    let mut first = WINDOW_COEFFICIENT_001 * square;
    let mut second = WINDOW_COEFFICIENT_002 * shape;
    second += first;
    first = WINDOW_COEFFICIENT_003 * shape;
    first *= square;
    let mut angle = WINDOW_COEFFICIENT_004 * shape;
    angle += first;
    let cosine = finite(math.cos(angle))?;
    let mut width_factor = calibration.corrected_coordinate * WINDOW_COEFFICIENT_005;
    width_factor *= cosine;
    width_factor += second;
    width_factor /= relative;
    let log_scale = WINDOW_COEFFICIENT_006 * relative;
    let mut logarithm = finite(math.log(shape))?;
    logarithm *= log_scale;
    let mut adjustment = WINDOW_COEFFICIENT_007 * relative;
    adjustment += logarithm;
    let scalar_scale = WINDOW_COEFFICIENT_008 * relative;
    let scalar_argument = WINDOW_COEFFICIENT_009 * shape;
    let mut scalar = super::scalar_math::finite_atan(scalar_argument)?;
    scalar *= scalar_scale;
    scalar += adjustment;
    adjustment = WINDOW_COEFFICIENT_010 * shape;
    adjustment *= relative;
    adjustment += scalar;

    finish_taps(cutoff, calibration, width_factor, adjustment, math)
}

fn finish_taps<M: DesignMath>(
    cutoff: f64,
    calibration: Calibration,
    width_factor: f64,
    adjustment: f64,
    math: &mut M,
) -> Result<TimeTaps, &'static str> {
    let shape = calibration.shape_coordinate;
    let mut radius = width_factor * super::tap_calibration_data::CALIBRATION_COEFFICIENT_001;
    radius /= cutoff;
    finite(radius)?;
    let half = floor_finite(radius)?;

    if half < 1.0 || half > 16383.0 { return Err("candidate half-tap bound1..16383 required"); }
    let half = half as usize;
    let mut carrier = SCALAR_COEFFICIENT_008 - adjustment;
    carrier *= WINDOW_COEFFICIENT_000;
    carrier *= cutoff;
    finite(carrier)?;
    let initial_sine = finite(math.sin(0.0-carrier))?;
    let cosine = finite(math.cos(carrier))?;
    let zero = 0.0 / radius;
    let exponent = f64::from_bits(shape.to_bits() & 0x7fff_ffff_ffff_ffff);
    let mut taps = vec![0.0;half*2+1];
    let mut radial = zero + zero;
    radial *= radial;
    radial = SCALAR_COEFFICIENT_008-radial;
    let window = self::window(radial,math)?;
    let power = signed_power(window,exponent,math)?;
    let mut center = carrier * power;
    center /= WINDOW_COEFFICIENT_000;
    taps[half] = center;
    let twice_cosine = cosine + cosine;
    let mut current_sine = 0.0 * twice_cosine;
    current_sine -= initial_sine;
    let mut previous_sine = 0.0;
    for n in 1..=half {
        let source_sine = current_sine;
        let mut position = n as f64 / radius;
        position += zero;
        position *= position;
        let radial = SCALAR_COEFFICIENT_008-position;
        let window = self::window(radial,math)?;
        let mut value = signed_power(window,exponent,math)?;
        current_sine = twice_cosine * source_sine;
        value *= source_sine;
        let denominator = n as f64 * WINDOW_COEFFICIENT_000;
        value /= denominator;
        finite(value)?;
        taps[half+n] = value;
        current_sine -= previous_sine;
        taps[half-n] = value;
        previous_sine = source_sine;
    }
    Ok(TimeTaps { calibration,width_factor,width_adjustment:adjustment,radius,carrier,taps })
}

pub fn phase0_supported_taps<M: WideDesignMath>(
    cutoff: f64,
    transition_percent: f64,
    attenuation: f64,
    math: &mut M,
) -> Result<TimeTaps, &'static str> {
    finite(cutoff)?;
    if !(cutoff > 0.0 && cutoff <= 1.0) {
        return Err("cutoff candidate scope (0,1] required");
    }
    let calibration = calibrate(transition_percent, attenuation, math)?;
    if !calibration.above_shape_threshold {
        let (width, adjustment) = lower_shape_width(calibration, math)?;
        return finish_taps(cutoff, calibration, width, adjustment, math);
    }
    if calibration.band == TransitionBand::BelowTenth {
        return narrow_taps_from_calibration(cutoff, calibration, math);
    }
    let shape = calibration.shape_coordinate;
    let mut argument = shape * shape;
    argument += 1.0;
    argument = argument.sqrt();
    argument += shape;
    let logarithm = finite(math.log(argument))?;
    let (width_factor, adjustment) = match calibration.band {
        TransitionBand::TenthToQuarter => {
            let scaled_logarithm = logarithm * f64::from_bits(0x4063c71fbff9e483);
            let mut argument = f64::from_bits(0x40450940622069cf) * shape;
            argument += f64::from_bits(0x3ff075c5b1fc92f0);
            let mut term = super::scalar_hyperbolic::finite_tanh(argument)?;
            term *= scaled_logarithm;
            term += f64::from_bits(0x3ff80b5c82e20f4a);
            let mut width = calibration.corrected_coordinate * f64::from_bits(0xbf8632486776168c);
            width += term;
            width /= calibration.relative_transition;
            let argument = finite(math.tan(shape))?;
            let argument = super::scalar_hyperbolic::finite_tanh(argument)?;
            let argument = super::scalar_hyperbolic::finite_tanh(argument)?;
            let mut correction = finite(argument.sqrt())?;
            correction *= f64::from_bits(0xc01af765b55e5959);
            correction += f64::from_bits(0xbfd01ca2dbf56391);
            correction /= width;
            let mut adjustment = calibration.relative_transition * f64::from_bits(0x3fefc2e1f06225d8);
            adjustment += correction;
            (width, adjustment)
        }
        TransitionBand::QuarterAndAbove => {
            let scaled_logarithm = logarithm * f64::from_bits(0x4063d1d2b05b8b03);
            let mut argument = super::scalar_hyperbolic::finite_tanh(shape)?;
            argument *= f64::from_bits(0x404cf92aff6c6c31);
            let mut term = super::scalar_hyperbolic::finite_tanh(argument)?;
            term *= scaled_logarithm;
            term += f64::from_bits(0x3ff80a953131762b);
            let mut width = calibration.corrected_coordinate * f64::from_bits(0xbf85981e29fbc0a3);
            width += term;
            width /= calibration.relative_transition;
            let sine = finite(math.sin(shape))?;
            let mut correction = finite(sine.sqrt())?;
            correction *= f64::from_bits(0xc01b7d6ea979007f);
            correction += f64::from_bits(0xbfce3e83184d548d);
            correction /= width;
            let mut adjustment = calibration.relative_transition * f64::from_bits(0x3fefcf0c430ef075);
            adjustment += correction;
            (width, adjustment)
        }
        TransitionBand::BelowTenth => return Err("source width branch dispatch invariant failed"),
    };
    finish_taps(cutoff, calibration, width_factor, adjustment, math)
}

fn lower_shape_width<M: WideDesignMath>(
    calibration: Calibration,
    math: &mut M,
) -> Result<(f64, f64), &'static str> {
    let shape = calibration.shape_coordinate;
    let relative = calibration.relative_transition;
    match calibration.band {
        TransitionBand::BelowTenth => {
            let scaled_shape = f64::from_bits(0x4070d2f059ea7c4f) * shape;
            let second = f64::from_bits(0x3ff1559cb01de21b) * shape;
            let mut argument = f64::from_bits(0xc031385c1e869c22) * shape;
            argument += f64::from_bits(0x3fefa4c02e1be287);
            let hyperbolic = finite(math.cosh(argument))?;
            let mut angle = super::scalar_angle::finite_atan2(hyperbolic, second)?;
            angle += f64::from_bits(0x4016edd4b2123ae1);
            let mut width = finite(math.cos(angle))?;
            width *= scaled_shape;
            width += f64::from_bits(0x4003a8bf8870c0bf);
            width /= relative;
            let mut correction = relative * relative;
            correction *= f64::from_bits(0x3f93e404980566e8);
            correction *= width;
            let mut combined = f64::from_bits(0x400255ed8506c31b) * relative;
            combined += correction;
            let mut adjustment = f64::from_bits(0xc012b3995da56ba4) * shape;
            adjustment *= relative;
            adjustment += combined;
            let power = finite(math.pow(shape, shape))?;
            let mut term = relative * f64::from_bits(0xbffab1e6ee724a31);
            term *= power;
            term += adjustment;
            Ok((width, term))
        }
        TransitionBand::TenthToQuarter => {
            let mut base = f64::from_bits(0x4061c20eeb881364) * shape;
            base += f64::from_bits(0x3ff911f73b97b0a4);
            let argument = f64::from_bits(0x406e7445ba6d88b8) * shape;
            let mut term = finite(math.cos(argument))?;
            term *= f64::from_bits(0x3f712e28ace87c7d);
            term += base;
            let mut width = calibration.corrected_coordinate * f64::from_bits(0xbf97811b8b95f19d);
            width += term;
            let argument = f64::from_bits(0x405fedcb94d6a4cb) * shape;
            let mut term = finite(math.cos(argument))?;
            term *= f64::from_bits(0xbf9b44c81d722cff);
            term += width;
            let width = term / relative;
            let mut argument = finite(shape.sqrt())?;
            argument *= f64::from_bits(0xc01738bc55709088);
            argument += f64::from_bits(0xbfd727741c563568);
            argument /= width;
            let mut square = argument * argument;
            square += 1.0;
            square = square.sqrt();
            square += argument;
            let mut correction = finite(math.log(square))?;
            correction *= f64::from_bits(0x3feffb6d4101ff72);
            let mut adjustment = relative * f64::from_bits(0x3fef6eff0f7526e0);
            adjustment += correction;
            Ok((width, adjustment))
        }
        TransitionBand::QuarterAndAbove => {
            let base = f64::from_bits(0x40056c2a510f746d) / relative;
            let argument = f64::from_bits(0x4005844bcf7e7f21) * shape;
            let argument = super::scalar_math::finite_atan(argument)?;
            let mut numerator = super::scalar_math::finite_atan(argument)?;
            numerator *= f64::from_bits(0x4072c8c2ee52812f);
            let mut argument = calibration.corrected_coordinate * calibration.corrected_coordinate;
            argument += 1.0;
            argument = argument.sqrt();
            argument += calibration.corrected_coordinate;
            let argument = finite(math.log(argument))?;
            let argument = finite(math.cos(argument))?;
            let mut hyperbolic = super::scalar_hyperbolic::finite_tanh(argument)?;
            hyperbolic *= relative;
            let mut denominator = f64::from_bits(0x40160a2a7dec9f26) * relative;
            denominator -= hyperbolic;
            numerator /= denominator;
            numerator += base;
            let divisor = f64::from_bits(0xc02c5d3e2548932f) + numerator;
            let mut offset = f64::from_bits(0xbff260b9d4b015b8) / divisor;
            offset += numerator;
            let mut exponent = f64::from_bits(0x4000e10d7e2d23c3) * numerator;
            exponent += f64::from_bits(0xc034f818c3112472);
            let angle = super::scalar_angle::finite_atan2(
                f64::from_bits(0x3ff1ec49e22dbe80), relative,
            )?;
            let power = finite(math.pow(angle, exponent))?;
            let mut width = finite(math.pow(f64::from_bits(0x3fee77b8d51b43a9), power))?;
            width += offset;
            let mut argument = finite(shape.sqrt())?;
            argument *= f64::from_bits(0xc018c3e7b490f880);
            argument += f64::from_bits(0xbfd49257cdf7bed0);
            let mut correction = super::scalar_angle::finite_atan2(argument, width)?;
            correction *= f64::from_bits(0x3ff0007b9daae809);
            let mut adjustment = relative * f64::from_bits(0x3fef972fafb7360c);
            adjustment += correction;
            Ok((width, adjustment))
        }
    }
}
