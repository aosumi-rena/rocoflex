pub use super::super::native_return::{Pop, StackAdjustment};
pub use super::native_entry::Register;
#[derive(Clone, Copy, Debug)]
pub struct Load { pub address: u64, pub word: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualBoundary { pub pc: u32, pub rsp: u64, pub rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn { pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64, pub saved: super::Saved }
pub trait Services: super::Services {
    fn restore_register(&mut self, owners: &mut super::Owners<Self>, pc: u32, register: Register, returned: &mut Option<Load>) -> Result<(), Self::Error>;
    fn add_rsp_30_57c4b7(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn pop_rdi_57c4bb(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn ret_57c4bc(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { RestoreRbx, RestoreRbp, RestoreRsi, AddStack, PopRdi, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub loads: [Option<Load>; 3], pub adjustment: Option<StackAdjustment>,
    pub pop: Option<Pop>, pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualBoundary, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, LoadBinding, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { body: super::Parts<S>, actual: ActualBoundary, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub completed: super::NativeReturnRequired<S>, pub actual: ActualBoundary }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub body: super::Parts<S>, pub actual: ActualBoundary, pub captures: Captures,
    pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_body(completed: super::NativeReturnRequired<S>, actual: ActualBoundary) -> Result<Self, Rejected<S>> {
        if actual.pc != 0x57c4a8 || actual.rsp != completed.parts.actual.work_rsp || actual.rax != completed.result() {
            return Err(Rejected { completed, actual });
        }
        Ok(Self { body: completed.into_parts(), actual, captures: Captures::default(), phase: Phase::RestoreRbx, attempted: None, stopped: false })
    }
    pub fn into_parts(self) -> Parts<S> { Parts { body: self.body, actual: self.actual, captures: self.captures,
        phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::Done { Poll::Ready(Returned { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::RestoreRbx | Phase::RestoreRbp | Phase::RestoreRsi => {
                let (index, register, pc, next) = match self.phase {
                    Phase::RestoreRbx => (0, Register::Rbx, 0x57c4a8, Phase::RestoreRbp),
                    Phase::RestoreRbp => (1, Register::Rbp, 0x57c4ad, Phase::RestoreRsi),
                    _ => (2, Register::Rsi, 0x57c4b2, Phase::AddStack),
                };
                self.body.services.restore_register(&mut self.body.owners, pc, register, &mut self.captures.loads[index]).map_err(Fault::Service)?;
                if self.captures.loads[index].ok_or(Fault::MissingReceipt)?.address != self.actual.rsp.wrapping_add(0x40 + (index as u64) * 8) { return Err(Fault::LoadBinding); }
                self.phase = next;
            }
            Phase::AddStack => {
                self.body.services.add_rsp_30_57c4b7(&mut self.body.owners, &mut self.captures.adjustment).map_err(Fault::Service)?;
                let a = self.captures.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != self.actual.rsp || a.after != self.actual.rsp.wrapping_add(0x30) { return Err(Fault::StackBinding); }
                self.phase = Phase::PopRdi;
            }
            Phase::PopRdi => {
                self.body.services.pop_rdi_57c4bb(&mut self.body.owners, &mut self.captures.pop).map_err(Fault::Service)?;
                let a = self.captures.pop.ok_or(Fault::MissingReceipt)?;
                if a.before != self.actual.rsp.wrapping_add(0x30) || a.after != self.actual.rsp.wrapping_add(0x38) { return Err(Fault::StackBinding); }
                self.phase = Phase::Return;
            }
            Phase::Return => {
                self.body.services.ret_57c4bc(&mut self.body.owners, &mut self.captures.actual_return).map_err(Fault::Service)?;
                let a = self.captures.actual_return.ok_or(Fault::MissingReceipt)?;
                let saved = super::Saved { rbx: self.captures.loads[0].ok_or(Fault::MissingReceipt)?.word,
                    rbp: self.captures.loads[1].ok_or(Fault::MissingReceipt)?.word,
                    rsi: self.captures.loads[2].ok_or(Fault::MissingReceipt)?.word,
                    rdi: self.captures.pop.ok_or(Fault::MissingReceipt)?.value };
                if a.before_rsp != self.actual.rsp.wrapping_add(0x38) || a.after_rsp != self.actual.rsp.wrapping_add(0x40)
                    || a.rax != self.actual.rax || a.saved != saved { return Err(Fault::ReturnBinding); }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
