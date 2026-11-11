pub use super::super::native_return::{Register, Pop, StackAdjustment};
pub trait Services: super::Services {
    fn mov_rax_rbx(&mut self, owners: &mut super::Owners<Self>, pc: u32, saved_rbx: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn add_rsp38(&mut self, owners: &mut super::Owners<Self>, pc: u32, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn pop_saved(&mut self, owners: &mut super::Owners<Self>, pc: u32, register: Register, returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn ret_1fb4f2(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct Input { pub actual_work_rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn { pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64,

    pub restored: [u64; 4] }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { MoveRax, AdjustStack, PopSaved, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub rax: Option<u64>, pub adjustment: Option<StackAdjustment>,
    pub pops: [Option<Pop>; 4], pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub input: Input, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, RaxBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { body: super::Parts<S>, input: Input, captures: Captures,
    phase: Phase, pop: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub completed: super::NativeReturnRequired<S>, pub input: Input }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub body: super::Parts<S>, pub input: Input, pub captures: Captures,
    pub phase: Phase, pub pop: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool }
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn history(&self) -> History { History { input: self.parts.input, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_body(completed: super::NativeReturnRequired<S>, input: Input) -> Result<Self, Rejected<S>> {
        if input.actual_work_rsp != completed.parts.actual.rbp.wrapping_sub(0x30) { return Err(Rejected { completed, input }); }
        Ok(Self { body: completed.into_parts(), input, captures: Captures::default(), phase: Phase::MoveRax,
            pop: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { body: self.body, input: self.input, captures: self.captures,
        phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some((self.phase, self.pop)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::Done { Poll::Ready(Returned { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::MoveRax => {
                let word = self.body.saved.ok_or(Fault::MissingReceipt)?.rbx;
                self.body.services.mov_rax_rbx(&mut self.body.owners, 0x1fb4e7, word, &mut self.captures.rax).map_err(Fault::Service)?;
                if self.captures.rax.ok_or(Fault::MissingReceipt)? != word { return Err(Fault::RaxBinding); }
                self.phase = Phase::AdjustStack;
            }
            Phase::AdjustStack => {
                self.body.services.add_rsp38(&mut self.body.owners, 0x1fb4ea, &mut self.captures.adjustment).map_err(Fault::Service)?;
                let a = self.captures.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != self.input.actual_work_rsp || a.after != a.before.wrapping_add(0x38) { return Err(Fault::StackBinding); }
                self.phase = Phase::PopSaved;
            }
            Phase::PopSaved => {
                let registers = [Register::Rbx, Register::Rdi, Register::Rsi, Register::Rbp];
                let pcs = [0x1fb4ee, 0x1fb4ef, 0x1fb4f0, 0x1fb4f1];
                if self.pop >= 4 { return Err(Fault::MissingReceipt); }
                self.body.services.pop_saved(&mut self.body.owners, pcs[self.pop], registers[self.pop], &mut self.captures.pops[self.pop]).map_err(Fault::Service)?;
                let a = self.captures.pops[self.pop].ok_or(Fault::MissingReceipt)?;
                let before = self.input.actual_work_rsp.wrapping_add(0x38 + self.pop as u64 * 8);
                if a.before != before || a.after != before.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.pop += 1; if self.pop == 4 { self.phase = Phase::Return; }
            }
            Phase::Return => {
                self.body.services.ret_1fb4f2(&mut self.body.owners, &mut self.captures.actual_return).map_err(Fault::Service)?;
                let a = self.captures.actual_return.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.input.actual_work_rsp.wrapping_add(0x58)
                    || a.after_rsp != self.input.actual_work_rsp.wrapping_add(0x60)
                    || Some(a.rax) != self.captures.rax { return Err(Fault::ReturnBinding); }
                for i in 0..4 { if a.restored[i] != self.captures.pops[i].ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); } }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
