mod data { include!("log_data.rs"); }

#[derive(Debug,PartialEq)]
pub enum Error { UnsupportedCpuProfile, UnsupportedEnvironment, PositiveFiniteRequired }

pub struct ScalarPositiveLog;
impl ScalarPositiveLog {
    pub fn bind(explicit_source_cpu_word:u32, mxcsr:u32)->Result<Self,Error> {
        if explicit_source_cpu_word!=0 {return Err(Error::UnsupportedCpuProfile);}
        if mxcsr & 0xffc0 != 0x1f80 {return Err(Error::UnsupportedEnvironment);}
        Ok(Self)
    }
    pub fn evaluate(&self,x:f32)->Result<f32,Error> {
        if !x.is_finite() || x<=0.0 {return Err(Error::PositiveFiniteRequired);}
        let mut word=x.to_bits();
        let mut exponent=((word>>23) as i32-127) as f32;
        let mut mantissa=word&0x007fffff;
        let difference=x-1.0;
        if exponent == -127.0 {
            let normalized=f32::from_bits(mantissa|0x3f800000)-1.0;
            let normalized_bits=normalized.to_bits();
            mantissa=normalized_bits&0x007fffff;
            word=mantissa;
            exponent=((normalized_bits>>23) as i32-253) as f32;
        }
        let index_word=(word&0x007f0000)+((word&0x00008000)<<1);
        if f32::from_bits(difference.to_bits()&0x7fffffff)<f32::from_bits(0x3d800000) {
            let delta=x-1.0;
            let denominator=2.0+delta;
            let mut ratio=delta/denominator;
            let correction=delta*ratio;
            ratio=ratio+ratio;
            let square=ratio*ratio;
            let cube=ratio*square;
            let polynomial=(square*f32::from_bits(0x3c4ccccd))+f32::from_bits(0x3daaaaab);
            let polynomial=polynomial*cube;
            let polynomial=polynomial-correction;
            return Ok(delta+polynomial);
        }
        let index=(index_word>>16) as usize;
        let normalized=f32::from_bits(mantissa|0x3f000000);
        let rounded=f32::from_bits(index_word|0x3f000000);
        let residual=(rounded-normalized)*f32::from_bits(data::RECIPROCAL[index]);
        let polynomial=residual*f32::from_bits(0x3eaaaaab);
        let square=residual*residual;
        let polynomial=polynomial+f32::from_bits(0x3f000000);
        let polynomial=polynomial*square;
        let low=f32::from_bits(0x3805fdf4)*exponent;
        let residual=residual+polynomial;
        let low=low-residual;
        let high=f32::from_bits(0x3f317000)*exponent;
        let low=low+f32::from_bits(data::LOG_LOW[index]);
        let high=high+f32::from_bits(data::LOG_HIGH[index]);
        Ok(high+low)
    }
}

