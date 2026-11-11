use super::fir_phase0::Phase0Fir;
use super::fractional_profile_data::FALSE_PROFILES;
use super::fractional_table::{rational_80_table,FractionalMath,FractionalMathNames};
use super::linear_phase_spectrum::LinearPhaseSpectrum;
use super::ooura_table_f64::Trig64;
use super::rational_147_80::{Rational147To80,RationalTable28,StreamState};
use super::time_taps::{phase0_narrow_taps,DesignMath};

pub trait Ordinary44100Math:DesignMath+Trig64+FractionalMath {}
impl<T:DesignMath+Trig64+FractionalMath> Ordinary44100Math for T {}

#[derive(Clone,Copy,Debug)]
pub struct OrdinaryMathNames {
    pub exp:&'static str,
    pub cos:&'static str,
    pub log:&'static str,
    pub sin:&'static str,
    pub pow:&'static str,
}

#[derive(Clone,Copy,Debug)]
pub struct Construction {
    pub generated_tap_count:usize,
    pub filter_fft_len:usize,
    pub first_stage_input_fft_len:usize,
    pub first_stage_output_fft_len:usize,
    pub first_stage_block_len:usize,
    pub first_stage_history_len:usize,
    pub first_stage_discard:usize,
    pub first_stage_fractional_latency:f64,
    pub fractional_profile:[f64;3],
    pub fractional_initialized_words:usize,
    pub startup_probe_extra_samples:usize,
    pub math_services:OrdinaryMathNames,
}

pub struct Ordinary44100Chain {
    first:Phase0Fir,
    second:Rational147To80,
    pub construction:Construction,
}

impl Ordinary44100Chain {
    pub fn new<M:Ordinary44100Math>(names:OrdinaryMathNames,math:&mut M)
        ->Result<Self,&'static str> {
        if [names.exp,names.cos,names.log,names.sin,names.pow].iter().any(|x|x.is_empty()) {
            return Err("explicit named source CRT/trigonometric services required");
        }
        let attenuation=f64::from_bits(0x4069dd1eb851eb85);
        let taps=phase0_narrow_taps(0.5,2.0,attenuation,math)?;
        let tap_count=taps.taps.len();
        let spectrum=LinearPhaseSpectrum::from_taps(&taps.taps,2.0,math)?;
        let first=Phase0Fir::from_generated_spectrum(spectrum,2,1,0.0)?;

        let input_latency=first.layout.fractional_latency;

        let table=rational_80_table(FALSE_PROFILES[10][2],FractionalMathNames {
            exp:names.exp,sin:names.sin,pow:names.pow,
        },math)?;
        let construction=Construction {
            generated_tap_count:tap_count,
            filter_fft_len:first.layout.fft_len,
            first_stage_input_fft_len:first.layout.input_fft_len,
            first_stage_output_fft_len:first.layout.output_fft_len,
            first_stage_block_len:first.layout.block_len,
            first_stage_history_len:first.layout.history_len,
            first_stage_discard:first.layout.initial_discard,
            first_stage_fractional_latency:input_latency,
            fractional_profile:table.profile,
            fractional_initialized_words:table.coefficients.len(),
            startup_probe_extra_samples:0,
            math_services:names,
        };
        let second=Rational147To80::new(
            RationalTable28::from_coeff1_stencil2(table.coefficients)?,input_latency)?;
        let mut value=Self {first,second,construction};

        let mut extra=0;
        loop {
            if !value.push_f64_no_drain(&[0.0],math)?.is_empty() {break;}
            extra+=1;
            if extra>1<<20 {return Err("own constructor startup-probe resource bound exceeded");}
        }
        value.reset()?;
        value.construction.startup_probe_extra_samples=extra;
        Ok(value)
    }

    pub fn reset(&mut self)->Result<(),&'static str> {
        self.first.reset()?;
        self.second.reset();
        Ok(())
    }

    pub fn fractional_state(&self)->StreamState {self.second.state()}

    pub fn pending_fir_discard(&self)->i32 {self.first.pending_discard()}

    pub fn push_f64_no_drain<M:Ordinary44100Math>(&mut self,input:&[f64],math:&mut M)
        ->Result<Vec<f64>,&'static str> {
        if input.iter().any(|x|!x.is_finite()) {
            return Err("finite source input scope required");
        }
        let intermediate=self.first.process(input,math)?;
        let mut output=Vec::new();
        self.second.process_into(&intermediate,&mut output)?;
        Ok(output)
    }
}
