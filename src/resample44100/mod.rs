#![allow(dead_code)]

mod finite_rounding;
mod scalar_math;
mod scalar_coefficients;
mod tap_calibration_data;
mod tap_calibration;
mod time_tap_data;
mod time_taps;
mod fractional_profile_data;
mod fractional_stencil;
mod fractional_table;
mod rational_147_80;
mod rational_fir;
mod generic_fractional;
mod ordinary_selector;
mod scalar_hyperbolic;
mod atan2_table;
mod scalar_angle;
mod source_chain;
mod source_reader;
mod ooura_table_f64;
mod fir_input;
mod fir_output;
mod fir_layout;
mod linear_phase_spectrum;
mod fir_phase0;
mod ordinary_44100_chain;
mod ordinary_44100_reader;
mod single_fir_reader;
mod halfband_up2;
mod halfband_down2;

use fractional_table::FractionalMath;
use ooura_table_f64::Trig64;
use ordinary_44100_chain::OrdinaryMathNames;
use tap_calibration::CalibrationMath;
use time_taps::{DesignMath, WideDesignMath};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    WindowsHostF64SourceOrder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedPlatform,
    UnsupportedFloatEnvironment { mxcsr: u32 },
    NumericDomain(&'static str),
}

fn check_mxcsr(mxcsr: u32) -> Result<(), Error> {

    if mxcsr & 0xe040 != 0 || mxcsr & 0x1f80 != 0x1f80 {
        return Err(Error::UnsupportedFloatEnvironment { mxcsr });
    }
    Ok(())
}

pub(crate) fn validate_environment(policy: Policy) -> Result<(), Error> {
    match policy {
        Policy::WindowsHostF64SourceOrder => {}
    }
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        let mut mxcsr = 0u32;
        unsafe {
            std::arch::asm!("stmxcsr [{p}]", p = in(reg) &mut mxcsr,
                options(nostack, preserves_flags));
        }
        check_mxcsr(mxcsr)
    }
    #[cfg(not(all(windows, target_arch = "x86_64")))]
    {
        Err(Error::UnsupportedPlatform)
    }
}

struct WindowsHostMath;

impl CalibrationMath for WindowsHostMath {
    fn cos(&mut self, value: f64) -> f64 { value.cos() }
    fn exp(&mut self, value: f64) -> f64 { value.exp() }
}

impl DesignMath for WindowsHostMath {
    fn log(&mut self, value: f64) -> f64 { value.ln() }
    fn sin(&mut self, value: f64) -> f64 { value.sin() }
    fn pow(&mut self, value: f64, exponent: f64) -> f64 { value.powf(exponent) }
}

impl WideDesignMath for WindowsHostMath {
    fn tan(&mut self, value: f64) -> f64 { value.tan() }
    fn cosh(&mut self, value: f64) -> f64 { value.cosh() }
}

impl FractionalMath for WindowsHostMath {
    fn exp(&mut self, value: f64) -> f64 { value.exp() }
    fn sin(&mut self, value: f64) -> f64 { value.sin() }
    fn pow(&mut self, value: f64, exponent: f64) -> f64 { value.powf(exponent) }
}

impl Trig64 for WindowsHostMath {
    fn sin(&mut self, value: f64) -> f64 { value.sin() }
    fn cos(&mut self, value: f64) -> f64 { value.cos() }
}

const MATH_NAMES: OrdinaryMathNames = OrdinaryMathNames {
    exp: "Windows HOST Rust f64::exp",
    cos: "Windows HOST Rust f64::cos",
    log: "Windows HOST Rust f64::ln",
    sin: "Windows HOST Rust f64::sin",
    pow: "Windows HOST Rust f64::powf",
};

pub(crate) fn output_count(source_count: usize) -> Result<usize, Error> {
    ordinary_44100_reader::short_import_output_count(source_count).map_err(Error::NumericDomain)
}

pub(crate) fn convert_channel0(input: &[f32], policy: Policy) -> Result<Vec<f32>, Error> {
    validate_environment(policy)?;
    ordinary_44100_reader::convert_short_channel0(input, MATH_NAMES, &mut WindowsHostMath)
        .map_err(Error::NumericDomain)
}

fn single_fir_rate(source_rate: u32) -> Result<single_fir_reader::SourceRate, Error> {
    match source_rate {
        4_000 => Ok(single_fir_reader::SourceRate::Hz4000),
        6_000 => Ok(single_fir_reader::SourceRate::Hz6000),
        8_000 => Ok(single_fir_reader::SourceRate::Hz8000),
        11_025 => Ok(single_fir_reader::SourceRate::Hz11025),
        12_000 => Ok(single_fir_reader::SourceRate::Hz12000),
        16_000 => Ok(single_fir_reader::SourceRate::Hz16000),
        22_050 => Ok(single_fir_reader::SourceRate::Hz22050),
        24_000 => Ok(single_fir_reader::SourceRate::Hz24000),
        32_000 => Ok(single_fir_reader::SourceRate::Hz32000),
        48_001 => Ok(single_fir_reader::SourceRate::Hz48001),
        64_000 => Ok(single_fir_reader::SourceRate::Hz64000),
        72_000 => Ok(single_fir_reader::SourceRate::Hz72000),
        88_200 => Ok(single_fir_reader::SourceRate::Hz88200),
        96_000 => Ok(single_fir_reader::SourceRate::Hz96000),
        144_000 => Ok(single_fir_reader::SourceRate::Hz144000),
        176_400 => Ok(single_fir_reader::SourceRate::Hz176400),
        192_000 => Ok(single_fir_reader::SourceRate::Hz192000),
        288_000 => Ok(single_fir_reader::SourceRate::Hz288000),
        352_800 => Ok(single_fir_reader::SourceRate::Hz352800),
        384_000 => Ok(single_fir_reader::SourceRate::Hz384000),
        _ => Err(Error::NumericDomain("unsupported single-FIR source rate")),
    }
}

pub(crate) fn single_fir_output_count(source_count: usize, source_rate: u32)
    -> Result<usize, Error>
{
    single_fir_reader::short_import_output_count(source_count, single_fir_rate(source_rate)?)
        .map_err(Error::NumericDomain)
}

pub(crate) fn convert_single_fir_channel0(input: &[f32], source_rate: u32, policy: Policy)
    -> Result<Vec<f32>, Error>
{
    validate_environment(policy)?;
    if source_rate == 48_001 {
        return source_reader::convert_source_channel0(
            input, source_rate, MATH_NAMES, &mut WindowsHostMath,
        ).map_err(Error::NumericDomain);
    }
    single_fir_reader::convert_short_channel0(
        input, single_fir_rate(source_rate)?, MATH_NAMES, &mut WindowsHostMath,
    ).map_err(Error::NumericDomain)
}

pub(crate) fn convert_borrowed_wav_channel0(
    source: crate::wav48k::Channel0Cursor<'_>, policy: Policy,
) -> Result<(crate::wav48k::Metadata, Vec<f32>), Error> {
    if source.metadata().sample_rate != 48_000 { validate_environment(policy)?; }
    source_reader::convert_wave_channel0(source, MATH_NAMES, &mut WindowsHostMath)
        .map_err(Error::NumericDomain)
}

pub(crate) fn source_plan_output_count(source_count: usize, source_rate: u32)
    -> Result<usize, Error>
{
    source_reader::frame_output_count(source_count, source_rate).map_err(Error::NumericDomain)
}

