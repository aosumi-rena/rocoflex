#[path = "parameter_number_builder_steps.rs"]
mod steps;
#[path = "parameter_number_builder_return.rs"]
pub mod native_return;
#[path = "parameter_number_builder_entry.rs"]
pub mod native_entry;
pub trait Services: Sized {
    type Storage;
    type CallOwner;
    type Error;
    fn read_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn read_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn read_gs58(&mut self, owners: &mut Owners<Self>, pc: u32) -> Result<u64, Self::Error>;
    fn write_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn write_dword(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;

    fn zero_xmm0(&mut self, owners: &mut Owners<Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn write_xmm0(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;

    fn call(&mut self, owners: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: [Option<CallReturned<S::CallOwner>>; 7] }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved { pub rbx: u64, pub rdi: u64, pub rsi: u64, pub rbp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: Option<u64>, pub cl: Option<u8> }
#[derive(Clone, Copy, Debug)]
pub enum Target { DirectRva(u32), LoadedIndirect(u64) }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { Allocate, SetClOne, Enter, GetObject, Virtual, Register, Leave }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::Allocate => 0, Self::SetClOne => 1, Self::Enter => 2,
        Self::GetObject => 3, Self::Virtual => 4, Self::Register => 5, Self::Leave => 6 } }
    pub fn pc(self) -> u32 { match self { Self::Allocate => 0x1fb44a, Self::SetClOne => 0x1fb454,
        Self::Enter => 0x1fb4fa, Self::GetObject => 0x1fb50c, Self::Virtual => 0x1fb51f,
        Self::Register => 0x1fb529, Self::Leave => 0x1fb535 } }
    pub fn resume_pc(self) -> u32 { match self { Self::Allocate => 0x1fb44f, Self::SetClOne => 0x1fb459,
        Self::Enter => 0x1fb4ff, Self::GetObject => 0x1fb511, Self::Virtual => 0x1fb522,
        Self::Register => 0x1fb52e, Self::Leave => 0x1fb53a } }
}
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: Target, pub args: Arguments,
    pub work_rsp: u64, pub required_saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub rbp: u64,
    pub object_rcx: u64, pub source_rdx: u64, pub entry_rdi: u64 }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Bind, StoreBaseVtable, Allocate, SetClOne, StoreFixed, ZeroXmm0, StoreZeroFirst,
    StoreZeroSecond, StoreVtable, ReadGlobal, ReadIndex, ReadGs58, ReadThread, ReadThreadWord,
    Compare, Enter, ReadCurrentGlobal, SaveObject, GetObject, ReadReturnedObject, StoreGlobalObject,
    ReadVtable, ReadTarget, Virtual, Register, Leave, ReloadObject, ReadFirstTarget, StoreFirst,
    ReadSecondTarget, StoreSecond, ReadThirdTarget, StoreThird, NativeReturnRequired }
#[derive(Clone, Copy, Debug)]
pub struct WordWrite { pub pc: u32, pub address: u64, pub word: u64 }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub native_entry: Option<native_entry::History>,
    pub requests: [Option<CallRequest>; 7], pub completed: [bool; 7], pub fixed_stores: [bool; 9],
    pub initial_vtable_returned: bool, pub allocation: Option<u64>, pub cl_one_result: Option<u64>,
    pub last_write: Option<WordWrite>, pub xmm0: Option<[u32; 4]>, pub zero_first: bool, pub zero_second: bool,
    pub vtable_returned: bool, pub global: Option<u32>, pub index: Option<u32>, pub gs58: Option<u64>,
    pub thread: Option<u64>, pub thread_word: Option<u32>, pub current_global: Option<u32>, pub object_saved: bool,
    pub object_query_return: Option<u64>, pub returned_object: Option<u64>, pub global_object_stored: bool,
    pub vtable: Option<u64>, pub target: Option<u64>, pub reloaded_object: Option<u64>,
    pub first_target: Option<u64>, pub first_stored: bool, pub second_target: Option<u64>, pub second_stored: bool,
    pub third_target: Option<u64>, pub third_stored: bool,
}
pub enum Fault<E> { Service(E), MissingState, CallBinding { site: CallSite }, OccupiedCall { site: CallSite }, XmmBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody,
    captures: Captures, saved: Option<Saved>, phase: Phase, index: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualBody,
    pub captures: Captures, pub saved: Option<Saved>, pub phase: Phase, pub index: usize,
    pub attempted: Option<(Phase, usize)>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn owners(&self) -> &Owners<S> { &self.owners }
}
pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1fb4e7 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x1fb435 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage,
            calls: [None, None, None, None, None, None, None] }, actual: input.actual, captures: Captures::default(),
            saved: None, phase: Phase::Bind, index: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, saved: self.saved, phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeReturnRequired && spent < budget {
            self.attempted = Some((self.phase, self.index)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::NativeReturnRequired { Poll::Ready(NativeReturnRequired { parts: self.into_parts() }) }
        else { Poll::Pending(self) }
    }
}
