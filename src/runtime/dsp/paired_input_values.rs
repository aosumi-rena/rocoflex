#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ScalarInputStage {
    Prepare112,SnapshotFrames,AdmitFrame,Copy,Module48,BeginFrontTrim,
    ObserveLatency,ObserveFrontCount,TrimFront,IncrementLatency,
    BeginEndTrim,ObserveEndCount,TrimLast,Complete,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ScalarInputProgress {
    pub stage:ScalarInputStage,pub required_rows:usize,pub ended_status:u8,
    pub frame_count:Option<usize>,pub frame:usize,pub sample_count:Option<usize>,pub sample:usize,
    pub front_limit:Option<i32>,pub latency:Option<i32>,pub feature_count:Option<usize>,
    pub end_trim:Option<bool>,pub failed:bool,
}
pub struct ScalarInputWork {progress:ScalarInputProgress}
impl ScalarInputWork {

    pub fn after_explicit_prefix(required_rows:usize,ended_status:u8)->Self {
        Self{progress:ScalarInputProgress{stage:ScalarInputStage::Prepare112,
            required_rows,ended_status,frame_count:None,frame:0,sample_count:None,sample:0,
            front_limit:None,latency:None,feature_count:None,end_trim:None,failed:false}}
    }
    pub fn progress(&self)->ScalarInputProgress {self.progress}
}
pub enum ScalarInputError<E> {PriorFailure,AlreadyComplete,EmptyAtEos,Service(E)}

pub struct ScalarInputComplete {required_rows:usize,frame_count:usize,ended_status:u8,end_trim:bool}
impl ScalarInputComplete {
    pub fn required_rows(&self)->usize {self.required_rows}
    pub fn frame_count(&self)->usize {self.frame_count}
    pub fn ended_status(&self)->u8 {self.ended_status}
    pub fn end_trim(&self)->bool {self.end_trim}
}
pub enum ScalarInputAdvance {Paused,Complete(ScalarInputComplete)}

pub trait ScalarInputServices {
    type Error;

    fn prepare112_and_set_status(&mut self,required_rows:usize,ended_status:u8)->Result<(),Self::Error>;

    fn snapshot_signal_frame_count(&mut self)->Result<usize,Self::Error>;

    fn admit_signal_frame(&mut self,frame:usize)->Result<usize,Self::Error>;

    fn copy_signal_sample(&mut self,frame:usize,sample:usize)->Result<(),Self::Error>;

    fn module48(&mut self)->Result<(),Self::Error>;

    fn begin_front_trim(&mut self)->Result<i32,Self::Error>;
    fn current_latency496(&mut self)->Result<i32,Self::Error>;

    fn current_feature_count(&mut self)->Result<usize,Self::Error>;

    fn trim_front_and_refresh_end(&mut self)->Result<(),Self::Error>;
    fn increment_latency496(&mut self)->Result<(),Self::Error>;

    fn begin_end_trim(&mut self)->Result<bool,Self::Error>;

    fn trim_last_and_refresh_end(&mut self)->Result<(),Self::Error>;
}

pub fn advance<S:ScalarInputServices>(work:&mut ScalarInputWork,services:&mut S,mut budget:usize)
    ->Result<ScalarInputAdvance,ScalarInputError<S::Error>> {
    if work.progress.failed{return Err(ScalarInputError::PriorFailure);}
    if work.progress.stage==ScalarInputStage::Complete{return Err(ScalarInputError::AlreadyComplete);}
    loop {
        if budget==0{return Ok(ScalarInputAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            ScalarInputStage::Prepare112=>services.prepare112_and_set_status(p.required_rows,p.ended_status),
            ScalarInputStage::SnapshotFrames=>services.snapshot_signal_frame_count().map(|count|{p.frame_count=Some(count);}),
            ScalarInputStage::AdmitFrame=>services.admit_signal_frame(p.frame).map(|count|{p.sample_count=Some(count);}),
            ScalarInputStage::Copy=>services.copy_signal_sample(p.frame,p.sample),
            ScalarInputStage::Module48=>services.module48(),
            ScalarInputStage::BeginFrontTrim=>services.begin_front_trim().map(|limit|{p.front_limit=Some(limit);}),
            ScalarInputStage::ObserveLatency=>services.current_latency496().map(|latency|{p.latency=Some(latency);}),
            ScalarInputStage::ObserveFrontCount|ScalarInputStage::ObserveEndCount=>
                services.current_feature_count().map(|count|{p.feature_count=Some(count);}),
            ScalarInputStage::TrimFront=>services.trim_front_and_refresh_end(),
            ScalarInputStage::IncrementLatency=>services.increment_latency496(),
            ScalarInputStage::BeginEndTrim=>services.begin_end_trim().map(|ended|{p.end_trim=Some(ended);}),
            ScalarInputStage::TrimLast=>services.trim_last_and_refresh_end(),
            ScalarInputStage::Complete=>unreachable!(),
        };
        if let Err(error)=result{p.failed=true;return Err(ScalarInputError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            ScalarInputStage::Prepare112=>ScalarInputStage::SnapshotFrames,
            ScalarInputStage::SnapshotFrames=>if p.frame_count==Some(0){ScalarInputStage::Module48}else{ScalarInputStage::AdmitFrame},
            ScalarInputStage::AdmitFrame=>if p.sample_count==Some(0){next_frame(p)}else{ScalarInputStage::Copy},
            ScalarInputStage::Copy=>{p.sample+=1;if Some(p.sample)==p.sample_count{next_frame(p)}else{ScalarInputStage::Copy}},
            ScalarInputStage::Module48=>ScalarInputStage::BeginFrontTrim,
            ScalarInputStage::BeginFrontTrim=>ScalarInputStage::ObserveLatency,
            ScalarInputStage::ObserveLatency=>if p.latency.expect("current latency")<p.front_limit.expect("front limit"){
                ScalarInputStage::ObserveFrontCount}else{ScalarInputStage::BeginEndTrim},
            ScalarInputStage::ObserveFrontCount=>if p.feature_count==Some(0){ScalarInputStage::BeginEndTrim}else{ScalarInputStage::TrimFront},
            ScalarInputStage::TrimFront=>ScalarInputStage::IncrementLatency,
            ScalarInputStage::IncrementLatency=>ScalarInputStage::ObserveLatency,
            ScalarInputStage::BeginEndTrim=>if p.end_trim==Some(true){ScalarInputStage::ObserveEndCount}else{ScalarInputStage::Complete},
            ScalarInputStage::ObserveEndCount=>{
                if p.feature_count==Some(0){p.failed=true;return Err(ScalarInputError::EmptyAtEos);}
                ScalarInputStage::TrimLast
            },
            ScalarInputStage::TrimLast=>ScalarInputStage::Complete,
            ScalarInputStage::Complete=>unreachable!(),
        };
        if p.stage==ScalarInputStage::Complete{return Ok(ScalarInputAdvance::Complete(ScalarInputComplete{
            required_rows:p.required_rows,frame_count:p.frame_count.expect("snapshotted frame count"),
            ended_status:p.ended_status,end_trim:p.end_trim.expect("current end observation")}));}
    }
}
fn next_frame(progress:&mut ScalarInputProgress)->ScalarInputStage {
    progress.frame+=1;progress.sample=0;progress.sample_count=None;
    if Some(progress.frame)==progress.frame_count{ScalarInputStage::Module48}else{ScalarInputStage::AdmitFrame}
}
