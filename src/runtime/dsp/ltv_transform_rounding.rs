use crate::ltv_first_call::{SourceTransformMathProfile,SourceTransformSelectionMath};
const SIGN:u64=0x8000_0000_0000_0000;
const MANTISSA:u64=0x000f_ffff_ffff_ffff;
const ONE:u64=0x3ff0_0000_0000_0000;
const LARGE:u64=0x4340_0000_0000_0000;
const INFINITY:u64=0x7ff0_0000_0000_0000;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TransformRoundingError{
    RequiredNanHelper596284{input_bits:u64},

    ConversionOutsideIntegralSigned32{input_bits:u64},
}
pub fn ceil57dc64_non_nan_bits(bits:u64)->Result<u64,TransformRoundingError>{
    let absolute=bits&!SIGN;let negative=bits!=absolute;
    if absolute>=LARGE{
        if absolute>INFINITY{return Err(TransformRoundingError::RequiredNanHelper596284{input_bits:bits});}
        return Ok(bits);
    }
    if absolute<ONE{
        return Ok(if absolute==0{bits}else if negative{SIGN}else{ONE});
    }

    let low_exponent=(bits>>52) as u8;
    let shift=(0x33_u8.wrapping_sub(low_exponent)&63) as u32;
    let clear_mask=!((1_u64<<shift).wrapping_sub(1));
    let truncated=bits&clear_mask;
    if negative||truncated==bits{return Ok(truncated);}

    let exponent=((truncated>>52)&0x7ff) as i32-1023;
    let integer=((1_u64<<52)|(truncated&MANTISSA))>>(52-exponent as u32);
    Ok(pack_positive_integer(integer+1))
}
fn pack_positive_integer(value:u64)->u64{
    let highest=63-value.leading_zeros();
    let significand=if highest<=52{value<<(52-highest)}else{value>>(highest-52)};
    (((highest as u64)+1023)<<52)|(significand&MANTISSA)
}

pub fn cvttsd2si_integral_i32(bits:u64)->Result<i32,TransformRoundingError>{
    let reject=||TransformRoundingError::ConversionOutsideIntegralSigned32{input_bits:bits};
    let absolute=bits&!SIGN;let negative=bits&SIGN!=0;
    if absolute==0{return Ok(0);}
    let raw_exponent=(absolute>>52)&0x7ff;
    if raw_exponent==0||raw_exponent==0x7ff{return Err(reject());}
    let exponent=raw_exponent as i32-1023;
    if !(0..=31).contains(&exponent){return Err(reject());}
    let fraction_bits=52-exponent as u32;
    let significand=(1_u64<<52)|(absolute&MANTISSA);
    if significand&((1_u64<<fraction_bits)-1)!=0{return Err(reject());}
    let magnitude=significand>>fraction_bits;
    let limit=if negative{0x8000_0000_u64}else{0x7fff_ffff_u64};
    if magnitude>limit{return Err(reject());}
    Ok(if negative{(0_u32.wrapping_sub(magnitude as u32)) as i32}else{magnitude as i32})
}

pub trait SourceTransformLog582ff8{
    type Error;
    fn explicit_profile(&self)->SourceTransformMathProfile;
    fn evaluate582ff8(&mut self,input:f64)->Result<f64,Self::Error>;
}
pub enum TransformSelectionServiceError<E>{Logarithm(E),Rounding(TransformRoundingError)}
pub struct SourceTransformMathWithBoundCeil<L>{logarithm:L}
impl<L> SourceTransformMathWithBoundCeil<L>{
    pub fn from_explicit_logarithm(logarithm:L)->Self{Self{logarithm}}
    pub fn into_logarithm(self)->L{self.logarithm}
}
impl<L:SourceTransformLog582ff8> SourceTransformSelectionMath for SourceTransformMathWithBoundCeil<L>{
    type Error=TransformSelectionServiceError<L::Error>;
    fn explicit_profile(&self)->SourceTransformMathProfile{self.logarithm.explicit_profile()}
    fn helper582ff8(&mut self,input:f64)->Result<f64,Self::Error>{self.logarithm.evaluate582ff8(input).map_err(TransformSelectionServiceError::Logarithm)}
    fn helper57dc64(&mut self,input:f64)->Result<f64,Self::Error>{
        ceil57dc64_non_nan_bits(input.to_bits()).map(f64::from_bits).map_err(TransformSelectionServiceError::Rounding)
    }
    fn cvttsd2si_i32(&mut self,input:f64)->Result<i32,Self::Error>{
        cvttsd2si_integral_i32(input.to_bits()).map_err(TransformSelectionServiceError::Rounding)
    }
}
