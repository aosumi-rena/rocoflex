use super::{inverse,Path};
#[path = "shader_point.rs"] pub mod point;
#[path = "shader_sample.rs"] pub mod sample;
#[path = "shader_blend.rs"] pub mod blend;
#[path = "shader_bulk.rs"] pub mod bulk;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    RcxFromSignedDwordR151c, EbxFromDwordR14, CmpEbx2,
    Work30FromRcx, EcxFromDwordR1404, RaxFromSignedDwordR150c,
    Work28FromEdx, RdxFromSignedEdx, AddRdxRax, WorkA8FromEdx,
    RaxFromQwordWork88, R8FromSignedDwordRax14, ImulR8Rdx,
    AddR8QwordRax, WorkB0FromR8, IncEbx, Work38FromR14, LeaRsiR140c, XorEax,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchKey<I> {
    pub owners:inverse::Owners<I>,pub path:Path,pub rva:u32,
    pub taken:bool,pub destination:u32,pub receipt_identity:I,
}
pub struct Branch<I> {pub taken:bool,pub retained:Option<BranchKey<I>>}
pub trait Services:super::Services {

    fn validate_loop_entry(&mut self,frame:&mut Self::Frame,owners:&inverse::Owners<Self::Identity>,path:Path,rva:u32)
        ->Result<(),Self::Error>;

    fn loop_instruction(&mut self,frame:&mut Self::Frame,rva:u32,op:Instruction)->Result<(),Self::Error>;

