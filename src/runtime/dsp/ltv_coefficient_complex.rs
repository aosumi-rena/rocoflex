#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage{Cosine,FirstScale,Sine,SecondScale,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Scale{pub bits:u32,pub source_status:u32}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Progress{
    pub real:u32,pub imaginary:u32,pub stage:Stage,
    pub cosine:Option<u32>,pub first:Option<Scale>,
    pub sine:Option<u32>,pub second:Option<Scale>,
}
pub trait Services{
    type Error;
    fn cosf_57e330(&mut self,phase:u32)->Result<u32,Self::Error>;
    fn scaled_exp_562834(&mut self,real:u32,multiplier:u32,exponent2:i16)->Result<Scale,Self::Error>;
    fn sinf_586ed0(&mut self,phase:u32)->Result<u32,Self::Error>;
}
pub struct Failure<E>{pub progress:Progress,pub cause:E}
pub struct Completed{pub bits:[u32;2],pub progress:Progress}

pub fn evaluate<S:Services>(services:&mut S,real:u32,imaginary:u32)->Result<Completed,Failure<S::Error>>{
    let mut progress=Progress{real,imaginary,stage:Stage::Cosine,cosine:None,first:None,sine:None,second:None};
    let cosine=services.cosf_57e330(imaginary).map_err(|cause|Failure{progress,cause})?;
    progress.cosine=Some(cosine);progress.stage=Stage::FirstScale;
    let first=services.scaled_exp_562834(real,cosine,0).map_err(|cause|Failure{progress,cause})?;
    progress.first=Some(first);progress.stage=Stage::Sine;
    let sine=services.sinf_586ed0(imaginary).map_err(|cause|Failure{progress,cause})?;
    progress.sine=Some(sine);progress.stage=Stage::SecondScale;
    let second=services.scaled_exp_562834(real,sine,0).map_err(|cause|Failure{progress,cause})?;
    progress.second=Some(second);progress.stage=Stage::Complete;
    Ok(Completed{bits:[first.bits,second.bits],progress})
}
