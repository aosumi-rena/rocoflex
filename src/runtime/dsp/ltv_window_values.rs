use crate::ltv_block::SourceLtvBlockConfig;
pub trait SourceLtvWindowCosine{
    type Error;
    fn cosf_57e330(&mut self,angle:f32)->Result<f32,Self::Error>;
}
pub enum LtvWindowValueError<E>{Math(E),Nonfinite,Normalization}

const TWO_PI_BITS:u32=0x40c9_0fdb;
const ONE_BITS:u32=0x3f80_0000;
const HALF_BITS:u32=0x3f00_0000;
pub fn normalization<M:SourceLtvWindowCosine>(config:SourceLtvBlockConfig,math:&mut M)->Result<f32,LtvWindowValueError<M::Error>>{
    let denominator=config.window_length() as f32;
    let two_pi=f32::from_bits(TWO_PI_BITS);let one=f32::from_bits(ONE_BITS);let half=f32::from_bits(HALF_BITS);
    let mut sum=0.0_f32;

    for i in (0..config.window_length()).step_by(config.hop()){
        let ratio=(i as f32)/denominator;let angle=ratio*two_pi;
        let cosine=math.cosf_57e330(angle).map_err(LtvWindowValueError::Math)?;
        let difference=one-cosine;let weighted=difference*half;sum=sum+weighted;
        if !cosine.is_finite()||!sum.is_finite(){return Err(LtvWindowValueError::Nonfinite);}
    }
    if sum<=0.0{return Err(LtvWindowValueError::Normalization);}
    Ok(sum)
}

pub fn write_samples<M:SourceLtvWindowCosine>(config:SourceLtvBlockConfig,sum:f32,math:&mut M,
    store:&mut impl FnMut(usize,f32))->Result<(),LtvWindowValueError<M::Error>>{
    let denominator=config.window_length() as f32;
    let two_pi=f32::from_bits(TWO_PI_BITS);let one=f32::from_bits(ONE_BITS);let half=f32::from_bits(HALF_BITS);
    for i in 0..config.window_length(){
        let ratio=(i as f32)/denominator;let angle=ratio*two_pi;
        let cosine=math.cosf_57e330(angle).map_err(LtvWindowValueError::Math)?;
        let difference=one-cosine;let weighted=difference*half;let value=weighted/sum;
        if !cosine.is_finite()||!value.is_finite(){return Err(LtvWindowValueError::Nonfinite);}
        store(i,value);
    }
    Ok(())
}
