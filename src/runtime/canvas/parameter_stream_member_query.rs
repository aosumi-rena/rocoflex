#[path = "parameter_stream_member_query_steps.rs"]
mod steps;
#[path = "parameter_stream_member_query_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_query_return.rs"]
pub mod native_return;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry { Context567c68, Word567c9c, Source56807c }
impl Entry {
    pub fn entry_pc(self) -> u32 { match self { Self::Context567c68 => 0x567c68, Self::Word567c9c => 0x567c9c, Self::Source56807c => 0x56807c } }
    pub fn body_pc(self) -> u32 { self.entry_pc() + 4 }
    fn read_reference_pc(self) -> u32 { match self { Self::Context567c68 => 0x567c76, Self::Word567c9c => 0x567caa, Self::Source56807c => 0x56808a } }
    fn save_reference_pc(self) -> u32 { match self { Self::Context567c68 => 0x567c7d, Self::Word567c9c => 0x567cb1, Self::Source56807c => 0x568091 } }
    fn refresh_pc(self) -> u32 { match self { Self::Context567c68 => 0x567c85, Self::Word567c9c => 0x567cb9, Self::Source56807c => 0x568099 } }
    fn reload_pc(self) -> u32 { match self { Self::Context567c68 => 0x567c8a, Self::Word567c9c => 0x567cbe, Self::Source56807c => 0x56809e } }
    fn projection_pc(self) -> u32 { match self { Self::Context567c68 => 0x567c8f, Self::Word567c9c => 0x567cc3, Self::Source56807c => 0x5680a3 } }
    pub fn return_boundary_pc(self) -> u32 { match self { Self::Context567c68 => 0x567c95, Self::Word567c9c => 0x567cc6, Self::Source56807c => 0x5680a6 } }
    pub fn ret_pc(self) -> u32 { match self { Self::Context567c68 => 0x567c99, Self::Word567c9c => 0x567cca, Self::Source56807c => 0x5680aa } }
}
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub entry: Entry, pub pc: u32, pub source_image_base: u64, pub work_rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { Context, Refresh }
impl CallSite { pub fn index(self) -> usize { match self { Self::Context => 0, Self::Refresh => 1 } } }
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: Option<u64>, pub rdx: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub pc: u32, pub resume_pc: u32, pub target_rva: u32,
    pub target: u64, pub work_rsp: u64, pub args: Arguments }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64 }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
pub type Calls<R> = [Option<CallReturned<R>>; 2];
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;

    fn call(&mut self, owners: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
    fn read_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn read_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn write_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
}
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner> }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Context, ReadReference, SaveReference, Refresh, ReloadReference, Project, NativeReturnRequired }
#[derive(Clone, Copy, Debug)]
pub struct MemoryAttempt { pub pc: u32, pub address: u64, pub width: u8, pub written: Option<u64> }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub requests: [Option<CallRequest>; 2], pub completed: [bool; 2], pub context: Option<u64>,
    pub reference: Option<u64>, pub saved_reference: bool, pub refreshed_reference: Option<u64>, pub rax: Option<u64>,

    pub memory: Option<MemoryAttempt> }
pub enum Fault<E> { Service(E), OccupiedCall { site: CallSite }, MissingState, CallBinding { site: CallSite }, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>, pub actual: ActualBody,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }

pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { self.parts.actual.entry.return_boundary_pc() }
    pub fn result(&self) -> u64 { self.parts.captures.rax.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != input.actual.entry.body_pc() { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, calls: [None, None] }, actual: input.actual,
            captures: Captures::default(), phase: Phase::Context, attempted: None, stopped: false })
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
