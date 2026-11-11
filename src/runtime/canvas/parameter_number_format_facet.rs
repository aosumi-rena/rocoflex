use super::{Owners, Saved, sentry};
#[path = "parameter_number_format_facet_steps.rs"]
mod steps;
pub type Calls<R> = [Option<CallReturned<R>>; 5];
pub trait Services: sentry::Services {
    type FacetCallOwner;
    fn facet_read_word(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut Calls<Self::FacetCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn facet_read_dword(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut Calls<Self::FacetCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn facet_read_byte(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut Calls<Self::FacetCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn facet_write_word(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut Calls<Self::FacetCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn facet_write_dword(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut Calls<Self::FacetCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn facet_write_byte(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut Calls<Self::FacetCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;

    fn facet_call(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut Calls<Self::FacetCallOwner>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { Acquire, GetFacet, QueryContext, DeleteObject, Format }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::Acquire => 0, Self::GetFacet => 1, Self::QueryContext => 2, Self::DeleteObject => 3, Self::Format => 4 } }
    pub fn pc(self) -> u32 { match self { Self::Acquire => 0x1fb6d4, Self::GetFacet => 0x1fb6db, Self::QueryContext => 0x1fb6ef, Self::DeleteObject => 0x1fb702, Self::Format => 0x1fb743 } }
    pub fn resume_pc(self) -> u32 { match self { Self::Acquire => 0x1fb6d7, Self::GetFacet => 0x1fb6e0, Self::QueryContext => 0x1fb6f2, Self::DeleteObject => 0x1fb705, Self::Format => 0x1fb745 } }
}
#[derive(Clone, Copy, Debug)]
pub enum Target { LoadedIndirect(u64), DirectRva(u32) }
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: u64, pub rdx: Option<u64>, pub r8: Option<u64>, pub r9: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct StackArguments { pub byte_address: u64, pub byte: u8, pub f64_address: u64, pub f64_bits: u64 }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: Target, pub args: Arguments, pub stack: Option<StackArguments>,
    pub work_rsp: u64, pub required_saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Saved }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ReadInitialOffset, ReadHolder, ReadContext, SaveContext, ReadAcquireVtable, ReadAcquireTarget,
    Acquire, GetFacet, ReloadContext, ReadQueryVtable, ReadQueryTarget, QueryContext, ReadDeleteVtable,
    ReadDeleteTarget, DeleteObject, ReloadTable, ReadFormatOffset, ReadFormatByte, ReadField68,
    StoreZero8, StoreField10, ReadFacetVtable, ReadFormatTarget, StoreStackF64, StoreStackByte,
    Format, ReadOutputByte, StoreOutputStatus, StatusRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub requests: [Option<CallRequest>; 5], pub completed: [bool; 5],
    pub initial_offset_word: Option<u32>, pub initial_displacement: Option<u64>, pub holder: Option<u64>, pub context: Option<u64>,
    pub context_stored: bool, pub acquire_vtable: Option<u64>, pub acquire_target: Option<u64>, pub facet: Option<u64>,
    pub reloaded_context: Option<u64>, pub query_vtable: Option<u64>, pub query_target: Option<u64>, pub query_result: Option<u64>,
    pub delete_vtable: Option<u64>, pub delete_target: Option<u64>, pub format_table: Option<u64>, pub format_offset_word: Option<u32>,
    pub format_displacement: Option<u64>, pub format_base: Option<u64>, pub format_byte: Option<u8>, pub field68: Option<u64>,
    pub zero8_stored: bool, pub field10_stored: bool, pub facet_vtable: Option<u64>, pub format_target: Option<u64>,
    pub stack_f64_bits: Option<u64>, pub stack_f64_stored: bool, pub stack_byte_stored: bool,
    pub output_byte: Option<u8>, pub output_status: Option<u32>, pub output_status_stored: bool }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall { site: CallSite }, CallBinding { site: CallSite }, AlreadyStopped }
pub struct Prepared<S: Services> { sentry: sentry::Parts<S>, calls: Calls<S::FacetCallOwner>,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(StatusRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) sentry: sentry::Parts<S>, pub(in super::super) calls: Calls<S::FacetCallOwner>,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn sentry(&self) -> &sentry::Parts<S> { &self.sentry }
    pub fn calls(&self) -> &Calls<S::FacetCallOwner> { &self.calls }
}

pub struct StatusRequired<S: Services> { parts: Parts<S> }
impl<S: Services> StatusRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1fb752 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_sentry(completed: sentry::FacetRequired<S>) -> Self {
        Self { sentry: completed.into_parts(), calls: [None, None, None, None, None], captures: Captures::default(),
            phase: Phase::ReadInitialOffset, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { sentry: self.sentry, calls: self.calls, captures: self.captures,
        phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::StatusRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::StatusRequired { Poll::Ready(StatusRequired { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
}
