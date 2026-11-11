#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum FilterStage {
    Ready,CoefficientTransform,SignalAppend,SignalInputFinish,PairingSnapshot,
    SignalPairClear,SignalPairReserve,SignalPairMove,SignalPrefixErase,
    CoefficientObserve,PairedCall,ObserverFinish,Complete,
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FilterProgress {
    pub stage:FilterStage,pub ordinal:usize,pub frame:i32,
    pub admitted_signal_count:usize,pub current_signal_count:usize,
    pub required:usize,pub available:usize,pub status:u8,
    pub pairing_observed:bool,pub observer_present:bool,pub failed:bool,
}

pub struct FilterWork {progress:FilterProgress,started:bool}
impl FilterWork {

    pub fn after_explicit_prefix(frame:i32,input_signal_count:usize,observer_present:bool)->Self {
        Self{progress:FilterProgress{stage:FilterStage::Ready,ordinal:0,frame,
            admitted_signal_count:input_signal_count,current_signal_count:input_signal_count,
            required:0,available:0,status:0,pairing_observed:false,observer_present,failed:false},started:false}
    }
    pub fn progress(&self)->FilterProgress {self.progress}
}

pub enum FilterValueError<E> {
    PriorUse,Service(E),Pairing{required:usize,available:usize,terminal:u8},
}

pub trait FilterValueServices {
    type Error;
    fn transform_coefficients(&mut self)->Result<(),Self::Error>;

    fn signal_input_count(&self)->usize;
    fn append_signal(&mut self,ordinal:usize,frame:i32)->Result<(),Self::Error>;
    fn finish_signal_input(&mut self)->Result<(),Self::Error>;
    fn coefficient_output_count(&self)->usize;
    fn signal_fifo_count(&self)->usize;
    fn coefficient_output_status(&self)->u8;
    fn clear_pair_signal_and_set_status(&mut self,status:u8)->Result<(),Self::Error>;
    fn reserve_pair_signal(&mut self,required:usize)->Result<(),Self::Error>;
    fn move_signal_to_pair(&mut self,ordinal:usize)->Result<(),Self::Error>;
    fn erase_signal_prefix(&mut self,required:usize)->Result<(),Self::Error>;

    fn observe_coefficient(&mut self,ordinal:usize)->Result<(),Self::Error>;
    fn paired_call(&mut self)->Result<(),Self::Error>;
    fn finish_observer(&mut self)->Result<(),Self::Error>;
}

pub fn forward_initialized<S:FilterValueServices>(work:&mut FilterWork,services:&mut S)
    ->Result<(),FilterValueError<S::Error>> {
    if work.started {return Err(FilterValueError::PriorUse);}
    work.started=true;
    let result=run(work,services);
    if result.is_err(){work.progress.failed=true;}
    result
}

fn run<S:FilterValueServices>(work:&mut FilterWork,services:&mut S)
    ->Result<(),FilterValueError<S::Error>> {
    let p=&mut work.progress;
    p.stage=FilterStage::CoefficientTransform;
    services.transform_coefficients().map_err(FilterValueError::Service)?;
    p.current_signal_count=services.signal_input_count();
    p.stage=FilterStage::SignalAppend;
    for i in 0..p.current_signal_count {
        p.ordinal=i;
        services.append_signal(i,p.frame).map_err(FilterValueError::Service)?;
    }
    p.ordinal=p.current_signal_count;
    p.stage=FilterStage::SignalInputFinish;
    services.finish_signal_input().map_err(FilterValueError::Service)?;
    p.stage=FilterStage::PairingSnapshot;
    p.required=services.coefficient_output_count();
    p.available=services.signal_fifo_count();
    p.status=services.coefficient_output_status();
    p.pairing_observed=true;
    if p.available<p.required||p.status!=0&&p.available!=p.required {
        return Err(FilterValueError::Pairing{required:p.required,available:p.available,terminal:p.status});
    }
    p.stage=FilterStage::SignalPairClear;
    services.clear_pair_signal_and_set_status(p.status).map_err(FilterValueError::Service)?;
    p.stage=FilterStage::SignalPairReserve;
    services.reserve_pair_signal(p.required).map_err(FilterValueError::Service)?;
    p.stage=FilterStage::SignalPairMove;
    for i in 0..p.required {
        p.ordinal=i;
        services.move_signal_to_pair(i).map_err(FilterValueError::Service)?;
    }
    p.ordinal=p.required;
    p.stage=FilterStage::SignalPrefixErase;
    services.erase_signal_prefix(p.required).map_err(FilterValueError::Service)?;
    if p.observer_present {
        p.stage=FilterStage::CoefficientObserve;
        for i in 0..p.required {
            p.ordinal=i;
            services.observe_coefficient(i).map_err(FilterValueError::Service)?;
        }
        p.ordinal=p.required;
    }
    p.stage=FilterStage::PairedCall;
    services.paired_call().map_err(FilterValueError::Service)?;
    if p.observer_present {
        p.stage=FilterStage::ObserverFinish;
        services.finish_observer().map_err(FilterValueError::Service)?;
    }
    p.stage=FilterStage::Complete;
    Ok(())
}
