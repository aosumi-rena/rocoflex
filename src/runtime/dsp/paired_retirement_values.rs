#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PrefixEnd {Front,Back}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct PositivePrefix {count:u32,end:PrefixEnd,observed_queue_count:u64}
impl PositivePrefix {
    pub fn count(&self)->u32 {self.count}
    pub fn end(&self)->PrefixEnd {self.end}
    pub fn observed_queue_count(&self)->u64 {self.observed_queue_count}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum RetirementStage {Decision,PreparePositive,ErasePositive,CurrentSubtract,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct RetirementProgress {
    pub stage:RetirementStage,pub observed_queue_count:u64,pub initial_cursor_count:usize,
    pub minimum:i32,pub request:Option<PositivePrefix>,pub fresh_cursor_count:Option<usize>,pub failed:bool,
}
pub struct RetirementWork {progress:RetirementProgress}
impl RetirementWork {

    pub fn after_explicit_snapshot(queue_count400:u64,logical_cursor_words:&[u32])->Self {
        let mut minimum=queue_count400 as u32 as i32;
        for &word in logical_cursor_words {minimum=minimum.min(word as i32);}
        Self{progress:RetirementProgress{stage:RetirementStage::Decision,
            observed_queue_count:queue_count400,initial_cursor_count:logical_cursor_words.len(),
            minimum,request:None,fresh_cursor_count:None,failed:false}}
    }
    pub fn progress(&self)->RetirementProgress {self.progress}
}
pub enum RetirementError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct Skipped {minimum:i32}
impl Skipped {pub fn minimum(&self)->i32 {self.minimum}}
pub struct Retired {request:PositivePrefix}
impl Retired {pub fn request(&self)->PositivePrefix {self.request}}
pub enum RetirementComplete {Skipped(Skipped),Retired(Retired)}
pub enum RetirementAdvance {Paused,Complete(RetirementComplete)}

pub trait RetirementServices {
    type Error;

    fn prepare_positive_prefix(&mut self,request:PositivePrefix)->Result<(),Self::Error>;

    fn erase_positive_prefix(&mut self,request:PositivePrefix)->Result<(),Self::Error>;

    fn current_cursor_words_after_erase(&mut self)->Result<&mut[u32],Self::Error>;
}

pub fn advance<S:RetirementServices>(work:&mut RetirementWork,services:&mut S,mut budget:usize)
    ->Result<RetirementAdvance,RetirementError<S::Error>> {
    if work.progress.failed{return Err(RetirementError::PriorFailure);}
    if work.progress.stage==RetirementStage::Complete{return Err(RetirementError::AlreadyComplete);}
    if work.progress.stage==RetirementStage::Decision {
        let p=&mut work.progress;
        if p.minimum<=0 {
            p.stage=RetirementStage::Complete;
            return Ok(RetirementAdvance::Complete(RetirementComplete::Skipped(Skipped{minimum:p.minimum})));
        }
        let count=p.minimum as u32;
        p.request=Some(PositivePrefix{count,observed_queue_count:p.observed_queue_count,
            end:if u64::from(count)<p.observed_queue_count{PrefixEnd::Front}else{PrefixEnd::Back}});
        p.stage=RetirementStage::PreparePositive;
    }
    loop {
        if budget==0{return Ok(RetirementAdvance::Paused);}
        let p=&mut work.progress;
        let request=p.request.expect("positive retirement request");
        let result=match p.stage {
            RetirementStage::PreparePositive=>services.prepare_positive_prefix(request),
            RetirementStage::ErasePositive=>services.erase_positive_prefix(request),
            RetirementStage::CurrentSubtract=>services.current_cursor_words_after_erase().map(|words|{
                p.fresh_cursor_count=Some(words.len());

                for word in words {*word=word.wrapping_sub(request.count);}
            }),
            RetirementStage::Decision|RetirementStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(RetirementError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            RetirementStage::PreparePositive=>RetirementStage::ErasePositive,
            RetirementStage::ErasePositive=>RetirementStage::CurrentSubtract,
            RetirementStage::CurrentSubtract=>RetirementStage::Complete,
            RetirementStage::Decision|RetirementStage::Complete=>unreachable!(),
        };
        if p.stage==RetirementStage::Complete {
            return Ok(RetirementAdvance::Complete(RetirementComplete::Retired(Retired{request})));
        }
    }
}
