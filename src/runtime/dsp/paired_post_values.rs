#[path="paired_post_lane.rs"]
pub mod lane;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PostStage {Module56,PrepareNoise,AdmitNoiseRow,NoiseLane,Cursor512,Module40,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct PostProgress {
    pub stage:PostStage,pub enabled:bool,pub groups:usize,pub noise_count:Option<usize>,
    pub row:usize,pub lane:usize,pub failed:bool,
}
pub struct PostWork {progress:PostProgress}
impl PostWork {

    pub fn after_explicit_admission(enabled516:bool,groups504:usize)->Self {
        Self{progress:PostProgress{stage:PostStage::Module56,enabled:enabled516,
            groups:groups504,noise_count:None,row:0,lane:0,failed:false}}
    }
    pub fn progress(&self)->PostProgress {self.progress}
}
pub enum PostError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct PostComplete {enabled:bool,noise_count:Option<usize>}
impl PostComplete {
    pub fn enabled(&self)->bool {self.enabled}
    pub fn noise_count(&self)->Option<usize> {self.noise_count}
}
pub enum PostAdvance {Paused,Complete(PostComplete)}
pub trait PostServices {
    type Error;

    fn module56(&mut self)->Result<(),Self::Error>;

    fn prepare_noise(&mut self)->Result<usize,Self::Error>;

    fn admit_noise_row(&mut self,row:usize)->Result<(),Self::Error>;

    fn apply_noise_lane(&mut self,row:usize,lane:usize)->Result<(),Self::Error>;

    fn commit_cursor512(&mut self,count:usize)->Result<(),Self::Error>;

    fn module40(&mut self)->Result<(),Self::Error>;
}

pub fn advance<S:PostServices>(work:&mut PostWork,services:&mut S,mut budget:usize)
    ->Result<PostAdvance,PostError<S::Error>> {
    if work.progress.failed{return Err(PostError::PriorFailure);}
    if work.progress.stage==PostStage::Complete{return Err(PostError::AlreadyComplete);}
    loop {
        if budget==0{return Ok(PostAdvance::Paused);}
        let p=&mut work.progress;
        let result=match p.stage {
            PostStage::Module56=>services.module56(),
            PostStage::PrepareNoise=>services.prepare_noise().map(|count|{p.noise_count=Some(count);}),
            PostStage::AdmitNoiseRow=>services.admit_noise_row(p.row),
            PostStage::NoiseLane=>services.apply_noise_lane(p.row,p.lane),
            PostStage::Cursor512=>services.commit_cursor512(p.noise_count.expect("prepared noise count")),
            PostStage::Module40=>services.module40(),
            PostStage::Complete=>unreachable!(),
        };
        if let Err(error)=result{p.failed=true;return Err(PostError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            PostStage::Module56=>if p.enabled{PostStage::PrepareNoise}else{PostStage::Module40},
            PostStage::PrepareNoise=>if p.noise_count==Some(0){PostStage::Cursor512}else{PostStage::AdmitNoiseRow},
            PostStage::AdmitNoiseRow=>if p.groups==0{next_row(p)}else{PostStage::NoiseLane},
            PostStage::NoiseLane=>{p.lane+=1;if p.lane==p.groups{next_row(p)}else{PostStage::NoiseLane}},
            PostStage::Cursor512=>PostStage::Module40,
            PostStage::Module40=>PostStage::Complete,
            PostStage::Complete=>unreachable!(),
        };
        if p.stage==PostStage::Complete{return Ok(PostAdvance::Complete(PostComplete{
            enabled:p.enabled,noise_count:p.noise_count}));}
    }
}
fn next_row(progress:&mut PostProgress)->PostStage {
    progress.row+=1;progress.lane=0;
    if Some(progress.row)==progress.noise_count{PostStage::Cursor512}else{PostStage::AdmitNoiseRow}
}
