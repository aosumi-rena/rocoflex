use super::finite_rounding::floor_finite;
use super::tap_calibration_data::*;

pub trait CalibrationMath {
    fn cos(&mut self, value: f64) -> f64;
    fn exp(&mut self, value: f64) -> f64;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionBand { BelowTenth, TenthToQuarter, QuarterAndAbove }

#[derive(Clone, Copy, Debug)]
pub struct Calibration {
    pub relative_transition: f64,
    pub band: TransitionBand,
    pub base_coordinate: f64,
    pub table_index: usize,
    pub table_value: i8,
    pub corrected_coordinate: f64,
    pub shape_coordinate: f64,
    pub above_shape_threshold: bool,
}

fn finite(value: f64) -> Result<f64, &'static str> {
    if value.is_finite() { Ok(value) } else { Err("finite calibration dependency/input required") }
}

pub fn calibrate<M: CalibrationMath>(
    transition_percent: f64, attenuation: f64, math: &mut M,
) -> Result<Calibration, &'static str> {
    finite(transition_percent)?;
    finite(attenuation)?;
    let relative_transition = finite(transition_percent * CALIBRATION_COEFFICIENT_000)?;
    if relative_transition <= 0.0 { return Err("positive transition scope required"); }
    let low_attenuation = attenuation < CALIBRATION_COEFFICIENT_004;
    let high_attenuation = attenuation >= CALIBRATION_COEFFICIENT_003;
    let (band, base, table, divisor) = if relative_transition >= CALIBRATION_COEFFICIENT_001 {
        (TransitionBand::QuarterAndAbove,
         if high_attenuation { CALIBRATION_COEFFICIENT_005 } else { [CALIBRATION_COEFFICIENT_026,CALIBRATION_COEFFICIENT_027][usize::from(low_attenuation)] },
         &WIDE_TRANSITION_CORRECTIONS, CALIBRATION_COEFFICIENT_014)
    } else if relative_transition >= CALIBRATION_COEFFICIENT_002 {
        (TransitionBand::TenthToQuarter,
         if high_attenuation { CALIBRATION_COEFFICIENT_006 } else { [CALIBRATION_COEFFICIENT_028,CALIBRATION_COEFFICIENT_029][usize::from(low_attenuation)] },
         &MID_TRANSITION_CORRECTIONS, CALIBRATION_COEFFICIENT_012)
    } else {
        (TransitionBand::BelowTenth,
         if high_attenuation { CALIBRATION_COEFFICIENT_007 } else { [CALIBRATION_COEFFICIENT_030,CALIBRATION_COEFFICIENT_031][usize::from(low_attenuation)] },
         &NARROW_TRANSITION_CORRECTIONS, CALIBRATION_COEFFICIENT_013)
    };
    let base_coordinate = finite(base - attenuation)?;
    let mut index_argument = CALIBRATION_COEFFICIENT_009 - base_coordinate;
    index_argument *= CALIBRATION_COEFFICIENT_010;
    index_argument /= CALIBRATION_COEFFICIENT_011;
    index_argument += CALIBRATION_COEFFICIENT_008;
    let floor = floor_finite(index_argument)?;

    if floor < i32::MIN as f64 || floor > i32::MAX as f64 {
        return Err("source index outside finite i32 conversion scope");
    }
    let table_index = (floor as i32).clamp(0,264) as usize;
    let table_value = table[table_index];
    let correction = f64::from(table_value) / divisor;
    let corrected_coordinate = base_coordinate - correction;
    let square = corrected_coordinate * corrected_coordinate;
    let theta = CALIBRATION_COEFFICIENT_015 * corrected_coordinate;
    let first_cos = finite(math.cos(theta))?;
    let y = first_cos * CALIBRATION_COEFFICIENT_016;
    let mut t = CALIBRATION_COEFFICIENT_017 * corrected_coordinate;
    t += CALIBRATION_COEFFICIENT_018;
    t = finite(math.cos(t))?;
    t *= y;
    let mut y = CALIBRATION_COEFFICIENT_019 * square;
    y += t;
    y += CALIBRATION_COEFFICIENT_020;
    let mut p = CALIBRATION_COEFFICIENT_021 * corrected_coordinate;
    p += y;
    let mut e = CALIBRATION_COEFFICIENT_022 * corrected_coordinate;
    e = finite(math.exp(e))?;
    e *= CALIBRATION_COEFFICIENT_023;
    p = e + p;

    let second_cos = finite(math.cos(theta))?;
    let mut t = CALIBRATION_COEFFICIENT_024 * corrected_coordinate;
    t *= second_cos;
    let shape_coordinate = finite(t + p)?;
    Ok(Calibration {
        relative_transition, band, base_coordinate, table_index, table_value,
        corrected_coordinate, shape_coordinate,
        above_shape_threshold: shape_coordinate > CALIBRATION_COEFFICIENT_025,
    })
}
