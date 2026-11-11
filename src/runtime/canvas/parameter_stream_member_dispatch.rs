#[path = "parameter_stream_member_dispatch_steps.rs"]
mod steps;
#[path = "parameter_stream_member_dispatch_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_dispatch_return.rs"]
pub mod native_return;
#[path = "parameter_stream_member_handler.rs"]
pub mod handler;
#[path = "parameter_stream_member_handler_call.rs"]
pub mod handler_call;
#[path = "parameter_stream_member_metadata.rs"]
pub mod metadata;
#[path = "parameter_stream_member_metadata_call.rs"]
pub mod metadata_call;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64, pub rbp: u64,
    pub rbx: u64, pub incoming_rax: u64, pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallSite { Dispatch, FirstRestore, SecondRestore }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::Dispatch => 0, Self::FirstRestore => 1, Self::SecondRestore => 2 } }
    pub fn pc(self) -> u32 { match self { Self::Dispatch => 0x57a3d1, Self::FirstRestore => 0x57a3f4, Self::SecondRestore => 0x57a409 } }
    pub fn target(self) -> u32 { match self { Self::Dispatch => 0x57a200, _ => 0x568a10 } }
    pub fn resume(self) -> u32 { self.pc() + 5 }
}
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: u64, pub work_rsp: u64,
    pub rcx: u64, pub rdx: Option<u64>, pub r8: Option<u64>, pub r9: Option<u64>,
    pub required_rax: u64, pub required_rbx: u64, pub required_rbp: u64,
    pub stack_arg5: Option<u64>, pub stack_arg6: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub rbx: u64, pub rbp: u64 }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
pub type Calls<R> = [Option<CallReturned<R>>; 3];
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner> }
#[derive(Clone, Copy, Debug)]
pub struct AndQword { pub address: u64, pub before: u64, pub after: u64 }
#[derive(Clone, Copy, Debug)]
pub struct AndDword { pub address: u64, pub before: u32, pub after: u32 }
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;

    fn and_qword_zero(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, out: &mut Option<AndQword>) -> Result<(), Self::Error>;
    fn and_dword_fd(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, out: &mut Option<AndDword>) -> Result<(), Self::Error>;
    fn read_byte(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn read_dword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn read_qword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn write_byte(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error>;
    fn write_dword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error>;
    fn write_qword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn load_movups(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error>;
    fn store_movdqu(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error>;

    fn call(&mut self, o: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ClearContext, ReadGlobal, Clear30, Clear18, Clear10, Clear8,
    BranchGlobal, LoadLiteral, Mark18, StoreLiteral, StoreSixth, LoadFifth, StoreFifth,
    Dispatch, Read18, Branch18, ReadContext, ClearContextFlag, Read10, Branch10,
    ReadFirstValue, FirstRestore, StoreFirstValue, Read8, Branch8, ReadSecondValue,
    SecondRestore, StoreSecondValue, NativeRestoreRequired }
#[derive(Clone, Copy, Debug)]
pub struct MemoryAttempt { pub pc: u32, pub address: u64, pub width: u8 }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>,
    pub handler_call: Option<handler_call::History>,
    pub metadata_calls: [Option<metadata_call::History>; 3],
    pub global: Option<u32>, pub flag18: Option<u8>, pub flag10: Option<u8>, pub flag8: Option<u8>,
    pub literal: Option<[u32; 4]>, pub fifth: Option<u64>, pub rax: Option<u64>, pub rbx: Option<u64>,
    pub and_qword: Option<AndQword>, pub and_dword: Option<AndDword>, pub memory: Option<MemoryAttempt>,
    pub requests: [Option<CallRequest>; 3], pub completed: [bool; 3] }
pub enum Fault<E> { Service(E), MissingState, RmwBinding, OccupiedCall, CallBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeRestoreRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>,
    pub actual: ActualBody, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct NativeRestoreRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeRestoreRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x57a411 }
    pub fn raw_rax(&self) -> u64 { self.parts.captures.rax.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x57a391 || input.actual.rbp != input.actual.work_rsp.wrapping_add(0x70) { return Err(Rejected { input }); }
        let mut captures = Captures::default(); captures.rax = Some(input.actual.incoming_rax); captures.rbx = Some(input.actual.rbx);
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, calls: [None, None, None] },
            actual: input.actual, captures, phase: Phase::ClearContext, attempted: None, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeRestoreRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::NativeRestoreRequired { Poll::Ready(NativeRestoreRequired { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
}
