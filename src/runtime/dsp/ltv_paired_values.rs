#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum LtvInputQueue{Signal40,Coefficient80}

pub struct PairedState{
    pub accepted_rows208:u64,pub terminal248:bool,pub halted:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PairedStage{
    Ready,ClearOutput,MoveSignal,MoveCoefficient,ProcessInput,AcceptInput,
    TerminalCheck,FlushCoefficient,FlushSignal,ProcessFlush,
    FinishInputHeaders,FinishInputFacts,Complete,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct PairedProgress{
    pub stage:PairedStage,pub ordinal:usize,pub input_count:usize,
    pub raw_status:u8,pub window_rows:usize,pub failed:bool,
}
pub struct PairedWork{progress:PairedProgress,started:bool}
impl PairedWork{

    pub fn after_explicit_admission(input_count:usize,raw_status:u8,window_rows:usize)->Self{
        Self{progress:PairedProgress{stage:PairedStage::Ready,ordinal:0,input_count,
            raw_status,window_rows,failed:false},started:false}
    }
    pub fn progress(&self)->PairedProgress{self.progress}
}
pub enum PairedValueError<E>{PriorUse,InvalidQueues,EmittedExceedsAccepted,Service(E)}

pub trait PairedValueServices{
    type Error;
    fn clear_output_and_set_status(&mut self,status:u8)->Result<(),Self::Error>;
    fn move_signal_input(&mut self,ordinal:usize)->Result<(),Self::Error>;
    fn move_coefficient_input(&mut self,ordinal:usize)->Result<(),Self::Error>;
    fn signal_count(&self)->usize;
    fn coefficient_count(&self)->usize;
    fn emitted_rows(&self)->u64;
    fn process_front(&mut self)->Result<(),Self::Error>;

    fn append_source_zero(&mut self,queue:LtvInputQueue)->Result<(),Self::Error>;

    fn finish_moved_input_headers(&mut self)->Result<(),Self::Error>;
    fn finish_input_owner_facts(&mut self)->Result<(),Self::Error>;
}

pub fn forward_admitted<S:PairedValueServices>(state:&mut PairedState,work:&mut PairedWork,services:&mut S)
    ->Result<(),PairedValueError<S::Error>>{
    if work.started{return Err(PairedValueError::PriorUse);}
    work.started=true;
    let result=run(state,work,services);
    if result.is_err(){work.progress.failed=true;}
    result
}
fn run<S:PairedValueServices>(state:&mut PairedState,work:&mut PairedWork,services:&mut S)
    ->Result<(),PairedValueError<S::Error>>{
    let p=&mut work.progress;
    state.halted=true;state.terminal248=p.raw_status!=0;
    p.stage=PairedStage::ClearOutput;
    services.clear_output_and_set_status(u8::from(state.terminal248)).map_err(PairedValueError::Service)?;
    for index in 0..p.input_count{
        p.ordinal=index;p.stage=PairedStage::MoveSignal;
        services.move_signal_input(index).map_err(PairedValueError::Service)?;
        p.stage=PairedStage::MoveCoefficient;
        services.move_coefficient_input(index).map_err(PairedValueError::Service)?;
        if services.signal_count()==p.window_rows{
            p.stage=PairedStage::ProcessInput;
            services.process_front().map_err(PairedValueError::Service)?;
        }
        p.stage=PairedStage::AcceptInput;

        state.accepted_rows208+=1;
    }
    p.ordinal=p.input_count;
    if state.terminal248{
        p.stage=PairedStage::TerminalCheck;
        while services.emitted_rows()!=state.accepted_rows208{
            if services.emitted_rows()>state.accepted_rows208{return Err(PairedValueError::EmittedExceedsAccepted);}
            if services.coefficient_count()==0{
                p.stage=PairedStage::FlushCoefficient;
                services.append_source_zero(LtvInputQueue::Coefficient80).map_err(PairedValueError::Service)?;
            }
            p.stage=PairedStage::TerminalCheck;
            if services.signal_count()>p.window_rows{return Err(PairedValueError::InvalidQueues);}
            while services.signal_count()<p.window_rows{
                p.stage=PairedStage::FlushSignal;
                services.append_source_zero(LtvInputQueue::Signal40).map_err(PairedValueError::Service)?;
            }
            p.stage=PairedStage::ProcessFlush;
            services.process_front().map_err(PairedValueError::Service)?;
            p.stage=PairedStage::TerminalCheck;
        }
    }
    p.stage=PairedStage::FinishInputHeaders;
    services.finish_moved_input_headers().map_err(PairedValueError::Service)?;
    p.stage=PairedStage::FinishInputFacts;
    services.finish_input_owner_facts().map_err(PairedValueError::Service)?;
    state.halted=false;p.stage=PairedStage::Complete;
    Ok(())
}
