#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum AppendStage {Ready,Append,Finish,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct AppendProgress {pub stage:AppendStage,pub count:usize,pub ordinal:usize,pub failed:bool}
pub struct AppendWork {progress:AppendProgress}
impl AppendWork {

    pub fn after_explicit_admission(count:usize)->Self {
        Self{progress:AppendProgress{stage:AppendStage::Ready,count,ordinal:0,failed:false}}
    }
    pub fn progress(&self)->AppendProgress {self.progress}
}
pub trait AppendServices {
    type Error;

    fn append_admitted_ordinal(&mut self,ordinal:usize)->Result<(),Self::Error>;

    fn finish_donor_header(&mut self)->Result<(),Self::Error>;
}
pub enum AppendError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct AppendComplete {count:usize}
impl AppendComplete {pub fn count(&self)->usize {self.count}}
pub enum AppendAdvance {Paused,Complete(AppendComplete)}
pub fn advance<S:AppendServices>(work:&mut AppendWork,services:&mut S,mut budget:usize)
    ->Result<AppendAdvance,AppendError<S::Error>> {
    if work.progress.failed{return Err(AppendError::PriorFailure);}
    if work.progress.stage==AppendStage::Complete{return Err(AppendError::AlreadyComplete);}
    if work.progress.stage==AppendStage::Ready {
        work.progress.stage=if work.progress.count==0{AppendStage::Finish}else{AppendStage::Append};
    }
    loop {
        if budget==0{return Ok(AppendAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            AppendStage::Append=>services.append_admitted_ordinal(p.ordinal),
            AppendStage::Finish=>services.finish_donor_header(),
            AppendStage::Ready|AppendStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(AppendError::Service(error));}
        budget-=1;
        if p.stage==AppendStage::Append {
            p.ordinal+=1;
            if p.ordinal==p.count{p.stage=AppendStage::Finish;}
        }else{
            p.stage=AppendStage::Complete;
            return Ok(AppendAdvance::Complete(AppendComplete{count:p.count}));
        }
    }
}
