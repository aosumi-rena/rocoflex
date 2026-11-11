pub use super::native_entry::{ActualInstruction, MemoryObservation, MemoryValue, Registers};

#[derive(Clone, Copy, Debug)]
pub struct ActualBoundary { pub pc: u32, pub registers: Registers }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction { RestoreXmm6, Add48 }
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub instruction: Instruction, pub pc: u32, pub next_pc: u32,
    pub before: Registers, pub address: Option<u64>,
}
#[derive(Clone, Copy, Debug)]
pub struct ReturnRequest { pub pc: u32, pub before: Registers }

#[derive(Clone, Copy, Debug, Default)]
pub struct ActualReturn {
    pub pc: Option<u32>, pub before: Option<Registers>, pub after: Option<Registers>,
    pub stack_address: Option<u64>, pub stack_word: Option<u64>, pub target: Option<u64>,
}
pub trait Services: super::Services {

    fn actual_parse_suffix(&mut self, storage: &mut Self::Storage, request: Request,
        returned: &mut Option<ActualInstruction>) -> Result<(), Self::Error>;

    fn actual_parse_ret(&mut self, storage: &mut Self::Storage, request: ReturnRequest,
        returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { RestoreXmm6, Add48, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub requests: [Option<Request>; 2], pub returned: [Option<ActualInstruction>; 2],
    pub return_request: Option<ReturnRequest>, pub actual_return: Option<ActualReturn>,
}
pub enum Fault<E> { Service(E), MissingReceipt, InstructionBinding, MemoryBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> {
    body: super::Parts<S>, actual: ActualBoundary, current: Registers,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool,
}
pub struct Rejected<S: Services> { pub completed: super::NativeRestoreRequired<S>, pub actual: ActualBoundary }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> {
    body: super::Parts<S>, actual: ActualBoundary, current: Registers,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn body(&self) -> &super::Parts<S> { &self.body }
    pub fn actual_boundary(&self) -> ActualBoundary { self.actual }

    pub fn current(&self) -> Registers { self.current }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<Phase> { self.attempted }
    pub fn stopped(&self) -> bool { self.stopped }
}
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn parts(&self) -> &Parts<S> { &self.parts }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_body(completed: super::NativeRestoreRequired<S>, actual: ActualBoundary)
        -> Result<Self, Rejected<S>> {
        let c = &completed.parts.captures;
        let Some(entry) = c.native_entry else { return Err(Rejected { completed, actual }); };
        let Some(cookie) = c.cookie_returned else { return Err(Rejected { completed, actual }); };
        let a = actual.registers;
        if actual.pc != 0x2b6f31 || a.rsp != completed.parts.input.actual.work_rsp
            || a.rsp != entry.actual.registers.rsp.wrapping_sub(0x48)
            || Some(a.xmm0) != c.xmm0 || a.xmm6 != cookie.xmm6 || a.rax != cookie.raw_rax {
            return Err(Rejected { completed, actual });
        }
        Ok(Self { body: completed.into_parts(), actual, current: a,
            captures: Captures::default(), phase: Phase::RestoreXmm6, attempted: None, stopped: false })
    }
    pub fn into_parts(self) -> Parts<S> { Parts { body: self.body, actual: self.actual, current: self.current,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::Done { Poll::Ready(Returned { parts: self.into_parts() }) }
        else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let b = self.current;
        if self.phase == Phase::Return {
            let request = ReturnRequest { pc: 0x2b6f3a, before: b };
            self.captures.return_request = Some(request);
            self.body.input.services.actual_parse_ret(&mut self.body.input.storage, request,
                &mut self.captures.actual_return).map_err(Fault::Service)?;
            let r = self.captures.actual_return.ok_or(Fault::MissingReceipt)?;
            let a = r.after.ok_or(Fault::MissingReceipt)?;
            let target = r.target.ok_or(Fault::MissingReceipt)?;
            let mut expected = b; expected.rsp = b.rsp.wrapping_add(8);
            if r.pc != Some(request.pc) || r.before != Some(b) || r.stack_address != Some(b.rsp)
                || r.stack_word != Some(target) || a != expected { return Err(Fault::ReturnBinding); }
            self.current = a; self.phase = Phase::Done; return Ok(());
        }
        let (i, instruction, pc, next_pc, address, next) = match self.phase {
            Phase::RestoreXmm6 => (0, Instruction::RestoreXmm6, 0x2b6f31, 0x2b6f36, Some(b.rsp.wrapping_add(0x30)), Phase::Add48),
            Phase::Add48 => (1, Instruction::Add48, 0x2b6f36, 0x2b6f3a, None, Phase::Return),
            Phase::Return | Phase::Done => return Ok(()),
        };
        let request = Request { instruction, pc, next_pc, before: b, address };
        self.captures.requests[i] = Some(request);
        self.body.input.services.actual_parse_suffix(&mut self.body.input.storage, request,
            &mut self.captures.returned[i]).map_err(Fault::Service)?;
        let r = self.captures.returned[i].ok_or(Fault::MissingReceipt)?;
        let a = r.after.ok_or(Fault::MissingReceipt)?;
        if r.pc != Some(pc) || r.next_pc != Some(next_pc) || r.before != Some(b) { return Err(Fault::InstructionBinding); }
        let mut expected = b;
        match instruction {
            Instruction::RestoreXmm6 => {
                let m = r.memory.ok_or(Fault::MissingReceipt)?;
                let Some(MemoryValue::Xmm(words)) = m.read else { return Err(Fault::MemoryBinding); };
                if m.address != address { return Err(Fault::MemoryBinding); }
                expected.xmm6 = words;
            }
            Instruction::Add48 => { expected.rsp = b.rsp.wrapping_add(0x48); expected.rflags = a.rflags; }
        }
        if a != expected { return Err(Fault::InstructionBinding); }
        self.current = a; self.phase = next; Ok(())
    }
}
