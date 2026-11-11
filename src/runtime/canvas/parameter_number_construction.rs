#[path = "parameter_number_construction_steps.rs"]
mod steps;
#[path = "parameter_number_construction_return.rs"]
pub mod native_return;
#[path = "parameter_number_construction_entry.rs"]
pub mod native_entry;
#[path = "parameter_number_text_copy.rs"]
pub mod text_copy;
#[path = "parameter_number_text_copy_call.rs"]
pub mod text_copy_call;
#[path = "parameter_number_builder.rs"]
pub mod builder;
#[path = "parameter_number_builder_call.rs"]
pub mod builder_call;
#[path = "parameter_number_format.rs"]
pub mod formatting;
#[path = "parameter_number_format_call.rs"]
pub mod format_call;
pub trait Services: Sized {
    type Storage;
    type CallOwner;
    type Error;
    fn read_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn write_word(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;

    fn xchg_ref_zero(&mut self, owners: &mut Owners<Self>, pc: u32, header: u64, prior: &mut Option<u32>) -> Result<(), Self::Error>;

    fn call(&mut self, owners: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: [Option<CallReturned<S::CallOwner>>; 7] }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]

pub struct Nonvolatile { pub rbx: u64, pub rbp: u64, pub rsi: u64, pub rdi: u64, pub xmm6: [u32; 4] }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Nonvolatile }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallSite { Builder, Format, Query, Delete, Free, Allocate, Copy }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::Builder => 0, Self::Format => 1, Self::Query => 2,
        Self::Delete => 3, Self::Free => 4, Self::Allocate => 5, Self::Copy => 6 } }
    pub fn pc(self) -> u32 { match self { Self::Builder => 0x1caeb3, Self::Format => 0x1caec5,
        Self::Query => 0x1caeed, Self::Delete => 0x1caf00, Self::Free => 0x1caf06,
        Self::Allocate => 0x1caf1a, Self::Copy => 0x1caf3e } }
    pub fn resume_pc(self) -> u32 { match self { Self::Builder => 0x1caeb8, Self::Format => 0x1caeca,
        Self::Query => 0x1caef0, Self::Delete => 0x1caf03, Self::Free => 0x1caf0b,
        Self::Allocate => 0x1caf1f, Self::Copy => 0x1caf43 } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target { DirectRva(u32), LoadedIndirect(u64) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arguments { pub rcx: u64, pub rdx: Option<u64>, pub r8: Option<u64>, pub r9: Option<u64>, pub xmm1: Option<[u32; 4]> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallRequest { pub site: CallSite, pub target: Target, pub args: Arguments, pub work_rsp: u64, pub required_saved: Nonvolatile }
#[derive(Clone, Copy, Debug)]
pub struct ActualBody {
    pub pc: u32, pub actual_image_base: u64, pub rbp: u64, pub output_rcx: u64,
    pub entry_rbx: u64, pub precision_r8d: u32, pub flag_r9d: u32, pub value_xmm1: [u32; 4],
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    SaveBindings, Builder, Format, StoreBuilderVtable, ReadCleanupPointer, ReadAttached,
    ReadQueryVtable, ReadQueryTarget, Query, ReadDeleteVtable, ReadDeleteTarget,
    Delete, Free, PrepareAllocation, Allocate, XchgReference, StoreCapacity,
    StoreOutput, StoreLocalData, Copy, NativeReturnRequired,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub native_entry: Option<native_entry::History>,
    pub text_copy_call: Option<text_copy_call::History>,
    pub builder_call: Option<builder_call::History>,
    pub format_call: Option<format_call::History>,
    pub bl_bound_rbx: Option<u64>, pub requests: [Option<CallRequest>; 7], pub completed: [bool; 7],
    pub length: Option<u64>, pub vtable_store_returned: bool, pub cleanup_pointer: Option<u64>,
    pub attached: Option<u64>, pub query_vtable: Option<u64>, pub query_target: Option<u64>, pub query_result: Option<u64>,
    pub delete_vtable: Option<u64>, pub delete_target: Option<u64>, pub rounded: Option<u64>, pub capacity: Option<u64>,
    pub allocation_bytes: Option<u64>, pub allocation_base: Option<u64>, pub prior_reference: Option<u32>,
    pub capacity_store_returned: bool, pub data: Option<u64>, pub output_store_returned: bool,
    pub local_store_returned: bool, pub copy_count_r8d: Option<u32>,
}
pub enum Fault<E> { Service(E), MissingState, ReturnBinding { site: CallSite }, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody,
    captures: Captures, saved: Option<Nonvolatile>, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualBody, pub captures: Captures,
    pub saved: Option<Nonvolatile>, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {

    pub fn services(&self) -> &S { &self.services }
    pub fn owners(&self) -> &Owners<S> { &self.owners }
}

pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1caf43 }
    pub fn saved_output_this(&self) -> u64 { self.parts.actual.output_rcx }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {

    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x1cae9f { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage,
            calls: [None, None, None, None, None, None, None] }, actual: input.actual, captures: Captures::default(),
            saved: None, phase: Phase::SaveBindings, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::NativeReturnRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::NativeReturnRequired { Poll::Ready(NativeReturnRequired { parts: self.into_parts() }) }
        else { Poll::Pending(self) }
    }
}
