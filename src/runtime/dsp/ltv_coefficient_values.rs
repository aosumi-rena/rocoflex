use crate::ltv_block::{LtvFftDirection,SourceLtvFft};
#[path="ltv_coefficient_complex.rs"]
pub mod complex;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum CoefficientError {WrongClass,UnsupportedTransform,AlreadyEnded,OutsidePositiveRowDomain,OddWidth,WidthExceedsTransform,Fft,Math}
pub trait SourceCoefficientMath {
    fn scalar_57f8e0(&mut self,value:f32)->std::result::Result<f32,CoefficientError>;

    fn complex_516c0(&mut self,real:f32,imag:f32)->std::result::Result<[f32;2],CoefficientError>;
}

pub fn transform_values_into(transform:usize,values:&[f32],packed:&mut[f32],fft:&mut dyn SourceLtvFft,
    math:&mut dyn SourceCoefficientMath)->std::result::Result<(),CoefficientError>{
    if values.is_empty(){return Err(CoefficientError::OutsidePositiveRowDomain);}
    if values.len()>transform{return Err(CoefficientError::WidthExceedsTransform);}
    if values.len()%2!=0{return Err(CoefficientError::OddWidth);}
    if packed.len()!=transform{return Err(CoefficientError::UnsupportedTransform);}
    packed.fill(0.0);let half=values.len()/2;
    for index in 0..=half{
        let numerator=values[index]*10.0f32;packed[index]=numerator/((index+1) as f32);
    }
    for index in 0..half-1{
        let numerator=values[half+1+index]*10.0f32;
        packed[transform+1-half+index]=numerator/((half-index) as f32);
    }
    fft.transform_in_place(LtvFftDirection::ForwardPlus1,packed).map_err(|_|CoefficientError::Fft)?;
    packed[0]=math.scalar_57f8e0(packed[0])?;
    packed[1]=math.scalar_57f8e0(packed[1])?;
    for bin in 1..transform/2{
        let pair=math.complex_516c0(packed[2*bin],packed[2*bin+1])?;
        packed[2*bin]=pair[0];packed[2*bin+1]=pair[1];
    }
    Ok(())
}
