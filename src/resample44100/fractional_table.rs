use super::finite_rounding::ceil_finite;
use super::fractional_profile_data::*;
use super::fractional_stencil::QuadraticCoefficients;
use super::time_tap_data::{WINDOW_COEFFICIENT_012,WINDOW_COEFFICIENT_013,WINDOW_COEFFICIENT_014,WINDOW_COEFFICIENT_015,WINDOW_COEFFICIENT_016,WINDOW_COEFFICIENT_017,
    WINDOW_COEFFICIENT_018,WINDOW_COEFFICIENT_019,WINDOW_COEFFICIENT_020,WINDOW_COEFFICIENT_021,WINDOW_COEFFICIENT_022,WINDOW_COEFFICIENT_023,WINDOW_COEFFICIENT_024,WINDOW_COEFFICIENT_025,
    WINDOW_COEFFICIENT_026,WINDOW_COEFFICIENT_027,WINDOW_COEFFICIENT_000};

pub trait FractionalMath {
    fn exp(&mut self,value:f64)->f64;
    fn sin(&mut self,value:f64)->f64;
    fn pow(&mut self,value:f64,exponent:f64)->f64;
}

pub struct GenericTable {
    pub variant:bool,
    pub profile_index:usize,
    pub profile:[f64;3],
    pub normalized_rows:Vec<f64>,
    pub table:QuadraticCoefficients,
}

fn finite(value:f64)->Result<f64,&'static str> {
    if value.is_finite() {Ok(value)} else {Err("finite source fractional-table domain required")}
}
fn small_i0_polynomial(square:f64)->f64 {
    let mut value=WINDOW_COEFFICIENT_022*square;
    for (index,coefficient) in [WINDOW_COEFFICIENT_023,WINDOW_COEFFICIENT_024,WINDOW_COEFFICIENT_025,WINDOW_COEFFICIENT_026,WINDOW_COEFFICIENT_027,1.0].into_iter().enumerate() {
        value+=coefficient;
        if index<5 {value*=square;}
    }
    value
}
fn large_i0_polynomial(ratio:f64)->f64 {
    let mut value=WINDOW_COEFFICIENT_013*ratio;
    for (index,coefficient) in [WINDOW_COEFFICIENT_014,WINDOW_COEFFICIENT_015,WINDOW_COEFFICIENT_016,WINDOW_COEFFICIENT_017,WINDOW_COEFFICIENT_018,WINDOW_COEFFICIENT_019,WINDOW_COEFFICIENT_020,WINDOW_COEFFICIENT_021].into_iter().enumerate() {
        value+=coefficient;
        if index<7 {value*=ratio;}
    }
    value
}
fn signed_power<M:FractionalMath>(value:f64,exponent:f64,math:&mut M)->Result<f64,&'static str> {
    if value<0.0 {
        let absolute=f64::from_bits(value.to_bits()^0x8000_0000_0000_0000);
        let powered=finite(math.pow(absolute,exponent))?;
        Ok(f64::from_bits(powered.to_bits()^0x8000_0000_0000_0000))
    } else {finite(math.pow(value,exponent))}
}
fn window<M:FractionalMath>(radial:f64,beta:f64,exponent:f64,normalizer:f64,math:&mut M)->Result<f64,&'static str> {
    let ratio=if radial<0.0 {0.0} else {
        let x=radial.sqrt()*beta;
        let absolute=f64::from_bits(x.to_bits()&0x7fff_ffff_ffff_ffff);
        let value=if absolute<WINDOW_COEFFICIENT_012 {
            let mut scaled=x/WINDOW_COEFFICIENT_012;
            scaled*=scaled;
            small_i0_polynomial(scaled)
        } else {
            let ratio=WINDOW_COEFFICIENT_012/absolute;
            let scaled=finite(math.exp(absolute))?/absolute.sqrt();
            large_i0_polynomial(ratio)*scaled
        };
        value/normalizer
    };
    signed_power(ratio,exponent,math)
}

