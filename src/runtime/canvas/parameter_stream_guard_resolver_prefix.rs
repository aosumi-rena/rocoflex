#[path = "text_allocation_handler_lookup.rs"]
mod saved_word_source;

pub(in super::super::super::super) fn decode_saved_handler_word(cookie: u64, encoded: u64) -> u64 {
    saved_word_source::decode_saved_word(cookie, encoded)
}
#[path = "parameter_stream_guard_resolver_prefix_steps.rs"]
mod steps;
#[path = "parameter_stream_guard_resolver_return.rs"]
pub mod native_return;
#[path = "parameter_stream_guard_resolver_call.rs"]
pub mod selected_call;
#[path = "parameter_stream_guard_resolver_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_guard_resolver_protected.rs"]
pub mod protected_call;

pub use super::super::super::native_return::{Pop, StackAdjustment};

fn encode_word(cookie: u64, word: u64, rotation: u32) -> u64 { word.rotate_right(rotation) ^ cookie }
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64,
    pub ecx: u32, pub rdx: u64, pub r8: u64, pub r9: u64, pub entry_rax: u64 }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug)]
pub struct ExchangeReturned { pub address: u64, pub written: u64, pub prior: u64 }
pub trait Services: Sized {
    type Storage; type Error;

    fn read_word(&mut self, storage: &mut Self::Storage, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn read_dword(&mut self, storage: &mut Self::Storage, pc: u32, address: u64) -> Result<u32, Self::Error>;

    fn xchg_word(&mut self, storage: &mut Self::Storage, pc: u32, address: u64, word: u64,
        returned: &mut Option<ExchangeReturned>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnReason { CachedWord, DecodedUnavailable, Exhausted, ResolvedWord, ResolutionUnavailable }
#[derive(Clone, Copy, Debug)]
pub enum ProtectedRequirement {

    LoadBeforePush { next_pc: u32, call_pc: u32, target_rva: u32, work_rsp: u64,
        rcx: u64, edx: u32, r8d: u32, push_rax: u64, saved_r14: u64 },

    ResolveBeforeArguments { next_pc: u32, call_pc: u32, target_rva: u32, work_rsp: u64,
        saved_rbx: u64, saved_r13: u64, saved_r14: u64 },

    LiveResolveBeforeArguments { next_pc: u32, call_pc: u32, target_rva: u32, work_rsp: u64, live: protected_call::Registers },
    LivePriorNonzeroBeforeSetup { next_pc: u32, call_pc: u32, target_rva: u32, work_rsp: u64,
        live: protected_call::Registers, push_rbp: u64 },
    LiveZeroLoadCall { next_pc: u32, call_pc: u32, target_rva: u32, work_rsp: u64, live: protected_call::Registers },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Bind, Encoded, Cookie, Decode, CacheBranch, CompareRange, ReadIndex, ReadProvider,
    ProviderBranch, Advance, RefreshCookie, ReadName, PrepareLoad, PrepareResolve, PreparePublication, Publish,
    ZeroReturn, CachedReturn, NativeReturnRequired, ProtectedRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub selector: Option<u64>, pub cache_slot: Option<u64>, pub encoded: Option<u64>, pub cookie: Option<u64>,
    pub decoded: Option<u64>, pub cursor: Option<u64>, pub last_index: Option<u32>, pub last_provider: Option<u64>, pub last_name: Option<u64>,

    pub completed_skips: u64, pub refreshed_cookie: Option<u64>, pub publication_cookie: Option<u64>, pub source_rotation_count: Option<u32>,
    pub encoded_sentinel: Option<u64>, pub exchange: Option<ExchangeReturned>, pub rax: Option<u64>, pub reason: Option<ReturnReason>,
    pub protected: Option<ProtectedRequirement>, pub native_entry: Option<native_entry::History>,
    pub resolution: Option<protected_call::resolution::History>, pub load_publication: Option<protected_call::load_publication::History>,
    pub prior_cleanup: Option<protected_call::prior_cleanup::History> }
pub enum Fault<E> { Service(E), MissingState, ExchangeBinding, AlreadyStopped }
pub struct Prepared<S: Services> { input: Input<S>, captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Next<S>), Failed(Stopped<S>) }
pub enum Next<S: Services> { NativeReturn(NativeReturnRequired<S>), Protected(ProtectedRequired<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) storage: S::Storage, pub actual: ActualBody,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn storage(&self) -> &S::Storage { &self.storage } }
pub struct NativeReturnRequired<S: Services> { parts: Parts<S> }
pub struct ProtectedRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeReturnRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x58ef49 }

    pub fn required_rax(&self) -> u64 { self.parts.captures.rax.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> ProtectedRequired<S> {
    pub fn requirement(&self) -> ProtectedRequirement { self.parts.captures.protected.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x58ee20 { return Err(Rejected { input }); }
        Ok(Self { input, captures: Captures::default(), phase: Phase::Bind, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.input.services, storage: self.input.storage, actual: self.input.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); } let mut spent = 0;
        while self.phase != Phase::NativeReturnRequired && self.phase != Phase::ProtectedRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        match self.phase { Phase::NativeReturnRequired => Poll::Ready(Next::NativeReturn(NativeReturnRequired { parts: self.into_parts() })),
            Phase::ProtectedRequired => Poll::Ready(Next::Protected(ProtectedRequired { parts: self.into_parts() })), _ => Poll::Pending(self) }
    }
}
