#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EmissionInputError{Lengths}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct EmissionInput{hop:usize,transform_length:usize}
impl EmissionInput{

    pub fn from_explicit_lengths(hop:usize,transform_length:usize)->Result<Self,EmissionInputError>{
        if hop==0||hop>i32::MAX as usize||transform_length==0
            ||transform_length>i32::MAX as usize||hop>transform_length{
            return Err(EmissionInputError::Lengths);
        }
        Ok(Self{hop,transform_length})
    }
    pub fn hop(self)->usize{self.hop}
    pub fn transform_length(self)->usize{self.transform_length}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct EmissionPlan{
    input:EmissionInput,phase_before:i64,next_phase:i64,visible_start:usize,visible_count:usize,
}
impl EmissionPlan{
    pub fn input(self)->EmissionInput{self.input}
    pub fn phase_before(self)->i64{self.phase_before}
    pub fn next_phase(self)->i64{self.next_phase}
    pub fn visible_start(self)->usize{self.visible_start}
    pub fn visible_count(self)->usize{self.visible_count}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EmissionStage{
    Ready,Planning,Admission,CounterAdmission,Arithmetic,Copy,ExchangeCurrent,
    CommitPhase,ShiftOverlap,ReleaseFronts,Complete,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct EmissionProgress{
    pub stage:EmissionStage,pub plan:Option<EmissionPlan>,pub consumed:usize,pub failed:bool,
}
pub struct EmissionWork{progress:EmissionProgress,started:bool}
impl EmissionWork{

    pub fn after_explicit_environment()->Self{
        Self{progress:EmissionProgress{stage:EmissionStage::Ready,plan:None,consumed:0,failed:false},started:false}
    }
    pub fn progress(&self)->EmissionProgress{self.progress}
}
pub enum EmissionValueError<E>{PriorUse,PhaseDomain,CounterOverflow,Service(E)}

pub trait EmissionValueServices{
    type Error;
    fn phase(&self)->i64;
    fn partial_count(&self)->usize;
    fn emitted_rows(&self)->u64;
    fn admit_window(&self,plan:EmissionPlan)->Result<(),Self::Error>;
    fn set_halted(&mut self,value:bool);
    fn accumulate_block(&mut self)->Result<(),Self::Error>;
    fn copy_overlap_to_current(&mut self,source:usize,destination:usize,count:usize)->Result<(),Self::Error>;
    fn set_partial_count(&mut self,value:usize);

    fn exchange_completed_current(&mut self)->Result<(),Self::Error>;
    fn set_emitted_rows(&mut self,value:u64);
    fn set_phase(&mut self,value:i64);

    fn shift_overlap_and_zero_tail(&mut self,hop:usize)->Result<(),Self::Error>;

    fn release_signal_then_coefficient_fronts(&mut self)->Result<(),Self::Error>;
}

pub fn process_window<S:EmissionValueServices>(input:EmissionInput,work:&mut EmissionWork,services:&mut S)
    ->Result<(),EmissionValueError<S::Error>>{
    if work.started{return Err(EmissionValueError::PriorUse);}
    work.started=true;
    let result=run(input,work,services);
    if result.is_err(){work.progress.failed=true;}
    result
}
fn run<S:EmissionValueServices>(input:EmissionInput,work:&mut EmissionWork,services:&mut S)
    ->Result<(),EmissionValueError<S::Error>>{
    let p=&mut work.progress;p.stage=EmissionStage::Planning;
    let phase=services.phase();
    let next_phase=phase.checked_add(input.hop as i64).ok_or(EmissionValueError::PhaseDomain)?;
    let visible_start=phase.max(0).checked_sub(phase).ok_or(EmissionValueError::PhaseDomain)?;
    let visible_count=next_phase.max(0).checked_sub(phase.max(0)).ok_or(EmissionValueError::PhaseDomain)?;

    let start=if visible_count==0{0}else{usize::try_from(visible_start).map_err(|_|EmissionValueError::PhaseDomain)?};
    let count=usize::try_from(visible_count).map_err(|_|EmissionValueError::PhaseDomain)?;
    let plan=EmissionPlan{input,phase_before:phase,next_phase,visible_start:start,visible_count:count};
    p.plan=Some(plan);p.stage=EmissionStage::Admission;
    services.admit_window(plan).map_err(EmissionValueError::Service)?;
    p.stage=EmissionStage::CounterAdmission;

    let completions=services.partial_count().checked_add(count).ok_or(EmissionValueError::CounterOverflow)?/input.hop;
    services.emitted_rows().checked_add(completions as u64).ok_or(EmissionValueError::CounterOverflow)?;
    services.set_halted(true);p.stage=EmissionStage::Arithmetic;
    services.accumulate_block().map_err(EmissionValueError::Service)?;
    while p.consumed<count{
        p.stage=EmissionStage::Copy;
        let partial=services.partial_count();
        let take=(input.hop-partial).min(count-p.consumed);
        services.copy_overlap_to_current(start+p.consumed,partial,take).map_err(EmissionValueError::Service)?;
        services.set_partial_count(partial+take);p.consumed+=take;
        if services.partial_count()==input.hop{
            p.stage=EmissionStage::ExchangeCurrent;
            services.exchange_completed_current().map_err(EmissionValueError::Service)?;
            services.set_partial_count(0);
            let emitted=services.emitted_rows();services.set_emitted_rows(emitted+1);
        }
    }
    p.stage=EmissionStage::CommitPhase;services.set_phase(next_phase);
    p.stage=EmissionStage::ShiftOverlap;
    services.shift_overlap_and_zero_tail(input.hop).map_err(EmissionValueError::Service)?;
    p.stage=EmissionStage::ReleaseFronts;
    services.release_signal_then_coefficient_fronts().map_err(EmissionValueError::Service)?;
    services.set_halted(false);p.stage=EmissionStage::Complete;
    Ok(())
}
