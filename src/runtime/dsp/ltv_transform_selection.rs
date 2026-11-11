use crate::model2_operator_binding::SourceLtvfs1Binding;
use crate::ltv_block::LtvError;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SourceTransformMathProfile{pub source_cpu_word:u32,pub mxcsr:u32}

pub trait SourceTransformSelectionMath{
    type Error;
    fn explicit_profile(&self)->SourceTransformMathProfile;
    fn helper582ff8(&mut self,input:f64)->Result<f64,Self::Error>;
    fn helper57dc64(&mut self,input:f64)->Result<f64,Self::Error>;

    fn cvttsd2si_i32(&mut self,input:f64)->Result<i32,Self::Error>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum FirstCallBindingError{OwnerWords,Configuration,WindowFactor,Frame,Pool,Transform(LtvError)}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SourceTransformSelector{
    pub(super) binding:SourceLtvfs1Binding,pub(super) owner_rcx:u64,pub(super) linked50:u64,
    pub(super) word_e8:i32,pub(super) word_f0:i32,
}
impl SourceTransformSelector{

    pub fn bind(binding:SourceLtvfs1Binding,owner_rcx:u64,linked50:u64,word_e8:i32,word_f0:i32)
        ->Result<Self,FirstCallBindingError>{
        if owner_rcx==0||linked50==0{return Err(FirstCallBindingError::OwnerWords);}
        if word_e8!=binding.config().raw_word0{return Err(FirstCallBindingError::Configuration);}
        if word_e8<=0{return Err(FirstCallBindingError::WindowFactor);}
        Ok(Self{binding,owner_rcx,linked50,word_e8,word_f0})
    }
    pub fn select<M:SourceTransformSelectionMath>(self,live_edx:i32,math:&mut M)->Result<SourceTransformSelection,M::Error>{
        if self.word_f0>0{return Ok(SourceTransformSelection{selector:self,live_edx,raw_eax:self.word_f0 as u32,arithmetic:None});}

        let integer=self.word_e8.wrapping_mul(live_edx).wrapping_sub(1);
        let input=integer as f64;let profile=math.explicit_profile();
        let first=math.helper582ff8(input)?;
        let second=math.helper57dc64(first)?;
        let count=math.cvttsd2si_i32(second)?;
        let raw_eax=2_u32.wrapping_shl(count as u32);
        Ok(SourceTransformSelection{selector:self,live_edx,raw_eax,
            arithmetic:Some(SourceTransformArithmetic{profile,integer,input_bits:input.to_bits(),
                helper582ff8_bits:first.to_bits(),helper57dc64_bits:second.to_bits(),shift_ecx:count})})
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SourceTransformArithmetic{
    pub profile:SourceTransformMathProfile,pub integer:i32,pub input_bits:u64,
    pub helper582ff8_bits:u64,pub helper57dc64_bits:u64,pub shift_ecx:i32,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SourceTransformSelection{pub(super) selector:SourceTransformSelector,pub(super) live_edx:i32,
    pub(super) raw_eax:u32,pub(super) arithmetic:Option<SourceTransformArithmetic>}
impl SourceTransformSelection{
    pub fn raw_eax(self)->u32{self.raw_eax}
    pub fn arithmetic(self)->Option<SourceTransformArithmetic>{self.arithmetic}
}
