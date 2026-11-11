pub use super::super::native_return::{Register, Pop};
pub trait Services: super::Services {
    fn pop_saved(&mut self, storage: &mut Self::Storage, pc: u32, register: Register,
        returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn ret_1fb416(&mut self, storage: &mut Self::Storage, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct Input { pub actual_body_rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn {
    pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64,

    pub restored: [u64; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { PopSaved, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub pops: [Option<Pop>; 4], pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub input: Input, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { body: super::Parts<S>, input: Input, captures: Captures,
    phase: Phase, pop: usize, attempted: Option<(Phase, usize)>, stopped: bool }
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

    pub fn from_completed_body(completed: super::NativeRestoreRequired<S>, input: Input) -> Self {
        Self { body: completed.into_parts(), input, captures: Captures::default(),
            phase: Phase::PopSaved, pop: 0, attempted: None, stopped: false }
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
            Phase::PopSaved => {
                let registers = [Register::Rbx, Register::Rbp, Register::Rdi, Register::Rsi];
                let pcs = [0x1fb412, 0x1fb413, 0x1fb414, 0x1fb415];
                if self.pop >= 4 { return Err(Fault::MissingReceipt); }
                self.body.services.pop_saved(&mut self.body.storage, pcs[self.pop], registers[self.pop], &mut self.captures.pops[self.pop]).map_err(Fault::Service)?;
                let returned = self.captures.pops[self.pop].ok_or(Fault::MissingReceipt)?;
                let before = self.input.actual_body_rsp.wrapping_add(self.pop as u64 * 8);
                if returned.before != before || returned.after != before.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.pop += 1; if self.pop == 4 { self.phase = Phase::Return; }
            }
            Phase::Return => {
                self.body.services.ret_1fb416(&mut self.body.storage, &mut self.captures.actual_return).map_err(Fault::Service)?;
                let returned = self.captures.actual_return.ok_or(Fault::MissingReceipt)?;
                let mut restored = [0; 4];
                for (index, slot) in restored.iter_mut().enumerate() { *slot = self.captures.pops[index].ok_or(Fault::MissingReceipt)?.value; }
                if returned.before_rsp != self.input.actual_body_rsp.wrapping_add(0x20)
                    || returned.after_rsp != self.input.actual_body_rsp.wrapping_add(0x28)
                    || Some(returned.rax) != self.body.captures.final_pointer || returned.restored != restored {
                    return Err(Fault::ReturnBinding);
                }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
