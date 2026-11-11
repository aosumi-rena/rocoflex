#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct WorkCount {pub available:i32,pub work:usize}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CountError {Status,EndedUnderflow}

pub fn select_work_count(feature_count:u64,condition_count:u64,cursor:i32,ended517:u8)
    ->Result<WorkCount,CountError> {
    if ended517>1{return Err(CountError::Status);}
    let available=(feature_count as u32).wrapping_sub(cursor as u32) as i32;
    let condition=condition_count as u32 as i32;
    if ended517!=0&&available<condition{return Err(CountError::EndedUnderflow);}
    let selected=available.min(condition);
    Ok(WorkCount{available,work:if selected>0{selected as usize}else{0}})
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ConditionStage {Ready,Bind176,Moving208,Erasing432,Worker64,Zero176,Cursor508,Cleanup208,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ConditionProgress {
    pub stage:ConditionStage,pub count:usize,pub cursor:i32,pub ended517:u8,
    pub moved:usize,pub failed:bool,
}
pub struct ConditionWork {progress:ConditionProgress}
impl ConditionWork {

    pub fn after_explicit_admission(count:usize,cursor:i32,ended517:u8)->Self {
        Self{progress:ConditionProgress{stage:ConditionStage::Ready,count,cursor,ended517,moved:0,failed:false}}
    }
    pub fn progress(&self)->ConditionProgress {self.progress}
}
pub trait ConditionServices {
    type Error;

    fn set208status_then_bind176(&mut self,cursor:i32,count:usize,ended517:u8)->Result<(),Self::Error>;

    fn move_condition208_ordinal(&mut self,ordinal:usize)->Result<(),Self::Error>;
    fn erase_condition432_prefix(&mut self,count:usize)->Result<(),Self::Error>;
    fn worker64_44180(&mut self)->Result<(),Self::Error>;
    fn zero176_pointer_words(&mut self)->Result<(),Self::Error>;

    fn advance_current_cursor508_then_finish176(&mut self,count:usize)->Result<(),Self::Error>;
    fn cleanup208(&mut self)->Result<(),Self::Error>;
}
pub enum ConditionError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct Skipped {cursor:i32,ended517:u8}
impl Skipped {
    pub fn cursor(&self)->i32 {self.cursor}
    pub fn ended517(&self)->u8 {self.ended517}
}
pub struct Worked {count:usize,cursor:i32,ended517:u8}
impl Worked {
    pub fn count(&self)->usize {self.count}
    pub fn cursor(&self)->i32 {self.cursor}
    pub fn ended517(&self)->u8 {self.ended517}
}
pub enum ConditionComplete {Skipped(Skipped),Worked(Worked)}
pub enum ConditionAdvance {Paused,Complete(ConditionComplete)}
pub fn advance<S:ConditionServices>(work:&mut ConditionWork,services:&mut S,mut budget:usize)
    ->Result<ConditionAdvance,ConditionError<S::Error>> {
    if work.progress.failed{return Err(ConditionError::PriorFailure);}
    if work.progress.stage==ConditionStage::Complete{return Err(ConditionError::AlreadyComplete);}
    if work.progress.stage==ConditionStage::Ready {
        if work.progress.count==0 {
            work.progress.stage=ConditionStage::Complete;
            return Ok(ConditionAdvance::Complete(ConditionComplete::Skipped(Skipped{
                cursor:work.progress.cursor,ended517:work.progress.ended517})));
        }
        work.progress.stage=ConditionStage::Bind176;
    }
    loop {
        if budget==0{return Ok(ConditionAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            ConditionStage::Bind176=>services.set208status_then_bind176(p.cursor,p.count,p.ended517),
            ConditionStage::Moving208=>services.move_condition208_ordinal(p.moved),
            ConditionStage::Erasing432=>services.erase_condition432_prefix(p.count),
            ConditionStage::Worker64=>services.worker64_44180(),
            ConditionStage::Zero176=>services.zero176_pointer_words(),
            ConditionStage::Cursor508=>services.advance_current_cursor508_then_finish176(p.count),
            ConditionStage::Cleanup208=>services.cleanup208(),
            ConditionStage::Ready|ConditionStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(ConditionError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            ConditionStage::Bind176=>ConditionStage::Moving208,
            ConditionStage::Moving208=>{p.moved+=1;if p.moved==p.count{ConditionStage::Erasing432}else{ConditionStage::Moving208}},
            ConditionStage::Erasing432=>ConditionStage::Worker64,
            ConditionStage::Worker64=>ConditionStage::Zero176,
            ConditionStage::Zero176=>ConditionStage::Cursor508,
            ConditionStage::Cursor508=>ConditionStage::Cleanup208,
            ConditionStage::Cleanup208=>ConditionStage::Complete,
            ConditionStage::Ready|ConditionStage::Complete=>unreachable!(),
        };
        if p.stage==ConditionStage::Complete {
            return Ok(ConditionAdvance::Complete(ConditionComplete::Worked(Worked{
                count:p.count,cursor:p.cursor,ended517:p.ended517})));
        }
    }
}
