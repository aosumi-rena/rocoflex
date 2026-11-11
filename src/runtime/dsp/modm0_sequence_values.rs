#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SequenceStage {Empty,Child,Publish,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SequenceProgress {pub stage:SequenceStage,pub child:usize,pub failed:bool}
pub struct SequenceWork {child_count:usize,progress:SequenceProgress}
impl SequenceWork {

    pub fn after_admission(child_count:usize)->Self {
        let stage=if child_count==0 {SequenceStage::Empty} else {SequenceStage::Child};
        Self{child_count,progress:SequenceProgress{stage,child:0,failed:false}}
    }
    pub fn progress(&self)->SequenceProgress {self.progress}
}
pub trait SequenceServices {
    type Error;

    fn empty_chain(&mut self)->Result<(),Self::Error>;

    fn forward_child(&mut self,index:usize,last:bool)->Result<(),Self::Error>;

    fn publish_final_root_status(&mut self)->Result<(),Self::Error>;
}
pub enum SequenceError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct SequenceComplete {child_count:usize}
impl SequenceComplete {pub fn child_count(&self)->usize {self.child_count}}
pub enum SequenceAdvance {Paused,Complete(SequenceComplete)}

pub fn advance_sequence<S:SequenceServices>(work:&mut SequenceWork,services:&mut S,mut budget:usize)
    ->Result<SequenceAdvance,SequenceError<S::Error>> {
    if work.progress.failed {return Err(SequenceError::PriorFailure);}
    if work.progress.stage==SequenceStage::Complete {return Err(SequenceError::AlreadyComplete);}
    loop {
        if budget==0 {return Ok(SequenceAdvance::Paused);}
        let result=match work.progress.stage {
            SequenceStage::Empty=>services.empty_chain(),
            SequenceStage::Child=>services.forward_child(work.progress.child,
                work.progress.child+1==work.child_count),
            SequenceStage::Publish=>services.publish_final_root_status(),
            SequenceStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {
            work.progress.failed=true;
            return Err(SequenceError::Service(error));
        }
        budget-=1;
        match work.progress.stage {
            SequenceStage::Child=>{
                work.progress.child+=1;
                if work.progress.child==work.child_count {work.progress.stage=SequenceStage::Publish;}
            }
            SequenceStage::Empty|SequenceStage::Publish=>{
                work.progress.stage=SequenceStage::Complete;
                return Ok(SequenceAdvance::Complete(SequenceComplete{child_count:work.child_count}));
            }
            SequenceStage::Complete=>unreachable!(),
        }
    }
}
