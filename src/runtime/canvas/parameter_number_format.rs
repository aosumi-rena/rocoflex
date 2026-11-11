#[path = "parameter_number_format_steps.rs"]
mod steps;
#[path = "parameter_number_format_sentry.rs"]
pub mod sentry;
#[path = "parameter_number_format_facet.rs"]
pub mod facet;
#[path = "parameter_number_format_finish.rs"]
pub mod finish;
#[path = "parameter_number_format_entry.rs"]
pub mod native_entry;
#[path = "parameter_number_format_constructor.rs"]
pub mod constructor;
#[path = "parameter_number_format_constructor_call.rs"]
pub mod constructor_call;
pub trait Services: Sized {
    type Storage; type ConstructorOwner; type Error;
    fn read_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn read_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn write_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn write_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn zero_xmm0(&mut self, owners: &mut Owners<Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn write_xmm0(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;

    fn or_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, mask: u32,
        returned: &mut Option<OrReturned>) -> Result<(), Self::Error>;

    fn construct_1fbcf0(&mut self, owners: &mut Owners<Self>, request: ConstructorRequest) -> Result<(), Self::Error>;
}
pub struct Owners<S: Services> { pub storage: S::Storage, pub constructor: Option<ConstructorReturned<S::ConstructorOwner>> }
pub struct ConstructorReturned<R> { pub actual: ConstructorActualReturn, pub owner: R }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved { pub rbx: u64, pub rsi: u64, pub rbp: u64, pub xmm6: [u32; 4] }
#[derive(Clone, Copy, Debug)]
pub struct ConstructorRequest { pub call_pc: u32, pub rcx: u64, pub rdx: u64, pub r8d: u32,
    pub work_rsp: u64, pub required_saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct ConstructorActualReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct OrReturned { pub before: u32, pub after: u32 }
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub rbp: u64, pub builder_rcx: u64,
    pub entry_rbx: u64, pub precision_r8d: u32, pub flag_r9d: u32, pub value_xmm1: [u32; 4] }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Bind, StoreTable20, Zero38, ZeroXmm0, Zero60, Zero70, StoreTable30, Zero2c,
    StoreBuilder18, Construct, PrecisionBranch, ReadFlagsTable, ReadFlagsOffset, PrepareMask, OrFlags,
    ReadValueTable, ReadValueOffset, StorePrecision, SentryRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub constructor_call: Option<constructor_call::History>, pub bound_rbx: Option<u64>, pub table20_stored: bool, pub zero38: bool, pub xmm0: Option<[u32; 4]>,
    pub zero60: bool, pub zero70: bool, pub table30_stored: bool, pub zero2c: bool, pub builder18_stored: bool,
    pub constructor_request: Option<ConstructorRequest>, pub constructor_returned: bool,
    pub flags_table: Option<u64>, pub flags_offset_word: Option<u32>, pub flags_displacement: Option<u64>,
    pub flag_mask: Option<u32>, pub flags_address: Option<u64>, pub or_returned: Option<OrReturned>,
    pub value_table: Option<u64>, pub value_offset_word: Option<u32>, pub value_displacement: Option<u64>,
    pub precision_address: Option<u64>, pub precision_stored: bool }
pub enum Fault<E> { Service(E), MissingState, XmmBinding, ConstructorBinding, OccupiedConstructor, OrBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    saved: Option<Saved>, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(SentryRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualBody,
    pub captures: Captures, pub saved: Option<Saved>, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn owners(&self) -> &Owners<S> { &self.owners }
}

pub struct SentryRequired<S: Services> { parts: Parts<S> }
impl<S: Services> SentryRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1fb632 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x1fb5b4 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, constructor: None },
            actual: input.actual, captures: Captures::default(), saved: None, phase: Phase::Bind, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::SentryRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::SentryRequired { Poll::Ready(SentryRequired { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
}
