pub use super::super::super::native_return::{Register, Pop, StackAdjustment};
pub trait Services: super::Services {
    fn nop_1fbdc6(&mut self, o: &mut super::Owners<Self>, next_pc: &mut Option<u32>) -> Result<(), Self::Error>;
    fn add_rsp60(&mut self, o: &mut super::Owners<Self>, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn pop_saved(&mut self, o: &mut super::Owners<Self>, pc: u32, register: Register, returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn ret_1fbdd1(&mut self, o: &mut super::Owners<Self>, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct Input { pub actual_work_rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn { pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64,

    pub restored: [u64; 5] }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Nop, AdjustStack, PopSaved, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub nop_next_pc: Option<u32>, pub adjustment: Option<StackAdjustment>, pub pops: [Option<Pop>; 5], pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub input: Input, pub required_rax: u64, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, InstructionBinding, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { body: super::Parts<S>, input: Input, required_rax: u64, captures: Captures,
    phase: Phase, pop: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub completed: super::NativeReturnRequired<S>, pub input: Input }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub body: super::Parts<S>, pub input: Input, pub required_rax: u64, pub captures: Captures,
    pub phase: Phase, pub pop: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool }
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn history(&self) -> History { History { input: self.parts.input, required_rax: self.parts.required_rax, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_body(completed: super::NativeReturnRequired<S>, input: Input) -> Result<Self, Rejected<S>> {
        if input.actual_work_rsp != completed.parts.actual.rbp.wrapping_sub(0x60) { return Err(Rejected { completed, input }); }
        let required_rax = completed.required_rax();
        Ok(Self { body: completed.into_parts(), input, required_rax, captures: Captures::default(), phase: Phase::Nop, pop: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { body: self.body, input: self.input, required_rax: self.required_rax,
        captures: self.captures, phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped } }
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
        let s = &mut self.body.services; let o = &mut self.body.owners; let c = &mut self.captures;
        match self.phase {
            Phase::Nop => { s.nop_1fbdc6(o, &mut c.nop_next_pc).map_err(Fault::Service)?; if c.nop_next_pc.ok_or(Fault::MissingReceipt)? != 0x1fbdc7 { return Err(Fault::InstructionBinding); } self.phase = Phase::AdjustStack; }
            Phase::AdjustStack => {
                s.add_rsp60(o, &mut c.adjustment).map_err(Fault::Service)?; let a = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != self.input.actual_work_rsp || a.after != a.before.wrapping_add(0x60) { return Err(Fault::StackBinding); } self.phase = Phase::PopSaved;
            }
            Phase::PopSaved => {
                let registers = [Register::Rbx, Register::Rdi, Register::Rsi, Register::R14, Register::Rbp]; let pcs = [0x1fbdcb, 0x1fbdcc, 0x1fbdcd, 0x1fbdce, 0x1fbdd0];
                if self.pop >= 5 { return Err(Fault::MissingReceipt); }
                s.pop_saved(o, pcs[self.pop], registers[self.pop], &mut c.pops[self.pop]).map_err(Fault::Service)?; let a = c.pops[self.pop].ok_or(Fault::MissingReceipt)?;
                let before = self.input.actual_work_rsp.wrapping_add(0x60 + self.pop as u64 * 8);
                if a.before != before || a.after != before.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.pop += 1; if self.pop == 5 { self.phase = Phase::Return; }
            }
            Phase::Return => {
                s.ret_1fbdd1(o, &mut c.actual_return).map_err(Fault::Service)?; let a = c.actual_return.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.input.actual_work_rsp.wrapping_add(0x88) || a.after_rsp != self.input.actual_work_rsp.wrapping_add(0x90) || a.rax != self.required_rax { return Err(Fault::ReturnBinding); }
                for i in 0..5 { if a.restored[i] != c.pops[i].ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); } } self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
