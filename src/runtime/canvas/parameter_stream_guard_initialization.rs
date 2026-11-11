#[path = "parameter_stream_guard_initialization_rows.rs"]
mod rows;
#[path = "parameter_stream_guard_initialization_steps.rs"]
mod steps;
#[path = "parameter_stream_guard_resolver_prefix.rs"]
pub mod resolver_prefix;
pub use super::StackAdjustment;
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub pc: u32, pub source_image_base: u64, pub rsp: u64, pub rax: u64 }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
pub type Calls<R> = [Option<CallReturned<R>>; 10];
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner>, pub tail: Option<TailReturned<S::TailOwner>> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register { R9, Rcx, R8, Rdx }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation { Lea64, Xor32, Mov32 }
#[derive(Clone, Copy, Debug)]
pub struct WriteRequest { pub pc: u32, pub register: Register, pub operation: Operation, pub full_word: u64 }
#[derive(Clone, Copy, Debug)]
pub struct WriteReturned { pub pc: u32, pub register: Register, pub full_word: u64, pub rsp: u64, pub rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64 }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub index: usize, pub pc: u32, pub target_rva: u32, pub expected_resume_pc: u32,
    pub work_rsp: u64, pub entry_rsp: u64, pub arguments: Arguments }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64 }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug)]
pub struct TailRequest { pub pc: u32, pub target_rva: u32, pub entry_rsp: u64, pub arguments: Arguments, pub rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualTailReturn { pub entry_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64 }
pub struct TailReturned<R> { pub actual: ActualTailReturn, pub owner: R }
pub trait Services: Sized {
    type Storage; type CallOwner; type TailOwner; type Error;

    fn sub_rsp28(&mut self, o: &mut Owners<Self>, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;

    fn write_argument(&mut self, o: &mut Owners<Self>, request: WriteRequest, returned: &mut Option<WriteReturned>) -> Result<(), Self::Error>;

    fn call_58ee04(&mut self, o: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
    fn add_rsp28(&mut self, o: &mut Owners<Self>, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn tail_58ee04(&mut self, o: &mut Owners<Self>, request: TailRequest) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Subtract, Argument, Call, AddStack, Tail, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub subtract: Option<StackAdjustment>, pub requests: [[Option<WriteRequest>; 4]; 11], pub writes: [[Option<WriteReturned>; 4]; 11],
    pub call_requests: [Option<CallRequest>; 10], pub completed: [bool; 10], pub adjustment: Option<StackAdjustment>,
    pub tail_request: Option<TailRequest>, pub actual_return: Option<ActualTailReturn>,
    pub resolver_calls: [Option<resolver_prefix::selected_call::History>; 11] }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, OccupiedCall, OccupiedTail, RegisterBinding, StackBinding, CallBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualEntry, captures: Captures, rax: u64,
    phase: Phase, row: usize, argument: usize, attempted: Option<(Phase, usize, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualEntry, pub captures: Captures,

    pub rax: u64, pub phase: Phase, pub row: usize, pub argument: usize, pub attempted: Option<(Phase, usize, usize)>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualTailReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x58eca8 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage,
            calls: [None,None,None,None,None,None,None,None,None,None], tail: None }, actual: input.actual, captures: Captures::default(), rax: input.actual.rax,
            phase: Phase::Subtract, row: 0, argument: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual, captures: self.captures,
        rax: self.rax, phase: self.phase, row: self.row, argument: self.argument, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); } let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some((self.phase, self.row, self.argument)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase == Phase::Done { Poll::Ready(Returned { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
}
