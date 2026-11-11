#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AcquireStage {Cached,Create,Admit,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct AcquireProgress {pub stage:AcquireStage,pub failed:bool}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AcquireKind {Cached,Created}
pub struct AcquireWork {progress:AcquireProgress}
impl AcquireWork {

    pub fn after_admission()->Self {
        Self{progress:AcquireProgress{stage:AcquireStage::Cached,failed:false}}
    }
    pub fn progress(&self)->AcquireProgress {self.progress}
}
pub trait AcquireServices {
    type Error;

    fn take_cached(&mut self)->Result<bool,Self::Error>;

    fn create_uncleared(&mut self,first:bool,second:bool)->Result<(),Self::Error>;

    fn admit_created_and_finish(&mut self)->Result<(),Self::Error>;
}
pub enum AcquireError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct AcquireComplete {kind:AcquireKind}
impl AcquireComplete {pub fn kind(&self)->AcquireKind {self.kind}}
pub enum AcquireAdvance {Paused,Complete(AcquireComplete)}

pub fn advance_acquire<S:AcquireServices>(work:&mut AcquireWork,services:&mut S,mut budget:usize)
    ->Result<AcquireAdvance,AcquireError<S::Error>> {
    if work.progress.failed {return Err(AcquireError::PriorFailure);}
    if work.progress.stage==AcquireStage::Complete {return Err(AcquireError::AlreadyComplete);}
    loop {
        if budget==0 {return Ok(AcquireAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            AcquireStage::Cached=>services.take_cached().map(|found|if found {Some(AcquireKind::Cached)} else {None}),
            AcquireStage::Create=>services.create_uncleared(false,false).map(|()|None),
            AcquireStage::Admit=>services.admit_created_and_finish().map(|()|Some(AcquireKind::Created)),
            AcquireStage::Complete=>unreachable!(),
        };
        let kind=match result {
            Ok(kind)=>kind,
            Err(error)=>{p.failed=true;return Err(AcquireError::Service(error));}
        };
        budget-=1;
        if let Some(kind)=kind {
            p.stage=AcquireStage::Complete;
            return Ok(AcquireAdvance::Complete(AcquireComplete{kind}));
        }
        p.stage=match p.stage {
            AcquireStage::Cached=>AcquireStage::Create,
            AcquireStage::Create=>AcquireStage::Admit,
            AcquireStage::Admit|AcquireStage::Complete=>unreachable!(),
        };
    }
}
