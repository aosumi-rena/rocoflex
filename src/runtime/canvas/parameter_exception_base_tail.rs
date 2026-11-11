use super::{base, Suspended};
pub use super::super::native_return::{Register, Pop, StackAdjustment};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailRequest {
    pub instruction_pc: u32, pub addressed_slot: u64, pub loaded_target: u64,
    pub entry_rsp: u64, pub receiver_rcx: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualTailReturn {
    pub binding: super::ActualBaseReturn, pub raw_rax: u64,

    pub restored: [u64; 3],
}
pub trait Services: super::super::Services + base::Services {

    fn base_tail_add_rsi18(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn base_tail_mov_rcx_rsi(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn base_tail_add_rsp20(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn base_tail_pop(&mut self, pc: u32, frame: &mut Self::Frame, destination: Register,
        returned: &mut Option<Pop>) -> Result<(), Self::Error>;

    fn base_tail_read_import(&mut self, pc: u32, addressed_slot: u64, frame: &mut Self::Frame,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn base_tail_transfer(&mut self, frame: &mut Self::Frame, request: TailRequest,
        returned: &mut Option<ActualTailReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { AddReceiver, SetRcx, AdjustStack, PopSaved, ReadImport, Transfer, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub rsi: Option<u64>, pub rcx: Option<u64>, pub adjustment: Option<StackAdjustment>,
    pub pops: [Option<Pop>; 3], pub target: Option<u64>, pub request: Option<TailRequest>,
    pub actual_return: Option<ActualTailReturn>,
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub requirement: base::TailRequirement, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, RegisterBinding, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services, T> {
    body: base::Parts<S, Suspended<S::Function, T>>, requirement: base::TailRequirement,
    captures: Captures, phase: Phase, pop: usize, attempted: Option<(Phase, usize)>, stopped: bool,
}
pub struct Parts<S: Services, T> {
    pub body: base::Parts<S, Suspended<S::Function, T>>, pub requirement: base::TailRequirement,
    pub captures: Captures, pub phase: Phase, pub pop: usize,
    pub attempted: Option<(Phase, usize)>, pub stopped: bool,
}
pub struct Stopped<S: Services, T> { pub pending: Prepared<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services, T> { Pending(Prepared<S, T>), Ready(Completed<S, T>), Failed(Stopped<S, T>) }
pub struct Completed<S: Services, T> { pending: Prepared<S, T> }
impl<S: Services, T> Completed<S, T> {
    pub fn captures(&self) -> &Captures { &self.pending.captures }
    pub fn into_parts(self) -> Parts<S, T> { self.pending.into_parts() }

    pub fn into_parent(self) -> Result<super::super::Prepared<S, T>, super::RejectedReturn<S, T>> {
        let actual = self.pending.captures.actual_return.unwrap();
        super::resume_after_base_return(self.pending.body, self.pending.requirement, actual.binding,
            Some(History { requirement: self.pending.requirement, captures: self.pending.captures }))
    }
}
impl<S: Services, T> Prepared<S, T> {
    pub fn from_completed_body(body: super::TailRequired<S, T>) -> Self {
        let requirement = body.requirement();
        Self { body: body.into_parts(), requirement, captures: Captures::default(), phase: Phase::AddReceiver,
            pop: 0, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { body: self.body, requirement: self.requirement, captures: self.captures,
            phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S, T> {
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
        Poll::Ready(Completed { pending: self })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let work = self.body.input.actual_work_rsp;
        let r = &mut self.body.input;
        let c = &mut self.captures;
        match self.phase {
            Phase::AddReceiver => {
                r.services.base_tail_add_rsi18(&mut r.frame, &mut c.rsi).map_err(Fault::Service)?;
                if c.rsi.ok_or(Fault::MissingReceipt)? != self.requirement.receiver_rcx { return Err(Fault::RegisterBinding); }
                self.phase = Phase::SetRcx;
            }
            Phase::SetRcx => {
                r.services.base_tail_mov_rcx_rsi(&mut r.frame, &mut c.rcx).map_err(Fault::Service)?;
                if c.rcx.ok_or(Fault::MissingReceipt)? != c.rsi.ok_or(Fault::MissingReceipt)? { return Err(Fault::RegisterBinding); }
                self.phase = Phase::AdjustStack;
            }
            Phase::AdjustStack => {
                r.services.base_tail_add_rsp20(&mut r.frame, &mut c.adjustment).map_err(Fault::Service)?;
                let actual = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if actual.before != work || actual.after != work.wrapping_add(0x20) { return Err(Fault::StackBinding); }
                self.phase = Phase::PopSaved;
            }
            Phase::PopSaved => {
                let registers = [Register::Rbx, Register::Rdi, Register::Rsi];
                if self.pop >= 3 { return Err(Fault::MissingReceipt); }
                r.services.base_tail_pop(0x2b3b3c + self.pop as u32, &mut r.frame,
                    registers[self.pop], &mut c.pops[self.pop]).map_err(Fault::Service)?;
                let actual = c.pops[self.pop].ok_or(Fault::MissingReceipt)?;
                let expected = work.wrapping_add(0x20 + 8 * self.pop as u64);
                if actual.before != expected || actual.after != expected.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.pop += 1; if self.pop == 3 { self.phase = Phase::ReadImport; }
            }
            Phase::ReadImport => {
                r.services.base_tail_read_import(self.requirement.indirect_pc, self.requirement.addressed_slot,
                    &mut r.frame, &mut c.target).map_err(Fault::Service)?;
                if c.target.is_none() { return Err(Fault::MissingReceipt); }
                self.phase = Phase::Transfer;
            }
            Phase::Transfer => {
                let request = TailRequest { instruction_pc: self.requirement.indirect_pc,
                    addressed_slot: self.requirement.addressed_slot, loaded_target: c.target.ok_or(Fault::MissingReceipt)?,
                    entry_rsp: self.requirement.required_entry_rsp, receiver_rcx: c.rcx.ok_or(Fault::MissingReceipt)? };
                c.request = Some(request);
                r.services.base_tail_transfer(&mut r.frame, request, &mut c.actual_return).map_err(Fault::Service)?;
                let actual = c.actual_return.ok_or(Fault::MissingReceipt)?;
                if actual.binding.actual_loaded_target != request.loaded_target || actual.binding.actual_tail_target != request.loaded_target
                    || actual.binding.actual_tail_entry_rsp != request.entry_rsp
                    || actual.binding.actual_tail_receiver_rcx != request.receiver_rcx { return Err(Fault::ReturnBinding); }
                for i in 0..3 {
                    if actual.restored[i] != c.pops[i].ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); }
                }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
