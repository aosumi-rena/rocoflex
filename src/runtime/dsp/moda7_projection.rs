use std::collections::TryReserveError;
use crate::conditional_gate::{Batch,BorrowedProjection,GateError,Projection};
use crate::logical_read_rows::ReadRows;
use crate::moda7_values::{self as values,BatchServices,SourceModa7RowServices};

pub struct PublishRefusal<E> {pub reason:E,pub output:Batch}

pub trait RequiredServices {
    type Error;

    fn acquire_output(&mut self)->Result<Batch,Self::Error>;

    fn prepare_output(&mut self,output:&mut Batch,count:usize,width:usize)->Result<(),Self::Error>;
    fn input_slot152_to_destination(&mut self,input:&[f32],output:&mut[f32])->Result<(),Self::Error>;
    fn input_slot112_to_destination(&mut self,input:&[f32],output:&mut[f32])->Result<(),Self::Error>;

    fn truncate_output(&mut self,output:&mut Batch,count:usize)->Result<(),Self::Error>;

    fn publish_output(&mut self,output:Batch)->Result<Batch,PublishRefusal<Self::Error>>;

    fn reset(&mut self,active_output:&mut Option<Batch>)->Result<(),Self::Error>;
}
pub struct Parts<S> {pub services:S,pub needs_reset:bool}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Phase {Ready,Admission,Acquire,Source,Publish,Reset,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Control {
    MissingInput{row:usize},MissingOutput{row:usize},
    OutputCount{expected:usize,actual:usize},RowWidth{row:usize,expected:usize,actual:usize},
}
pub enum SourceFailure<E> {Control(Control),Service(E)}
pub enum Failure<E> {
    CountDomain,Acquire(E),Source{progress:values::Progress,reason:SourceFailure<E>},Publish(E),
}
pub enum ResetFailure<E> {Retention(TryReserveError),Provider(E),OutputPending}
pub struct Ready<S:RequiredServices> {
    pub parts:Parts<S>,pub previous_failures:Vec<Failure<S::Error>>,pub work:Option<values::Work>,
}
pub struct Moda7Projection<S:RequiredServices> {
    parts:Parts<S>,phase:Phase,output:Option<Batch>,work:Option<values::Work>,
    failure:Option<Failure<S::Error>>,previous_failures:Vec<Failure<S::Error>>,
    reset_failure:Option<ResetFailure<S::Error>>,
}
impl<S:RequiredServices> Moda7Projection<S> {

    pub fn from_explicit_parts(parts:Parts<S>)->Self {
        Self{parts,phase:Phase::Ready,output:None,work:None,failure:None,
            previous_failures:Vec::new(),reset_failure:None}
    }
    pub fn services(&self)->&S {&self.parts.services}
    pub fn needs_reset(&self)->bool {self.parts.needs_reset}
    pub fn phase(&self)->Phase {self.phase}

