use super as p;
use super::super as r;
#[derive(Clone, Copy, Debug)]
pub struct ActualContinuation { pub pc: u32, pub rsp: u64,

    pub r15: u64, pub rdi: u64 }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub raw_rax: Option<u64>, pub cookie_pc: Option<u32>, pub cookie: Option<u64>,
    pub rotation: Option<u32>, pub publication_word: Option<u64>, pub encoded: Option<u64>,
    pub address: Option<u64>, pub exchange_pc: Option<u32>, pub exchange: Option<r::ExchangeReturned> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualContinuation, pub requirement: r::ProtectedRequirement,
    pub protected_setup: p::Captures, pub actual_exit: p::ActualExit, pub captures: Captures }
pub trait Services: p::Services {

    fn retain_resolution_exit(&mut self, storage: &mut Self::Storage, history: History,
        held: &mut Option<p::Exit<Self::ProtectedOwner>>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Branch, Cookie, Encode, Publish, RetainExit, Done }
pub enum Fault<E> { Service(E), MissingState, ExchangeBinding, ExitNotRetained, AlreadyStopped }
pub struct Prepared<S: Services> { protected: p::Parts<S>, actual: ActualContinuation, captures: Captures,
    phase: Phase, attempted: Option<Phase>, history: Option<History>, stopped: bool }
pub struct Rejected<S: Services> { pub completed: p::ContinuationRequired<S>, pub actual: ActualContinuation }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(r::NativeReturnRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) protected: p::Parts<S>, pub actual: ActualContinuation,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub history: Option<History>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn protected(&self) -> &p::Parts<S> { &self.protected } }
impl<S: Services> Prepared<S> {

    pub fn from_actual_resolve_exit(completed: p::ContinuationRequired<S>, actual: ActualContinuation) -> Result<Self, Rejected<S>> {
        let prefix = &completed.parts.prefix; let exit = completed.actual_exit();
        if actual.pc != 0x58ef92 || !matches!(completed.parts.requirement, r::ProtectedRequirement::ResolveBeforeArguments { .. } | r::ProtectedRequirement::LiveResolveBeforeArguments { .. })
            || exit.pc != prefix.actual.source_image_base.wrapping_add(u64::from(actual.pc))
            || actual.rsp != prefix.actual.work_rsp || exit.rsp != actual.rsp { return Err(Rejected { completed, actual }); }
        Ok(Self { protected: completed.into_parts(), actual, captures: Captures::default(), phase: Phase::Branch,
            attempted: None, history: None, stopped: false })
    }
    pub fn into_parts(self) -> Parts<S> { Parts { protected: self.protected, actual: self.actual, captures: self.captures,
        phase: self.phase, attempted: self.attempted, history: self.history, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase != Phase::Done { return Poll::Pending(self); }

        let history = self.history.unwrap(); let mut prefix = self.protected.prefix;
        prefix.captures.rax = self.captures.raw_rax;
        prefix.captures.reason = Some(if history.actual_exit.registers.rax == 0 { r::ReturnReason::ResolutionUnavailable } else { r::ReturnReason::ResolvedWord });
        prefix.captures.resolution = Some(history); prefix.phase = r::Phase::NativeReturnRequired; prefix.attempted = None; prefix.stopped = false;
        Poll::Ready(r::NativeReturnRequired { parts: prefix })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let prefix = &mut self.protected.prefix;
        match self.phase {
            Phase::Branch => {
                let exit = self.protected.exit.as_ref().ok_or(Fault::MissingState)?.actual;
                let raw = exit.registers.rax; self.captures.raw_rax = Some(raw);
                self.captures.cookie_pc = Some(if raw == 0 { 0x58ef25 } else { 0x58ef97 });
                self.captures.exchange_pc = Some(if raw == 0 { 0x58ef3f } else { 0x58efb6 });

                self.captures.publication_word = Some(if raw == 0 { self.actual.rdi } else { raw });
                self.captures.address = Some(exit.registers.r14.wrapping_add(self.actual.r15.wrapping_mul(8)).wrapping_add(0x861700));
                self.phase = Phase::Cookie;
            }
            Phase::Cookie => {
                let pc = self.captures.cookie_pc.ok_or(Fault::MissingState)?;
                self.captures.cookie = Some(prefix.services.read_word(&mut prefix.storage, pc,
                    prefix.actual.source_image_base.wrapping_add(0x820dd8)).map_err(Fault::Service)?); self.phase = Phase::Encode;
            }
            Phase::Encode => {
                let cookie = self.captures.cookie.ok_or(Fault::MissingState)?;
                let rotation = 64u32.wrapping_sub((cookie as u32) & 0x3f); self.captures.rotation = Some(rotation);
                self.captures.encoded = Some(r::encode_word(cookie, self.captures.publication_word.ok_or(Fault::MissingState)?, rotation)); self.phase = Phase::Publish;
            }
            Phase::Publish => {
                let pc = self.captures.exchange_pc.ok_or(Fault::MissingState)?; let address = self.captures.address.ok_or(Fault::MissingState)?;
                let word = self.captures.encoded.ok_or(Fault::MissingState)?;
                prefix.services.xchg_word(&mut prefix.storage, pc, address, word, &mut self.captures.exchange).map_err(Fault::Service)?;
                let a = self.captures.exchange.ok_or(Fault::MissingState)?;
                if a.address != address || a.written != word { return Err(Fault::ExchangeBinding); } self.phase = Phase::RetainExit;
            }
            Phase::RetainExit => {
                let actual_exit = self.protected.exit.as_ref().ok_or(Fault::MissingState)?.actual;
                let history = History { actual: self.actual, requirement: self.protected.requirement,
                    protected_setup: self.protected.captures, actual_exit, captures: self.captures }; self.history = Some(history);
                prefix.services.retain_resolution_exit(&mut prefix.storage, history, &mut self.protected.exit).map_err(Fault::Service)?;
                if self.protected.exit.is_some() { return Err(Fault::ExitNotRetained); } self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
