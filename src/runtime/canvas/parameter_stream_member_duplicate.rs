#[path = "parameter_stream_member_duplicate_steps.rs"]
mod steps;
#[path = "parameter_stream_member_duplicate_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_duplicate_return.rs"]
pub mod native_return;
#[path = "parameter_stream_member_copy.rs"]
pub mod copy;
#[path = "parameter_stream_member_copy_call.rs"]
pub mod copy_call;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64,
    pub source_rcx: u64, pub original_rbx: u64, pub original_rsi: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved { pub rbx: u64, pub rbp: u64, pub rsi: u64, pub rdi: u64 }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { Allocate, Copy, Diagnostic }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::Allocate => 0, Self::Copy => 1, Self::Diagnostic => 2 } }
    pub fn pc(self) -> u32 { match self { Self::Allocate => 0x57c482, Self::Copy => 0x57c498, Self::Diagnostic => 0x57c4cc } }
    pub fn target_rva(self) -> u32 { match self { Self::Allocate => 0x583918, Self::Copy => 0x58e310, Self::Diagnostic => 0x57a054 } }
    pub fn resume_pc(self) -> u32 { self.pc() + 5 }
}
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: Option<u64>, pub rdx: Option<u64>, pub r8: Option<u64>, pub r9: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: u64, pub work_rsp: u64, pub args: Arguments,
    pub stack_arg5: Option<u64>, pub required_saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Saved }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
pub type Calls<R> = [Option<CallReturned<R>>; 3];
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner> }
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;

    fn call(&mut self, owners: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
    fn read_word16(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn write_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { BindSaved, BranchSource, AdvanceScan, ReadWord, IncludeZero, Allocate, BranchAllocation,
    Copy, BranchCopy, SaveFifthArgument, Diagnostic, NativeReturnRequired, BreakpointRequired }
#[derive(Clone, Copy, Debug)]
pub struct MemoryAttempt { pub pc: u32, pub address: u64, pub width: u8, pub written: Option<u64> }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub requests: [Option<CallRequest>; 3],
    pub copy_call: Option<copy_call::History>,
    pub completed: [bool; 3], pub saved: Option<Saved>, pub word: Option<u16>, pub allocation: Option<u64>,
    pub copy_rax: Option<u64>, pub diagnostic_rax: Option<u64>, pub result: Option<u64>,

    pub memory: Option<MemoryAttempt> }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall { site: CallSite }, CallBinding { site: CallSite }, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Next<S>), Failed(Stopped<S>) }
pub enum Next<S: Services> { Normal(NativeReturnRequired<S>), Breakpoint(BreakpointRequired<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>,
    pub actual: ActualBody, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x57c4a8 }
    pub fn result(&self) -> u64 { self.parts.captures.result.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}

pub struct BreakpointRequired<S: Services> { parts: Parts<S> }
impl<S: Services> BreakpointRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x57c4d1 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x57c464 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, calls: [None, None, None] },
            actual: input.actual, captures: Captures::default(), phase: Phase::BindSaved, attempted: None, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while !matches!(self.phase, Phase::NativeReturnRequired | Phase::BreakpointRequired) && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        match self.phase {
            Phase::NativeReturnRequired => Poll::Ready(Next::Normal(NativeReturnRequired { parts: self.into_parts() })),
            Phase::BreakpointRequired => Poll::Ready(Next::Breakpoint(BreakpointRequired { parts: self.into_parts() })),
            _ => Poll::Pending(self),
        }
    }
}
