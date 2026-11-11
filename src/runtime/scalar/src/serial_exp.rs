use super::finite_power::{MathErrorCallbackCompletion, round_exact_normal_u128};
use super::retained_exp_constants::{EXP_TABLE, LN2_DIV64, POLY_HALF, POLY_THIRD};
use super::selected_exp::{SourceExpEntry, SourceExpPlan, SourceExpWithState, rne_i32_lane};
use super::vorbis_power::{Arithmetic, VorbisScalarError};

const SIGN: u64 = 0x8000_0000_0000_0000;

pub const SOURCE_RADIAL_EXP_UPPER_F64: u64 = 0x8000_0000_8000_0000;

pub(super) enum SerialExpError {
    UpperRequired { bits: u64, entry: SourceExpEntry },
    OrdinaryRequired { entry: SourceExpEntry },
    Arithmetic(VorbisScalarError),
}

pub(super) struct SerialExpSites(pub(super) [&'static str; 10]);

pub(super) fn ordinary(entry: SourceExpEntry, upper: u64, sites: &SerialExpSites)
    -> Result<SourceExpWithState, SerialExpError>
{
    if upper & !SIGN != 0 && upper != SOURCE_RADIAL_EXP_UPPER_F64 {
        return Err(SerialExpError::UpperRequired { bits: upper, entry });
    }
    let SourceExpPlan::Ordinary { input_bits, scaled_bits } = entry.plan
        else { return Err(SerialExpError::OrdinaryRequired { entry }); };
    let radial = upper == SOURCE_RADIAL_EXP_UPPER_F64;
    let mut a = Arithmetic { state: entry.exception_state };

    let (integer, flags) = rne_i32_lane(scaled_bits);
    a.state = a.state.with_source_status(flags);
    let n = if integer == 0 { 0 }
        else { round_exact_normal_u128(u128::from(integer.unsigned_abs()), 0).0 };
    let n = if integer < 0 { n | SIGN } else { n };
    let (wide, _) = a.state.widen_f32_for_math_error(input_bits);

    let product = a.multiply(sites.0[0], LN2_DIV64.to_bits(), n)
        .map_err(SerialExpError::Arithmetic)?;
    let r = a.subtract(sites.0[1], wide, product)
        .map_err(SerialExpError::Arithmetic)?;

    if radial { a.state = a.state.with_source_status(2); }
    let p = a.multiply(sites.0[2], POLY_THIRD.to_bits(), r)
        .map_err(SerialExpError::Arithmetic)?;

    if radial { a.state = a.state.with_source_status(2); }
    let square = a.multiply(sites.0[3], r, r)
        .map_err(SerialExpError::Arithmetic)?;

    if radial { a.state = a.state.with_source_status(0x32); }
    let p = a.add(sites.0[4], p, POLY_HALF.to_bits())
        .map_err(SerialExpError::Arithmetic)?;
    let reduced = a.multiply(sites.0[5], square, p)
        .map_err(SerialExpError::Arithmetic)?;
    let reduced = a.add(sites.0[6], reduced, r)
        .map_err(SerialExpError::Arithmetic)?;

    let index = (integer as u32) & 63;
    let scale_exponent = (integer - index as i32) >> 6;
    let table = EXP_TABLE[index as usize].to_bits();
    let expanded = a.multiply(sites.0[7], reduced, table)
        .map_err(SerialExpError::Arithmetic)?;
    let expanded = a.add(sites.0[8], expanded, table)
        .map_err(SerialExpError::Arithmetic)?;
    let scale = ((scale_exponent + 1023) as u64) << 52;
    let result = a.multiply(sites.0[9], expanded, scale)
        .map_err(SerialExpError::Arithmetic)?;
    let result = MathErrorCallbackCompletion { result_f64_bits: result, advanced: a.state }
        .into_rne_f32().map_err(|required| SerialExpError::Arithmetic(
            VorbisScalarError::ConversionRequired(required)))?;
    Ok(SourceExpWithState { bits: result.bits, exception_state: result.advanced })
}
