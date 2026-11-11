#[path = "parameter_stream_object_lookup_memory.rs"]
mod memory;
#[path = "parameter_stream_object_lookup_steps.rs"]
mod steps;
#[path = "parameter_stream_object_lookup_return.rs"]
pub mod native_return;
#[path = "parameter_stream_object_lookup_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_guard.rs"]
pub mod guard;
#[path = "parameter_stream_guard_call.rs"]
pub mod guard_call;
#[path = "parameter_stream_fallback_context.rs"]
pub mod fallback;
#[path = "parameter_stream_fallback_call.rs"]
pub mod fallback_call;
#[path = "parameter_stream_object_register.rs"]
pub mod registration;
#[path = "parameter_stream_register_call.rs"]
pub mod registration_call;
#[path = "parameter_stream_member_construction.rs"]
pub mod member_construction;
#[path = "parameter_stream_member_call.rs"]
pub mod member_call;

pub use super::{Arguments, Target, Saved, CallReturned, ActualCallReturn};
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;
    fn read_word(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn read_dword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn read_byte(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn write_word(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn write_dword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;

    fn call(&mut self, o: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
}
pub type Calls<R> = [Option<CallReturned<R>>; 11];
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner> }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { EnterOuter, EnterInner, LeaveInner, Fallback, Allocate, ConstructTemporary, ConstructMember, DestroyTemporary, Register, AcquireVirtual, LeaveOuter }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::EnterOuter => 0, Self::EnterInner => 1, Self::LeaveInner => 2, Self::Fallback => 3, Self::Allocate => 4,
        Self::ConstructTemporary => 5, Self::ConstructMember => 6, Self::DestroyTemporary => 7, Self::Register => 8, Self::AcquireVirtual => 9, Self::LeaveOuter => 10 } }
    pub fn pc(self) -> u32 { match self { Self::EnterOuter => 0x18c2f6, Self::EnterInner => 0x18c314, Self::LeaveInner => 0x18c33e, Self::Fallback => 0x18c36b,
        Self::Allocate => 0x18c394, Self::ConstructTemporary => 0x18c3c2, Self::ConstructMember => 0x18c3e3, Self::DestroyTemporary => 0x18c3ec,
        Self::Register => 0x18c3f4, Self::AcquireVirtual => 0x18c403, Self::LeaveOuter => 0x18c411 } }
    pub fn resume_pc(self) -> u32 { match self { Self::EnterOuter => 0x18c2fb, Self::EnterInner => 0x18c319, Self::LeaveInner => 0x18c343, Self::Fallback => 0x18c370,
        Self::Allocate => 0x18c399, Self::ConstructTemporary => 0x18c3c7, Self::ConstructMember => 0x18c3e8, Self::DestroyTemporary => 0x18c3f1,
        Self::Register => 0x18c3f9, Self::AcquireVirtual => 0x18c406, Self::LeaveOuter => 0x18c416 } }
}
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: Target, pub args: Arguments, pub work_rsp: u64, pub required_saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub rbp: u64, pub holder_rcx: u64,
    pub entry_rbx: u64, pub entry_rdi: u64, pub entry_r14: u64 }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Bind, EnterOuter, ReadCached, ReadIndex, EnterInner, ReadCurrentIndex, ReadCounter, IncrementCounter,
    StoreCounter, StoreIndex, LeaveInner, ReloadIndex, ReadContext, ReadCount, ReadArray, ReadCandidate, ReadFallbackFlag,
    Fallback, ReadFallbackCount, ReadFallbackArray, ReadFallbackCandidate, UseCached, Allocate, ReadAllocationContext,
    SaveAllocation, ChooseName, ReadNameOverride, ConstructTemporary, ReloadObject, ZeroObjectCount, StoreVtable,
    ConstructMember, DestroyTemporary, Register, ReloadPublishedObject, ReadVtable, ReadAcquireTarget, AcquireVirtual,
    Publish, LeaveOuter, NativeReturnRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub guard_calls: [Option<guard_call::History>; 4], pub fallback_call: Option<fallback_call::History>, pub registration_call: Option<registration_call::History>, pub member_call: Option<member_call::History>, pub requests: [Option<CallRequest>; 11], pub completed: [bool; 11], pub cached: Option<u64>, pub initial_index: Option<u64>,
    pub current_index: Option<u64>, pub counter: Option<u32>, pub next_counter: Option<u64>, pub counter_stored: bool, pub index_stored: bool,
    pub reloaded_index: Option<u64>, pub context: Option<u64>, pub count: Option<u64>, pub array: Option<u64>, pub candidate: Option<u64>,
    pub fallback_flag: Option<u8>, pub fallback_context: Option<u64>, pub fallback_count: Option<u64>, pub fallback_array: Option<u64>, pub fallback_candidate: Option<u64>,
    pub allocation: Option<u64>, pub allocation_context: Option<u64>, pub allocation_saved: bool, pub name_override: Option<u64>, pub name_input: Option<u64>,
    pub reloaded_object: Option<u64>, pub object_count_zeroed: bool, pub vtable_stored: bool, pub published_object: Option<u64>, pub vtable: Option<u64>,
    pub acquire_target: Option<u64>, pub published: bool }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall { site: CallSite }, CallBinding { site: CallSite }, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures, saved: Option<Saved>,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualBody, pub captures: Captures,
    pub saved: Option<Saved>, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }

pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x18c416 }
    pub fn selected_r14(&self) -> u64 { self.parts.saved.unwrap().r14 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x18c2ed { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage,
            calls: [None, None, None, None, None, None, None, None, None, None, None] }, actual: input.actual, captures: Captures::default(),
            saved: None, phase: Phase::Bind, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual, captures: self.captures,
        saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeReturnRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase == Phase::NativeReturnRequired { Poll::Ready(NativeReturnRequired { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
}
