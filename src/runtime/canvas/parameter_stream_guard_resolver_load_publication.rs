use super as p;
use super::super as r;
#[path = "parameter_stream_guard_resolver_load_chain.rs"]
pub mod next_call;
#[derive(Clone, Copy, Debug)]
pub struct ActualContinuation { pub pc: u32, pub rsp: u64,

    pub rsi: u64, pub rbp: u64 }
#[derive(Clone, Copy, Debug)]
pub enum Requirement {

    ZeroLoadCall { pc: u32, target_rva: u32, target: u64, rsp: u64, registers: p::Registers },

    PriorNonzeroBeforeSetup { next_pc: u32, call_pc: u32, target_rva: u32, target: u64,
        rsp: u64, registers: p::Registers, push_rbp: u64 },

    PriorZeroBeforeSetup { next_pc: u32, call_pc: u32, target_rva: u32, target: u64,
        rsp: u64, registers: p::Registers },
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub loaded: Option<u64>, pub registers: Option<p::Registers>, pub provider_slot: Option<u64>,
    pub exchange: Option<r::ExchangeReturned>, pub requirement: Option<Requirement> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualContinuation, pub previous_requirement: r::ProtectedRequirement,
    pub previous_setup: p::Captures, pub actual_exit: p::ActualExit, pub captures: Captures, pub next_actual: p::ActualBoundary }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { SaveLoad, BranchLoad, PreparePublication, Publish, BranchPrior, Required }
pub enum Fault<E> { Service(E), MissingState, ExchangeBinding, AlreadyStopped }
pub struct Prepared<S: p::Services> { protected: p::Parts<S>, actual: ActualContinuation,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: p::Services> { pub completed: p::ContinuationRequired<S>, pub actual: ActualContinuation }
pub struct Stopped<S: p::Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: p::Services> { Pending(Prepared<S>), Ready(RequiredNext<S>), Failed(Stopped<S>) }
pub struct Parts<S: p::Services> { pub(in super::super) protected: p::Parts<S>, pub actual: ActualContinuation,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: p::Services> Parts<S> { pub fn protected(&self) -> &p::Parts<S> { &self.protected } }

pub struct RequiredNext<S: p::Services> { parts: Parts<S> }
impl<S: p::Services> RequiredNext<S> {
    pub fn requirement(&self) -> Requirement { self.parts.captures.requirement.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: p::Services> Prepared<S> {
    pub fn from_actual_load_exit(completed: p::ContinuationRequired<S>, actual: ActualContinuation) -> Result<Self, Rejected<S>> {
        let prefix = &completed.parts.prefix; let exit = completed.actual_exit();
        if actual.pc != 0x58eeaa || !matches!(completed.parts.requirement, r::ProtectedRequirement::LoadBeforePush { .. })
            || exit.pc != prefix.actual.source_image_base.wrapping_add(u64::from(actual.pc))
            || exit.rsp != actual.rsp || actual.rsp != prefix.actual.work_rsp { return Err(Rejected { completed, actual }); }
        Ok(Self { protected: completed.into_parts(), actual, captures: Captures::default(), phase: Phase::SaveLoad, attempted: None, stopped: false })
    }
    pub fn into_parts(self) -> Parts<S> { Parts { protected: self.protected, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::Required && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase == Phase::Required { Poll::Ready(RequiredNext { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let base = self.protected.prefix.actual.source_image_base;
        match self.phase {
            Phase::SaveLoad => {
                let mut regs = self.protected.exit.as_ref().ok_or(Fault::MissingState)?.actual.registers;
                self.captures.loaded = Some(regs.rax); regs.rbx = regs.rax; self.captures.registers = Some(regs); self.phase = Phase::BranchLoad;
            }
            Phase::BranchLoad => {
                let registers = self.captures.registers.ok_or(Fault::MissingState)?;
                if self.captures.loaded.ok_or(Fault::MissingState)? == 0 {
                    self.captures.requirement = Some(Requirement::ZeroLoadCall { pc: 0x58eeb6, target_rva: 0xbf59c7,
                        target: base.wrapping_add(0xbf59c7), rsp: self.actual.rsp, registers }); self.phase = Phase::Required;
                } else { self.phase = Phase::PreparePublication; }
            }
            Phase::PreparePublication => {
                let mut regs = self.captures.registers.ok_or(Fault::MissingState)?;
                regs.rax = regs.rbx; regs.r14 = base; self.captures.registers = Some(regs);

                self.captures.provider_slot = Some(base.wrapping_add(self.actual.rsi.wrapping_mul(8)).wrapping_add(0x861650));
                self.phase = Phase::Publish;
            }
            Phase::Publish => {
                let address = self.captures.provider_slot.ok_or(Fault::MissingState)?;
                let word = self.captures.registers.ok_or(Fault::MissingState)?.rax;
                let prefix = &mut self.protected.prefix;
                prefix.services.xchg_word(&mut prefix.storage, 0x58ef70, address, word, &mut self.captures.exchange).map_err(Fault::Service)?;
                let a = self.captures.exchange.ok_or(Fault::MissingState)?;
                if a.address != address || a.written != word { return Err(Fault::ExchangeBinding); }
                let mut regs = self.captures.registers.ok_or(Fault::MissingState)?; regs.rax = a.prior;
                self.captures.registers = Some(regs); self.phase = Phase::BranchPrior;
            }
            Phase::BranchPrior => {
                let registers = self.captures.registers.ok_or(Fault::MissingState)?;
                self.captures.requirement = Some(if registers.rax == 0 {
                    Requirement::PriorZeroBeforeSetup { next_pc: 0x58ef86, call_pc: 0x58ef8d, target_rva: 0xcc12e9,
                        target: base.wrapping_add(0xcc12e9), rsp: self.actual.rsp, registers }
                } else {
                    Requirement::PriorNonzeroBeforeSetup { next_pc: 0x58ef7d, call_pc: 0x58ef81, target_rva: 0xb6af5b,
                        target: base.wrapping_add(0xb6af5b), rsp: self.actual.rsp, registers, push_rbp: self.actual.rbp }
                }); self.phase = Phase::Required;
            }
            Phase::Required => {}
        }
        Ok(())
    }
}
