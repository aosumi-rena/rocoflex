use super::{inverse,sample,blend};
#[path = "shader_bulk_body.rs"] mod body;
#[path = "shader_bulk_prefix.rs"] mod prefix;
#[path = "shader_bulk_b.rs"] mod b;
pub use body::source;
pub use b::source as source_b;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {A27fc10,B2816b0}
impl Kind {
    pub fn caller_rva(self)->u32 {match self {Self::A27fc10=>0x27aae2,Self::B2816b0=>0x27b037}}
    pub fn callee_rva(self)->u32 {match self {Self::A27fc10=>0x27fc10,Self::B2816b0=>0x2816b0}}
    pub fn caller_return_rva(self)->u32 {match self {Self::A27fc10=>0x27aae7,Self::B2816b0=>0x27b03c}}
    pub fn sample_site(self)->sample::Site {match self {Self::A27fc10=>sample::Site::Bulk27fc6e,Self::B2816b0=>sample::Site::Bulk28170e}}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry<I> {
    pub kind:Kind,
    pub owners:inverse::Owners<I>,pub callee_frame:I,pub callee_context:I,pub callee_stack:I,
    pub actual_call_receipt:I,pub descriptor:I,pub native_output:I,pub memory_owner:I,pub receipt_owner:I,
    pub prior_sample_history:Option<sample::Retained<I>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {At27fc3c,At27fc4c,At2816dc,At2816ec}
impl Resource {
    pub fn call_rva(self)->u32 {match self {Self::At27fc3c=>0x27fc3c,Self::At27fc4c=>0x27fc4c,Self::At2816dc=>0x2816dc,Self::At2816ec=>0x2816ec}}
    pub fn callee_rva(self)->u32 {match self {Self::At27fc3c|Self::At2816dc=>0x580c24,Self::At27fc4c|Self::At2816ec=>0x583918}}
    pub fn return_rva(self)->u32 {match self {Self::At27fc3c=>0x27fc41,Self::At27fc4c=>0x27fc51,Self::At2816dc=>0x2816e1,Self::At2816ec=>0x2816f1}}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceTarget<I> {
    pub entry:Entry<I>,pub site:Resource,pub rcx:u64,pub callee_module:I,pub callee_context:I,
    pub physical_h:I,pub actual_argument_backing:I,pub effects_owner:I,pub receipt_owner:I,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceReturn<I> {
    pub target:ResourceTarget<I>,pub callee_frame:I,pub identity:I,pub observed:inverse::Observation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceEffects<I> {pub target:ResourceTarget<I>,pub callee_frame:I,pub identity:I}
pub struct ResourceNormal<I> {
    pub observed:inverse::Observation,pub returned:Option<ResourceReturn<I>>,pub effects:Option<ResourceEffects<I>>,
}
pub struct ResourceFailure<I,E> {
    pub error:E,pub observed_normal_return:Option<inverse::Observation>,
    pub returned:Option<ResourceReturn<I>>,pub effects:Option<ResourceEffects<I>>,
}
#[derive(Clone, Copy, Debug)]
pub struct ResourceProgress<I> {
    pub target:ResourceTarget<I>,pub observed:Option<inverse::Observation>,
    pub returned:Option<ResourceReturn<I>>,pub effects:Option<ResourceEffects<I>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnKey<I> {pub entry:Entry<I>,pub identity:I,pub observed:inverse::Observation}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputKey<I> {pub entry:Entry<I>,pub identity:I}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Normal<I> {
    pub observed:inverse::Observation,pub returned:Option<ReturnKey<I>>,pub output:Option<OutputKey<I>>,
    pub sample_history:Option<sample::Retained<I>>,
}
pub struct RetFailure<I,E> {pub error:E,pub partial:Normal<I>}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Push(sample::helper::Saved),SubRsp20,EbxFromR9d,EsiFromR8d,EbpFromEdx,R14FromRcx,
    CmpDwordRcx80R8d,RdiFromSignedEsi,QwordR1480FromRdi,RcxFromQwordR1478,
    TestEdi,ShlRdi2,RcxFromRdi,RdiFromCurrentRax,RdiFromQwordR1478,XorEdi,QwordR1478FromRdi,
    RcxFromR14,RdxFromRdi,R8dFromEbp,R9dFromEsi,
    RaxFromQwordR1448,RaxFromSignedDwordRax18,RcxFromSignedEbp,ImulRcxRax,AddRcxQwordR1470,
    ImulEbxDwordR1458,SarEbx8,IncEsi,CmpEbxFe,NopDwordRax,
    AddRcxRax,DecEsi,CmpEsi1,AddRsp20,Pop(sample::helper::Saved),
    RcxFromQwordR1470,EdxFromDwordRax18,EaxFromEdx,ImulEaxEbp,R8FromSignedEax,RaxFromSignedEdx,
    CmpEdx1,RdxFromSignedEbp,MovzxEdxByteRcxRdxPlus1,AddRcxR8,AddRcxTwo,NopWordRaxRax,NopDwordRaxRax,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {SignedGreaterEqual,Equal,NotEqual,SignedGreater}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control {pub rva:u32,pub condition:Condition,pub taken:u32,pub fallthrough:u32}
pub trait Services:blend::Services+sample::helper::Services {

    fn enter_bulk(&mut self,frame:&mut Self::Frame,owners:&inverse::Owners<Self::Identity>,kind:Kind,
        prior:Option<&sample::Retained<Self::Identity>>)->Result<Entry<Self::Identity>,Self::Error>;
    fn validate_bulk_entry(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>)->Result<(),Self::Error>;

    fn validate_bulk_body_continuation(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,rva:u32)
        ->Result<(),Self::Error>;

    fn bulk_instruction(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,rva:u32,op:Instruction)
        ->Result<(),Self::Error>;

    fn bulk_branch(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,control:Control)->Result<bool,Self::Error>;
    fn bulk_jump(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,rva:u32,target:u32)->Result<(),Self::Error>;

    fn bind_bulk_resource(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,site:Resource)
        ->Result<ResourceTarget<Self::Identity>,Self::Error>;

    fn actual_bulk_resource(&mut self,frame:&mut Self::Frame,target:&ResourceTarget<Self::Identity>)
        ->Result<ResourceNormal<Self::Identity>,ResourceFailure<Self::Identity,Self::Error>>;
    fn validate_bulk_resource_return(&mut self,frame:&mut Self::Frame,packet:&ResourceReturn<Self::Identity>)->Result<(),Self::Error>;
    fn validate_bulk_resource_effects(&mut self,frame:&mut Self::Frame,packet:&ResourceEffects<Self::Identity>)->Result<(),Self::Error>;
    fn validate_bulk_resource_continuation(&mut self,frame:&mut Self::Frame,target:&ResourceTarget<Self::Identity>,rva:u32)->Result<(),Self::Error>;

    fn bind_bulk_sample(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>)
        ->Result<sample::Target<Self::Identity>,Self::Error>;

    fn actual_bulk_ret_27fe45(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,
        history:Option<&sample::Retained<Self::Identity>>)->Result<Normal<Self::Identity>,RetFailure<Self::Identity,Self::Error>>;

    fn actual_bulk_ret_281a65(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,
        history:Option<&sample::Retained<Self::Identity>>)->Result<Normal<Self::Identity>,RetFailure<Self::Identity,Self::Error>>;
    fn validate_bulk_return(&mut self,frame:&mut Self::Frame,returned:&ReturnKey<Self::Identity>)->Result<(),Self::Error>;
    fn validate_bulk_output(&mut self,frame:&mut Self::Frame,output:&OutputKey<Self::Identity>)->Result<(),Self::Error>;
    fn validate_bulk_caller(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,rva:u32)->Result<(),Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {Entry,Prefix,Resource,ResourceStopped,Sample,SampleStopped,RetainSample,SampleRetained,
    BlendSetup,Blend,Branch,RequiredBody,Epilogue,Ret,RetStopped,ValidateRet,Returned}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {WrongEntry,EntryMismatch,ResourceTargetMismatch,MissingReturn,MissingEffects,ReturnMismatch,
    OutputMismatch,CalleeMismatch,ObservationMismatch,SampleTargetMismatch,MissingRetention,RetentionMismatch,HistoryMismatch,HelperStopped}
pub enum Cause<E> {Provider(E),Stopped(Stop)}
#[derive(Clone, Copy, Debug)]
pub struct Progress<I> {
    pub entry:Entry<I>,pub phase:Phase,pub attempted_rva:u32,pub completed_instructions:usize,
    pub last_control:Option<Control>,pub last_control_taken:Option<bool>,pub resources:[Option<ResourceProgress<I>>;2],
    pub sample_target:Option<sample::Target<I>>,pub sample_observed:Option<inverse::Observation>,
    pub sample_history:Option<sample::Retained<I>>,pub sample_retention_attempt:Option<sample::Retained<I>>,
    pub normal:Option<Normal<I>>,
    pub helper_progress:Option<sample::helper::Progress<I>>,
}
#[must_use = "all native/call/partial sample packets stay in original Frame/error; no retry"]
pub struct Failure<I,R,O,E> {
    pub progress:Progress<I>,pub cause:Cause<E>,pub sample_normal:Option<sample::NormalReturn<I,R,O>>,
    pub sample_return:Option<sample::Returned<I,R>>,pub sample_output:Option<sample::Output<I,O>>,
    pub helper_failure:Option<sample::helper::InvocationFailure<I,R,O,E>>,
}
#[must_use]
pub struct Completed<I> {pub progress:Progress<I>,pub normal:Normal<I>}
#[must_use = "genuine B bulk remains IN FLIGHT, no native RET/caller restoration"]
pub struct RequiredBody<I> {pub progress:Progress<I>,pub next_rva:u32}
#[must_use]
pub enum BodyOutcome<I> {Returned(Completed<I>),Required(RequiredBody<I>)}
#[must_use]
pub struct Required<F,I,AR,AO> {pub state:super::State<F,I,AR,AO>,pub body:RequiredBody<I>}
#[must_use]
pub enum CallOutcome<F,I,AR,AO> {
    Returned(super::Required<F,I,AR,AO>),Required(Required<F,I,AR,AO>),
}

pub(super) fn finish<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)
    ->Result<Normal<S::Identity>,Cause<S::Error>>
{
    let entry=p.entry;
    let pc=|a,b|match entry.kind {Kind::A27fc10=>a,Kind::B2816b0=>b};
    use sample::helper::Saved;use Instruction::*;
    p.phase=Phase::Epilogue;
    for &(a,b,op) in &[(0x27fe3b,0x281a5b,AddRsp20),(0x27fe3f,0x281a5f,Pop(Saved::Rbx)),
        (0x27fe40,0x281a60,Pop(Saved::Rbp)),(0x27fe41,0x281a61,Pop(Saved::Rdi)),
        (0x27fe42,0x281a62,Pop(Saved::Rsi)),(0x27fe43,0x281a63,Pop(Saved::R14))] {
        let rva=pc(a,b);p.attempted_rva=rva;
        host.bulk_instruction(frame,&entry,rva,op).map_err(Cause::Provider)?;
        p.completed_instructions=p.completed_instructions.wrapping_add(1);
    }
    p.phase=Phase::Ret;p.attempted_rva=pc(0x27fe45,0x281a65);
    let returned=match entry.kind {
        Kind::A27fc10=>host.actual_bulk_ret_27fe45(frame,&entry,p.sample_history.as_ref()),
        Kind::B2816b0=>host.actual_bulk_ret_281a65(frame,&entry,p.sample_history.as_ref()),
    };
    let normal=match returned {
        Ok(normal)=>normal,Err(RetFailure {error,partial})=>{
            p.phase=Phase::RetStopped;p.normal=Some(partial);return Err(Cause::Provider(error));
        }
    };
    p.normal=Some(normal);p.phase=Phase::ValidateRet;
    let returned=normal.returned.ok_or(Cause::Stopped(Stop::MissingReturn))?;
    let output=normal.output.ok_or(Cause::Stopped(Stop::MissingEffects))?;
    if returned.entry!=entry {return Err(Cause::Stopped(Stop::ReturnMismatch));}
    if output.entry!=entry {return Err(Cause::Stopped(Stop::OutputMismatch));}
    if returned.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
    if normal.sample_history!=p.sample_history {return Err(Cause::Stopped(Stop::HistoryMismatch));}
    host.validate_bulk_return(frame,&returned).map_err(Cause::Provider)?;
    host.validate_bulk_output(frame,&output).map_err(Cause::Provider)?;
    host.validate_bulk_caller(frame,&entry,entry.kind.caller_return_rva()).map_err(Cause::Provider)?;
    p.phase=Phase::Returned;Ok(normal)
}

pub(super) fn resource<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>,slot:usize,site:Resource)
    ->Result<(),Cause<S::Error>>
{
    p.phase=Phase::Resource;p.attempted_rva=site.call_rva();
    let target=host.bind_bulk_resource(frame,&p.entry,site).map_err(Cause::Provider)?;
    p.resources[slot]=Some(ResourceProgress {target,observed:None,returned:None,effects:None});
    if target.entry!=p.entry||target.site!=site||target.callee_module!=p.entry.owners.source_module {
        return Err(Cause::Stopped(Stop::ResourceTargetMismatch));
    }
    let normal=match host.actual_bulk_resource(frame,&target) {
        Ok(normal)=>normal,
        Err(ResourceFailure {error,observed_normal_return,returned,effects})=>{
            p.phase=Phase::ResourceStopped;p.resources[slot]=Some(ResourceProgress {target,observed:observed_normal_return,returned,effects});
            return Err(Cause::Provider(error));
        }
    };
    p.resources[slot]=Some(ResourceProgress {target,observed:Some(normal.observed),returned:normal.returned,effects:normal.effects});
    let returned=normal.returned.ok_or(Cause::Stopped(Stop::MissingReturn))?;
    let effects=normal.effects.ok_or(Cause::Stopped(Stop::MissingEffects))?;
    if returned.target!=target {return Err(Cause::Stopped(Stop::ReturnMismatch));}
    if effects.target!=target {return Err(Cause::Stopped(Stop::OutputMismatch));}
    if returned.callee_frame!=effects.callee_frame {return Err(Cause::Stopped(Stop::CalleeMismatch));}
    if returned.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
    host.validate_bulk_resource_return(frame,&returned).map_err(Cause::Provider)?;
    host.validate_bulk_resource_effects(frame,&effects).map_err(Cause::Provider)?;
    host.validate_bulk_resource_continuation(frame,&target,site.return_rva()).map_err(Cause::Provider)
}

#[must_use]
pub struct CallerFailure<F,I,AR,AO,R,O,E> {
    pub state:super::State<F,I,AR,AO>,pub entry_error:Option<E>,pub body:Option<Failure<I,R,O,E>>,pub stopped:Option<Stop>,
    pub observed_entry:Option<Entry<I>>,
}
pub fn call<S:Services>(host:&mut S,required:super::Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<CallOutcome<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        CallerFailure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    let super::Required {mut state,next_rva}=required;
    let (kind,path)=match (state.progress.path,next_rva) {
        (Some(super::Path::A),0x27aae2)=>(Kind::A27fc10,super::Path::A),
        (Some(super::Path::B),0x27b037)=>(Kind::B2816b0,super::Path::B),
        _=>return Err(CallerFailure {state,entry_error:None,body:None,stopped:Some(Stop::WrongEntry),observed_entry:None}),
    };
    if state.progress.phase!=super::Phase::Complete {
        return Err(CallerFailure {state,entry_error:None,body:None,stopped:Some(Stop::WrongEntry),observed_entry:None});
    }
    let owners=match state.shader.inverse.owners {Some(owners)=>owners,None=>return Err(CallerFailure {state,entry_error:None,body:None,stopped:Some(Stop::EntryMismatch),observed_entry:None})};
    if let Err(error)=host.validate_point_entry(&mut state.shader.frame,&owners,path,next_rva) {
        return Err(CallerFailure {state,entry_error:Some(error),body:None,stopped:None,observed_entry:None});
    }
    let prior=state.progress.sample_history;
    let entry=match host.enter_bulk(&mut state.shader.frame,&owners,kind,prior.as_ref()) {
        Ok(entry)=>entry,Err(error)=>return Err(CallerFailure {state,entry_error:Some(error),body:None,stopped:None,observed_entry:None}),
    };
    if entry.kind!=kind||entry.owners!=owners||entry.prior_sample_history!=prior {

        return Err(CallerFailure {state,entry_error:None,body:None,stopped:Some(Stop::EntryMismatch),observed_entry:Some(entry)});
    }
    if kind==Kind::B2816b0 {
        let result=source_b(host,&mut state.shader.frame,entry);
        return join_body::<S>(state,entry,result);
    }
    let result=source(host,&mut state.shader.frame,entry).map(BodyOutcome::Returned);
    join_body::<S>(state,entry,result)
}

pub fn resume_b<S:Services>(host:&mut S,required:Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<CallOutcome<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        CallerFailure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    let Required {mut state,body}=required;
    let entry=body.progress.entry;
    if entry.kind!=Kind::B2816b0||state.progress.path!=Some(super::Path::B)
        ||state.shader.inverse.owners!=Some(entry.owners) {
        return Err(CallerFailure {state,entry_error:None,
            body:Some(prefix::Pending::new().failure(body.progress,Cause::Stopped(Stop::WrongEntry))),
            stopped:None,observed_entry:Some(entry)});
    }
    let result=b::resume(host,&mut state.shader.frame,body);
    join_body::<S>(state,entry,result)
}

fn join_body<S:Services>(mut state:super::State<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
    entry:Entry<S::Identity>,result:Result<BodyOutcome<S::Identity>,Failure<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>)
    ->Result<CallOutcome<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        CallerFailure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    match result {
        Ok(BodyOutcome::Required(body))=>{
            state.progress.sample_history=body.progress.sample_history;
            if let Some(progress)=body.progress.helper_progress {state.progress.helper_history=Some(progress);}
            state.progress.bulk_history=Some(body.progress);
            Ok(CallOutcome::Required(Required {state,body}))
        },
        Ok(BodyOutcome::Returned(completed))=>{
            state.progress.sample_history=completed.progress.sample_history;
            if let Some(progress)=completed.progress.helper_progress {state.progress.helper_history=Some(progress);}
            state.progress.bulk_history=Some(completed.progress);
            state.progress.phase=super::Phase::Complete;
            Ok(CallOutcome::Returned(super::Required {state,next_rva:entry.kind.caller_return_rva()}))
        },
        Err(failure)=>{
            if let Some(progress)=failure.progress.helper_progress {state.progress.helper_history=Some(progress);}
            state.progress.sample_history=failure.progress.sample_history;state.progress.bulk_history=Some(failure.progress);
            Err(CallerFailure {state,entry_error:None,body:Some(failure),stopped:None,observed_entry:Some(entry)})
        }
    }
}
