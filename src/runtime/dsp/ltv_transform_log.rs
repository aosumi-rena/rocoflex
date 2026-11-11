use crate::ltv_first_call::SourceTransformMathProfile;
use crate::ltv_transform_rounding::{SourceTransformLog582ff8,cvttsd2si_integral_i32};
use crate::vector_environment::{check_current_vector_environment,VectorError};
pub enum TransformLogError{Environment(VectorError),DeclaredEnvironment,OutsidePositiveIntegralSigned32,NonfiniteResult}
pub struct SourcePositiveIntegerTransformLog{profile:SourceTransformMathProfile}
impl SourcePositiveIntegerTransformLog{
    pub fn from_explicit_profile(profile:SourceTransformMathProfile)->Result<Self,TransformLogError>{
        if profile.mxcsr&0xffff_ffc0!=0x1f80{return Err(TransformLogError::DeclaredEnvironment);}
        Ok(Self{profile})
    }
    pub fn evaluate(&mut self,input:f64)->Result<f64,TransformLogError>{
        let bits=input.to_bits();
        if cvttsd2si_integral_i32(bits).ok().is_none_or(|v|v<=0){return Err(TransformLogError::OutsidePositiveIntegralSigned32);}
        check_current_vector_environment().map_err(TransformLogError::Environment)?;
        let result=evaluate_normal_graph(input);
        if !result.is_finite(){return Err(TransformLogError::NonfiniteResult);}Ok(result)
    }
}
impl SourceTransformLog582ff8 for SourcePositiveIntegerTransformLog{
    type Error=TransformLogError;
    fn explicit_profile(&self)->SourceTransformMathProfile{self.profile}
    fn evaluate582ff8(&mut self,input:f64)->Result<f64,Self::Error>{self.evaluate(input)}
}
pub enum PositiveNormalLog2Error {
    DeclaredEnvironment,
    OutsidePositiveNormal { input_bits: u64 },
    Environment(VectorError),
    NonfiniteResult { input_bits: u64, result_bits: u64 },
}
pub struct PositiveNormalLog2BindingFailure {
    pub profile: SourceTransformMathProfile, pub cause: PositiveNormalLog2Error,
}

pub struct SourcePositiveNormalLog2 { profile: SourceTransformMathProfile }
impl SourcePositiveNormalLog2 {
    pub fn from_explicit_profile(profile: SourceTransformMathProfile)
        -> Result<Self, PositiveNormalLog2BindingFailure>
    {
        if profile.mxcsr & 0xffff_ffc0 != 0x1f80 {
            return Err(PositiveNormalLog2BindingFailure {
                profile, cause: PositiveNormalLog2Error::DeclaredEnvironment });
        }
        Ok(Self { profile })
    }
    pub fn explicit_profile(&self) -> SourceTransformMathProfile { self.profile }
    pub fn evaluate(&mut self, input: f64) -> Result<f64, PositiveNormalLog2Error> {
        let bits = input.to_bits();
        let exponent = (bits >> 52) & 0x7ff;
        if bits >> 63 != 0 || exponent == 0 || exponent == 0x7ff {
            return Err(PositiveNormalLog2Error::OutsidePositiveNormal { input_bits: bits });
        }
        check_current_vector_environment().map_err(PositiveNormalLog2Error::Environment)?;
        let result = evaluate_normal_graph(input);
        if !result.is_finite() {
            return Err(PositiveNormalLog2Error::NonfiniteResult {
                input_bits: bits, result_bits: result.to_bits() });
        }
        Ok(result)
    }
}

fn evaluate_normal_graph(input: f64) -> f64 {
    let bits = input.to_bits();
    if bits.wrapping_add(0xc011_f056_0000_0000) <= 0x0002_f882_0000_0000 {
        near_one(input)
    } else { ordinary(bits) }
}
fn c(bits:u64)->f64{f64::from_bits(bits)}
fn near_one(input:f64)->f64{

    let mut x5=input-c(0x3ff0_0000_0000_0000);
    let mut x1=x5;let mut x0=x5;
    x0=x0+c(0x4000_0000_0000_0000);
    let mut x4=f64::from_bits(x5.to_bits()&0xffff_ffff_0000_0000);
    x1=x1/x0;let mut x2=x1;x1=x1+x1;x2=x2*x5;x5=x5-x4;
    x0=x1;x0=x0*x1;
    let mut x3=x0;x3=x3*c(0x3f3c_8034_c85d_fff0);x3=x3+c(0x3f62_4923_07f1_519f);
    x3=x3*x0;x3=x3+c(0x3f89_9999_99ba_c6d4);x3=x3*x0;
    x0=x0*x1;x3=x3+c(0x3fb5_5555_5555_54e6);x3=x3*x0;x3=x3-x2;
    x2=x4;x2=x2*c(0x3ecb_295c_17f0_bbbe);x4=x4*c(0x3ff7_1544_0000_0000);
    x3=x3+x5;x0=x3;x3=x3*c(0x3ff7_1544_0000_0000);x0=x0*c(0x3ecb_295c_17f0_bbbe);
    x0=x0+x2;x0=x0+x3;x0=x0+x4;x0
}
fn ordinary(bits:u64)->f64{

    let normalized=(bits&0x000f_ffff_ffff_ffff)|0x3fe0_0000_0000_0000;
    let index=(((bits>>46)&0x3f)|0x40)+((bits>>45)&1);
    let exponent=(((bits>>52)&0x7ff) as i32)-1023;
    let offset=(index-64) as usize;
    let mut x6=c(HEAD[offset]);let mut x2=c(normalized);
    let mut x1=index as f64;x1=x1*c(0x3f80_0000_0000_0000);x2=x2-x1;
    let mut x0=x2;x0=x0*c(0x3fe0_0000_0000_0000);x0=x0+x1;x2=x2/x0;
    x0=exponent as f64;x1=x2;x1=x1*x2;
    let mut x5=x1;x5=x5*c(0x3f62_4942_3bd9_4741);x5=x5+c(0x3f89_9999_9986_5ede);
    x5=x5*x1;x5=x5+c(0x3fb5_5555_5555_5557);x5=x5*x1;
    x1=x6;x6=x6*c(0x3ecb_295c_17f0_bbbe);x1=x1*c(0x3ff7_1544_0000_0000);
    x5=x5*x2;x0=x0+x1;x5=x5+x2;x5=x5+c(TAIL[offset]);
    let mut x4=x5;x5=x5*c(0x3ff7_1544_0000_0000);x4=x4*c(0x3ecb_295c_17f0_bbbe);
    x4=x4+x6;x4=x4+x5;x0=x0+x4;x0
}

