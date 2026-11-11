#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum LaneStage {
    ReadTarget,CheckTarget,Exp,Add,CheckExponentialSum,Div,ReadNoise,
    CheckNoise,Mul,CheckGainResult,Store,Complete,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct LanePartial {
    pub row:usize,pub lane:usize,pub stage:LaneStage,
    pub target:Option<u32>,pub negated:Option<u32>,pub exponential:Option<u32>,
    pub sum:Option<u32>,pub gain:Option<u32>,pub noise:Option<u32>,pub result:Option<u32>,

    pub store_completed:bool,
}
pub enum LaneFailure<E> {
    Service{error:E,partial:LanePartial},
    NonfiniteRead{partial:LanePartial},NonfiniteResult{partial:LanePartial},
}
impl<E> LaneFailure<E> {
    pub fn partial(&self)->LanePartial {
        match self {Self::Service{partial,..}|Self::NonfiniteRead{partial}
            |Self::NonfiniteResult{partial}=>*partial}
    }
}
pub struct LaneComplete {partial:LanePartial}
impl LaneComplete {pub fn partial(&self)->LanePartial {self.partial}}

pub trait LaneServices {
    type Error;
    fn read_target_bits(&mut self)->Result<u32,Self::Error>;
    fn scalar_57f8e0_bits(&mut self,value:u32)->Result<u32,Self::Error>;
    fn addss_bits(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
    fn divss_bits(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
    fn read_noise_bits(&mut self)->Result<u32,Self::Error>;
    fn mulss_bits(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
    fn store_target_bits(&mut self,value:u32)->Result<(),Self::Error>;
}
fn finite(bits:u32)->bool {bits&0x7f80_0000!=0x7f80_0000}

pub fn apply_lane<S:LaneServices>(row:usize,lane:usize,services:&mut S)
    ->Result<LaneComplete,LaneFailure<S::Error>> {
    let mut p=LanePartial{row,lane,stage:LaneStage::ReadTarget,target:None,negated:None,
        exponential:None,sum:None,gain:None,noise:None,result:None,store_completed:false};
    let target=services.read_target_bits().map_err(|error|LaneFailure::Service{error,partial:p})?;
    p.target=Some(target);p.stage=LaneStage::CheckTarget;
    if !finite(target){return Err(LaneFailure::NonfiniteRead{partial:p});}
    let negated=target^0x8000_0000;p.negated=Some(negated);p.stage=LaneStage::Exp;
    let exponential=services.scalar_57f8e0_bits(negated)
        .map_err(|error|LaneFailure::Service{error,partial:p})?;
    p.exponential=Some(exponential);p.stage=LaneStage::Add;
    let sum=services.addss_bits(exponential,0x3f80_0000)
        .map_err(|error|LaneFailure::Service{error,partial:p})?;
    p.sum=Some(sum);p.stage=LaneStage::CheckExponentialSum;

    if !finite(exponential)||!finite(sum){return Err(LaneFailure::NonfiniteResult{partial:p});}
    p.stage=LaneStage::Div;
    let gain=services.divss_bits(0x3f80_0000,sum)
        .map_err(|error|LaneFailure::Service{error,partial:p})?;
    p.gain=Some(gain);p.stage=LaneStage::ReadNoise;
    let noise=services.read_noise_bits().map_err(|error|LaneFailure::Service{error,partial:p})?;
    p.noise=Some(noise);p.stage=LaneStage::CheckNoise;
    if !finite(noise){return Err(LaneFailure::NonfiniteRead{partial:p});}
    p.stage=LaneStage::Mul;
    let result=services.mulss_bits(gain,noise)
        .map_err(|error|LaneFailure::Service{error,partial:p})?;
    p.result=Some(result);p.stage=LaneStage::CheckGainResult;

    if !finite(gain)||!finite(result){return Err(LaneFailure::NonfiniteResult{partial:p});}
    p.stage=LaneStage::Store;
    services.store_target_bits(result).map_err(|error|LaneFailure::Service{error,partial:p})?;
    p.store_completed=true;p.stage=LaneStage::Complete;Ok(LaneComplete{partial:p})
}
