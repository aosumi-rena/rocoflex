pub trait SourceModa7RowServices<Row> {
    type Error;
    fn input_slot152_to_destination(&mut self,input:&mut Row,destination:&mut Row)->Result<(),Self::Error>;
    fn input_slot112_to_destination(&mut self,input:&mut Row,destination:&mut Row)->Result<(),Self::Error>;
}

pub fn transform_row<Row,S:SourceModa7RowServices<Row>>(input:&mut Row,destination:&mut Row,services:&mut S)
    ->Result<(),S::Error> {
    services.input_slot152_to_destination(input,destination)?;
    services.input_slot112_to_destination(input,destination)
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {Prepare,Row,Truncate,Status,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Progress {pub stage:Stage,pub row:usize,pub failed:bool}
pub struct Work {count:usize,progress:Progress}
impl Work {

    pub fn after_admission(count:usize)->Self {
        Self{count,progress:Progress{stage:if count==0{Stage::Truncate}else{Stage::Prepare},row:0,failed:false}}
    }
    pub fn progress(&self)->Progress {self.progress}
}
pub trait BatchServices {
    type Error;

    fn prepare_nonempty(&mut self,count:usize)->Result<(),Self::Error>;

    fn transform_row(&mut self,row:usize)->Result<(),Self::Error>;

    fn truncate_output(&mut self,count:usize)->Result<(),Self::Error>;

    fn publish_input_status(&mut self)->Result<(),Self::Error>;
}
pub enum Error<E> {PriorFailure,AlreadyComplete,Service(E)}
pub struct Complete {count:usize}
impl Complete {pub fn count(&self)->usize {self.count}}
pub enum Advance {Paused,Complete(Complete)}

pub fn advance<S:BatchServices>(work:&mut Work,services:&mut S,mut budget:usize)->Result<Advance,Error<S::Error>> {
    if work.progress.failed {return Err(Error::PriorFailure);}
    if work.progress.stage==Stage::Complete {return Err(Error::AlreadyComplete);}
    loop {
        if budget==0 {return Ok(Advance::Paused);}
        let result=match work.progress.stage {
            Stage::Prepare=>services.prepare_nonempty(work.count),
            Stage::Row=>services.transform_row(work.progress.row),
            Stage::Truncate=>services.truncate_output(work.count),
            Stage::Status=>services.publish_input_status(),
            Stage::Complete=>unreachable!(),
        };
        if let Err(error)=result {work.progress.failed=true;return Err(Error::Service(error));}
        budget-=1;
        work.progress.stage=match work.progress.stage {
            Stage::Prepare=>Stage::Row,
            Stage::Row=>{
                work.progress.row+=1;
                if work.progress.row==work.count {Stage::Truncate}else{Stage::Row}
            }
            Stage::Truncate=>Stage::Status,
            Stage::Status=>{
                work.progress.stage=Stage::Complete;
                return Ok(Advance::Complete(Complete{count:work.count}));
            }
            Stage::Complete=>unreachable!(),
        };
    }
}