pub fn generic_auto_table<M:FractionalMath>(attenuation:f64,variant:bool,math:&mut M)->Result<GenericTable,&'static str> {
    finite(attenuation)?;
    let (thresholds,profiles,base):(&[f64],&[[f64;3]],usize)=if variant {
        (&TRUE_THRESHOLDS,&TRUE_PROFILES,6)
    } else {(&FALSE_THRESHOLDS,&FALSE_PROFILES,8)};
    let profile_index=thresholds.partition_point(|threshold|attenuation>*threshold);
    let profile=profiles[profile_index];
    let taps=base+profile_index*2;
    let exponent_argument=profile[2]/FRACTIONAL_COEFFICIENT_000;
    let count_argument=finite(math.pow(FRACTIONAL_COEFFICIENT_001,exponent_argument))?;
    let count=ceil_finite(count_argument)?;
    if count<8.0 || count>i32::MAX as f64-8.0 {return Err("generic source phase-count candidate scope required");}
    let phases=count as usize;
    let normalized_rows=source_normalized_rows(phases,taps,8,profile,math)?;
    let table=QuadraticCoefficients::from_source_normalized_rows(&normalized_rows,phases,taps)?;
    Ok(GenericTable {variant,profile_index,profile,normalized_rows,table})
}

fn source_normalized_rows<M:FractionalMath>(phases:usize,taps:usize,stencil:usize,
    profile:[f64;3],math:&mut M)->Result<Vec<f64>,&'static str> {
    let sample_words=(phases+stencil).checked_mul(taps).ok_or("source table sample extent overflow")?;
    if sample_words>(1<<20) {return Err("private source table resource bound exceeded");}

    let beta=profile[0].clamp(SCALAR_COEFFICIENT_008,FRACTIONAL_COEFFICIENT_002);
    let exponent=f64::from_bits(profile[1].to_bits()&0x7fff_ffff_ffff_ffff);
    let beta_ratio=WINDOW_COEFFICIENT_012/beta;
    let beta_polynomial=large_i0_polynomial(beta_ratio);
    let beta_sqrt=beta.sqrt();
    let mut beta_square=beta/WINDOW_COEFFICIENT_012;
    beta_square*=beta_square;
    let beta_small=small_i0_polynomial(beta_square);
    let half=(taps/2) as i32;
    let half_float=half as f64;
    let mut normalized_rows=Vec::with_capacity(sample_words);
    let half_stencil=(stencil/2) as i32;
    for index in 1-half_stencil..=phases as i32+half_stencil {

        let normalizer=if beta<WINDOW_COEFFICIENT_012 {beta_small} else {
            let mut value=finite(math.exp(beta))?/beta_sqrt;
            value*=beta_polynomial;
            value
        };
        let remaining=phases as i32-index;
        let fraction=remaining as f64/phases as f64;
        let fractional_position=fraction/half_float;
        let mut sine_argument=fraction*WINDOW_COEFFICIENT_000;
        sine_argument=finite(math.sin(sine_argument))?;
        let sine=sine_argument/WINDOW_COEFFICIENT_000;
        let negative_sine=f64::from_bits(sine.to_bits()^0x8000_0000_0000_0000);
        let near_one=(fraction+(-1.0)).abs()<SOURCE_NEAR_ZERO;
        let near_zero=fraction.abs()<SOURCE_NEAR_ZERO;
        let start=normalized_rows.len();
        for integer in -half..half {

            if (integer==-half && fraction<0.0) || (integer==half-1 && fraction+integer as f64>half_float) {
                normalized_rows.push(0.0);
                continue;
            }
            let mut position=integer as f64/half_float;
            position+=fractional_position;
            position*=position;
            let radial=SCALAR_COEFFICIENT_008-position;
            let window=window(radial,beta,exponent,normalizer,math)?;
            let value=if (near_one && integer==-1) || (near_zero && integer==0) {window} else {
                let alternating=if integer&1==0 {sine} else {negative_sine};
                let numerator=alternating*window;
                numerator/(integer as f64+fraction)
            };
            normalized_rows.push(finite(value)?);
        }
        let row=&mut normalized_rows[start..];
        let mut sum=0.0;
        for value in row.iter() {sum+=*value;}
        let reciprocal=finite(1.0/sum)?;
        for value in row { *value*=reciprocal; }
    }
    Ok(normalized_rows)
}

