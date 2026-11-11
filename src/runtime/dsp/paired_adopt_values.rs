#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AdoptStage {Worker,Post,Packing,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct AdoptProgress {pub stage:AdoptStage,pub ended:u8,pub failed:bool}
pub struct AdoptWork {progress:AdoptProgress}
impl AdoptWork {

    pub fn after_scalar_prefix(ended:u8)->Self {
        Self{progress:AdoptProgress{stage:AdoptStage::Worker,ended,failed:false}}
    }
    pub fn progress(&self)->AdoptProgress {self.progress}
}
pub trait AdoptServices {
    type Error;
    fn adopt_worker_ended517(&mut self,ended:u8)->Result<(),Self::Error>;
    fn adopt_post_ended517(&mut self,ended:u8)->Result<(),Self::Error>;
    fn adopt_packing_ended517(&mut self,ended:u8)->Result<(),Self::Error>;
}
pub enum AdoptError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct Adopted {ended:u8}
impl Adopted {pub fn ended(&self)->u8 {self.ended}}
pub enum AdoptAdvance {Paused,Complete(Adopted)}
pub fn advance<S:AdoptServices>(work:&mut AdoptWork,services:&mut S,mut budget:usize)
    ->Result<AdoptAdvance,AdoptError<S::Error>> {
    if work.progress.failed{return Err(AdoptError::PriorFailure);}
    if work.progress.stage==AdoptStage::Complete{return Err(AdoptError::AlreadyComplete);}
    loop {
        if budget==0{return Ok(AdoptAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            AdoptStage::Worker=>services.adopt_worker_ended517(p.ended),
            AdoptStage::Post=>services.adopt_post_ended517(p.ended),
            AdoptStage::Packing=>services.adopt_packing_ended517(p.ended),
            AdoptStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(AdoptError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            AdoptStage::Worker=>AdoptStage::Post,
            AdoptStage::Post=>AdoptStage::Packing,
            AdoptStage::Packing=>AdoptStage::Complete,
            AdoptStage::Complete=>unreachable!(),
        };
        if p.stage==AdoptStage::Complete {
            return Ok(AdoptAdvance::Complete(Adopted{ended:p.ended}));
        }
    }
}
