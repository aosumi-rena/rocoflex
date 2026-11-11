#[path = "parameter_number_format_constructor_calls.rs"]
mod calls;
#[path = "parameter_number_format_constructor_steps.rs"]
mod steps;
#[path = "parameter_number_format_constructor_return.rs"]
pub mod native_return;
#[path = "parameter_number_format_constructor_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_object_lookup.rs"]
pub mod object_lookup;
#[path = "parameter_stream_object_lookup_call.rs"]
pub mod object_lookup_call;
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;
    fn read_word(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn read_dword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn write_word(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn write_dword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn write_byte(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
    fn zero_xmm0(&mut self, o: &mut Owners<Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn write_xmm0(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;

    fn call(&mut self, o: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
}
pub type Calls<R> = [Option<CallReturned<R>>; 11];
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner> }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved { pub rbx: u64, pub rdi: u64, pub rsi: u64, pub r14: u64, pub rbp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct Arguments { pub rcx: Option<u64>, pub rdx: Option<u64>, pub r8: Option<u64>, pub cl: Option<u8>, pub dl: Option<u8> }
#[derive(Clone, Copy, Debug)]
pub enum Target { DirectRva(u32), LoadedIndirect(u64) }
#[derive(Clone, Copy, Debug)]
pub enum CallSite { Allocate, SetClOne, Acquire, GetObject, Translate, Query, Delete, Optional, ErrorConstruct, ErrorMessage, Throw }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::Allocate => 0, Self::SetClOne => 1, Self::Acquire => 2, Self::GetObject => 3,
        Self::Translate => 4, Self::Query => 5, Self::Delete => 6, Self::Optional => 7, Self::ErrorConstruct => 8, Self::ErrorMessage => 9, Self::Throw => 10 } }
    pub fn pc(self) -> u32 { match self { Self::Allocate => 0x1fbd33, Self::SetClOne => 0x1fbd3d, Self::Acquire => 0x1fbd60,
        Self::GetObject => 0x1fbd67, Self::Translate => 0x1fbd74, Self::Query => 0x1fbd86, Self::Delete => 0x1fbd99,
        Self::Optional => 0x1fbdc1, Self::ErrorConstruct => 0x1fbe00, Self::ErrorMessage => 0x1fbe12, Self::Throw => 0x1fbe21 } }
    pub fn resume_pc(self) -> u32 { match self { Self::Allocate => 0x1fbd38, Self::SetClOne => 0x1fbd42, Self::Acquire => 0x1fbd63,
        Self::GetObject => 0x1fbd6c, Self::Translate => 0x1fbd78, Self::Query => 0x1fbd89, Self::Delete => 0x1fbd9c,
        Self::Optional => 0x1fbdc6, Self::ErrorConstruct => 0x1fbe05, Self::ErrorMessage => 0x1fbe17, Self::Throw => 0x1fbe26 } }
}
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: Target, pub args: Arguments, pub work_rsp: u64, pub required_saved: Saved }
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub rbp: u64, pub object_rcx: u64,
    pub builder_rdx: u64, pub entry_r8: u64, pub entry_rbx: u64, pub entry_r14: u64 }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Bind, ZeroXmm0, InitialStores, Allocate, SetClOne, StoreAllocationLink, StoreAllocation,
    StoreBuilder, Zero50, SaveContext, ReadAcquireVtable, ReadAcquireTarget, Acquire, GetObject, ReadTranslateVtable,
    ReadTranslateTarget, Translate, ReadCleanupContext, ReadQueryVtable, ReadQueryTarget, Query, ReadDeleteVtable,
    ReadDeleteTarget, Delete, StoreByte58, ReadCurrentBuilder, ReadStatus, ComposeStatus, StoreStatus, ReadMask,
    MaskStatus, OptionalBranch, Optional, SelectMessage, ErrorConstruct, BindErrorObject, ErrorMessage, Throw,
    NativeReturnRequired, BreakpointRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub object_lookup_call: Option<object_lookup_call::History>, pub requests: [Option<CallRequest>; 11], pub completed: [bool; 11], pub bound_rbx: Option<u64>,
    pub xmm0: Option<[u32; 4]>, pub initial_stores: [bool; 5], pub allocation: Option<u64>, pub context: Option<u64>,
    pub allocation_link_stored: bool, pub allocation_stored: bool, pub builder_stored: bool, pub zero50_stored: bool, pub context_saved: bool,
    pub acquire_vtable: Option<u64>, pub acquire_target: Option<u64>, pub get_object: Option<u64>, pub translate_vtable: Option<u64>,
    pub translate_target: Option<u64>, pub translated_rax: Option<u64>, pub cleanup_context: Option<u64>, pub query_vtable: Option<u64>,
    pub query_target: Option<u64>, pub query_result: Option<u64>, pub delete_vtable: Option<u64>, pub delete_target: Option<u64>,
    pub byte58_stored: bool, pub current_builder: Option<u64>, pub status: Option<u32>, pub composed: Option<u32>, pub status_stored: bool,
    pub mask: Option<u32>, pub masked: Option<u32>, pub selected_rdx: Option<u64>, pub selected_message: Option<u64>, pub rax: Option<u64> }
pub enum Fault<E> { Service(E), MissingState, XmmBinding, OccupiedCall { site: CallSite }, CallBinding { site: CallSite }, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    saved: Option<Saved>, phase: Phase, index: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Next<S>), Failed(Stopped<S>) }
pub enum Next<S: Services> { Normal(NativeReturnRequired<S>), Breakpoint(BreakpointRequired<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) owners: Owners<S>, pub actual: ActualBody, pub captures: Captures,
    pub saved: Option<Saved>, pub phase: Phase, pub index: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
pub struct BreakpointRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x1fbdc6 }
    pub fn required_rax(&self) -> u64 { self.parts.captures.rax.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> BreakpointRequired<S> {

    pub fn required_next_pc(&self) -> u32 { 0x1fbe26 }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x1fbd07 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage,
            calls: [None, None, None, None, None, None, None, None, None, None, None] }, actual: input.actual,
            captures: Captures::default(), saved: None, phase: Phase::Bind, index: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, saved: self.saved, phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeReturnRequired && self.phase != Phase::BreakpointRequired && spent < budget {
            self.attempted = Some((self.phase, self.index)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        match self.phase { Phase::NativeReturnRequired => Poll::Ready(Next::Normal(NativeReturnRequired { parts: self.into_parts() })),
            Phase::BreakpointRequired => Poll::Ready(Next::Breakpoint(BreakpointRequired { parts: self.into_parts() })), _ => Poll::Pending(self) }
    }
}
