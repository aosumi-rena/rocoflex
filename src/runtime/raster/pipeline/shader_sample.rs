use super::{inverse,Path,Required,State};
#[path = "shader_sample_helper.rs"] pub mod helper;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site {AFull,APartial,AFinalFull,AFinalPartial,BFull,BPartial,BFinalFull,BFinalPartial,Bulk27fc6e,Bulk28170e}
impl Site {
    pub fn call_rva(self)->u32 {match self {Self::AFull=>0x27a9d6,Self::APartial=>0x27ab0b,Self::AFinalFull=>0x27ac5d,
        Self::AFinalPartial=>0x27a7dc,Self::BFull=>0x27af46,Self::BPartial=>0x27b060,Self::BFinalFull=>0x27b19d,Self::BFinalPartial=>0x27ad69,Self::Bulk27fc6e=>0x27fc6e,Self::Bulk28170e=>0x28170e}}
    pub fn return_rva(self)->u32 {match self {Self::AFull=>0x27a9db,Self::APartial=>0x27ab10,Self::AFinalFull=>0x27ac62,
        Self::AFinalPartial=>0x27a7e1,Self::BFull=>0x27af4b,Self::BPartial=>0x27b065,Self::BFinalFull=>0x27b1a2,Self::BFinalPartial=>0x27ad6e,Self::Bulk27fc6e=>0x27fc73,Self::Bulk28170e=>0x281713}}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arguments {pub rcx:u64,pub rdx:u64,pub r8d:u32,pub r9d:u32}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentOrigin<I> {
    ShaderWork {stack:I,work40:u64,work24:u64},
    BulkRegisters {bulk_frame:I,bulk_call_receipt:I,current_r14:u64,current_rdi:u64},
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target<I> {
    pub owners:inverse::Owners<I>,pub site:Site,pub arguments:Arguments,
    pub origin:ArgumentOrigin<I>,pub original_descriptor:I,pub native_output:I,
    pub memory_owner:I,pub callee_module:I,pub callee_context:I,pub receipt_owner:I,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnKey<I> {pub target:Target<I>,pub callee_frame:I,pub identity:I,pub observed:inverse::Observation}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputKey<I> {pub target:Target<I>,pub callee_frame:I,pub identity:I}
#[must_use]
pub struct Returned<I,P> {pub key:ReturnKey<I>,pub packet:P}
#[must_use]
pub struct Output<I,P> {pub key:OutputKey<I>,pub packet:P}
pub struct NormalReturn<I,R,O> {
    pub observed:inverse::Observation,pub returned:Option<Returned<I,R>>,pub output:Option<Output<I,O>>,
}
pub struct CallFailure<I,R,O,E> {
    pub error:E,pub observed_normal_return:Option<inverse::Observation>,
    pub returned:Option<Returned<I,R>>,pub output:Option<Output<I,O>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retained<I> {
    pub frame_owner:I,pub history_owner:I,pub node:I,pub previous:Option<I>,
    pub returned:ReturnKey<I>,pub output:OutputKey<I>,
}
pub struct RetentionFailure<I,R,O,E> {
    pub error:E,pub retained:Option<Retained<I>>,pub returned:Option<Returned<I,R>>,pub output:Option<Output<I,O>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    RaxFromQwordWork88,RaxFromSignedDwordRax18,ImulRaxR12,ImulRaxRdi,
    RcxFromQwordWorkB0,EdxFromDwordWork98,R9dFromDwordWork24,
    R8dFromDwordWork98,EdxFromDwordWork24,RcxFromQwordWork88,RaxFromQwordWorkB0,
    RcxFromSignedDwordRcx18,ImulR13dDwordWork98,ImulRcxR12,ShrR13d8,
    ImulEsiDwordWork98,ImulRcxRdi,ShrEsi8,
}
pub trait Services:super::point::Services {
    type SampleReturnPacket;
    type SampleOutputPacket;

    fn bind_sample_27fe50(&mut self,frame:&mut Self::Frame,owners:&inverse::Owners<Self::Identity>,site:Site)
        ->Result<Target<Self::Identity>,Self::Error>;
    fn validate_sample_return(&mut self,frame:&mut Self::Frame,packet:&Returned<Self::Identity,Self::SampleReturnPacket>)
        ->Result<(),Self::Error>;

    fn validate_sample_output(&mut self,frame:&mut Self::Frame,packet:&Output<Self::Identity,Self::SampleOutputPacket>)
        ->Result<(),Self::Error>;
    fn validate_sample_continuation(&mut self,frame:&mut Self::Frame,target:&Target<Self::Identity>,rva:u32)
        ->Result<(),Self::Error>;

    fn retain_sample_packets(&mut self,frame:&mut Self::Frame,previous:Option<&Retained<Self::Identity>>,
        returned:Returned<Self::Identity,Self::SampleReturnPacket>,output:Output<Self::Identity,Self::SampleOutputPacket>)
        ->Result<Option<Retained<Self::Identity>>,RetentionFailure<Self::Identity,Self::SampleReturnPacket,Self::SampleOutputPacket,Self::Error>>;
    fn validate_sample_history(&mut self,frame:&mut Self::Frame,retained:&Retained<Self::Identity>)
        ->Result<(),Self::Error>;

    fn sample_post_instruction(&mut self,frame:&mut Self::Frame,rva:u32,op:Instruction)->Result<(),Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {Admission,Bind,Call,CallStopped,Returned,ValidateReturn,ValidateOutput,
    ValidateContinuation,Retain,RetentionStopped,ValidateHistory,Postcall,Complete}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {WrongEntry,MissingOwners,TargetMismatch,MissingReturn,MissingOutput,
    ReturnMismatch,OutputMismatch,ObservationMismatch,CalleeMismatch,MissingRetention,RetentionMismatch,HelperStopped}
pub enum Cause<E> {Provider(E),Stopped(Stop)}
#[derive(Clone, Copy, Debug)]
pub struct Progress<I> {
    pub phase:Phase,pub attempted_rva:u32,pub site:Option<Site>,pub target:Option<Target<I>>,
    pub observed:Option<inverse::Observation>,pub returned:Option<ReturnKey<I>>,pub output:Option<OutputKey<I>>,
    pub retained:Option<Retained<I>>,pub completed_post_instructions:usize,
    pub helper_progress:Option<helper::Progress<I>>,
}
#[must_use = "preserve SAME loop Frame/history and original partial call packets; no retry"]
pub struct Failure<F,I,AR,AO,R,O,E> {
    pub state:State<F,I,AR,AO>,pub progress:Progress<I>,
    pub returned:Option<Returned<I,R>>,pub output:Option<Output<I,O>>,pub cause:Cause<E>,
    pub helper_failure:Option<helper::InvocationFailure<I,R,O,E>>,
}

pub fn call<S:helper::Services>(host:&mut S,required:Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    let Required {mut state,next_rva}=required;
    let mut p=Progress {phase:Phase::Admission,attempted_rva:next_rva,site:None,target:None,
        observed:None,returned:None,output:None,retained:None,completed_post_instructions:0,helper_progress:None};
    let mut pending_return=None;let mut pending_output=None;
    let mut helper_failure=None;
    let result=(||->Result<u32,Cause<S::Error>> {
        let site=match (state.progress.path,next_rva,state.progress.phase) {
            (Some(Path::A),0x27a9d6,super::Phase::Complete)=>Site::AFull,
            (Some(Path::A),0x27ab0b,super::Phase::Complete)=>Site::APartial,
            (Some(Path::A),0x27ac5d,super::Phase::Complete)=>Site::AFinalFull,
            (Some(Path::A),0x27a7dc,super::Phase::Complete)=>Site::AFinalPartial,
            (Some(Path::B),0x27af46,super::Phase::Complete)=>Site::BFull,
            (Some(Path::B),0x27b060,super::Phase::Complete)=>Site::BPartial,
            (Some(Path::B),0x27b19d,super::Phase::Complete)=>Site::BFinalFull,
            (Some(Path::B),0x27ad69,super::Phase::Complete)=>Site::BFinalPartial,
            _=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        p.site=Some(site);
        let owners=state.shader.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        let path=match site {
            Site::AFull|Site::APartial|Site::AFinalFull|Site::AFinalPartial=>Path::A,
            Site::BFull|Site::BPartial|Site::BFinalFull|Site::BFinalPartial=>Path::B,
            Site::Bulk27fc6e|Site::Bulk28170e=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        host.validate_point_entry(&mut state.shader.frame,&owners,path,next_rva).map_err(Cause::Provider)?;
        p.phase=Phase::Bind;
        let target=host.bind_sample_27fe50(&mut state.shader.frame,&owners,site).map_err(Cause::Provider)?;p.target=Some(target);
        let shader_origin=match target.origin {ArgumentOrigin::ShaderWork {stack,work40,work24}=>
            stack==owners.work_stack&&target.arguments.rcx==work40&&target.arguments.rdx==work24,
            ArgumentOrigin::BulkRegisters {..}=>false};
        if target.owners!=owners||target.site!=site||target.callee_module!=owners.source_module
            ||!shader_origin||target.arguments.r9d!=1 {
            return Err(Cause::Stopped(Stop::TargetMismatch));
        }
        p.phase=Phase::Call;
        let normal=match helper::invoke(host,&mut state.shader.frame,target) {
            Ok(completed)=>{
                p.helper_progress=Some(completed.progress);state.progress.helper_history=Some(completed.progress);completed.normal
            },
            Err(failure)=> {
                p.phase=Phase::CallStopped;p.helper_progress=failure.progress().copied();
                if let Some(progress)=p.helper_progress {state.progress.helper_history=Some(progress);}
                helper_failure=Some(failure);return Err(Cause::Stopped(Stop::HelperStopped));
            }
        };
        p.phase=Phase::Returned;p.attempted_rva=site.return_rva();p.observed=Some(normal.observed);
        pending_return=normal.returned;pending_output=normal.output;
        p.returned=pending_return.as_ref().map(|p|p.key);p.output=pending_output.as_ref().map(|p|p.key);
        let returned=pending_return.as_ref().ok_or(Cause::Stopped(Stop::MissingReturn))?;
        let output=pending_output.as_ref().ok_or(Cause::Stopped(Stop::MissingOutput))?;
        let rk=returned.key;let ok=output.key;
        if rk.target!=target {return Err(Cause::Stopped(Stop::ReturnMismatch));}
        if ok.target!=target {return Err(Cause::Stopped(Stop::OutputMismatch));}
        if rk.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
        if rk.callee_frame!=ok.callee_frame {return Err(Cause::Stopped(Stop::CalleeMismatch));}
        p.phase=Phase::ValidateReturn;host.validate_sample_return(&mut state.shader.frame,returned).map_err(Cause::Provider)?;
        p.phase=Phase::ValidateOutput;host.validate_sample_output(&mut state.shader.frame,output).map_err(Cause::Provider)?;
        p.phase=Phase::ValidateContinuation;
        host.validate_sample_continuation(&mut state.shader.frame,&target,site.return_rva()).map_err(Cause::Provider)?;
        p.phase=Phase::Retain;
        let (returned,output)=match (pending_return.take(),pending_output.take()) {
            (Some(r),Some(o))=>(r,o),(r,o)=>{pending_return=r;pending_output=o;return Err(Cause::Stopped(Stop::MissingRetention));}
        };
        let previous=state.progress.sample_history;
        let retained=match host.retain_sample_packets(&mut state.shader.frame,previous.as_ref(),returned,output) {
            Ok(Some(retained))=>retained,Ok(None)=>return Err(Cause::Stopped(Stop::MissingRetention)),
            Err(RetentionFailure {error,retained,returned,output})=> {
                p.phase=Phase::RetentionStopped;p.retained=retained;pending_return=returned;pending_output=output;
                return Err(Cause::Provider(error));
            }
        };
        p.retained=Some(retained);
        if retained.frame_owner!=owners.frame||retained.returned!=rk||retained.output!=ok
            ||retained.previous!=previous.map(|h|h.node)
            ||previous.map(|h|h.history_owner!=retained.history_owner).unwrap_or(false) {
            return Err(Cause::Stopped(Stop::RetentionMismatch));
        }

        state.progress.sample_history=Some(retained);
        p.phase=Phase::ValidateHistory;host.validate_sample_history(&mut state.shader.frame,&retained).map_err(Cause::Provider)?;
        p.phase=Phase::Postcall;
        use Instruction::*;
        let (plan,next):(&[(u32,Instruction)],u32)=match site {
            Site::AFull=>(&[(0x27a9db,RaxFromQwordWork88),(0x27a9e3,RaxFromSignedDwordRax18),
                (0x27a9e7,ImulRaxR12),(0x27a9eb,RcxFromQwordWorkB0),(0x27a9f3,EdxFromDwordWork98),(0x27a9fa,R9dFromDwordWork24)],0x27a9ff),
            Site::AFinalFull=>(&[(0x27ac62,RaxFromQwordWork88),(0x27ac6a,RaxFromSignedDwordRax18),
                (0x27ac6e,ImulRaxRdi),(0x27ac72,RcxFromQwordWorkB0),(0x27ac7a,EdxFromDwordWork98),(0x27ac81,R9dFromDwordWork24)],0x27ac86),
            Site::BFull=>(&[(0x27af4b,RaxFromQwordWork88),(0x27af53,RaxFromSignedDwordRax18),
                (0x27af57,ImulRaxR12),(0x27af5b,RcxFromQwordWorkB0),(0x27af63,R8dFromDwordWork98),(0x27af6b,EdxFromDwordWork24)],0x27af6f),
            Site::BFinalFull=>(&[(0x27b1a2,RaxFromQwordWork88),(0x27b1aa,RaxFromSignedDwordRax18),
                (0x27b1ae,ImulRaxRdi),(0x27b1b2,RcxFromQwordWorkB0),(0x27b1ba,R8dFromDwordWork98),(0x27b1c2,EdxFromDwordWork24)],0x27b1c6),
            Site::APartial=>(&[(0x27ab10,RcxFromQwordWork88),(0x27ab18,RaxFromQwordWorkB0),
                (0x27ab20,RcxFromSignedDwordRcx18),(0x27ab24,ImulR13dDwordWork98),(0x27ab2d,ImulRcxR12),
                (0x27ab31,ShrR13d8),(0x27ab35,EdxFromDwordWork24)],0x27ab39),
            Site::BPartial=>(&[(0x27b065,RcxFromQwordWork88),(0x27b06d,RaxFromQwordWorkB0),
                (0x27b075,RcxFromSignedDwordRcx18),(0x27b079,ImulRcxR12),(0x27b07d,ImulR13dDwordWork98),
                (0x27b086,ShrR13d8),(0x27b08a,EdxFromDwordWork24)],0x27b08e),
            Site::AFinalPartial=>(&[(0x27a7e1,RcxFromQwordWork88),(0x27a7e9,RaxFromQwordWorkB0),
                (0x27a7f1,RcxFromSignedDwordRcx18),(0x27a7f5,ImulEsiDwordWork98),(0x27a7fd,ImulRcxRdi),
                (0x27a801,ShrEsi8),(0x27a804,EdxFromDwordWork24)],0x27a808),
            Site::BFinalPartial=>(&[(0x27ad6e,RcxFromQwordWork88),(0x27ad76,RaxFromQwordWorkB0),
                (0x27ad7e,RcxFromSignedDwordRcx18),(0x27ad82,ImulRcxRdi),(0x27ad86,ImulEsiDwordWork98),
                (0x27ad8e,ShrEsi8),(0x27ad91,EdxFromDwordWork24)],0x27ad95),

            Site::Bulk27fc6e|Site::Bulk28170e=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        for &(rva,op) in plan {
            p.attempted_rva=rva;state.progress.attempted_rva=rva;
            host.sample_post_instruction(&mut state.shader.frame,rva,op).map_err(Cause::Provider)?;
            p.completed_post_instructions+=1;state.progress.completed_instructions+=1;
        }
        p.phase=Phase::Complete;state.progress.phase=super::Phase::Complete;Ok(next)
    })();
    match result {Ok(next_rva)=>Ok(Required {state,next_rva}),Err(cause)=>Err(Failure {state,progress:p,returned:pending_return,output:pending_output,cause,helper_failure})}
}
