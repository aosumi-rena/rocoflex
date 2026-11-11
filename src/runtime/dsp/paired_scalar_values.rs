#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ScalarPackStage {ClearOutput,Copy,Append,ResetFilled,Acquire,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ScalarPackProgress {
    pub stage:ScalarPackStage,pub row:usize,pub input_count:usize,
    pub ended_status:u8,pub failed:bool,
}
pub struct ScalarPackWork {progress:ScalarPackProgress}
impl ScalarPackWork {

    pub fn after_explicit_admission(input_count:usize,ended_status:u8)->Self {
        Self{progress:ScalarPackProgress{stage:ScalarPackStage::ClearOutput,
            row:0,input_count,ended_status,failed:false}}
    }
    pub fn progress(&self)->ScalarPackProgress {self.progress}
}
pub enum ScalarPackError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct PackingComplete {input_count:usize,ended_status:u8}
impl PackingComplete {
    pub fn input_count(&self)->usize {self.input_count}
    pub fn ended_status(&self)->u8 {self.ended_status}
}
pub enum ScalarPackAdvance {Paused,Complete(PackingComplete)}

pub trait ScalarPackServices {
    type Error;

    fn clear_output_and_set_status(&mut self,ended_status:u8)->Result<(),Self::Error>;

    fn copy_lane0_and_test_full(&mut self,row:usize)->Result<bool,Self::Error>;

    fn append_completed_current(&mut self,row:usize)->Result<(),Self::Error>;

    fn reset_filled_after_append(&mut self,row:usize)->Result<(),Self::Error>;

    fn acquire_install_admit_current(&mut self,row:usize)->Result<(),Self::Error>;
}

pub fn advance<S:ScalarPackServices>(work:&mut ScalarPackWork,services:&mut S,
    mut budget:usize)->Result<ScalarPackAdvance,ScalarPackError<S::Error>> {
    if work.progress.failed {return Err(ScalarPackError::PriorFailure);}
    if work.progress.stage==ScalarPackStage::Complete {return Err(ScalarPackError::AlreadyComplete);}
    loop {
        if budget==0 {return Ok(ScalarPackAdvance::Paused);}
        let p=&mut work.progress;
        let mut full=false;
        let result=match p.stage {
            ScalarPackStage::ClearOutput=>services.clear_output_and_set_status(p.ended_status),
            ScalarPackStage::Copy=>services.copy_lane0_and_test_full(p.row).map(|value|{full=value;}),
            ScalarPackStage::Append=>services.append_completed_current(p.row),
            ScalarPackStage::ResetFilled=>services.reset_filled_after_append(p.row),
            ScalarPackStage::Acquire=>services.acquire_install_admit_current(p.row),
            ScalarPackStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(ScalarPackError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            ScalarPackStage::ClearOutput=>if p.input_count==0 {ScalarPackStage::Complete}else{ScalarPackStage::Copy},
            ScalarPackStage::Copy=>if full {ScalarPackStage::Append}else{next_row(p)},
            ScalarPackStage::Append=>ScalarPackStage::ResetFilled,
            ScalarPackStage::ResetFilled=>ScalarPackStage::Acquire,
            ScalarPackStage::Acquire=>next_row(p),
            ScalarPackStage::Complete=>unreachable!(),
        };
        if p.stage==ScalarPackStage::Complete {
            return Ok(ScalarPackAdvance::Complete(PackingComplete{
                input_count:p.input_count,ended_status:p.ended_status}));
        }
    }
}
fn next_row(progress:&mut ScalarPackProgress)->ScalarPackStage {
    progress.row+=1;
    if progress.row==progress.input_count {ScalarPackStage::Complete}else{ScalarPackStage::Copy}
}
