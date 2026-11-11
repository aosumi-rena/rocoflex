#[path = "parameter_stream_member_construction_steps.rs"]
mod steps;
#[path = "parameter_stream_member_construction_return.rs"]
pub mod native_return;
#[path = "parameter_stream_member_construction_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_query.rs"]
pub mod query;
#[path = "parameter_stream_member_query_call.rs"]
pub mod query_call;
#[path = "parameter_stream_member_duplicate.rs"]
pub mod duplicate;
#[path = "parameter_stream_member_duplicate_call.rs"]
pub mod duplicate_call;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64, pub receiver_rcx: u64 }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { QueryWord, Allocate, CopySource, FallbackSource, QueryContext, Duplicate }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::QueryWord => 0, Self::Allocate => 1, Self::CopySource => 2, Self::FallbackSource => 3, Self::QueryContext => 4, Self::Duplicate => 5 } }
    pub fn pc(self) -> u32 { match self { Self::QueryWord => 0x5612f9, Self::Allocate => 0x56130a, Self::CopySource => 0x561318, Self::FallbackSource => 0x56137d, Self::QueryContext => 0x56138a, Self::Duplicate => 0x56139c } }
    pub fn resume_pc(self) -> u32 { self.pc() + 5 }
    pub fn target_rva(self) -> u32 { match self { Self::QueryWord => 0x567c9c, Self::Allocate => 0x57dc5c, Self::CopySource | Self::FallbackSource => 0x56807c, Self::QueryContext => 0x567c68, Self::Duplicate => 0x57c450 } }
}
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: Option<u64>, pub rdx: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: u64, pub args: Arguments, pub work_rsp: u64, pub required_rbx: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub rbx: u64 }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
pub type Calls<R> = [Option<CallReturned<R>>; 6];
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner> }
#[derive(Clone, Copy, Debug)]
pub struct AndDwordReturned { pub address: u64, pub before: u32, pub after: u32 }
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;

    fn call(&mut self, owners: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
    fn read_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn write_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn write_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;

    fn read_vector(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error>;
    fn write_vector(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error>;

    fn and_dword_zero(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64,
        returned: &mut Option<AndDwordReturned>) -> Result<(), Self::Error>;

}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { QueryWord, StoreWord, Allocate, StoreAllocation, BranchAllocation, CopySource, ReloadDestination,
    LoadVector, StoreVector, AdvanceDestination, AdvanceSource, DecrementCopy, StoreOwnedFlag,
    FallbackSource, ClearOwnedFlag, StoreFallback, QueryContext, ReadContextMember, StoreContextMember,
    Duplicate, StoreDuplicate, NativeReturnRequired }
#[derive(Clone, Copy, Debug)]
pub struct MemoryAttempt { pub pc: u32, pub address: u64, pub width: u8 }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub requests: [Option<CallRequest>; 6], pub completed: [bool; 6],
    pub query_calls: [Option<query_call::History>; 6],
    pub duplicate_call: Option<duplicate_call::History>,
    pub query_word: Option<u64>, pub allocation: Option<u64>, pub source: Option<u64>, pub destination: Option<u64>,
    pub remaining: u64, pub lane: usize, pub vector: Option<[u8; 16]>, pub copied_vectors: u8,
    pub fallback: Option<u64>, pub cleared: Option<AndDwordReturned>, pub context: Option<u64>,
    pub context_member: Option<u64>, pub duplicate: Option<u64>,

    pub memory: Option<MemoryAttempt> }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall { site: CallSite }, CallBinding { site: CallSite }, RmwBinding, CopyPhase, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualBody,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }

pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x5613a5 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x5612f6 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, calls: [None, None, None, None, None, None] },
            actual: input.actual, captures: Captures::default(), phase: Phase::QueryWord, attempted: None, stopped: false })
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
