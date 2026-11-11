pub const PHASE_CONSTANT_BITS:u64=0x401921fb54442d18;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Mode{Zero,One}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PhaseOperation{PhaseConvert,Scale,PeriodConvert,Divide,FrequencyConvert,Angle,Sin586960,Cos57ddb0,NarrowSin,NarrowCos}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Site{Dct,Prefix(usize),Phase{index:usize,operation:PhaseOperation},SinStore(usize),CosStore(usize),PhaseDomain(usize)}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage{Ready,Pending(Site),Complete,Refused(Site)}

#[derive(Default)]
pub struct PhaseWork{pub phase:Option<f64>,pub scaled:Option<f64>,pub period:Option<f64>,pub divided:Option<f64>,
    pub frequency:Option<f64>,pub angle:Option<f64>,pub sine:Option<f64>,pub cosine:Option<f64>,pub sine_f32:Option<f32>,pub cosine_f32:Option<f32>}
pub struct Work{pub dct_prefix:Vec<f32>,pub phase:PhaseWork,pub stage:Stage}
pub trait PhaseServices{
    type Error;
    fn phase_u32_to_f64(&mut self,value:u32)->Result<f64,Self::Error>;
    fn period_u16_to_f64(&mut self,value:u16)->Result<f64,Self::Error>;
    fn frequency_f32_to_f64(&mut self,value:f32)->Result<f64,Self::Error>;
    fn multiply_f64(&mut self,left:f64,right:f64)->Result<f64,Self::Error>;
    fn divide_f64(&mut self,left:f64,right:f64)->Result<f64,Self::Error>;
    fn sin_586960(&mut self,angle:f64)->Result<f64,Self::Error>;
    fn cos_57ddb0(&mut self,angle:f64)->Result<f64,Self::Error>;
    fn narrow_f64_to_f32(&mut self,value:f64)->Result<f32,Self::Error>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Width{Input,Destination,DctPrefix}
pub enum RowError<E>{Configuration,Width{buffer:Width,expected:usize,actual:usize},Control{stage:Stage},
    Callback{site:Site,reason:E},PhaseDomain{index:usize}}
fn operation<T,E>(stage:&mut Stage,site:Site,result:Result<T,E>)->Result<T,RowError<E>>{
    match result{Ok(value)=>Ok(value),Err(reason)=>{*stage=Stage::Refused(site);Err(RowError::Callback{site,reason})}}
}

pub fn forward_row<E,P:PhaseServices<Error=E>>(input:&[f32],mode:Mode,prefix:usize,period:u16,phase_word:u32,
    frequencies:&[f32],destination_width:usize,work:&mut Work,
    dct:&mut impl FnMut(&[f32],&mut[f32])->Result<(),E>,
    store:&mut impl FnMut(usize,f32)->Result<(),E>,phase:&mut P)->Result<(),RowError<E>>{
    if !matches!(work.stage,Stage::Ready|Stage::Complete){return Err(RowError::Control{stage:work.stage});}
    if prefix==0||period==0{return Err(RowError::Configuration);}
    let (start,trim)=match mode{Mode::Zero=>(3,8),Mode::One=>(0,1)};
    if input.len()<=trim{return Err(RowError::Width{buffer:Width::Input,expected:trim+1,actual:input.len()});}
    let width=frequencies.len().checked_mul(2).and_then(|n|n.checked_add(prefix)).ok_or(RowError::Configuration)?;
    if width!=destination_width{return Err(RowError::Width{buffer:Width::Destination,expected:width,actual:destination_width});}
    if work.dct_prefix.len()!=prefix{return Err(RowError::Width{buffer:Width::DctPrefix,expected:prefix,actual:work.dct_prefix.len()});}
    work.stage=Stage::Pending(Site::Dct);
    operation(&mut work.stage,Site::Dct,dct(&input[start..start+input.len()-trim],&mut work.dct_prefix))?;
    for i in 0..prefix{
        let site=Site::Prefix(i);work.stage=Stage::Pending(site);
        operation(&mut work.stage,site,store(i,work.dct_prefix[i]))?;
    }
    for(i,&frequency)in frequencies.iter().enumerate(){
        work.phase=PhaseWork::default();

        macro_rules! phase_op{($op:ident,$expression:expr)=>{{
            let site=Site::Phase{index:i,operation:PhaseOperation::$op};work.stage=Stage::Pending(site);
            operation(&mut work.stage,site,$expression)?
        }}}
        let phase_word_f64=phase_op!(PhaseConvert,phase.phase_u32_to_f64(phase_word));work.phase.phase=Some(phase_word_f64);
        let scaled=phase_op!(Scale,phase.multiply_f64(phase_word_f64,f64::from_bits(PHASE_CONSTANT_BITS)));work.phase.scaled=Some(scaled);
        let period_f64=phase_op!(PeriodConvert,phase.period_u16_to_f64(period));work.phase.period=Some(period_f64);
        let divided=phase_op!(Divide,phase.divide_f64(scaled,period_f64));work.phase.divided=Some(divided);
        let frequency_f64=phase_op!(FrequencyConvert,phase.frequency_f32_to_f64(frequency));work.phase.frequency=Some(frequency_f64);
        let angle=phase_op!(Angle,phase.multiply_f64(divided,frequency_f64));work.phase.angle=Some(angle);
        let sine=phase_op!(Sin586960,phase.sin_586960(angle));work.phase.sine=Some(sine);
        let cosine=phase_op!(Cos57ddb0,phase.cos_57ddb0(angle));work.phase.cosine=Some(cosine);
        let a=phase_op!(NarrowSin,phase.narrow_f64_to_f32(sine));work.phase.sine_f32=Some(a);
        let b=phase_op!(NarrowCos,phase.narrow_f64_to_f32(cosine));work.phase.cosine_f32=Some(b);
        if !sine.is_finite()||!cosine.is_finite()||!a.is_finite()||!b.is_finite(){
            work.stage=Stage::Refused(Site::PhaseDomain(i));return Err(RowError::PhaseDomain{index:i});
        }
        let site=Site::SinStore(i);work.stage=Stage::Pending(site);operation(&mut work.stage,site,store(prefix+i,a))?;
        let site=Site::CosStore(i);work.stage=Stage::Pending(site);operation(&mut work.stage,site,store(prefix+frequencies.len()+i,b))?;
    }
    work.stage=Stage::Complete;Ok(())
}
