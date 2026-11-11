use super::{NativeReturnRequired, Owners};

pub use super::super::super::super::super::super::retirement::native_return::{Pop, Register, StackAdjustment};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input { pub actual_work_rsp: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualReturn {
    pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64,

    pub restored: [u64; 4], pub xmm6: [u32; 4],
}
pub trait Services: super::Services {
    fn mov_rax_rsi(&mut self, owners: &mut Owners<Self>, pc: u32, saved_output: u64,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn restore_xmm6(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64,
        returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn add_rsp_d8(&mut self, owners: &mut Owners<Self>, pc: u32,
        returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn pop_saved(&mut self, owners: &mut Owners<Self>, pc: u32, destination: Register,
        returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn ret1caf55(&mut self, owners: &mut Owners<Self>, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { MoveRax, RestoreXmm6, AdjustStack, PopSaved, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub rax: Option<u64>, pub xmm6: Option<[u32; 4]>, pub adjustment: Option<StackAdjustment>,
    pub pops: [Option<Pop>; 4], pub actual_return: Option<ActualReturn>,
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub input: Input, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, RaxBinding, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { body: super::Parts<S>, input: Input, captures: Captures,
    phase: Phase, pop: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub completed: NativeReturnRequired<S>, pub input: Input }
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
    pub fn from_completed_body(completed: NativeReturnRequired<S>, input: Input) -> Result<Self, Rejected<S>> {
        if input.actual_work_rsp != completed.parts.actual.rbp.wrapping_sub(0x80) {
            return Err(Rejected { completed, input });
        }
        Ok(Self { body: completed.into_parts(), input, captures: Captures::default(), phase: Phase::MoveRax,
            pop: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { body: self.body, input: self.input, captures: self.captures,
        phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
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
            Phase::MoveRax => {
                let saved = self.body.actual.output_rcx;
                s.mov_rax_rsi(o, 0x1caf43, saved, &mut c.rax).map_err(Fault::Service)?;
                if c.rax.ok_or(Fault::MissingReceipt)? != saved { return Err(Fault::RaxBinding); }
                self.phase = Phase::RestoreXmm6;
            }
            Phase::RestoreXmm6 => {
                s.restore_xmm6(o, 0x1caf46, self.body.actual.rbp.wrapping_add(0x40), &mut c.xmm6).map_err(Fault::Service)?;
                if c.xmm6.is_none() { return Err(Fault::MissingReceipt); }
                self.phase = Phase::AdjustStack;
            }
            Phase::AdjustStack => {
                s.add_rsp_d8(o, 0x1caf4a, &mut c.adjustment).map_err(Fault::Service)?;
                let actual = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if actual.before != self.input.actual_work_rsp || actual.after != actual.before.wrapping_add(0xd8) { return Err(Fault::StackBinding); }
                self.phase = Phase::PopSaved;
            }
            Phase::PopSaved => {
                let registers = [Register::Rbx, Register::Rdi, Register::Rsi, Register::Rbp];
                let pcs = [0x1caf51, 0x1caf52, 0x1caf53, 0x1caf54];
                if self.pop >= 4 { return Err(Fault::MissingReceipt); }
                s.pop_saved(o, pcs[self.pop], registers[self.pop], &mut c.pops[self.pop]).map_err(Fault::Service)?;
                let actual = c.pops[self.pop].ok_or(Fault::MissingReceipt)?;
                let expected = self.input.actual_work_rsp.wrapping_add(0xd8 + self.pop as u64 * 8);
                if actual.before != expected || actual.after != expected.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.pop += 1; if self.pop == 4 { self.phase = Phase::Return; }
            }
            Phase::Return => {
                s.ret1caf55(o, &mut c.actual_return).map_err(Fault::Service)?;
                let actual = c.actual_return.ok_or(Fault::MissingReceipt)?;
                if actual.before_rsp != self.input.actual_work_rsp.wrapping_add(0xf8)
                    || actual.after_rsp != self.input.actual_work_rsp.wrapping_add(0x100)
                    || actual.rax != c.rax.ok_or(Fault::MissingReceipt)? || actual.xmm6 != c.xmm6.ok_or(Fault::MissingReceipt)? {
                    return Err(Fault::ReturnBinding);
                }
                for i in 0..4 { if actual.restored[i] != c.pops[i].ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); } }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
