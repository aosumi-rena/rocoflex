#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ClearStage {Observe,Retire,Finish,Storage,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ClearProgress {pub stage:ClearStage,pub failed:bool}
pub struct ClearWork {progress:ClearProgress}
impl ClearWork {

    pub fn after_admission()->Self {
        Self{progress:ClearProgress{stage:ClearStage::Observe,failed:false}}
    }
    pub fn progress(&self)->ClearProgress {self.progress}
}
pub trait ClearServices {
    type Error;

    fn current_live_count(&self)->usize;

    fn retire_current_back(&mut self)->Result<(),Self::Error>;

    fn finish_current_back(&mut self)->Result<(),Self::Error>;

    fn finish_storage(&mut self)->Result<(),Self::Error>;
}
pub enum ClearError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct ClearComplete {_private:()}
pub enum ClearAdvance {Paused,Complete(ClearComplete)}

pub fn advance_clear<S:ClearServices>(work:&mut ClearWork,services:&mut S,mut budget:usize)
    ->Result<ClearAdvance,ClearError<S::Error>> {
    if work.progress.failed {return Err(ClearError::PriorFailure);}
    if work.progress.stage==ClearStage::Complete {return Err(ClearError::AlreadyComplete);}
    loop {
        if budget==0 {return Ok(ClearAdvance::Paused);}
        let p=&mut work.progress;
        if p.stage==ClearStage::Observe {
            p.stage=if services.current_live_count()!=0 {ClearStage::Retire} else {ClearStage::Storage};
        }
        let result=match p.stage {
            ClearStage::Retire=>services.retire_current_back(),
            ClearStage::Finish=>services.finish_current_back(),
            ClearStage::Storage=>services.finish_storage(),
            ClearStage::Observe|ClearStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(ClearError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            ClearStage::Retire=>ClearStage::Finish,
            ClearStage::Finish=>ClearStage::Observe,
            ClearStage::Storage=>ClearStage::Complete,
            ClearStage::Observe|ClearStage::Complete=>unreachable!(),
        };
        if p.stage==ClearStage::Complete {
            return Ok(ClearAdvance::Complete(ClearComplete{_private:()}));
        }
    }
}
