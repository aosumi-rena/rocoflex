#[path = "parameter_stream_member_copy_steps.rs"]
mod steps;
#[path = "parameter_stream_member_copy_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_copy_return.rs"]
pub mod native_return;
#[path = "parameter_stream_member_accessor.rs"]
pub mod accessor;
#[path = "parameter_stream_member_accessor_call.rs"]
pub mod accessor_call;
#[path = "parameter_stream_member_diagnostic.rs"]
pub mod diagnostic;
#[path = "parameter_stream_member_diagnostic_call.rs"]
pub mod diagnostic_call;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64,
    pub destination_rcx: u64, pub capacity_rdx: u64, pub source_r8: u64 }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { InvalidPointer, ExhaustedPointer, Diagnostic }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::InvalidPointer => 0, Self::ExhaustedPointer => 1, Self::Diagnostic => 2 } }
    pub fn pc(self) -> u32 { match self { Self::InvalidPointer => 0x58e32a, Self::ExhaustedPointer => 0x58e369, Self::Diagnostic => 0x58e336 } }
    pub fn resume_pc(self) -> u32 { self.pc() + 5 }
    pub fn target_rva(self) -> u32 { match self { Self::InvalidPointer | Self::ExhaustedPointer => 0x578eb0, Self::Diagnostic => 0x57a004 } }
}
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: Option<u64>, pub rdx: Option<u64>, pub r8: Option<u64>, pub r9: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: u64, pub work_rsp: u64, pub args: Arguments, pub required_rbx: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub rbx: u64 }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
pub type Calls<R> = [Option<CallReturned<R>>; 3];
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner> }
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;

    fn call(&mut self, owners: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
    fn read_word16(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn write_word16(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error>;
    fn write_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ZeroRbx, Validate, ReadWord, WriteWord, Advance, BranchWord, Decrement,
    TestRemaining, ClearDestination, ErrorPointer, SetError, WriteError, Diagnostic, NativeReturnRequired }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCause { NullDestination, ZeroCapacity, NullSource, Exhausted }
#[derive(Clone, Copy, Debug)]
pub struct MemoryAttempt { pub pc: u32, pub address: u64, pub width: u8, pub written: Option<u64> }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub requests: [Option<CallRequest>; 3], pub completed: [bool; 3],
    pub accessor_calls: [Option<accessor_call::History>; 3],
    pub diagnostic_call: Option<diagnostic_call::History>,
    pub rbx: Option<u64>, pub remaining: Option<u64>, pub delta: Option<u64>, pub cursor: Option<u64>, pub word: Option<u16>,
    pub cause: Option<ErrorCause>, pub pointer: Option<u64>, pub diagnostic_rax: Option<u64>, pub result: Option<u64>,

    pub memory: Option<MemoryAttempt> }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall { site: CallSite }, CallBinding { site: CallSite }, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>,
    pub actual: ActualBody, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x58e33d }
    pub fn result(&self) -> u64 { self.parts.captures.result.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x58e316 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, calls: [None, None, None] },
            actual: input.actual, captures: Captures::default(), phase: Phase::ZeroRbx, attempted: None, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeReturnRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::NativeReturnRequired { Poll::Ready(NativeReturnRequired { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
}
