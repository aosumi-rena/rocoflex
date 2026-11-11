use super::{ConstructorReturnRequired, Owner};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input { pub actual_work_rsp: u64, pub actual_rbp: u64, pub actual_live_r14: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register { Rbx, Rdi, Rsi, R14, R15, Rbp }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Xmm { Xmm6, Xmm7 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackAdjustment { pub before: u64, pub after: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pop { pub before: u64, pub after: u64, pub value: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualReturn {
    pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64,

    pub restored: [u64; 6], pub xmm6: [u32; 4], pub xmm7: [u32; 4],
}
pub trait Services: super::Services {

    fn mov_rax_r14(&mut self, frame: &mut Self::Frame, actual_r14: u64,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn restore_movaps(&mut self, pc: u32, frame: &mut Self::Frame,
        actual_rbp: u64, offset: u32, destination: Xmm,
        returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn add_rsp_f8(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;

    fn pop_saved(&mut self, pc: u32, frame: &mut Self::Frame, destination: Register,
        returned: &mut Option<Pop>) -> Result<(), Self::Error>;

    fn actual_ret_2b32fb(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { MoveRax, RestoreXmm6, RestoreXmm7, AdjustStack, PopSaved, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub rax: Option<u64>, pub xmm6: Option<[u32; 4]>, pub xmm7: Option<[u32; 4]>,
    pub adjustment: Option<StackAdjustment>, pub pops: [Option<Pop>; 6],
    pub actual_return: Option<ActualReturn>,
}
pub enum Fault<E> {
    Service(E), MissingReceipt, RaxBinding, StackBinding, ReturnBinding, AlreadyStopped,
}
pub struct Prepared<S: Services> {
    owner: Owner<S>, input: Input, captures: Captures, phase: Phase,
    pop: usize, attempted: Option<(Phase, usize)>, stopped: bool,
}
pub struct Rejected<S: Services> { pub completed_retirement: ConstructorReturnRequired<S>, pub input: Input }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }

pub struct Returned<S: Services> { parts: Parts<S> }
pub struct Parts<S: Services> {
    owner: Owner<S>, pub input: Input, pub captures: Captures,
    pub phase: Phase, pub pop: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool,
}
impl<S: Services> Parts<S> {

    pub fn owner(&self) -> &Owner<S> { &self.owner }
}
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn owner(&self) -> &Owner<S> { &self.parts.owner }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {

    pub fn from_completed_retirement(completed_retirement: ConstructorReturnRequired<S>, input: Input)
        -> Result<Self, Rejected<S>> {
        if input.actual_rbp != input.actual_work_rsp.wrapping_add(0x80) {
            return Err(Rejected { completed_retirement, input });
        }
        Ok(Self { owner: completed_retirement.into_required_native_return(), input,
            captures: Captures::default(), phase: Phase::MoveRax, pop: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { owner: self.owner, input: self.input, captures: self.captures,
            phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some((self.phase, self.pop)); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::Done { return Poll::Pending(self); }
        Poll::Ready(Returned { parts: self.into_parts() })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let p = &mut self.owner.constructor.parent;
        let c = &mut self.captures;
        match self.phase {
            Phase::MoveRax => {
                p.services.mov_rax_r14(&mut p.frame, self.input.actual_live_r14, &mut c.rax).map_err(Fault::Service)?;
                let value = c.rax.ok_or(Fault::MissingReceipt)?;
                if value != self.input.actual_live_r14 { return Err(Fault::RaxBinding); }
                self.phase = Phase::RestoreXmm6;
            }
            Phase::RestoreXmm6 => {
                p.services.restore_movaps(0x2b32e4, &mut p.frame, self.input.actual_rbp, 0x50,
                    Xmm::Xmm6, &mut c.xmm6).map_err(Fault::Service)?;
                if c.xmm6.is_none() { return Err(Fault::MissingReceipt); }
                self.phase = Phase::RestoreXmm7;
            }
            Phase::RestoreXmm7 => {
                p.services.restore_movaps(0x2b32e8, &mut p.frame, self.input.actual_rbp, 0x60,
                    Xmm::Xmm7, &mut c.xmm7).map_err(Fault::Service)?;
                if c.xmm7.is_none() { return Err(Fault::MissingReceipt); }
                self.phase = Phase::AdjustStack;
            }
            Phase::AdjustStack => {
                p.services.add_rsp_f8(&mut p.frame, &mut c.adjustment).map_err(Fault::Service)?;
                let actual = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if actual.before != self.input.actual_work_rsp || actual.after != actual.before.wrapping_add(0xf8) {
                    return Err(Fault::StackBinding);
                }
                self.phase = Phase::PopSaved;
            }
            Phase::PopSaved => {
                let registers = [Register::Rbx, Register::Rdi, Register::Rsi, Register::R14, Register::R15, Register::Rbp];
                let pcs = [0x2b32f3, 0x2b32f4, 0x2b32f5, 0x2b32f6, 0x2b32f8, 0x2b32fa];
                if self.pop >= registers.len() { return Err(Fault::MissingReceipt); }
                p.services.pop_saved(pcs[self.pop], &mut p.frame, registers[self.pop], &mut c.pops[self.pop]).map_err(Fault::Service)?;
                let actual = c.pops[self.pop].ok_or(Fault::MissingReceipt)?;
                let expected = self.input.actual_work_rsp.wrapping_add(0xf8 + (self.pop as u64) * 8);
                if actual.before != expected || actual.after != expected.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.pop += 1;
                if self.pop == 6 { self.phase = Phase::Return; }
            }
            Phase::Return => {
                p.services.actual_ret_2b32fb(&mut p.frame, &mut c.actual_return).map_err(Fault::Service)?;
                let actual = c.actual_return.ok_or(Fault::MissingReceipt)?;
                let expected = self.input.actual_work_rsp.wrapping_add(0x128);
                if actual.before_rsp != expected || actual.after_rsp != expected.wrapping_add(8)
                    || actual.rax != c.rax.ok_or(Fault::MissingReceipt)?
                    || actual.xmm6 != c.xmm6.ok_or(Fault::MissingReceipt)?
                    || actual.xmm7 != c.xmm7.ok_or(Fault::MissingReceipt)? {
                    return Err(Fault::ReturnBinding);
                }
                for i in 0..6 {
                    if actual.restored[i] != c.pops[i].ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); }
                }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