#[derive(Clone,Copy,Debug)]
pub struct FractionalMathNames {
    pub exp: &'static str,
    pub sin: &'static str,
    pub pow: &'static str,
}

pub struct Rational80Table {
    pub profile_index:usize,
    pub profile:[f64;3],
    pub math_services:FractionalMathNames,
    pub coefficients:Vec<f64>,
}

impl Rational80Table {
    pub const PHASES:usize=80;
    pub const TAPS:usize=28;
    pub const STENCIL:usize=2;
    pub const COEFFICIENTS_PER_TAP:usize=1;
    pub const INITIALIZED_WORDS:usize=82*28;
    pub const NATIVE_ALLOCATION_BYTES:usize=Self::INITIALIZED_WORDS*8+64;
}

pub fn rational_80_table<M:FractionalMath>(attenuation:f64,services:FractionalMathNames,
    math:&mut M)->Result<Rational80Table,&'static str> {
    finite(attenuation)?;
    if [services.exp,services.sin,services.pow].iter().any(|x|x.is_empty()) {
        return Err("explicit named fractional math services required");
    }
    let profile_index=FALSE_THRESHOLDS.partition_point(|threshold|attenuation>*threshold);
    if profile_index!=10 {
        return Err("source-selected false-profile28tap rational80 path required");
    }
    let profile=FALSE_PROFILES[profile_index];

    let coefficients=source_normalized_rows(80,28,2,profile,math)?;
    Ok(Rational80Table {profile_index,profile,math_services:services,coefficients})
}

pub fn rational_28_rows<M: FractionalMath>(
    phases: usize,
    attenuation: f64,
    services: FractionalMathNames,
    math: &mut M,
) -> Result<Vec<f64>, &'static str> {
    finite(attenuation)?;
    if phases == 80 {
        return Ok(rational_80_table(attenuation, services, math)?.coefficients);
    }
    if !matches!(phases, 40 | 160) {
        return Err("explicit 40/80/160-phase rational path required");
    }
    if [services.exp, services.sin, services.pow].iter().any(|x| x.is_empty()) {
        return Err("explicit named fractional math services required");
    }
    let profile_index = FALSE_THRESHOLDS.partition_point(|threshold| attenuation > *threshold);
    if profile_index != 10 {
        return Err("source-selected false-profile28tap rational path required");
    }
    source_normalized_rows(phases, 28, 2, FALSE_PROFILES[profile_index], math)
}

pub fn rational_22_rows<M: FractionalMath>(
    phases: usize,
    attenuation: f64,
    services: FractionalMathNames,
    math: &mut M,
) -> Result<Vec<f64>, &'static str> {
    finite(attenuation)?;
    if phases != 40 {
        return Err("explicit 40-phase true-profile rational path required");
    }
    if [services.exp, services.sin, services.pow].iter().any(|x| x.is_empty()) {
        return Err("explicit named fractional math services required");
    }
    let profile_index = TRUE_THRESHOLDS.partition_point(|threshold| attenuation > *threshold);
    if profile_index != 8 {
        return Err("source-selected true-profile22tap rational path required");
    }
    source_normalized_rows(phases, 22, 2, TRUE_PROFILES[profile_index], math)
}

pub fn source_rational_rows<M: FractionalMath>(
    phases: usize,
    variant: bool,
    services: FractionalMathNames,
    math: &mut M,
) -> Result<Vec<f64>, &'static str> {
    if phases == 0 || phases >= 1501 {
        return Err("positive source rational phase count below1501 required");
    }
    if [services.exp, services.sin, services.pow].iter().any(|x| x.is_empty()) {
        return Err("explicit named fractional math services required");
    }
    if variant {
        source_normalized_rows(phases, 22, 2, TRUE_PROFILES[8], math)
    } else {
        source_normalized_rows(phases, 28, 2, FALSE_PROFILES[10], math)
    }
}
