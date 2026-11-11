#[path = "parameter_stream_guard_native_tails_steps.rs"]
mod steps;
#[path = "parameter_stream_guard_initialization.rs"]
pub mod initialization;
#[path = "parameter_stream_guard_initialization_call.rs"]
pub mod initialization_call;
pub use super::native_return::StackAdjustment;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry { At57a7e0, At57a7fc, At561fa4, At561fac }
impl Entry {
    pub fn pc(self) -> u32 { match self { Self::At57a7e0 => 0x57a7e0, Self::At57a7fc => 0x57a7fc, Self::At561fa4 => 0x561fa4, Self::At561fac => 0x561fac } }
    pub fn tail_pc(self) -> u32 { match self { Self::At57a7e0 => 0x57a7f4, Self::At57a7fc => 0x57a803, Self::At561fa4 => 0x561fa4, Self::At561fac => 0x561fac } }
    pub fn target_slot_rva(self) -> u32 { match self { Self::At57a7e0 | Self::At561fa4 => 0x77f908, Self::At57a7fc | Self::At561fac => 0x77fb88 } }
}
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub entry: Entry, pub pc: u32, pub source_image_base: u64, pub rsp: u64, pub rcx: u64, pub rax: u64 }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
pub struct Owners<S: Services> { pub storage: S::Storage, pub call: Option<CallReturned<S::CallOwner>>, pub tail: Option<TailReturned<S::TailOwner>> }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub source_pc: u32, pub target_rva: u32, pub expected_resume_pc: u32, pub work_rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64 }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug)]
pub struct LeaReturned { pub rcx: u64, pub rsp: u64, pub rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct TailRequest { pub source_pc: u32, pub slot_address: u64, pub loaded_target: u64, pub entry_rsp: u64, pub rcx: u64, pub rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualTailReturn { pub entry_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64, pub rbx: u64 }
pub struct TailReturned<R> { pub actual: ActualTailReturn, pub owner: R }
pub trait Services: Sized {
    type Storage; type CallOwner; type TailOwner; type Error;

    fn sub_rsp28(&mut self, o: &mut Owners<Self>, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;

    fn call_58eca8(&mut self, o: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
    fn lea_rcx(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, returned: &mut Option<LeaReturned>) -> Result<(), Self::Error>;
    fn add_rsp28(&mut self, o: &mut Owners<Self>, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;

    fn read_target(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;

    fn tail(&mut self, o: &mut Owners<Self>, request: TailRequest) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Subtract, Call, Receiver, AddStack, ReadTarget, Tail, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub initialization_call: Option<initialization_call::History>, pub subtract: Option<StackAdjustment>, pub call_request: Option<CallRequest>, pub call_returned: bool,
    pub lea: Option<LeaReturned>, pub adjustment: Option<StackAdjustment>, pub slot_address: Option<u64>, pub loaded_target: Option<u64>,
    pub tail_request: Option<TailRequest>, pub actual_return: Option<ActualTailReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, OccupiedCall, OccupiedTail, CallBinding, RegisterBinding, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualEntry, captures: Captures,
    live_rsp: u64, live_rcx: u64, live_rax: u64, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualEntry, pub captures: Captures,

    pub live_rsp: u64, pub live_rcx: u64, pub live_rax: u64, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualTailReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != input.actual.entry.pc() { return Err(Rejected { input }); }
        let phase = match input.actual.entry { Entry::At57a7e0 => Phase::Subtract, Entry::At57a7fc => Phase::Receiver, _ => Phase::ReadTarget };
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, call: None, tail: None }, actual: input.actual,
            captures: Captures::default(), live_rsp: input.actual.rsp, live_rcx: input.actual.rcx, live_rax: input.actual.rax,
            phase, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual, captures: self.captures,
        live_rsp: self.live_rsp, live_rcx: self.live_rcx, live_rax: self.live_rax, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); } let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase == Phase::Done { Poll::Ready(Returned { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
}
