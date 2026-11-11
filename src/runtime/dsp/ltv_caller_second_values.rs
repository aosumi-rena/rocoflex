#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SecondSignalStage {
    StoreStatus,SnapshotOldRange,ReleaseOld,ResetEnd,Reserve,
    RefillCoefficients328,SnapshotSignalPrefix,AdmitMoveCount,MoveSignal,
    EraseUsingFreshHead,Complete,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SecondSignalProgress {
    pub stage:SecondSignalStage,

    pub selected_count:i64,
    pub raw_status:u8,
    pub old_count:Option<usize>,
    pub move_count:Option<usize>,

    pub ordinal:usize,
    pub completed_service_effects:u64,
    pub failed:bool,
}

pub struct SecondSignalWork {progress:SecondSignalProgress}
impl SecondSignalWork {

    pub fn after_explicit_selection(selected_count:i64,raw_status:u8)->Self {
        Self{progress:SecondSignalProgress{stage:SecondSignalStage::StoreStatus,
            selected_count,raw_status,old_count:None,move_count:None,ordinal:0,
            completed_service_effects:0,failed:false}}
    }
    pub fn progress(&self)->SecondSignalProgress {self.progress}
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SecondSignalControlError {

    SelectedCountNotRepresentable{selected_count:i64},
}
pub enum SecondSignalError<E> {
    PriorFailure,AlreadyComplete,Service(E),Control(SecondSignalControlError),
}

pub struct ReadyForReceiverSelection {
    selected_count:i64,raw_status:u8,moved_count:usize,
}
impl ReadyForReceiverSelection {
    pub fn selected_count(&self)->i64 {self.selected_count}
    pub fn raw_status(&self)->u8 {self.raw_status}
    pub fn moved_count(&self)->usize {self.moved_count}
}
pub enum SecondSignalAdvance {
    Paused,ReadyForReceiverSelection(ReadyForReceiverSelection),
}

pub trait SecondSignalServices {
    type Error;

    fn store_signal_status(&mut self,raw_status:u8)->Result<(),Self::Error>;

    fn snapshot_old_signal_range(&mut self)->Result<usize,Self::Error>;

    fn release_old_signal(&mut self,ordinal:usize)->Result<(),Self::Error>;

    fn reset_signal_end_to_original_begin(&mut self)->Result<(),Self::Error>;

    fn reserve_signal_for_selected_count(&mut self,selected_count:i64)->Result<(),Self::Error>;

    fn refill_coefficients328(&mut self)->Result<(),Self::Error>;

    fn snapshot_signal_prefix(&mut self,selected_count:i64)->Result<(),Self::Error>;

    fn move_signal_ordinal(&mut self,ordinal:usize)->Result<(),Self::Error>;

    fn erase_moved_prefix_using_fresh_head(&mut self,selected_count:i64)->Result<(),Self::Error>;
}

pub fn advance<S:SecondSignalServices>(work:&mut SecondSignalWork,services:&mut S,
    mut budget:usize)->Result<SecondSignalAdvance,SecondSignalError<S::Error>> {
    if work.progress.failed {return Err(SecondSignalError::PriorFailure);}
    if work.progress.stage==SecondSignalStage::Complete {return Err(SecondSignalError::AlreadyComplete);}
    loop {

        if work.progress.stage==SecondSignalStage::AdmitMoveCount {
            match usize::try_from(work.progress.selected_count) {
                Ok(count)=>{work.progress.move_count=Some(count);work.progress.ordinal=0;
                    work.progress.stage=SecondSignalStage::MoveSignal;},
                Err(_)=>{
                    work.progress.failed=true;
                    return Err(SecondSignalError::Control(
                        SecondSignalControlError::SelectedCountNotRepresentable{
                            selected_count:work.progress.selected_count}));
                },
            }
        }
        if work.progress.stage==SecondSignalStage::ReleaseOld
            &&Some(work.progress.ordinal)==work.progress.old_count {
            work.progress.ordinal=0;work.progress.stage=SecondSignalStage::ResetEnd;
        }
        if work.progress.stage==SecondSignalStage::MoveSignal
            &&Some(work.progress.ordinal)==work.progress.move_count {
            work.progress.stage=SecondSignalStage::EraseUsingFreshHead;
        }
        if budget==0 {return Ok(SecondSignalAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            SecondSignalStage::StoreStatus=>services.store_signal_status(p.raw_status),
            SecondSignalStage::SnapshotOldRange=>services.snapshot_old_signal_range().map(|count|{
                p.old_count=Some(count);}),
            SecondSignalStage::ReleaseOld=>services.release_old_signal(p.ordinal),
            SecondSignalStage::ResetEnd=>services.reset_signal_end_to_original_begin(),
            SecondSignalStage::Reserve=>services.reserve_signal_for_selected_count(p.selected_count),
            SecondSignalStage::RefillCoefficients328=>services.refill_coefficients328(),
            SecondSignalStage::SnapshotSignalPrefix=>services.snapshot_signal_prefix(p.selected_count),
            SecondSignalStage::MoveSignal=>services.move_signal_ordinal(p.ordinal),
            SecondSignalStage::EraseUsingFreshHead=>services.erase_moved_prefix_using_fresh_head(p.selected_count),

            SecondSignalStage::AdmitMoveCount|SecondSignalStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {
            p.failed=true;return Err(SecondSignalError::Service(error));
        }
        budget-=1;

        p.completed_service_effects=p.completed_service_effects.saturating_add(1);
        p.stage=match p.stage {
            SecondSignalStage::StoreStatus=>SecondSignalStage::SnapshotOldRange,
            SecondSignalStage::SnapshotOldRange=>SecondSignalStage::ReleaseOld,
            SecondSignalStage::ReleaseOld=>{p.ordinal+=1;SecondSignalStage::ReleaseOld},
            SecondSignalStage::ResetEnd=>SecondSignalStage::Reserve,
            SecondSignalStage::Reserve=>SecondSignalStage::RefillCoefficients328,
            SecondSignalStage::RefillCoefficients328=>SecondSignalStage::SnapshotSignalPrefix,
            SecondSignalStage::SnapshotSignalPrefix=>SecondSignalStage::AdmitMoveCount,
            SecondSignalStage::MoveSignal=>{p.ordinal+=1;SecondSignalStage::MoveSignal},
            SecondSignalStage::EraseUsingFreshHead=>SecondSignalStage::Complete,
            SecondSignalStage::AdmitMoveCount|SecondSignalStage::Complete=>unreachable!(),
        };
        if p.stage==SecondSignalStage::Complete {
            return Ok(SecondSignalAdvance::ReadyForReceiverSelection(
                ReadyForReceiverSelection{selected_count:p.selected_count,raw_status:p.raw_status,
                    moved_count:p.ordinal}));
        }
    }
}
