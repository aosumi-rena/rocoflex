use super::ProtectedRequirement;
#[path = "parameter_stream_guard_resolver_resolution.rs"]
pub mod resolution;
#[path = "parameter_stream_guard_resolver_load_publication.rs"]
pub mod load_publication;
#[path = "parameter_stream_guard_resolver_prior_cleanup.rs"]
pub mod prior_cleanup;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registers { pub rax: u64, pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64,
    pub rbx: u64, pub r13: u64, pub r14: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualBoundary { pub pc: u32, pub rsp: u64, pub registers: Registers }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation { MoveRdxR13, MoveRcxRbx, PushRax, PushRcx, PushRbp { word: u64 } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stored { pub address: u64, pub word: u64 }
#[derive(Clone, Copy, Debug)]
pub struct InstructionRequest { pub pc: u32, pub next_pc: u32, pub operation: Operation,
    pub before_rsp: u64, pub after_rsp: u64, pub before: Registers, pub after: Registers, pub stored: Option<Stored> }
#[derive(Clone, Copy, Debug)]
pub struct InstructionReturned { pub pc: u32, pub next_pc: u32, pub rsp: u64, pub registers: Registers, pub stored: Option<Stored> }
#[derive(Clone, Copy, Debug)]
pub struct CallRequest { pub pc: u32, pub target_rva: u32, pub target: u64, pub rsp_before_call: u64, pub registers: Registers }

#[derive(Clone, Copy, Debug)]
pub struct ActualExit { pub pc: u64, pub rsp: u64, pub registers: Registers }
pub struct Exit<R> { pub actual: ActualExit, pub owner: R }
pub trait Services: super::Services {
    type ProtectedOwner;

    fn setup_instruction(&mut self, storage: &mut Self::Storage, request: InstructionRequest,
        returned: &mut Option<InstructionReturned>) -> Result<(), Self::Error>;

    fn protected_call(&mut self, storage: &mut Self::Storage, request: CallRequest,
        returned: &mut Option<Exit<Self::ProtectedOwner>>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Setup, ProtectedCall, ContinuationRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub requests: [Option<InstructionRequest>; 3], pub instructions: [Option<InstructionReturned>; 3], pub call: Option<CallRequest> }
pub enum Fault<E> { Service(E), MissingReceipt, InstructionBinding, OccupiedExit, WrongPhase, AlreadyStopped }
pub struct Prepared<S: Services> { prefix: super::Parts<S>, actual: ActualBoundary, requirement: ProtectedRequirement,
    captures: Captures, registers: Registers, rsp: u64, index: usize, phase: Phase,
    attempted: Option<(Phase, usize)>, exit: Option<Exit<S::ProtectedOwner>>, stopped: bool }
pub struct Rejected<S: Services> { pub completed: super::ProtectedRequired<S>, pub actual: ActualBoundary }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(ContinuationRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) prefix: super::Parts<S>, pub(super) exit: Option<Exit<S::ProtectedOwner>>,
    pub actual: ActualBoundary, pub requirement: ProtectedRequirement, pub captures: Captures,

    pub registers: Registers, pub rsp: u64, pub index: usize, pub phase: Phase, pub attempted: Option<(Phase, usize)>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn prefix(&self) -> &super::Parts<S> { &self.prefix }
    pub fn exit(&self) -> Option<&Exit<S::ProtectedOwner>> { self.exit.as_ref() }
}

pub struct ContinuationRequired<S: Services> { parts: Parts<S> }
impl<S: Services> ContinuationRequired<S> {
    pub fn actual_exit(&self) -> &ActualExit { &self.parts.exit.as_ref().unwrap().actual }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_prefix(completed: super::ProtectedRequired<S>, actual: ActualBoundary) -> Result<Self, Rejected<S>> {
        let requirement = completed.requirement(); let a = completed.parts.actual;
        let (pc, rsp, registers) = match requirement {
            ProtectedRequirement::LoadBeforePush { next_pc, work_rsp, rcx, edx, r8d, push_rax, saved_r14, .. } =>
                (next_pc, work_rsp, Some(Registers { rax: push_rax, rcx, rdx: u64::from(edx), r8: u64::from(r8d), r9: a.r9,
                    rbx: 0, r13: a.rdx, r14: saved_r14 })),
            ProtectedRequirement::ResolveBeforeArguments { next_pc, work_rsp, saved_rbx, saved_r13, saved_r14, .. } =>
                (next_pc, work_rsp, completed.parts.captures.cookie.map(|cookie| Registers { rax: a.entry_rax,
                    rcx: u64::from((cookie as u32) & 0x3f), rdx: a.rdx, r8: a.r8, r9: a.r9,
                    rbx: saved_rbx, r13: saved_r13, r14: saved_r14 })),
            ProtectedRequirement::LiveResolveBeforeArguments { next_pc, work_rsp, live, .. }
                | ProtectedRequirement::LivePriorNonzeroBeforeSetup { next_pc, work_rsp, live, .. }
                | ProtectedRequirement::LiveZeroLoadCall { next_pc, work_rsp, live, .. } => (next_pc, work_rsp, Some(live)),
        };
        if actual.pc != pc || actual.rsp != rsp || registers != Some(actual.registers) { return Err(Rejected { completed, actual }); }
        let phase = if matches!(requirement, ProtectedRequirement::LiveZeroLoadCall { .. }) { Phase::ProtectedCall } else { Phase::Setup };
        Ok(Self { prefix: completed.into_parts(), actual, requirement, captures: Captures::default(), registers: actual.registers,
            rsp: actual.rsp, index: 0, phase, attempted: None, exit: None, stopped: false })
    }
    pub fn into_parts(self) -> Parts<S> { Parts { prefix: self.prefix, exit: self.exit, actual: self.actual, requirement: self.requirement,
        captures: self.captures, registers: self.registers, rsp: self.rsp, index: self.index, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::ContinuationRequired && spent < budget {
            self.attempted = Some((self.phase, self.index)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase == Phase::ContinuationRequired { Poll::Ready(ContinuationRequired { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::Setup => {
                let (pc, next_pc, operation) = match self.requirement {
                    ProtectedRequirement::LoadBeforePush { .. } => (0x58eea4, 0x58eea5, Operation::PushRax),
                    ProtectedRequirement::ResolveBeforeArguments { .. } | ProtectedRequirement::LiveResolveBeforeArguments { .. } => [(0x58ef86, 0x58ef89, Operation::MoveRdxR13),
                        (0x58ef89, 0x58ef8c, Operation::MoveRcxRbx), (0x58ef8c, 0x58ef8d, Operation::PushRcx)][self.index],
                    ProtectedRequirement::LivePriorNonzeroBeforeSetup { push_rbp, .. } =>
                        if self.index == 0 { (0x58ef7d, 0x58ef80, Operation::MoveRcxRbx) }
                        else { (0x58ef80, 0x58ef81, Operation::PushRbp { word: push_rbp }) },
                    ProtectedRequirement::LiveZeroLoadCall { .. } => return Err(Fault::WrongPhase),
                };
                let mut after = self.registers; let mut after_rsp = self.rsp;
                let stored = match operation {
                    Operation::MoveRdxR13 => { after.rdx = self.registers.r13; None },
                    Operation::MoveRcxRbx => { after.rcx = self.registers.rbx; None },
                    Operation::PushRax => { after_rsp = after_rsp.wrapping_sub(8); Some(Stored { address: after_rsp, word: self.registers.rax }) },
                    Operation::PushRcx => { after_rsp = after_rsp.wrapping_sub(8); Some(Stored { address: after_rsp, word: self.registers.rcx }) },
                    Operation::PushRbp { word } => { after_rsp = after_rsp.wrapping_sub(8); Some(Stored { address: after_rsp, word }) },
                };
                let request = InstructionRequest { pc, next_pc, operation, before_rsp: self.rsp, after_rsp, before: self.registers, after, stored };
                self.captures.requests[self.index] = Some(request);
                self.prefix.services.setup_instruction(&mut self.prefix.storage, request, &mut self.captures.instructions[self.index]).map_err(Fault::Service)?;
                let b = self.captures.instructions[self.index].ok_or(Fault::MissingReceipt)?;
                if b.pc != pc || b.next_pc != next_pc || b.rsp != after_rsp || b.registers != after || b.stored != stored { return Err(Fault::InstructionBinding); }
                self.registers = b.registers; self.rsp = b.rsp; self.index += 1;
                if matches!(operation, Operation::PushRax | Operation::PushRcx | Operation::PushRbp { .. }) { self.phase = Phase::ProtectedCall; }
            }
            Phase::ProtectedCall => {
                if self.exit.is_some() { return Err(Fault::OccupiedExit); }
                let (pc, target_rva) = match self.requirement {
                    ProtectedRequirement::LoadBeforePush { call_pc, target_rva, .. } | ProtectedRequirement::ResolveBeforeArguments { call_pc, target_rva, .. }
                        | ProtectedRequirement::LiveResolveBeforeArguments { call_pc, target_rva, .. }
                        | ProtectedRequirement::LivePriorNonzeroBeforeSetup { call_pc, target_rva, .. }
                        | ProtectedRequirement::LiveZeroLoadCall { call_pc, target_rva, .. } => (call_pc, target_rva),
                };
                let request = CallRequest { pc, target_rva, target: self.prefix.actual.source_image_base.wrapping_add(u64::from(target_rva)),
                    rsp_before_call: self.rsp, registers: self.registers };
                self.captures.call = Some(request);
                self.prefix.services.protected_call(&mut self.prefix.storage, request, &mut self.exit).map_err(Fault::Service)?;
                if self.exit.is_none() { return Err(Fault::MissingReceipt); } self.phase = Phase::ContinuationRequired;
            }
            Phase::ContinuationRequired => {}
        }
        Ok(())
    }
}
