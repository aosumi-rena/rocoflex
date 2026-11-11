#[path = "parameter_stream_guard_steps.rs"]
mod steps;
#[path = "parameter_stream_guard_return.rs"]
pub mod native_return;
#[path = "parameter_stream_guard_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_guard_native_tails.rs"]
pub mod native_tails;
#[path = "parameter_stream_guard_wrapper_call.rs"]
pub mod wrapper_call;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry { Construct55e0a0, Destroy55e0dc }
impl Entry { pub fn body_pc(self) -> u32 { match self { Self::Construct55e0a0 => 0x55e0a6, Self::Destroy55e0dc => 0x55e0e0 } } }
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub entry: Entry, pub pc: u32, pub source_image_base: u64, pub work_rsp: u64, pub guard_rcx: u64,

    pub constructor_edx: Option<u32> }
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;
    fn write_index(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn read_index(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;

    fn call(&mut self, o: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct Owners<S: Services> { pub storage: S::Storage, pub call: Option<CallReturned<S::CallOwner>> }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub rbx: u64 }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { ZeroConstruct, IndexedConstruct, IndexedDestroy }
impl CallSite {
    pub fn pc(self) -> u32 { match self { Self::ZeroConstruct => 0x55e0af, Self::IndexedConstruct => 0x55e0cd, Self::IndexedDestroy => 0x55e104 } }
    pub fn target(self) -> u32 { match self { Self::ZeroConstruct => 0x57a7e0, Self::IndexedConstruct => 0x561fa4, Self::IndexedDestroy => 0x561fac } }
    pub fn resume_pc(self) -> u32 { match self { Self::ZeroConstruct => 0x55e0b4, Self::IndexedConstruct => 0x55e0d2, Self::IndexedDestroy => 0x55e109 } }
}
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub rcx: u64, pub work_rsp: u64,

    pub required_rbx: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct IndexedReceiver { pub signed_index: i32, pub extended: u64, pub times_five: u64, pub base: u64, pub receiver: u64 }

pub fn indexed_receiver(actual_source_image_base: u64, index: u32) -> IndexedReceiver {
    let signed_index = index as i32; let extended = (signed_index as i64) as u64;
    let times_five = extended.wrapping_add(extended.wrapping_mul(4)); let base = actual_source_image_base.wrapping_add(0x85f480);
    IndexedReceiver { signed_index, extended, times_five, base, receiver: base.wrapping_add(times_five.wrapping_mul(8)) }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { StoreConstructorIndex, BindRbx, ReadDestructorIndex, Branch, PrepareIndexed, Call, NormalRequired, TailRestoreRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub wrapper_call: Option<wrapper_call::History>, pub index: Option<u32>, pub index_store_returned: bool, pub bound_rbx: Option<u64>, pub indexed: Option<IndexedReceiver>,
    pub request: Option<CallRequest>, pub call_returned: bool, pub rax: Option<u64> }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall, CallBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Next<S>), Failed(Stopped<S>) }
pub enum Next<S: Services> { Normal(NormalRequired<S>), Tail(TailRestoreRequired<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualBody, pub captures: Captures,
    pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct NormalRequired<S: Services> { parts: Parts<S> }
pub struct TailRestoreRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NormalRequired<S> {
    pub fn required_next_pc(&self) -> u32 { match self.parts.actual.entry { Entry::Construct55e0a0 => 0x55e0d2, Entry::Destroy55e0dc => 0x55e109 } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> TailRestoreRequired<S> { pub fn required_next_pc(&self) -> u32 { 0x55e0e7 } pub fn into_parts(self) -> Parts<S> { self.parts } }
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != input.actual.entry.body_pc() || (input.actual.entry == Entry::Construct55e0a0 && input.actual.constructor_edx.is_none()) { return Err(Rejected { input }); }
        let phase = match input.actual.entry { Entry::Construct55e0a0 => Phase::StoreConstructorIndex, Entry::Destroy55e0dc => Phase::ReadDestructorIndex };
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, call: None }, actual: input.actual,
            captures: Captures::default(), phase, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); } let mut spent = 0;
        while self.phase != Phase::NormalRequired && self.phase != Phase::TailRestoreRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        match self.phase { Phase::NormalRequired => Poll::Ready(Next::Normal(NormalRequired { parts: self.into_parts() })),
            Phase::TailRestoreRequired => Poll::Ready(Next::Tail(TailRestoreRequired { parts: self.into_parts() })), _ => Poll::Pending(self) }
    }
}
