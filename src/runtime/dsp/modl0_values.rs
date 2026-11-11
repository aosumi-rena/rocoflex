#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {Prepare,Matrix,SelectPost,PostRow,Status,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Progress {pub stage:Stage,pub row:usize,pub post_selected:Option<bool>,pub failed:bool}
pub struct Work {count:usize,progress:Progress}
impl Work {

    pub fn after_admission(count:usize)->Self {
        Self{count,progress:Progress{stage:Stage::Prepare,row:0,post_selected:None,failed:false}}
    }
    pub fn progress(&self)->Progress {self.progress}
}
pub trait Services {
    type Error;

    fn prepare_output(&mut self,count:usize)->Result<(),Self::Error>;

    fn matrix_all_rows(&mut self)->Result<(),Self::Error>;

    fn select_postprocessor(&mut self)->Result<bool,Self::Error>;

    fn postprocess_row(&mut self,row:usize)->Result<(),Self::Error>;

    fn publish_input_status(&mut self)->Result<(),Self::Error>;
}
pub enum Error<E> {PriorFailure,AlreadyComplete,Service(E)}
pub struct Complete {count:usize,post_selected:bool}
impl Complete {
    pub fn count(&self)->usize {self.count}
    pub fn post_selected(&self)->bool {self.post_selected}
}
pub enum Advance {Paused,Complete(Complete)}

pub fn advance<S:Services>(work:&mut Work,services:&mut S,mut budget:usize)->Result<Advance,Error<S::Error>> {
    if work.progress.failed {return Err(Error::PriorFailure);}
    if work.progress.stage==Stage::Complete {return Err(Error::AlreadyComplete);}
    loop {
        if budget==0 {return Ok(Advance::Paused);}
        let result=match work.progress.stage {
            Stage::Prepare=>services.prepare_output(work.count).map(|()|None),
            Stage::Matrix=>services.matrix_all_rows().map(|()|None),
            Stage::SelectPost=>services.select_postprocessor().map(Some),
            Stage::PostRow=>services.postprocess_row(work.progress.row).map(|()|None),
            Stage::Status=>services.publish_input_status().map(|()|None),
            Stage::Complete=>unreachable!(),
        };
        let selected=match result {Ok(value)=>value,Err(error)=>{work.progress.failed=true;return Err(Error::Service(error));}};
        budget-=1;
        work.progress.stage=match work.progress.stage {
            Stage::Prepare=>Stage::Matrix,
            Stage::Matrix=>Stage::SelectPost,
            Stage::SelectPost=>{
                let selected=selected.expect("actual postprocessor selection");
                work.progress.post_selected=Some(selected);
                if selected&&work.count!=0 {Stage::PostRow}else{Stage::Status}
            }
            Stage::PostRow=>{
                work.progress.row+=1;
                if work.progress.row==work.count {Stage::Status}else{Stage::PostRow}
            }
            Stage::Status=>{
                work.progress.stage=Stage::Complete;
                return Ok(Advance::Complete(Complete{count:work.count,
                    post_selected:work.progress.post_selected.expect("post selection precedes status")}));
            }
            Stage::Complete=>unreachable!(),
        };
    }
}
