use super::{Owners, Saved, facet, sentry};
#[path = "parameter_number_format_finish_memory.rs"]
mod memory;
#[path = "parameter_number_format_finish_steps.rs"]
mod steps;
#[path = "parameter_number_format_return.rs"]
pub mod native_return;
pub type Calls<R> = [Option<CallReturned<R>>; 7];
pub trait Services: facet::Services {
    type FinishCallOwner;
    fn finish_read_word(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn finish_read_dword(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn finish_read_128(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error>;
    fn finish_write_word(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn finish_write_dword(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;

    fn finish_write_128(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;

    fn finish_call(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { Query, Flush, CleanupVirtual, Destroy, ErrorConstruct, ErrorMessage, Throw }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::Query => 0, Self::Flush => 1, Self::CleanupVirtual => 2, Self::Destroy => 3, Self::ErrorConstruct => 4, Self::ErrorMessage => 5, Self::Throw => 6 } }
    pub fn pc(self) -> u32 { match self { Self::Query => 0x1fb781, Self::Flush => 0x1fb791, Self::CleanupVirtual => 0x1fb7b1, Self::Destroy => 0x1fb7c3, Self::ErrorConstruct => 0x1fb816, Self::ErrorMessage => 0x1fb82e, Self::Throw => 0x1fb849 } }
    pub fn resume_pc(self) -> u32 { match self { Self::Query => 0x1fb786, Self::Flush => 0x1fb796, Self::CleanupVirtual => 0x1fb7b4, Self::Destroy => 0x1fb7c8, Self::ErrorConstruct => 0x1fb81b, Self::ErrorMessage => 0x1fb833, Self::Throw => 0x1fb84e } }
}
#[derive(Clone, Copy, Debug)]
pub enum Target { DirectRva(u32), LoadedIndirect(u64) }
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: Option<u64>, pub rdx: Option<u64>, pub r8: Option<u64>, pub r9: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: Target, pub args: Arguments, pub work_rsp: u64, pub required_saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Saved }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ReadTable, ReadOffset, ReadStatus, ReadField68, ReadExtra, Compose, StoreStatus, ReadMask, Branch,
    Query, ReadFlushStream, Flush, ReadCleanupStream, ReadCleanupVtable, ReadCleanupOffset, ReadCleanupObject,
    ReadCleanupObjectVtable, ReadCleanupTarget, CleanupVirtual, StoreCleanupVtable, Destroy,
    ReadBuilder, ReadFirstPointer, ReadSecondPointer, ReadSecondValue, ReadFirstValue,
    SelectMessage, ErrorConstruct, ReadErrorVector, StoreErrorVector, ErrorMessage, StoreErrorVtable, Throw,
    NativeRestoreRequired, BreakpointRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub requests: [Option<CallRequest>; 7], pub completed: [bool; 7],
    pub table: Option<u64>, pub offset_word: Option<u32>, pub displacement: Option<u64>,
    pub status: Option<u32>, pub field68: Option<u64>, pub extra: Option<u32>, pub composed: Option<u32>,
    pub status_stored: bool, pub mask: Option<u32>, pub masked: Option<u32>, pub query_rax: Option<u64>,
    pub flush_stream: Option<u64>, pub cleanup_stream: Option<u64>, pub cleanup_vtable: Option<u64>,
    pub cleanup_offset_word: Option<u32>, pub cleanup_displacement: Option<u64>, pub cleanup_object: Option<u64>,
    pub cleanup_object_vtable: Option<u64>, pub cleanup_target: Option<u64>, pub cleanup_vtable_stored: bool,
    pub builder: Option<u64>, pub first_pointer: Option<u64>, pub second_pointer: Option<u64>,
    pub second_value: Option<u64>, pub first_value: Option<u64>, pub return_rax: Option<u64>,
    pub selected_rdx: Option<u64>, pub selected_message: Option<u64>, pub error_vector: Option<[u32; 4]>,
    pub error_vector_stored: bool, pub error_vtable_stored: bool }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall { site: CallSite }, CallBinding { site: CallSite }, AlreadyStopped }

pub(in super::super) enum Prior<S: Services> { Sentry(sentry::Parts<S>), Facet(facet::Parts<S>) }
impl<S: Services> Prior<S> {
    fn prefix(&self) -> &super::Parts<S> { match self { Self::Sentry(p) => &p.prefix, Self::Facet(p) => &p.sentry.prefix } }
    fn prefix_mut(&mut self) -> &mut super::Parts<S> { match self { Self::Sentry(p) => &mut p.prefix, Self::Facet(p) => &mut p.sentry.prefix } }
}
pub struct Prepared<S: Services> { prior: Prior<S>, calls: Calls<S::FinishCallOwner>, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Next<S>), Failed(Stopped<S>) }
pub enum Next<S: Services> { Normal(NativeRestoreRequired<S>), Breakpoint(BreakpointRequired<S>) }
pub struct Parts<S: Services> { pub(in super::super) prior: Prior<S>, pub(in super::super) calls: Calls<S::FinishCallOwner>,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn prefix(&self) -> &super::Parts<S> { self.prior.prefix() }
    pub fn calls(&self) -> &Calls<S::FinishCallOwner> { &self.calls }
    pub fn sentry(&self) -> &sentry::Parts<S> { match &self.prior { Prior::Sentry(p) => p, Prior::Facet(p) => &p.sentry } }
    pub fn facet(&self) -> Option<&facet::Parts<S>> { match &self.prior { Prior::Sentry(_) => None, Prior::Facet(p) => Some(p) } }
}
pub struct NativeRestoreRequired<S: Services> { parts: Parts<S> }
pub struct BreakpointRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeRestoreRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1fb7da }
    pub fn required_rax(&self) -> u64 { self.parts.captures.return_rax.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> BreakpointRequired<S> {

    pub fn required_next_pc(&self) -> u32 { 0x1fb84e }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_sentry(completed: sentry::StatusRequired<S>) -> Self { Self::start(Prior::Sentry(completed.into_parts())) }
    pub fn from_facet(completed: facet::StatusRequired<S>) -> Self { Self::start(Prior::Facet(completed.into_parts())) }
    fn start(prior: Prior<S>) -> Self { Self { prior, calls: [None, None, None, None, None, None, None],
        captures: Captures::default(), phase: Phase::ReadTable, attempted: None, stopped: false } }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { prior: self.prior, calls: self.calls, captures: self.captures,
        phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeRestoreRequired && self.phase != Phase::BreakpointRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        match self.phase {
            Phase::NativeRestoreRequired => Poll::Ready(Next::Normal(NativeRestoreRequired { parts: self.into_parts() })),
            Phase::BreakpointRequired => Poll::Ready(Next::Breakpoint(BreakpointRequired { parts: self.into_parts() })),
            _ => Poll::Pending(self),
        }
    }
}