const HEAD:[u64;65]=[
0x0000000000000000,0x3f8fc0a800000000,0x3f9f829800000000,0x3fa7745800000000,
0x3faf0a3000000000,0x3fb341d700000000,0x3fb6f0d200000000,0x3fba926d00000000,
0x3fbe270700000000,0x3fc0d77e00000000,0x3fc2955280000000,0x3fc44d2b00000000,
0x3fc5ff3000000000,0x3fc7ab8900000000,0x3fc9525a80000000,0x3fcaf3c900000000,
0x3fcc8ff780000000,0x3fce270700000000,0x3fcfb91800000000,0x3fd0a324c0000000,
0x3fd1675c80000000,0x3fd22941c0000000,0x3fd2e8e280000000,0x3fd3a64c40000000,
0x3fd4618bc0000000,0x3fd51aad80000000,0x3fd5d1bd80000000,0x3fd686c800000000,
0x3fd739d7c0000000,0x3fd7eaf800000000,0x3fd89a3380000000,0x3fd9479400000000,
0x3fd9f323c0000000,0x3fda9cec80000000,0x3fdb44f740000000,0x3fdbeb4d80000000,
0x3fdc8ff7c0000000,0x3fdd32fe40000000,0x3fddd46a00000000,0x3fde744240000000,
0x3fdf128f40000000,0x3fdfaf5880000000,0x3fe02552a0000000,0x3fe0723e40000000,
0x3fe0be72e0000000,0x3fe109f380000000,0x3fe154c3c0000000,0x3fe19ee6a0000000,
0x3fe1e85f40000000,0x3fe23130c0000000,0x3fe2795e00000000,0x3fe2c0e9e0000000,
0x3fe307d720000000,0x3fe34e2880000000,0x3fe393e0c0000000,0x3fe3d90260000000,
0x3fe41d8fe0000000,0x3fe4618bc0000000,0x3fe4a4f840000000,0x3fe4e7d800000000,
0x3fe52a2d20000000,0x3fe56bf9c0000000,0x3fe5ad4040000000,0x3fe5ee02a0000000,
0x3fe62e42e0000000,
];
const TAIL:[u64;65]=[
0x0000000000000000,0x3e361f807c79f3db,0x3e6873c1980267c8,0x3e5ec65b9f88c69e,
0x3e58022c54cc2f99,0x3e62c37a3a125330,0x3e615cad69737c93,0x3e4d256ab1b285e9,
0x3e5b8abcb97a7aa2,0x3e6f34239659a5dc,0x3e6e07fd48d30177,0x3e6b32df4799f4f6,
0x3e6c29e4f4f21cf8,0x3e1086c848df1b59,0x3e4cf456b4764130,0x3e63a02ffcb63398,
0x3e61e6a6886b0976,0x3e6b8abcb97a7aa2,0x3e6b578f8aa35552,0x3e6139c871afb9fc,
0x3e65d5d30701ce64,0x3e6de7bcb2d12142,0x3e6d708e984e1664,0x3e556945e9c72f36,
0x3e20e2f613e85bda,0x3e3cb7e0b42724f6,0x3e6fac04e52846c7,0x3e5e9b14aec442be,
0x3e6b5de8034e7126,0x3e6dc157e1b259d3,0x3e3b05096ad69c62,0x3e5c2116faba4cdd,
0x3e665fcc25f95b47,0x3e5a9a08498d4850,0x3e6de647b1465f77,0x3e5da71b7bf7861d,
0x3e3e6a6886b09760,0x3e6f0075eab0ef64,0x3e33071282fb989b,0x3e60eb43c3f1bed2,
0x3e5faf06ecb35c84,0x3e4ef1e63db35f68,0x3e469743fb1a71a5,0x3e6c1cdf404e5796,
0x3e4094aa0ada625e,0x3e6e2d4c96fde3ec,0x3e62f4d5e9a98f34,0x3e6467c96ecc5cbe,
0x3e6e7040d03dec5a,0x3e67bebf4282de36,0x3e6289b11aeb783f,0x3e5a891d1772f538,
0x3e634f10be1fb591,0x3e6d9ce1d316eb93,0x3e63562a19a9c442,0x3e54e2adf548084c,
0x3e508ce55cc8c97a,0x3e30e2f613e85bda,0x3e6db03ebb0227bf,0x3e61b75bb09cb098,
0x3e496f16abb9df22,0x3e65b3f399411c62,0x3e586b3e59f65355,0x3e52482ceae1ac12,
0x3e6efa39ef35793c,
];
