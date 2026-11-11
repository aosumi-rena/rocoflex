#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum FrameStage {Ready,Copy,Gaussian,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct FrameProgress {pub stage:FrameStage,pub groups:usize,pub lane:usize,pub failed:bool}
pub struct FrameWork {progress:FrameProgress}
impl FrameWork {

    pub fn after_explicit_admission(groups:usize)->Self {
        Self{progress:FrameProgress{stage:FrameStage::Ready,groups,lane:0,failed:false}}
    }
    pub fn progress(&self)->FrameProgress {self.progress}
}
pub trait FrameServices {
    type Error;

    fn copy_input_lane(&mut self,lane:usize)->Result<(),Self::Error>;

    fn standard_gaussian_and_store(&mut self,lane:usize)->Result<(),Self::Error>;
}
pub enum FrameError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct FrameComplete {groups:usize}
impl FrameComplete {pub fn groups(&self)->usize {self.groups}}
pub enum FrameAdvance {Paused,Complete(FrameComplete)}
pub fn advance_frame<S:FrameServices>(work:&mut FrameWork,services:&mut S,mut budget:usize)
    ->Result<FrameAdvance,FrameError<S::Error>> {
    if work.progress.failed{return Err(FrameError::PriorFailure);}
    if work.progress.stage==FrameStage::Complete{return Err(FrameError::AlreadyComplete);}
    if work.progress.stage==FrameStage::Ready {
        if work.progress.groups==0 {
            work.progress.stage=FrameStage::Complete;
            return Ok(FrameAdvance::Complete(FrameComplete{groups:0}));
        }
        work.progress.stage=FrameStage::Copy;
    }
    loop {
        if budget==0{return Ok(FrameAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            FrameStage::Copy=>services.copy_input_lane(p.lane),
            FrameStage::Gaussian=>services.standard_gaussian_and_store(p.lane),
            FrameStage::Ready|FrameStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(FrameError::Service(error));}
        budget-=1;p.lane+=1;
        if p.lane==p.groups {
            if p.stage==FrameStage::Copy {p.stage=FrameStage::Gaussian;p.lane=0;}
            else {p.stage=FrameStage::Complete;return Ok(FrameAdvance::Complete(FrameComplete{groups:p.groups}));}
        }
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum QueueStage {Ready,BorrowInput,PrepareFrame,AppendFrame,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct QueueProgress {pub stage:QueueStage,pub input_count:usize,pub row:usize,pub failed:bool}
pub struct QueueWork {progress:QueueProgress}
impl QueueWork {

    pub fn after_explicit_admission(input_count:usize)->Self {
        Self{progress:QueueProgress{stage:QueueStage::Ready,input_count,row:0,failed:false}}
    }
    pub fn progress(&self)->QueueProgress {self.progress}
}
pub trait QueueServices {
    type Error;

    fn borrow_input_row(&mut self,row:usize)->Result<(),Self::Error>;

    fn prepare_noise_frame(&mut self,row:usize)->Result<(),Self::Error>;

    fn append_prepared_frame(&mut self,row:usize)->Result<(),Self::Error>;
}
pub enum QueueError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct QueueComplete {input_count:usize}
impl QueueComplete {pub fn input_count(&self)->usize {self.input_count}}
pub enum QueueAdvance {Paused,Complete(QueueComplete)}
pub fn advance_queue<S:QueueServices>(work:&mut QueueWork,services:&mut S,mut budget:usize)
    ->Result<QueueAdvance,QueueError<S::Error>> {
    if work.progress.failed{return Err(QueueError::PriorFailure);}
    if work.progress.stage==QueueStage::Complete{return Err(QueueError::AlreadyComplete);}
    if work.progress.stage==QueueStage::Ready {
        if work.progress.input_count==0 {
            work.progress.stage=QueueStage::Complete;
            return Ok(QueueAdvance::Complete(QueueComplete{input_count:0}));
        }
        work.progress.stage=QueueStage::BorrowInput;
    }
    loop {
        if budget==0{return Ok(QueueAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            QueueStage::BorrowInput=>services.borrow_input_row(p.row),
            QueueStage::PrepareFrame=>services.prepare_noise_frame(p.row),
            QueueStage::AppendFrame=>services.append_prepared_frame(p.row),
            QueueStage::Ready|QueueStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(QueueError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            QueueStage::BorrowInput=>QueueStage::PrepareFrame,
            QueueStage::PrepareFrame=>QueueStage::AppendFrame,
            QueueStage::AppendFrame=>{p.row+=1;if p.row==p.input_count{QueueStage::Complete}else{QueueStage::BorrowInput}},
            QueueStage::Ready|QueueStage::Complete=>unreachable!(),
        };
        if p.stage==QueueStage::Complete {
            return Ok(QueueAdvance::Complete(QueueComplete{input_count:p.input_count}));
        }
    }
}
