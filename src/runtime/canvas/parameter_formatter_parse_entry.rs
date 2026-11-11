#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registers {
    pub rsp: u64, pub rax: u64, pub rcx: u64, pub rdx: u64,
    pub xmm0: [u32; 4], pub xmm6: [u32; 4], pub mxcsr: u32, pub rflags: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryValue { Qword(u64), Xmm([u32; 4]) }

#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryObservation {
    pub address: Option<u64>, pub read: Option<MemoryValue>,
    pub before: Option<MemoryValue>, pub after: Option<MemoryValue>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction { Subtract48, SaveXmm6, LoadCookie, XorCookie, StoreCookie }
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub instruction: Instruction, pub pc: u32, pub next_pc: u32,
    pub before: Registers, pub address: Option<u64>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ActualInstruction {
    pub pc: Option<u32>, pub next_pc: Option<u32>,
    pub before: Option<Registers>, pub after: Option<Registers>,
    pub memory: Option<MemoryObservation>,
}
pub trait Services: super::Services {

    fn actual_parse_prefix(&mut self, storage: &mut Self::Storage, request: Request,
        returned: &mut Option<ActualInstruction>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub pc: u32, pub source_image_base: u64, pub registers: Registers }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Subtract48, SaveXmm6, LoadCookie, XorCookie, StoreCookie, BodyRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub requests: [Option<Request>; 5], pub returned: [Option<ActualInstruction>; 5] }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, InstructionBinding, MemoryBinding, AlreadyStopped }
pub struct Prepared<S: Services> {
    input: Input<S>, current: Registers, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool,
}
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> {
    input: Input<S>, current: Registers, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn input(&self) -> &Input<S> { &self.input }
    pub fn current(&self) -> Registers { self.current }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn attempted(&self) -> Option<Phase> { self.attempted }
    pub fn stopped(&self) -> bool { self.stopped }
}
pub struct Entered<S: Services> { parts: Parts<S> }
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.input.actual, captures: self.parts.captures } }
    pub fn actual_body(&self) -> super::ActualBody {
        super::ActualBody { pc: 0x2b6f08, work_rsp: self.parts.current.rsp, input_rdx: self.parts.current.rdx }
    }

    pub fn into_body(self) -> super::Prepared<S> {
        let history = self.history(); let actual = self.actual_body();
        let mut captures = super::Captures::default(); captures.native_entry = Some(history);
        super::Prepared { input: super::Input { services: self.parts.input.services,
            storage: self.parts.input.storage, actual }, captures,
            phase: super::Phase::ReadInputWord, attempted: None, stopped: false }
    }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x2b6ef0 { return Err(Rejected { input }); }
        let current = input.actual.registers;
        Ok(Self { input, current, captures: Captures::default(), phase: Phase::Subtract48,
            attempted: None, stopped: false })
    }
    pub fn into_parts(self) -> Parts<S> { Parts { input: self.input, current: self.current,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::BodyRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::BodyRequired { Poll::Ready(Entered { parts: self.into_parts() }) }
        else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let b = self.current;
        let (i, instruction, pc, next_pc, address, next) = match self.phase {
            Phase::Subtract48 => (0, Instruction::Subtract48, 0x2b6ef0, 0x2b6ef4, None, Phase::SaveXmm6),
            Phase::SaveXmm6 => (1, Instruction::SaveXmm6, 0x2b6ef4, 0x2b6ef9, Some(b.rsp.wrapping_add(0x30)), Phase::LoadCookie),
            Phase::LoadCookie => (2, Instruction::LoadCookie, 0x2b6ef9, 0x2b6f00,
                Some(self.input.actual.source_image_base.wrapping_add(0x820dd8)), Phase::XorCookie),
            Phase::XorCookie => (3, Instruction::XorCookie, 0x2b6f00, 0x2b6f03, None, Phase::StoreCookie),
            Phase::StoreCookie => (4, Instruction::StoreCookie, 0x2b6f03, 0x2b6f08, Some(b.rsp.wrapping_add(0x28)), Phase::BodyRequired),
            Phase::BodyRequired => return Ok(()),
        };
        let request = Request { instruction, pc, next_pc, before: b, address };
        self.captures.requests[i] = Some(request);
        self.input.services.actual_parse_prefix(&mut self.input.storage, request,
            &mut self.captures.returned[i]).map_err(Fault::Service)?;
        let r = self.captures.returned[i].ok_or(Fault::MissingReceipt)?;
        let a = r.after.ok_or(Fault::MissingReceipt)?;
        if r.pc != Some(pc) || r.next_pc != Some(next_pc) || r.before != Some(b) { return Err(Fault::InstructionBinding); }
        let mut expected = b;
        match instruction {
            Instruction::Subtract48 => { expected.rsp = b.rsp.wrapping_sub(0x48); expected.rflags = a.rflags; }
            Instruction::SaveXmm6 | Instruction::StoreCookie => {
                let m = r.memory.ok_or(Fault::MissingReceipt)?;
                let value = if instruction == Instruction::SaveXmm6 { MemoryValue::Xmm(b.xmm6) } else { MemoryValue::Qword(b.rax) };
                if m.address != address || m.after != Some(value) { return Err(Fault::MemoryBinding); }
            }
            Instruction::LoadCookie => {
                let m = r.memory.ok_or(Fault::MissingReceipt)?;
                let Some(MemoryValue::Qword(word)) = m.read else { return Err(Fault::MemoryBinding); };
                if m.address != address { return Err(Fault::MemoryBinding); }
                expected.rax = word;
            }
            Instruction::XorCookie => { expected.rax = b.rax ^ b.rsp; expected.rflags = a.rflags; }
        }

        if a != expected { return Err(Fault::InstructionBinding); }
        self.current = a; self.phase = next; Ok(())
    }
}
