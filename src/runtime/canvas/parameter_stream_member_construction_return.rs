pub use super::super::native_return::{Pop, StackAdjustment};
#[derive(Clone, Copy, Debug)]
pub struct ActualBoundary { pub pc: u32, pub rsp: u64, pub rbx: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn { pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64, pub rbx: u64 }
pub trait Services: super::Services {

    fn mov_rax_rbx_5613a5(&mut self, owners: &mut super::Owners<Self>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn add_rsp_20_5613a8(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn pop_rbx_5613ac(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn ret_5613ad(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { MoveRax, AddStack, PopRbx, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub rax: Option<u64>, pub adjustment: Option<StackAdjustment>, pub pop: Option<Pop>, pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualBoundary, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, RaxBinding, StackBinding, ReturnBinding, AlreadyStopped }
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
        if actual.pc != 0x5613a5 || actual.rsp != completed.parts.actual.work_rsp || actual.rbx != completed.parts.actual.receiver_rcx {
            return Err(Rejected { completed, actual });
        }
        Ok(Self { body: completed.into_parts(), actual, captures: Captures::default(), phase: Phase::MoveRax, attempted: None, stopped: false })
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
        let services = &mut self.body.services; let owners = &mut self.body.owners;
        match self.phase {
            Phase::MoveRax => {
                services.mov_rax_rbx_5613a5(owners, self.actual.rbx, &mut self.captures.rax).map_err(Fault::Service)?;
                if self.captures.rax.ok_or(Fault::MissingReceipt)? != self.actual.rbx { return Err(Fault::RaxBinding); }
                self.phase = Phase::AddStack;
            }
            Phase::AddStack => {
                services.add_rsp_20_5613a8(owners, &mut self.captures.adjustment).map_err(Fault::Service)?;
                let a = self.captures.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != self.actual.rsp || a.after != self.actual.rsp.wrapping_add(0x20) { return Err(Fault::StackBinding); }
                self.phase = Phase::PopRbx;
            }
            Phase::PopRbx => {
                services.pop_rbx_5613ac(owners, &mut self.captures.pop).map_err(Fault::Service)?;
                let a = self.captures.pop.ok_or(Fault::MissingReceipt)?;
                if a.before != self.actual.rsp.wrapping_add(0x20) || a.after != self.actual.rsp.wrapping_add(0x28) { return Err(Fault::StackBinding); }
                self.phase = Phase::Return;
            }
            Phase::Return => {
                services.ret_5613ad(owners, &mut self.captures.actual_return).map_err(Fault::Service)?;
                let a = self.captures.actual_return.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.actual.rsp.wrapping_add(0x28) || a.after_rsp != self.actual.rsp.wrapping_add(0x30)
                    || Some(a.rax) != self.captures.rax || a.rbx != self.captures.pop.ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
