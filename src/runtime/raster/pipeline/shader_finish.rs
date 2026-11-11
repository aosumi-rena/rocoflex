use super::{inverse,Path};
use super::super::Services as PrefixServices;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    RaxFromQwordWorkB8,RcxFromCurrentRax, Xmm6FromAlignedWorkD0, AddRspE8,
    PopRbx,PopRbp,PopRdi,PopRsi,PopR12,PopR13,PopR14,PopR15,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallTarget<I> {
    pub owners:inverse::Owners<I>,pub rcx:u64,pub callee_module:I,
    pub callee_context:I,pub argument_backing:I,pub effects_owner:I,pub receipt_owner:I,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallKey<I> {
    pub target:CallTarget<I>,pub callee_frame:I,pub receipt_identity:I,
    pub observed:inverse::Observation,
}
#[must_use]
pub struct Called<I,P> {pub key:CallKey<I>,pub packet:P}
pub struct CallReturn<I,P> {pub observed:inverse::Observation,pub returned:Option<Called<I,P>>}
pub struct CallFailure<I,P,E> {
    pub error:E,pub observed_normal_return:Option<inverse::Observation>,pub returned:Option<Called<I,P>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetTarget<I> {
    pub owners:inverse::Owners<I>,pub actual_caller:I,pub return_stack_backing:I,pub receipt_owner:I,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetObservation {pub rip:u64,pub rsp:u64,pub raw_rax:u64,pub mxcsr:u32}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetKey<I> {pub target:RetTarget<I>,pub receipt_identity:I,pub observed:RetObservation}
#[must_use]
pub struct Returned<I,P> {pub key:RetKey<I>,pub packet:P}
pub struct RetReturn<I,P> {pub observed:RetObservation,pub returned:Option<Returned<I,P>>}
pub struct RetFailure<I,P,E> {
    pub error:E,pub observed_return:Option<RetObservation>,pub returned:Option<Returned<I,P>>,
}
pub trait Services:super::Services {
    type CleanupPacket;
    type ShaderReturnPacket;

    fn finish_instruction(&mut self,frame:&mut Self::Frame,rva:u32,op:Instruction)->Result<(),Self::Error>;

    fn validate_row_exit(&mut self,frame:&mut Self::Frame,owners:&inverse::Owners<Self::Identity>,
        control:super::root_loop::point::Control)->Result<(),Self::Error>;

    fn bind_cleanup_580c24(&mut self,frame:&mut Self::Frame,owners:&inverse::Owners<Self::Identity>)
        ->Result<CallTarget<Self::Identity>,Self::Error>;

    fn actual_cleanup_580c24(&mut self,frame:&mut Self::Frame,target:&CallTarget<Self::Identity>)
        ->Result<CallReturn<Self::Identity,Self::CleanupPacket>,CallFailure<Self::Identity,Self::CleanupPacket,Self::Error>>;
    fn validate_cleanup_return(&mut self,frame:&mut Self::Frame,packet:&Called<Self::Identity,Self::CleanupPacket>)
        ->Result<(),Self::Error>;
    fn validate_cleanup_continuation(&mut self,frame:&mut Self::Frame,target:&CallTarget<Self::Identity>,rva:u32)
        ->Result<(),Self::Error>;

    fn bind_shader_ret(&mut self,frame:&mut Self::Frame,owners:&inverse::Owners<Self::Identity>)
        ->Result<RetTarget<Self::Identity>,Self::Error>;

    fn actual_shader_ret_27fc0a(&mut self,frame:&mut Self::Frame,target:&RetTarget<Self::Identity>)
        ->Result<RetReturn<Self::Identity,Self::ShaderReturnPacket>,RetFailure<Self::Identity,Self::ShaderReturnPacket,Self::Error>>;
    fn validate_shader_ret(&mut self,frame:&mut Self::Frame,packet:&Returned<Self::Identity,Self::ShaderReturnPacket>)
        ->Result<(),Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {Admission,MoveArgument,BindCall,Call,CallStopped,CallReturned,ValidateCall,
    Epilogue,BindRet,Ret,RetStopped,RetReturned,ValidateRet,Complete}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {WrongEntry,MissingBranch,BranchMismatch,MissingOwners,TargetMismatch,
    MissingCallReturn,CallMismatch,ObservationMismatch,MissingShaderReturn,ShaderReturnMismatch}
pub enum Cause<E> {Provider(E),Stopped(Stop)}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {InitialHeightJle,RowExitJge}
#[derive(Clone, Copy, Debug)]
pub struct Progress<I> {
    pub phase:Phase,pub admission:Admission,pub attempted_rva:u32,pub completed_instructions:usize,
    pub target:Option<CallTarget<I>>,pub call_observed:Option<inverse::Observation>,pub called:Option<CallKey<I>>,
    pub ret_target:Option<RetTarget<I>>,pub ret_observed:Option<RetObservation>,pub returned:Option<RetKey<I>>,
}
#[must_use = "retain SAME original Frame and all inverse/allocation/cleanup/RET packets"]
pub struct State<F,I,AR,AO,C,R> {
    pub shader:super::State<F,I,AR,AO>,pub progress:Progress<I>,
    pub loop_history:Option<super::root_loop::Progress<I>>,
    pub called:Option<Called<I,C>>,pub returned:Option<Returned<I,R>>,
}
#[must_use]
pub struct Failure<T,E> {pub state:T,pub cause:Cause<E>}

#[must_use]
pub struct Completed<T> {pub state:T}
pub type StateOf<S> = State<<S as PrefixServices>::Frame,<S as inverse::Services>::Identity,
    <S as super::Services>::AllocationReturnPacket,<S as super::Services>::AllocationOutputPacket,
    <S as Services>::CleanupPacket,<S as Services>::ShaderReturnPacket>;

pub fn from_signed_height_branch<S:Services>(host:&mut S,
    required:super::Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Completed<StateOf<S>>,Failure<StateOf<S>,S::Error>>
{
    let super::Required {state:shader,next_rva}=required;
    let state=State {shader,loop_history:None,progress:Progress {phase:Phase::Admission,admission:Admission::InitialHeightJle,attempted_rva:next_rva,
        completed_instructions:0,target:None,call_observed:None,called:None,ret_target:None,
        ret_observed:None,returned:None},called:None,returned:None};
    run_common(host,state,next_rva)
}

pub fn from_row_exit<S:Services>(host:&mut S,
    required:super::root_loop::Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Completed<StateOf<S>>,Failure<StateOf<S>,S::Error>>
{
    let super::root_loop::Required {state:super::root_loop::State {shader,progress:loop_history},next_rva}=required;
    let state=State {shader,loop_history:Some(loop_history),progress:Progress {phase:Phase::Admission,
        admission:Admission::RowExitJge,attempted_rva:next_rva,completed_instructions:0,target:None,
        call_observed:None,called:None,ret_target:None,ret_observed:None,returned:None},called:None,returned:None};
    run_common(host,state,next_rva)
}

fn run_common<S:Services>(host:&mut S,mut state:StateOf<S>,next_rva:u32)
    ->Result<Completed<StateOf<S>>,Failure<StateOf<S>,S::Error>>
{
    let result=(||->Result<(),Cause<S::Error>> {
        let owners=state.shader.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        match state.progress.admission {
        Admission::InitialHeightJle=>{
        if next_rva!=0x27fbe7||state.shader.progress.phase!=super::Phase::Complete
            ||state.shader.progress.height_branch_taken!=Some(true) {
            return Err(Cause::Stopped(Stop::WrongEntry));
        }
        let branch=state.shader.progress.height_branch.ok_or(Cause::Stopped(Stop::MissingBranch))?;
        let branch_rva=match branch.path {Path::A=>0x27a67b,Path::B=>0x27a718};
        if branch.owners!=owners||Some(branch.path)!=state.shader.progress.path
            ||branch.rva!=branch_rva||!branch.taken||branch.destination!=0x27fbe7 {
            return Err(Cause::Stopped(Stop::BranchMismatch));
        }
        host.validate_height_branch(&mut state.shader.frame,&branch).map_err(Cause::Provider)?;
        },
        Admission::RowExitJge=>{
            use super::root_loop::{Phase as LoopPhase,point::{Control,Condition}};
            if next_rva!=0x27fbdf {return Err(Cause::Stopped(Stop::WrongEntry));}
            let history=state.loop_history.as_ref().ok_or(Cause::Stopped(Stop::MissingBranch))?;
            if history.phase!=LoopPhase::Complete||history.point_control_taken!=Some(true) {
                return Err(Cause::Stopped(Stop::BranchMismatch));
            }
            let (rva,fallthrough)=match history.path {
                Some(Path::A)=>(0x27a8df,0x27a8e5),Some(Path::B)=>(0x27ae54,0x27ae5a),
                None=>return Err(Cause::Stopped(Stop::BranchMismatch)),
            };
            let control=Control {rva,condition:Condition::SignedGreaterEqual,taken:0x27fbdf,fallthrough};
            if history.point_control!=Some(control) {return Err(Cause::Stopped(Stop::BranchMismatch));}
            host.validate_row_exit(&mut state.shader.frame,&owners,control).map_err(Cause::Provider)?;
        }}
        macro_rules! ins {($rva:expr,$op:expr)=>{{
            state.progress.attempted_rva=$rva;
            host.finish_instruction(&mut state.shader.frame,$rva,$op).map_err(Cause::Provider)?;
            state.progress.completed_instructions+=1;
        }};}
        state.progress.phase=Phase::MoveArgument;
        if state.progress.admission==Admission::RowExitJge {ins!(0x27fbdf,Instruction::RaxFromQwordWorkB8);}

        ins!(0x27fbe7,Instruction::RcxFromCurrentRax);
        state.progress.phase=Phase::BindCall;state.progress.attempted_rva=0x27fbea;
        let target=host.bind_cleanup_580c24(&mut state.shader.frame,&owners).map_err(Cause::Provider)?;
        state.progress.target=Some(target);
        if target.owners!=owners||target.callee_module!=owners.source_module {
            return Err(Cause::Stopped(Stop::TargetMismatch));
        }
        state.progress.phase=Phase::Call;
        let normal=match host.actual_cleanup_580c24(&mut state.shader.frame,&target) {
            Ok(normal)=>normal,
            Err(CallFailure {error,observed_normal_return,returned})=> {
                state.progress.phase=Phase::CallStopped;state.progress.call_observed=observed_normal_return;
                state.progress.called=returned.as_ref().map(|p|p.key);state.called=returned;
                return Err(Cause::Provider(error));
            }
        };
        state.progress.phase=Phase::CallReturned;state.progress.attempted_rva=0x27fbef;
        state.progress.call_observed=Some(normal.observed);state.called=normal.returned;
        state.progress.called=state.called.as_ref().map(|p|p.key);
        let called=state.called.as_ref().ok_or(Cause::Stopped(Stop::MissingCallReturn))?;
        if called.key.target!=target {return Err(Cause::Stopped(Stop::CallMismatch));}
        if called.key.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
        state.progress.phase=Phase::ValidateCall;
        host.validate_cleanup_return(&mut state.shader.frame,called).map_err(Cause::Provider)?;
        host.validate_cleanup_continuation(&mut state.shader.frame,&target,0x27fbef).map_err(Cause::Provider)?;
        state.progress.phase=Phase::Epilogue;
        for (rva,op) in [(0x27fbef,Instruction::Xmm6FromAlignedWorkD0),(0x27fbf7,Instruction::AddRspE8),
            (0x27fbfe,Instruction::PopRbx),(0x27fbff,Instruction::PopRbp),
            (0x27fc00,Instruction::PopRdi),(0x27fc01,Instruction::PopRsi),
            (0x27fc02,Instruction::PopR12),(0x27fc04,Instruction::PopR13),
            (0x27fc06,Instruction::PopR14),(0x27fc08,Instruction::PopR15)] {ins!(rva,op);}
        state.progress.phase=Phase::BindRet;state.progress.attempted_rva=0x27fc0a;
        let ret_target=host.bind_shader_ret(&mut state.shader.frame,&owners).map_err(Cause::Provider)?;
        state.progress.ret_target=Some(ret_target);
        if ret_target.owners!=owners||ret_target.actual_caller!=owners.caller_context {
            return Err(Cause::Stopped(Stop::TargetMismatch));
        }
        state.progress.phase=Phase::Ret;
        let normal=match host.actual_shader_ret_27fc0a(&mut state.shader.frame,&ret_target) {
            Ok(normal)=>normal,
            Err(RetFailure {error,observed_return,returned})=> {
                state.progress.phase=Phase::RetStopped;state.progress.ret_observed=observed_return;
                state.progress.returned=returned.as_ref().map(|p|p.key);state.returned=returned;
                return Err(Cause::Provider(error));
            }
        };
        state.progress.phase=Phase::RetReturned;state.progress.ret_observed=Some(normal.observed);
        state.returned=normal.returned;state.progress.returned=state.returned.as_ref().map(|p|p.key);
        let returned=state.returned.as_ref().ok_or(Cause::Stopped(Stop::MissingShaderReturn))?;
        if returned.key.target!=ret_target {return Err(Cause::Stopped(Stop::ShaderReturnMismatch));}
        if returned.key.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
        state.progress.phase=Phase::ValidateRet;
        host.validate_shader_ret(&mut state.shader.frame,returned).map_err(Cause::Provider)?;
        state.progress.phase=Phase::Complete;Ok(())
    })();
    match result {Ok(())=>Ok(Completed {state}),Err(cause)=>Err(Failure {state,cause})}
}
