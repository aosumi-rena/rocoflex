#[path = "parameter_stream_member_handler_steps.rs"]
mod steps;
#[path = "parameter_stream_member_handler_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_member_handler_return.rs"]
pub mod native_return;
#[path = "parameter_stream_member_encoded_slot.rs"]
pub mod encoded_slot;
#[path = "parameter_stream_member_encoded_slot_call.rs"]
pub mod encoded_slot_call;
#[path = "parameter_stream_member_handler_context.rs"]
pub mod context;
#[path = "parameter_stream_member_handler_context_call.rs"]
pub mod context_call;
use super::super::super::super::super::super::guard::native_tails::initialization::resolver_prefix::decode_saved_handler_word;
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub source_image_base: u64, pub work_rsp: u64,
    pub rax: u64, pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved { pub rbx: u64, pub rbp: u64, pub rsi: u64, pub rdi: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallSite { Context, EncodedSlot, Indirect, Diagnostic }
impl CallSite {
    pub fn index(self) -> usize { match self { Self::Context => 0, Self::EncodedSlot => 1, Self::Indirect => 2, Self::Diagnostic => 3 } }
    pub fn pc(self) -> u32 { match self { Self::Context => 0x57a225, Self::EncodedSlot => 0x57a282, Self::Indirect => 0x57a25b, Self::Diagnostic => 0x57a2c6 } }
    pub fn resume(self) -> u32 { self.pc() + if self == Self::Indirect { 6 } else { 5 } }
}
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub site: CallSite, pub target: u64, pub work_rsp: u64,
    pub rcx: u64, pub rdx: Option<u64>, pub r8: Option<u64>, pub r9: Option<u64>,
    pub required_rax: u64, pub required_r10: Option<u64>, pub saved: Saved,
    pub stack_arg5: Option<u64>, pub loaded_target_slot: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct ActualCallReturn { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64, pub saved: Saved }
pub struct CallReturned<R> { pub actual: ActualCallReturn, pub owner: R }
pub type Calls<R> = [Option<CallReturned<R>>; 4];
pub struct Owners<S: Services> { pub storage: S::Storage, pub calls: Calls<S::CallOwner> }
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;

    fn read_qword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn write_qword(&mut self, o: &mut Owners<Self>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;

    fn call(&mut self, o: &mut Owners<Self>, request: CallRequest) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { BindSaved, ReadSixth, BindRemaining, ContextCall, TestContext, ReadContextWord, TestContextWord,
    ReadSixthAgain, SlotCall, ReadEncoded, ReadCookie, Decode, BranchDecoded,
    ReadFifthRcx, StoreFifthRcx, LoadIndirectTarget, IndirectCall,
    ReadFifthRax, StoreFifthRax, DiagnosticCall, NativeRestoreRequired, BreakpointRequired }
#[derive(Clone, Copy, Debug)]
pub struct MemoryAttempt { pub pc: u32, pub address: u64, pub written: Option<u64> }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub native_entry: Option<native_entry::History>, pub encoded_slot_call: Option<encoded_slot_call::History>,
    pub context_call: Option<context_call::History>,
    pub saved: Option<Saved>, pub bound_rbp: Option<u64>, pub bound_rbx: Option<u64>,
    pub rax: Option<u64>, pub sixth: Option<u64>, pub sixth_again: Option<u64>,
    pub context: Option<u64>, pub context_word: Option<u64>, pub encoded: Option<u64>, pub cookie: Option<u64>,
    pub decoded: Option<u64>, pub fifth: Option<u64>, pub indirect_target: Option<u64>, pub memory: Option<MemoryAttempt>,
    pub requests: [Option<CallRequest>; 4], pub completed: [bool; 4] }
pub enum Fault<E> { Service(E), MissingState, OccupiedCall, CallBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualBody, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeRestoreRequired<S>), Breakpoint(BreakpointRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>,
    pub actual: ActualBody, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct NativeRestoreRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeRestoreRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x57a261 }
    pub fn raw_rax(&self) -> u64 { self.parts.captures.rax.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
pub struct BreakpointRequired<S: Services> { parts: Parts<S> }
impl<S: Services> BreakpointRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x57a2cb }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x57a214 { return Err(Rejected { input }); }
        let mut captures = Captures::default(); captures.rax = Some(input.actual.rax);
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, calls: [None, None, None, None] },
            actual: input.actual, captures, phase: Phase::BindSaved, attempted: None, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::NativeRestoreRequired && self.phase != Phase::BreakpointRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        match self.phase { Phase::NativeRestoreRequired => Poll::Ready(NativeRestoreRequired { parts: self.into_parts() }),
            Phase::BreakpointRequired => Poll::Breakpoint(BreakpointRequired { parts: self.into_parts() }), _ => Poll::Pending(self) }
    }
}
