use std::collections::TryReserveError;
use crate::conditional_gate::{Batch,BorrowedProjection,GateError,Projection};
use crate::logical_read_rows::ReadRows;
use crate::modm0_sequence_values::{self as sequence,SequenceWork,SequenceProgress,
    SequenceServices,SequenceAdvance,SequenceError};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CachePolicy {DistinctReturnedValues}

pub trait RequiredReset<F,P> {
    type Error;
    fn reset(&mut self,first:&mut F,following:&mut[P],intermediates:&mut Vec<Batch>)->Result<(),Self::Error>;
}
pub struct Parts<F,P,R> {
    pub first:F,pub following:Vec<P>,

    pub intermediates:Vec<Batch>,pub reset:R,
    pub cache_policy:CachePolicy,pub needs_reset:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Admission {CountDomain,IntermediateExtent}
pub struct Refusal<F,P,R> {pub parts:Parts<F,P,R>,pub reason:Admission}
#[derive(Debug)]
pub enum CallFailure {
    EmptyRouteRequired,MissingChild{index:usize},MissingIntermediate{index:usize},
    MissingFinalOutput,Child{index:usize,reason:GateError},
    Publication{index:usize,reason:TryReserveError},
}
#[derive(Debug)]
pub enum ResetFailure<E> {
    RetainedBatchReservation(TryReserveError),FailureReservation(TryReserveError),Provider(E),
}

pub struct Preserved<F,P,R:RequiredReset<F,P>> {
    pub parts:Parts<F,P,R>,pub retired:Vec<Batch>,pub pending_displaced:Option<Batch>,
    pub returned:Option<Batch>,pub work:Option<SequenceWork>,
    pub pending_failure:Option<CallFailure>,pub previous_failures:Vec<CallFailure>,
    pub reset_failure:Option<ResetFailure<R::Error>>,
}
pub struct LogicalSequence<F,P,R:RequiredReset<F,P>> {
    parts:Parts<F,P,R>,retired:Vec<Batch>,pending_displaced:Option<Batch>,
    returned:Option<Batch>,work:Option<SequenceWork>,pending_failure:Option<CallFailure>,
    previous_failures:Vec<CallFailure>,reset_failure:Option<ResetFailure<R::Error>>,
}
impl<F:BorrowedProjection,P:Projection,R:RequiredReset<F,P>> LogicalSequence<F,P,R> {

    pub fn from_explicit_parts(parts:Parts<F,P,R>)->Result<Self,Refusal<F,P,R>> {
        let reason=if parts.following.len()>=i32::MAX as usize||parts.intermediates.len()>i32::MAX as usize {
            Some(Admission::CountDomain)
        }else if parts.intermediates.capacity()<parts.following.len() {Some(Admission::IntermediateExtent)}else{None};
        if let Some(reason)=reason {return Err(Refusal{parts,reason});}
        Ok(Self{parts,retired:Vec::new(),pending_displaced:None,returned:None,work:None,
            pending_failure:None,previous_failures:Vec::new(),reset_failure:None})
    }
    pub fn parts(&self)->&Parts<F,P,R> {&self.parts}
    pub fn needs_reset(&self)->bool {self.parts.needs_reset}

    pub fn progress(&self)->Option<SequenceProgress> {self.work.as_ref().map(SequenceWork::progress)}
    pub fn retired(&self)->&[Batch] {&self.retired}
    pub fn pending_displaced(&self)->Option<&Batch> {self.pending_displaced.as_ref()}
    pub fn returned(&self)->Option<&Batch> {self.returned.as_ref()}
    pub fn pending_failure(&self)->Option<&CallFailure> {self.pending_failure.as_ref()}
    pub fn previous_failures(&self)->&[CallFailure] {&self.previous_failures}
    pub fn reset_failure(&self)->Option<&ResetFailure<R::Error>> {self.reset_failure.as_ref()}
    fn into_preserved(self)->Preserved<F,P,R> {
        let Self{parts,retired,pending_displaced,returned,work,pending_failure,previous_failures,reset_failure}=self;
        Preserved{parts,retired,pending_displaced,returned,work,pending_failure,previous_failures,reset_failure}
    }

