use super::{Owners, Saved};
#[path = "parameter_number_format_sentry_steps.rs"]
mod steps;
pub type Calls<R> = [Option<CallReturned<R>>; 2];
pub trait Services: super::Services {
    type SentryCallOwner;
    fn sentry_read_word(&mut self, prefix: &mut Owners<Self>, calls: &mut Calls<Self::SentryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn sentry_read_dword(&mut self, prefix: &mut Owners<Self>, calls: &mut Calls<Self::SentryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn sentry_write_word(&mut self, prefix: &mut Owners<Self>, calls: &mut Calls<Self::SentryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn sentry_write_dword(&mut self, prefix: &mut Owners<Self>, calls: &mut Calls<Self::SentryCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn sentry_write_byte(&mut self, prefix: &mut Owners<Self>, calls: &mut Calls<Self::SentryCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;

    fn sentry_call(&mut self, prefix: &mut Owners<Self>, calls: &mut Calls<Self::SentryCallOwner>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { PreVirtual, Tied }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::PreVirtual => 0, Self::Tied => 1 } }
    pub fn pc(self) -> u32 { match self { Self::PreVirtual => 0x1fb652, Self::Tied => 0x1fb68e } }
    pub fn resume_pc(self) -> u32 { match self { Self::PreVirtual => 0x1fb655, Self::Tied => 0x1fb693 } }
}
#[derive(Clone, Copy, Debug)]
pub enum Target { LoadedIndirect(u64), DirectRva(u32) }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: Target, pub rcx: u64, pub work_rsp: u64, pub required_saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Saved }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { BindSentry, SaveStream, ReadTable, ReadOffset, ReadPreObject, ReadPreVtable, ReadPreTarget, PreCall,
    ReloadTable, ReloadOffset, ReadInitialStatus, StoreFalse, ZeroInitialStatus, ReadTie, TieBranch, TieCall,
    ReloadTieTable, ReloadTieOffset, ZeroTieStatus, ReadTieStatus, StoreTieEqual, StoreTrue, FacetRequired, StatusRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub requests: [Option<CallRequest>; 2], pub completed: [bool; 2], pub stream_saved: bool,
    pub current_vtable: Option<u64>, pub current_offset_word: Option<u32>, pub current_displacement: Option<u64>,
    pub pre_object: Option<u64>, pub pre_vtable: Option<u64>, pub pre_target: Option<u64>, pub initial_status: Option<u32>,
    pub sentry_byte: Option<u8>, pub sentry_byte_stored: bool, pub a4_zero_stored: bool,
    pub tie_object: Option<u64>, pub tie_is_self: Option<bool>, pub tie_status: Option<u32> }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall { site: CallSite }, CallBinding { site: CallSite }, AlreadyStopped }
pub struct Prepared<S: Services> { prefix: super::Parts<S>, calls: Calls<S::SentryCallOwner>,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Next<S>), Failed(Stopped<S>) }
pub enum Next<S: Services> { Facet(FacetRequired<S>), Status(StatusRequired<S>) }
pub struct Parts<S: Services> { pub(in super::super) prefix: super::Parts<S>, pub(in super::super) calls: Calls<S::SentryCallOwner>,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn prefix(&self) -> &super::Parts<S> { &self.prefix }
    pub fn calls(&self) -> &Calls<S::SentryCallOwner> { &self.calls }
}

pub struct FacetRequired<S: Services> { parts: Parts<S> }
pub struct StatusRequired<S: Services> { parts: Parts<S> }
impl<S: Services> FacetRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1fb6c0 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> StatusRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1fb752 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_prefix(completed: super::SentryRequired<S>) -> Self {
        Self { prefix: completed.into_parts(), calls: [None, None], captures: Captures::default(), phase: Phase::BindSentry, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { prefix: self.prefix, calls: self.calls, captures: self.captures,
        phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::FacetRequired && self.phase != Phase::StatusRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        match self.phase {
            Phase::FacetRequired => Poll::Ready(Next::Facet(FacetRequired { parts: self.into_parts() })),
            Phase::StatusRequired => Poll::Ready(Next::Status(StatusRequired { parts: self.into_parts() })),
            _ => Poll::Pending(self),
        }
    }
}
