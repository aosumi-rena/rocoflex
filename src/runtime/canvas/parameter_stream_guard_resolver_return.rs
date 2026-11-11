pub use super::{Pop, StackAdjustment};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register { Rbx, Rbp, Rsi, R15, R14, R13, R12, Rdi }
#[derive(Clone, Copy, Debug)]
pub struct Restored { pub pc: u32, pub register: Register, pub address: u64, pub word: u64, pub rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn { pub entry_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64,

    pub restored: [u64; 8] }
pub trait Services: super::Services {

    fn restore_word(&mut self, storage: &mut Self::Storage, pc: u32, register: Register, address: u64,
        returned: &mut Option<Restored>) -> Result<(), Self::Error>;
    fn add_rsp20(&mut self, storage: &mut Self::Storage, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn pop(&mut self, storage: &mut Self::Storage, pc: u32, register: Register, returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn ret(&mut self, storage: &mut Self::Storage, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Restore, AddStack, Pop, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub restores: [Option<Restored>; 3], pub adjustment: Option<StackAdjustment>, pub pops: [Option<Pop>; 5], pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub work_rsp: u64, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, RestoreBinding, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { body: super::Parts<S>, captures: Captures, phase: Phase, index: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Parts<S: Services> { pub body: super::Parts<S>, pub captures: Captures, pub phase: Phase, pub index: usize,
    pub attempted: Option<(Phase, usize)>, pub stopped: bool }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn history(&self) -> History { History { work_rsp: self.parts.body.actual.work_rsp, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_prefix(completed: super::NativeReturnRequired<S>) -> Self {
        Self { body: completed.into_parts(), captures: Captures::default(), phase: Phase::Restore, index: 0, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { body: self.body, captures: self.captures, phase: self.phase,
        index: self.index, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); } let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some((self.phase, self.index)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase == Phase::Done { Poll::Ready(Returned { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let work = self.body.actual.work_rsp; let s = &mut self.body.services; let o = &mut self.body.storage; let c = &mut self.captures;
        match self.phase {
            Phase::Restore => {
                let (pc, register, offset) = [(0x58ef49, Register::Rbx, 0x50u64), (0x58ef4e, Register::Rbp, 0x58), (0x58ef53, Register::Rsi, 0x60)][self.index];
                let address = work.wrapping_add(offset);
                s.restore_word(o, pc, register, address, &mut c.restores[self.index]).map_err(Fault::Service)?;
                let a = c.restores[self.index].ok_or(Fault::MissingReceipt)?;
                if a.pc != pc || a.register != register || a.address != address || a.rsp != work { return Err(Fault::RestoreBinding); }
                self.index += 1; if self.index == 3 { self.index = 0; self.phase = Phase::AddStack; }
            }
            Phase::AddStack => {
                s.add_rsp20(o, &mut c.adjustment).map_err(Fault::Service)?; let a = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != work || a.after != work.wrapping_add(0x20) { return Err(Fault::StackBinding); } self.phase = Phase::Pop;
            }
            Phase::Pop => {
                let (pc, register) = [(0x58ef5c, Register::R15), (0x58ef5e, Register::R14), (0x58ef60, Register::R13), (0x58ef62, Register::R12), (0x58ef64, Register::Rdi)][self.index];
                s.pop(o, pc, register, &mut c.pops[self.index]).map_err(Fault::Service)?; let a = c.pops[self.index].ok_or(Fault::MissingReceipt)?;
                let before = work.wrapping_add(0x20).wrapping_add((self.index as u64).wrapping_mul(8));
                if a.before != before || a.after != before.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.index += 1; if self.index == 5 { self.index = 0; self.phase = Phase::Return; }
            }
            Phase::Return => {
                s.ret(o, &mut c.actual_return).map_err(Fault::Service)?; let a = c.actual_return.ok_or(Fault::MissingReceipt)?;
                let values = [c.restores[0].ok_or(Fault::MissingReceipt)?.word, c.restores[1].ok_or(Fault::MissingReceipt)?.word,
                    c.restores[2].ok_or(Fault::MissingReceipt)?.word, c.pops[0].ok_or(Fault::MissingReceipt)?.value,
                    c.pops[1].ok_or(Fault::MissingReceipt)?.value, c.pops[2].ok_or(Fault::MissingReceipt)?.value,
                    c.pops[3].ok_or(Fault::MissingReceipt)?.value, c.pops[4].ok_or(Fault::MissingReceipt)?.value];
                if a.entry_rsp != work.wrapping_add(0x48) || a.after_rsp != work.wrapping_add(0x50)
                    || Some(a.rax) != self.body.captures.rax || a.restored != values { return Err(Fault::ReturnBinding); }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