    pub fn into_ready_parts(self)->Result<Preserved<F,P,R>,Self> {
        if self.parts.needs_reset||self.pending_failure.is_some()||self.reset_failure.is_some()
            ||self.pending_displaced.is_some()||self.returned.is_some() {return Err(self);}
        Ok(self.into_preserved())
    }
    fn forward_rows(&mut self,root:&dyn ReadRows)->Result<Batch,GateError> {
        if self.parts.needs_reset||self.pending_failure.is_some()||self.reset_failure.is_some()
            ||self.pending_displaced.is_some()||self.returned.is_some() {return Err(GateError::ResetRequired);}

        self.parts.needs_reset=true;
        let mut work=SequenceWork::after_admission(self.parts.following.len()+1);
        let result={
            let mut services=CallServices{root,first:&mut self.parts.first,following:&mut self.parts.following,
                intermediates:&mut self.parts.intermediates,retired:&mut self.retired,
                pending_displaced:&mut self.pending_displaced,returned:&mut self.returned};
            sequence::advance_sequence(&mut work,&mut services,usize::MAX)
        };
        self.work=Some(work);
        match result {
            Ok(SequenceAdvance::Complete(_))=>{
                let output=self.returned.take().expect("completed sequence owns final returned Batch");
                self.parts.needs_reset=false;
                Ok(output)
            }
            Err(SequenceError::Service(reason))=>{
                let error=match &reason {
                    CallFailure::Child{reason,..}=>reason.clone(),
                    CallFailure::Publication{..}=>GateError::Dependency("logical sequence displaced-output retention stopped"),
                    _=>GateError::Dependency("logical sequence control stopped"),
                };
                self.pending_failure=Some(reason);
                Err(error)
            }
            Ok(SequenceAdvance::Paused)|Err(SequenceError::PriorFailure)|Err(SequenceError::AlreadyComplete)=>
                unreachable!("fresh signed32-bounded synchronous sequence"),
        }
    }
    fn reset_explicit(&mut self)->Result<(),GateError> {
        if self.reset_failure.is_some() {return Err(GateError::ResetRequired);}
        self.parts.needs_reset=true;

        let batches=usize::from(self.pending_displaced.is_some())+usize::from(self.returned.is_some());
        if let Err(error)=self.retired.try_reserve(batches) {
            self.reset_failure=Some(ResetFailure::RetainedBatchReservation(error));
            return Err(GateError::Dependency("logical sequence reset retention allocation stopped"));
        }
        if let Err(error)=self.previous_failures.try_reserve(usize::from(self.pending_failure.is_some())) {
            self.reset_failure=Some(ResetFailure::FailureReservation(error));
            return Err(GateError::Dependency("logical sequence reset failure-history allocation stopped"));
        }
        if let Err(error)=self.parts.reset.reset(&mut self.parts.first,&mut self.parts.following,&mut self.parts.intermediates) {
            self.reset_failure=Some(ResetFailure::Provider(error));
            return Err(GateError::Dependency("required logical sequence reset stopped"));
        }

        if let Some(batch)=self.pending_displaced.take() {self.retired.push(batch);}
        if let Some(batch)=self.returned.take() {self.retired.push(batch);}
        if let Some(error)=self.pending_failure.take() {self.previous_failures.push(error);}
        self.parts.needs_reset=false;
        Ok(())
    }
}
impl<F:BorrowedProjection,P:Projection,R:RequiredReset<F,P>> Projection for LogicalSequence<F,P,R> {
    fn reset(&mut self)->Result<(),GateError> {self.reset_explicit()}
    fn forward(&mut self,input:&Batch)->Result<Batch,GateError> {self.forward_rows(input)}
}
impl<F:BorrowedProjection,P:Projection,R:RequiredReset<F,P>> BorrowedProjection for LogicalSequence<F,P,R> {
    fn forward_borrowed(&mut self,input:&dyn ReadRows)->Result<Batch,GateError> {self.forward_rows(input)}
}

struct CallServices<'a,F,P> {
    root:&'a dyn ReadRows,first:&'a mut F,following:&'a mut[P],intermediates:&'a mut Vec<Batch>,
    retired:&'a mut Vec<Batch>,pending_displaced:&'a mut Option<Batch>,returned:&'a mut Option<Batch>,
}
impl<F:BorrowedProjection,P:Projection> SequenceServices for CallServices<'_,F,P> {
    type Error=CallFailure;
    fn empty_chain(&mut self)->Result<(),CallFailure> {Err(CallFailure::EmptyRouteRequired)}
    fn forward_child(&mut self,index:usize,last:bool)->Result<(),CallFailure> {
        let result=if index==0 {self.first.forward_borrowed(self.root)}else{
            let input=self.intermediates.get(index-1).ok_or(CallFailure::MissingIntermediate{index:index-1})?;
            let child=self.following.get_mut(index-1).ok_or(CallFailure::MissingChild{index})?;
            child.forward(input)
        };

        *self.returned=Some(result.map_err(|reason|CallFailure::Child{index,reason})?);
        if !last {
            if index==self.intermediates.len()&&self.intermediates.capacity()>index {
                self.intermediates.push(self.returned.take().expect("actual child returned Batch"));
                return Ok(());
            }
            let slot=self.intermediates.get_mut(index).ok_or(CallFailure::MissingIntermediate{index})?;
            *self.pending_displaced=Some(std::mem::replace(slot,self.returned.take().expect("actual child returned Batch")));
            self.retired.try_reserve(1).map_err(|reason|CallFailure::Publication{index,reason})?;
            self.retired.push(self.pending_displaced.take().expect("actual displaced cache Batch"));
        }
        Ok(())
    }
    fn publish_final_root_status(&mut self)->Result<(),CallFailure> {
        let output=self.returned.as_mut().ok_or(CallFailure::MissingFinalOutput)?;

        output.status=self.root.source_status();
        Ok(())
    }
}
