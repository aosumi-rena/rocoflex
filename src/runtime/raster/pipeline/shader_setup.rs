use super::{inverse, Progress as PrefixProgress};
#[path = "shader_finish.rs"] pub mod finish;
#[path = "shader_loop.rs"] pub mod root_loop;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path { A, B }
impl Path {
    fn inverse_site(self) -> inverse::Site { match self {
        Self::A=>inverse::Site::At27a60d,Self::B=>inverse::Site::At27a6aa } }
    fn setup(self) -> [u32;14] { match self {
        Self::A=>[0x27a612,0x27a61b,0x27a622,0x27a62a,0x27a632,0x27a639,
            0x27a640,0x27a643,0x27a645,0x27a64c,0x27a64f,0x27a651,0x27a658,0x27a664],
        Self::B=>[0x27a6af,0x27a6b8,0x27a6bf,0x27a6c7,0x27a6cf,0x27a6d6,
            0x27a6dd,0x27a6e0,0x27a6e2,0x27a6e9,0x27a6ec,0x27a6ee,0x27a6f5,0x27a701],
    } }
    pub fn call_rva(self)->u32 { match self { Self::A=>0x27a669,Self::B=>0x27a706 } }
    pub fn return_rva(self)->u32 { match self { Self::A=>0x27a66e,Self::B=>0x27a70b } }
    fn tail(self)->[u32;5] { match self {
        Self::A=>[0x27a676,0x27a67b,0x27a681,0x27a684,0x27a686],
        Self::B=>[0x27a713,0x27a718,0x27a71e,0x27a721,0x27a723],
    } }
    fn loop_rva(self)->u32 { match self { Self::A=>0x27a8e5,Self::B=>0x27ae5a } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {

    Work80FromXmm6, Work84FromEsi, Work88FromR14, Work90FromRdi,
    Work98FromEbx, Work9cFromEbp, EaxFromRdi1c, DecEax, WorkA0FromEax,
    EaxFromRdi20, WorkA4FromEax, QwordWorkC0From800, EcxFrom2000,
    WorkB8FromRax, CmpDwordR1514Zero, R14FromQwordR15, XorEdx,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target<I> {
    pub owners: inverse::Owners<I>, pub path: Path, pub rcx: u64,
    pub callee_module: I, pub callee_context: I, pub caller_frame: I,
    pub physical_h: I, pub native_output: I, pub memory_owner: I, pub receipt_owner: I,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnKey<I> {
    pub target: Target<I>, pub callee_frame: I, pub receipt_identity: I,
    pub observed: inverse::Observation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputKey<I> {
    pub target: Target<I>, pub callee_frame: I, pub effects_identity: I,
}
#[must_use]
pub struct Returned<I,P> { pub key: ReturnKey<I>, pub packet: P }
#[must_use]
pub struct Output<I,P> { pub key: OutputKey<I>, pub packet: P }
#[must_use]
pub struct NormalReturn<I,R,O> {
    pub observed: inverse::Observation,
    pub returned: Option<Returned<I,R>>, pub output: Option<Output<I,O>>,
}
#[must_use = "partial actual call effects/resources remain with SAME Frame"]
pub struct CallFailure<I,R,O,E> {
    pub error: E, pub observed_normal_return: Option<inverse::Observation>,
    pub returned: Option<Returned<I,R>>, pub output: Option<Output<I,O>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeightBranchKey<I> {
    pub owners:inverse::Owners<I>,pub path:Path,pub rva:u32,
    pub taken:bool,pub destination:u32,pub receipt_identity:I,
}

pub struct HeightBranch<I> { pub taken:bool,pub retained:Option<HeightBranchKey<I>> }
pub trait Services: inverse::Services {
    type AllocationReturnPacket;
    type AllocationOutputPacket;

    fn setup_instruction(&mut self, frame:&mut Self::Frame,rva:u32,op:Instruction)
        ->Result<(),Self::Error>;

    fn bind_allocation_583918(&mut self,frame:&mut Self::Frame,
        owners:&inverse::Owners<Self::Identity>,path:Path)
        ->Result<Target<Self::Identity>,Self::Error>;

    fn actual_allocation_583918(&mut self,frame:&mut Self::Frame,target:&Target<Self::Identity>)
        ->Result<NormalReturn<Self::Identity,Self::AllocationReturnPacket,Self::AllocationOutputPacket>,
            CallFailure<Self::Identity,Self::AllocationReturnPacket,Self::AllocationOutputPacket,Self::Error>>;

    fn validate_allocation_return(&mut self,frame:&mut Self::Frame,
        packet:&Returned<Self::Identity,Self::AllocationReturnPacket>)->Result<(),Self::Error>;
    fn validate_allocation_output(&mut self,frame:&mut Self::Frame,
        packet:&Output<Self::Identity,Self::AllocationOutputPacket>)->Result<(),Self::Error>;
    fn validate_allocation_continuation(&mut self,frame:&mut Self::Frame,
        target:&Target<Self::Identity>,return_rva:u32)->Result<(),Self::Error>;

    fn setup_jle(&mut self,frame:&mut Self::Frame,rva:u32,taken:u32,fallthrough:u32)
        ->Result<HeightBranch<Self::Identity>,Self::Error>;

    fn validate_height_branch(&mut self,frame:&mut Self::Frame,key:&HeightBranchKey<Self::Identity>)
        ->Result<(),Self::Error>;
    fn setup_jump(&mut self,frame:&mut Self::Frame,rva:u32,target:u32)->Result<(),Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Entry, Setup, Bind, Call, CallStopped, Return, ValidateReturn,
    ValidateOutput, ValidateContinuation, Tail, Complete }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop { WrongInverseExit, MissingOwners, TargetMismatch, MissingReturn,
    MissingOutput, ReturnMismatch, OutputMismatch, ObservationMismatch, CalleeMismatch,
    MissingHeightBranch,HeightBranchMismatch }
pub enum Cause<E> { Provider(E), Stopped(Stop) }
#[derive(Clone, Copy, Debug)]
pub struct Progress<I> {
    pub phase:Phase,pub attempted_rva:u32,pub path:Option<Path>,
    pub completed_instructions:usize,pub target:Option<Target<I>>,
    pub observed:Option<inverse::Observation>,pub returned:Option<ReturnKey<I>>,
    pub output:Option<OutputKey<I>>,pub height_branch_taken:Option<bool>,
    pub height_branch:Option<HeightBranchKey<I>>,
}

#[must_use]
pub struct State<F,I,R,O> {
    pub frame:F,pub prefix:PrefixProgress,pub inverse:inverse::Progress<I>,
    pub progress:Progress<I>,pub returned:Option<Returned<I,R>>,pub output:Option<Output<I,O>>,
}
#[must_use = "next actual source body remains REQUIRED; not shader normal return"]
pub struct Required<F,I,R,O> { pub state:State<F,I,R,O>,pub next_rva:u32 }
#[must_use]
pub struct Failure<F,I,R,O,E> { pub state:State<F,I,R,O>,pub cause:Cause<E> }

pub fn continue_after_inverse<S:Services>(host:&mut S,after:inverse::AfterReturn<S::Frame,S::Identity>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let inverse::AfterReturn {frame,prefix,progress:prior,next_rva}=after;
    let mut state=State {frame,prefix,inverse:prior,progress:Progress {
        phase:Phase::Entry,attempted_rva:next_rva,path:None,completed_instructions:0,
        target:None,observed:None,returned:None,output:None,height_branch_taken:None,height_branch:None },returned:None,output:None};
    let result=(||->Result<u32,Cause<S::Error>> {
        let path=match (state.inverse.site,next_rva,state.inverse.phase) {
            (Some(inverse::Site::At27a60d),0x27a612,inverse::Phase::Complete)=>Path::A,
            (Some(inverse::Site::At27a6aa),0x27a6af,inverse::Phase::Complete)=>Path::B,
            _=>return Err(Cause::Stopped(Stop::WrongInverseExit)),
        };
        state.progress.path=Some(path);
        let owners=state.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;

        let inverse_target=state.inverse.target.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        if inverse_target.site!=path.inverse_site()||inverse_target.owners!=owners {
            return Err(Cause::Stopped(Stop::WrongInverseExit));
        }
        host.validate_inverse_continuation(&mut state.frame,&inverse_target,next_rva).map_err(Cause::Provider)?;
        macro_rules! ins { ($rva:expr,$op:expr)=>{{
            state.progress.attempted_rva=$rva;
            host.setup_instruction(&mut state.frame,$rva,$op).map_err(Cause::Provider)?;
            state.progress.completed_instructions+=1;
        }}; }
        use Instruction::*;
        state.progress.phase=Phase::Setup;
        let ops=[Work80FromXmm6,Work84FromEsi,Work88FromR14,Work90FromRdi,
            Work98FromEbx,Work9cFromEbp,EaxFromRdi1c,DecEax,WorkA0FromEax,
            EaxFromRdi20,DecEax,WorkA4FromEax,QwordWorkC0From800,EcxFrom2000];
        for (rva,op) in path.setup().into_iter().zip(ops) { ins!(rva,op); }
        state.progress.phase=Phase::Bind;state.progress.attempted_rva=path.call_rva();
        let target=host.bind_allocation_583918(&mut state.frame,&owners,path).map_err(Cause::Provider)?;
        state.progress.target=Some(target);
        if target.owners!=owners||target.path!=path||target.rcx!=0x2000
            ||target.caller_frame!=owners.frame||target.callee_module!=owners.source_module {
            return Err(Cause::Stopped(Stop::TargetMismatch));
        }
        state.progress.phase=Phase::Call;
        let normal=match host.actual_allocation_583918(&mut state.frame,&target) {
            Ok(normal)=>normal,
            Err(CallFailure {error,observed_normal_return,returned,output})=> {
                state.progress.phase=Phase::CallStopped;state.progress.observed=observed_normal_return;
                state.progress.returned=returned.as_ref().map(|p|p.key);
                state.progress.output=output.as_ref().map(|p|p.key);
                state.returned=returned;state.output=output;return Err(Cause::Provider(error));
            }
        };
        state.progress.phase=Phase::Return;state.progress.attempted_rva=path.return_rva();
        state.progress.observed=Some(normal.observed);
        state.returned=normal.returned;state.output=normal.output;
        state.progress.returned=state.returned.as_ref().map(|p|p.key);
        state.progress.output=state.output.as_ref().map(|p|p.key);
        let returned=state.returned.as_ref().ok_or(Cause::Stopped(Stop::MissingReturn))?;
        let output=state.output.as_ref().ok_or(Cause::Stopped(Stop::MissingOutput))?;
        if returned.key.target!=target {return Err(Cause::Stopped(Stop::ReturnMismatch));}
        if output.key.target!=target {return Err(Cause::Stopped(Stop::OutputMismatch));}
        if returned.key.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
        if returned.key.callee_frame!=output.key.callee_frame {return Err(Cause::Stopped(Stop::CalleeMismatch));}
        state.progress.phase=Phase::ValidateReturn;
        host.validate_allocation_return(&mut state.frame,returned).map_err(Cause::Provider)?;
        state.progress.phase=Phase::ValidateOutput;
        host.validate_allocation_output(&mut state.frame,output).map_err(Cause::Provider)?;
        state.progress.phase=Phase::ValidateContinuation;
        host.validate_allocation_continuation(&mut state.frame,&target,path.return_rva()).map_err(Cause::Provider)?;
        state.progress.phase=Phase::Tail;

        ins!(path.return_rva(),WorkB8FromRax);
        let [cmp,jle,load,xor,jump]=path.tail();
        ins!(cmp,CmpDwordR1514Zero);
        state.progress.attempted_rva=jle;
        let branch=host.setup_jle(&mut state.frame,jle,0x27fbe7,load).map_err(Cause::Provider)?;
        let taken=branch.taken;
        state.progress.height_branch_taken=Some(taken);
        state.progress.height_branch=branch.retained;
        let receipt=branch.retained.ok_or(Cause::Stopped(Stop::MissingHeightBranch))?;
        if receipt.owners!=owners||receipt.path!=path||receipt.rva!=jle||receipt.taken!=taken
            ||receipt.destination!=if taken {0x27fbe7}else{load} {
            return Err(Cause::Stopped(Stop::HeightBranchMismatch));
        }
        host.validate_height_branch(&mut state.frame,&receipt).map_err(Cause::Provider)?;
        if taken {state.progress.phase=Phase::Complete;return Ok(0x27fbe7);}
        ins!(load,R14FromQwordR15);ins!(xor,XorEdx);
        state.progress.attempted_rva=jump;
        host.setup_jump(&mut state.frame,jump,path.loop_rva()).map_err(Cause::Provider)?;
        state.progress.phase=Phase::Complete;Ok(path.loop_rva())
    })();
    match result {Ok(next_rva)=>Ok(Required {state,next_rva}),Err(cause)=>Err(Failure {state,cause})}
}