    fn loop_jl(&mut self,frame:&mut Self::Frame,rva:u32,taken:u32,fallthrough:u32)
        ->Result<Branch<Self::Identity>,Self::Error>;
    fn validate_loop_branch(&mut self,frame:&mut Self::Frame,key:&BranchKey<Self::Identity>)
        ->Result<(),Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {Admission,Prefix,Branch,ValidateBranch,Setup,Jump,Point,PointBranch,Row,Blend,Complete}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {WrongEntry,MissingOwners,MissingHeightBranch,HeightBranchMismatch,MissingBranch,BranchMismatch,MissingSampleHistory}
pub enum Cause<E> {Provider(E),Stopped(Stop)}
#[derive(Clone, Copy, Debug)]
pub struct Progress<I> {
    pub phase:Phase,pub attempted_rva:u32,pub path:Option<Path>,pub completed_instructions:usize,
    pub branch_taken:Option<bool>,pub branch:Option<BranchKey<I>>,
    pub point_control:Option<point::Control>,pub point_control_taken:Option<bool>,
    pub sample_history:Option<sample::Retained<I>>,
    pub final_store_row:Option<point::FinalStoreRow<I>>,
    pub bulk_history:Option<bulk::Progress<I>>,
    pub helper_history:Option<sample::helper::Progress<I>>,
}
#[must_use = "retain SAME advanced shader Frame and all prior packets"]
pub struct State<F,I,R,O> {pub shader:super::State<F,I,R,O>,pub progress:Progress<I>}
#[must_use = "next actual source continuation remains REQUIRED"]
pub struct Required<F,I,R,O> {pub state:State<F,I,R,O>,pub next_rva:u32}
#[must_use]
pub struct Failure<F,I,R,O,E> {pub state:State<F,I,R,O>,pub cause:Cause<E>}

pub fn prefix<S:Services>(host:&mut S,required:super::Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let super::Required {state:shader,next_rva}=required;
    let state=State {shader,progress:Progress {phase:Phase::Admission,attempted_rva:next_rva,
        path:None,completed_instructions:0,branch_taken:None,branch:None,
        point_control:None,point_control_taken:None,sample_history:None,final_store_row:None,bulk_history:None,helper_history:None}};
    run_prefix(host,state,next_rva,true)
}

pub fn resume_prefix<S:Services>(host:&mut S,required:Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let Required {state,next_rva}=required;
    let valid=match (state.progress.path,state.progress.phase,state.progress.point_control,state.progress.point_control_taken,next_rva) {
        (Some(Path::A),Phase::Complete,Some(c),Some(false),0x27a8e5)=>c==point::Control {
            rva:0x27a8df,condition:point::Condition::SignedGreaterEqual,taken:0x27fbdf,fallthrough:0x27a8e5},
        (Some(Path::B),Phase::Complete,Some(c),Some(false),0x27ae5a)=>c==point::Control {
            rva:0x27ae54,condition:point::Condition::SignedGreaterEqual,taken:0x27fbdf,fallthrough:0x27ae5a},
        _=>false,
    };
    if !valid {return Err(Failure {state,cause:Cause::Stopped(Stop::WrongEntry)});}
    run_prefix(host,state,next_rva,false)
}

fn run_prefix<S:Services>(host:&mut S,mut state:State<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,next_rva:u32,initial:bool)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let result=(||->Result<u32,Cause<S::Error>> {
        let path=match (state.shader.progress.path,next_rva) {
            (Some(Path::A),0x27a8e5)=>Path::A,(Some(Path::B),0x27ae5a)=>Path::B,
            _=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        state.progress.path=Some(path);
        if initial&&(state.shader.progress.phase!=super::Phase::Complete||state.shader.progress.height_branch_taken!=Some(false)) {
            return Err(Cause::Stopped(Stop::WrongEntry));
        }
        let owners=state.shader.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        if initial {
        let height=state.shader.progress.height_branch.ok_or(Cause::Stopped(Stop::MissingHeightBranch))?;
        let (height_rva,height_destination)=match path {Path::A=>(0x27a67b,0x27a681),Path::B=>(0x27a718,0x27a71e)};
        if height.owners!=owners||height.path!=path||height.rva!=height_rva||height.taken||height.destination!=height_destination {
            return Err(Cause::Stopped(Stop::HeightBranchMismatch));
        }
        host.validate_height_branch(&mut state.shader.frame,&height).map_err(Cause::Provider)?;
        }
        host.validate_loop_entry(&mut state.shader.frame,&owners,path,next_rva).map_err(Cause::Provider)?;

        state.progress.point_control=None;state.progress.point_control_taken=None;
        state.progress.final_store_row=None;
        macro_rules! ins {($rva:expr,$op:expr)=>{{
            state.progress.attempted_rva=$rva;
            host.loop_instruction(&mut state.shader.frame,$rva,$op).map_err(Cause::Provider)?;
            state.progress.completed_instructions+=1;
        }};}
        use Instruction::*;
        let (initial,jl,unknown,body,jump,target)=match path {
            Path::A=>([0x27a8e5,0x27a8e9,0x27a8ec],0x27a8ef,0x27a8d5,
                [0x27a8f1,0x27a8f6,0x27a8fa,0x27a8fe,0x27a902,0x27a905,0x27a908,0x27a90f,
                 0x27a917,0x27a91b,0x27a91f,0x27a922,0x27a92a,0x27a92c,0x27a931,0x27a935],0x27a937,0x27a95d),
            Path::B=>([0x27ae5a,0x27ae5e,0x27ae61],0x27ae64,0x27ae4a,
                [0x27ae66,0x27ae6b,0x27ae6f,0x27ae73,0x27ae77,0x27ae7a,0x27ae7d,0x27ae84,
                 0x27ae8c,0x27ae90,0x27ae94,0x27ae97,0x27ae9f,0x27aea1,0x27aea6,0x27aeaa],0x27aeac,0x27aecd),
        };
        state.progress.phase=Phase::Prefix;
        for (rva,op) in initial.into_iter().zip([RcxFromSignedDwordR151c,EbxFromDwordR14,CmpEbx2]) {ins!(rva,op);}
        state.progress.phase=Phase::Branch;state.progress.attempted_rva=jl;
        let branch=host.loop_jl(&mut state.shader.frame,jl,unknown,body[0]).map_err(Cause::Provider)?;
        state.progress.branch_taken=Some(branch.taken);state.progress.branch=branch.retained;
        let key=branch.retained.ok_or(Cause::Stopped(Stop::MissingBranch))?;
        if key.owners!=owners||key.path!=path||key.rva!=jl||key.taken!=branch.taken
            ||key.destination!=if branch.taken {unknown}else{body[0]} {
            return Err(Cause::Stopped(Stop::BranchMismatch));
        }
        state.progress.phase=Phase::ValidateBranch;
        host.validate_loop_branch(&mut state.shader.frame,&key).map_err(Cause::Provider)?;
        if branch.taken {state.progress.phase=Phase::Complete;return Ok(unknown);}
        state.progress.phase=Phase::Setup;
        let ops=[Work30FromRcx,EcxFromDwordR1404,RaxFromSignedDwordR150c,Work28FromEdx,
            RdxFromSignedEdx,AddRdxRax,WorkA8FromEdx,RaxFromQwordWork88,
            R8FromSignedDwordRax14,ImulR8Rdx,AddR8QwordRax,WorkB0FromR8,IncEbx,
            Work38FromR14,LeaRsiR140c,XorEax];
        for (rva,op) in body.into_iter().zip(ops) {ins!(rva,op);}
        state.progress.phase=Phase::Jump;state.progress.attempted_rva=jump;
        host.setup_jump(&mut state.shader.frame,jump,target).map_err(Cause::Provider)?;
        state.progress.phase=Phase::Complete;Ok(target)
    })();
    match result {Ok(next_rva)=>Ok(Required {state,next_rva}),Err(cause)=>Err(Failure {state,cause})}
}
