#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error { Environment, NonPositiveFinite, NonFinite, UnsupportedScaledResult }
#[derive(Clone,Copy,Debug)]
pub struct ScalarPowerServices;
impl ScalarPowerServices {
    pub fn bind(mxcsr:u32)->Result<Self,Error> {
        if mxcsr & 0xffc0 != 0x1f80 {return Err(Error::Environment)}
        Ok(Self)
    }
    pub fn exponent(self,value:f32)->Result<i32,Error> {
        let bits=value.to_bits();
        if bits==0 || bits>=0x7f800000 {return Err(Error::NonPositiveFinite)}
        let (exponent,_) = normalized(bits);
        Ok(exponent-127)
    }
    pub fn scale(self,value:f32,shift:i32)->Result<f32,Error> {
        let bits=value.to_bits();let magnitude=bits&0x7fffffff;
        if magnitude>=0x7f800000 {return Err(Error::NonFinite)}
        if magnitude==0 {return Ok(value)}
        let (exponent,fraction)=normalized(bits);
        let target=i64::from(exponent)+i64::from(shift);
        if !(1..=254).contains(&target) {return Err(Error::UnsupportedScaledResult)}
        Ok(f32::from_bits((bits&0x80000000)|((target as u32)<<23)|fraction))
    }
}
fn normalized(bits:u32)->(i32,u32) {
    let exponent=((bits>>23)&255) as i32;
    if exponent!=0 {return (exponent,bits&0x7fffff)}
    let fraction=bits&0x7fffff;
    debug_assert!(fraction!=0);
    let top=31-fraction.leading_zeros();
    (top as i32-22,(fraction<<(23-top))&0x7fffff)
}