    pub fn progress(&self)->Option<values::Progress> {self.work.as_ref().map(values::Work::progress)}
    pub fn active_output(&self)->Option<&Batch> {self.output.as_ref()}
    pub fn failure(&self)->Option<&Failure<S::Error>> {self.failure.as_ref()}
    pub fn previous_failures(&self)->&[Failure<S::Error>] {&self.previous_failures}
    pub fn reset_failure(&self)->Option<&ResetFailure<S::Error>> {self.reset_failure.as_ref()}
    pub fn into_ready_parts(self)->Result<Ready<S>,Self> {
        if self.parts.needs_reset||self.output.is_some()||self.failure.is_some()||self.reset_failure.is_some() {return Err(self);}
        Ok(Ready{parts:self.parts,previous_failures:self.previous_failures,work:self.work})
    }
    fn forward_rows(&mut self,input:&dyn ReadRows)->Result<Batch,GateError> {
        if self.parts.needs_reset||self.output.is_some()||self.failure.is_some()||self.reset_failure.is_some() {
            return Err(GateError::ResetRequired);
        }
        let count=input.row_count();
        self.parts.needs_reset=true;self.phase=Phase::Admission;
        if count>i32::MAX as usize {
            self.failure=Some(Failure::CountDomain);
            return Err(GateError::Dependency("moda7 logical row-count domain"));
        }
        self.phase=Phase::Acquire;
        match self.parts.services.acquire_output() {
            Ok(output)=>self.output=Some(output),
            Err(error)=>{self.failure=Some(Failure::Acquire(error));return Err(GateError::Dependency("moda7 output acquisition stopped"));}
        }
        self.phase=Phase::Source;
        let mut work=values::Work::after_admission(count);
        let result={
            let mut adapter=LogicalBatch{input,output:self.output.as_mut().expect("actual acquired output"),services:&mut self.parts.services};
            values::advance(&mut work,&mut adapter,usize::MAX)
        };
        let progress=work.progress();self.work=Some(work);
        match result {
            Ok(values::Advance::Complete(_))=>{},
            Err(values::Error::Service(reason))=>{
                let error=match &reason {
                    SourceFailure::Control(Control::MissingInput{row})=>GateError::MissingInput{row:*row},
                    SourceFailure::Control(Control::RowWidth{expected,actual,..})=>GateError::RowWidth{expected:*expected,actual:*actual},
                    _=>GateError::Dependency("moda7 logical source effect stopped"),
                };
                self.failure=Some(Failure::Source{progress,reason});return Err(error);
            }
            Ok(values::Advance::Paused)|Err(values::Error::PriorFailure)|Err(values::Error::AlreadyComplete)=>
                unreachable!("fresh signed32-bounded synchronous moda7 work"),
        }
        self.phase=Phase::Publish;
        let output=self.output.take().expect("completed moda7 output owner");
        match self.parts.services.publish_output(output) {
            Ok(output)=>{self.phase=Phase::Complete;self.parts.needs_reset=false;Ok(output)},
            Err(refusal)=>{

                self.output=Some(refusal.output);self.failure=Some(Failure::Publish(refusal.reason));
                Err(GateError::Dependency("moda7 output publication stopped"))
            }
        }
    }
    fn reset_explicit(&mut self)->Result<(),GateError> {
        if self.reset_failure.is_some() {return Err(GateError::ResetRequired);}
        self.parts.needs_reset=true;self.phase=Phase::Reset;
        if let Err(error)=self.previous_failures.try_reserve(usize::from(self.failure.is_some())) {
            self.reset_failure=Some(ResetFailure::Retention(error));return Err(GateError::Dependency("moda7 reset failure retention stopped"));
        }
        if let Err(error)=self.parts.services.reset(&mut self.output) {
            self.reset_failure=Some(ResetFailure::Provider(error));return Err(GateError::Dependency("required moda7 reset stopped"));
        }
        if self.output.is_some() {
            self.reset_failure=Some(ResetFailure::OutputPending);return Err(GateError::Dependency("moda7 reset retains active output"));
        }
        if let Some(error)=self.failure.take() {self.previous_failures.push(error);}
        self.phase=Phase::Ready;self.parts.needs_reset=false;Ok(())
    }
}
impl<S:RequiredServices> Projection for Moda7Projection<S> {
    fn reset(&mut self)->Result<(),GateError> {self.reset_explicit()}
    fn forward(&mut self,input:&Batch)->Result<Batch,GateError> {self.forward_rows(input)}
}
impl<S:RequiredServices> BorrowedProjection for Moda7Projection<S> {
    fn forward_borrowed(&mut self,input:&dyn ReadRows)->Result<Batch,GateError> {self.forward_rows(input)}
}

struct LogicalBatch<'a,S> {input:&'a dyn ReadRows,output:&'a mut Batch,services:&'a mut S}
impl<S:RequiredServices> BatchServices for LogicalBatch<'_,S> {
    type Error=SourceFailure<S::Error>;
    fn prepare_nonempty(&mut self,count:usize)->Result<(),Self::Error> {
        let width=self.input.read_row(0).ok_or(SourceFailure::Control(Control::MissingInput{row:0}))?.len();
        self.services.prepare_output(self.output,count,width).map_err(SourceFailure::Service)?;
        if self.output.rows.len()!=count {return Err(SourceFailure::Control(Control::OutputCount{expected:count,actual:self.output.rows.len()}));}
        for row in 0..count {
            let source=self.input.read_row(row).ok_or(SourceFailure::Control(Control::MissingInput{row}))?;
            let destination=self.output.rows.get(row).ok_or(SourceFailure::Control(Control::MissingOutput{row}))?;
            if source.len()!=width {return Err(SourceFailure::Control(Control::RowWidth{row,expected:width,actual:source.len()}));}
            if destination.len()!=width {return Err(SourceFailure::Control(Control::RowWidth{row,expected:width,actual:destination.len()}));}
        }
        Ok(())
    }
    fn transform_row(&mut self,row:usize)->Result<(),Self::Error> {
        let source=self.input.read_row(row).ok_or(SourceFailure::Control(Control::MissingInput{row}))?;
        let destination=self.output.rows.get_mut(row).ok_or(SourceFailure::Control(Control::MissingOutput{row}))?;

        let mut source=RowBorrow::Input(source);let mut destination=RowBorrow::Output(destination.as_mut_slice());
        let mut math=SliceMath(&mut *self.services);
        values::transform_row(&mut source,&mut destination,&mut math).map_err(SourceFailure::Service)
    }
    fn truncate_output(&mut self,count:usize)->Result<(),Self::Error> {
        self.services.truncate_output(self.output,count).map_err(SourceFailure::Service)
    }
    fn publish_input_status(&mut self)->Result<(),Self::Error> {self.output.status=self.input.source_status();Ok(())}
}
enum RowBorrow<'a> {Input(&'a[f32]),Output(&'a mut[f32])}
struct SliceMath<'a,S>(&'a mut S);
impl<'a,S:RequiredServices> SourceModa7RowServices<RowBorrow<'a>> for SliceMath<'_,S> {
    type Error=S::Error;
    fn input_slot152_to_destination(&mut self,input:&mut RowBorrow<'a>,output:&mut RowBorrow<'a>)->Result<(),Self::Error> {
        let source=match input {RowBorrow::Input(values)=>*values,RowBorrow::Output(_)=>unreachable!("private input role")};
        let destination=match output {RowBorrow::Output(values)=>&mut **values,RowBorrow::Input(_)=>unreachable!("private destination role")};
        self.0.input_slot152_to_destination(source,destination)
    }
    fn input_slot112_to_destination(&mut self,input:&mut RowBorrow<'a>,output:&mut RowBorrow<'a>)->Result<(),Self::Error> {
        let source=match input {RowBorrow::Input(values)=>*values,RowBorrow::Output(_)=>unreachable!("private input role")};
        let destination=match output {RowBorrow::Output(values)=>&mut **values,RowBorrow::Input(_)=>unreachable!("private destination role")};
        self.0.input_slot112_to_destination(source,destination)
    }
}
