#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub pc: u32, pub source_image_base: u64, pub rsp: u64,
    pub rax: u64, pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64,

    pub saved: [u64; 8] }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register { Rbx, Rbp, Rsi, Rdi, R12, R13, R14, R15 }
#[derive(Clone, Copy, Debug)]
pub enum Operation { StoreShadow { register: Register, address: u64, word: u64 }, Push { register: Register, word: u64 }, Subtract20 }
#[derive(Clone, Copy, Debug)]
pub struct Request { pub pc: u32, pub next_pc: u32, pub before_rsp: u64, pub after_rsp: u64, pub operation: Operation }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stored { pub address: u64, pub word: u64 }
#[derive(Clone, Copy, Debug)]
pub struct InstructionReturned { pub pc: u32, pub next_pc: u32, pub rsp: u64, pub rax: u64,
    pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64, pub stored: Option<Stored> }
pub trait Services: super::Services {

    fn prologue_instruction(&mut self, storage: &mut Self::Storage, request: Request,
        returned: &mut Option<InstructionReturned>) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub requests: [Option<Request>; 9], pub returned: [Option<InstructionReturned>; 9] }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, InstructionBinding, AlreadyStopped }
pub struct Prepared<S: Services> { input: Input<S>, captures: Captures, index: usize, attempted: Option<usize>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(super) services: S, pub(super) storage: S::Storage, pub actual: ActualEntry,
    pub captures: Captures, pub index: usize, pub attempted: Option<usize>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn storage(&self) -> &S::Storage { &self.storage } }
pub struct Entered<S: Services> { parts: Parts<S> }

pub trait DirectBodyServices: Services {}
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn actual_body(&self) -> super::ActualBody {
        let a = self.parts.actual;
        super::ActualBody { pc: 0x58ee20, source_image_base: a.source_image_base, work_rsp: a.rsp.wrapping_sub(0x48),
            ecx: a.rcx as u32, rdx: a.rdx, r8: a.r8, r9: a.r9, entry_rax: a.rax }
    }
    pub fn into_parts(self) -> Parts<S> { self.parts }
    pub fn into_body(self) -> super::Prepared<S> where S: DirectBodyServices { self.into_checked_body() }
    pub(super) fn into_checked_body(self) -> super::Prepared<S> {
        let history = self.history(); let actual = self.actual_body();
        let mut captures = super::Captures::default(); captures.native_entry = Some(history);
        super::Prepared { input: super::Input { services: self.parts.services, storage: self.parts.storage, actual },
            captures, phase: super::Phase::Bind, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x58ee04 { return Err(Rejected { input }); }
        Ok(Self { input, captures: Captures::default(), index: 0, attempted: None, stopped: false })
    }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.input.services, storage: self.input.storage, actual: self.input.actual,
        captures: self.captures, index: self.index, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.index < 9 && spent < budget {
            self.attempted = Some(self.index); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.index += 1; self.attempted = None;
        }
        if self.index == 9 { Poll::Ready(Entered { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let a = self.input.actual; let n = self.index;
        let (pc, next_pc, before_rsp, after_rsp, operation, stored) = if n < 3 {
            let (pc, next, register) = [(0x58ee04, 0x58ee09, Register::Rbx), (0x58ee09, 0x58ee0e, Register::Rbp), (0x58ee0e, 0x58ee13, Register::Rsi)][n];
            let address = a.rsp.wrapping_add(8 + (n as u64).wrapping_mul(8)); let word = a.saved[n];
            (pc, next, a.rsp, a.rsp, Operation::StoreShadow { register, address, word }, Some(Stored { address, word }))
        } else if n < 8 {
            let j = n - 3; let (pc, next, register) = [(0x58ee13, 0x58ee14, Register::Rdi), (0x58ee14, 0x58ee16, Register::R12),
                (0x58ee16, 0x58ee18, Register::R13), (0x58ee18, 0x58ee1a, Register::R14), (0x58ee1a, 0x58ee1c, Register::R15)][j];
            let before = a.rsp.wrapping_sub((j as u64).wrapping_mul(8)); let after = before.wrapping_sub(8); let word = a.saved[7 - j];
            (pc, next, before, after, Operation::Push { register, word }, Some(Stored { address: after, word }))
        } else {
            (0x58ee1c, 0x58ee20, a.rsp.wrapping_sub(0x28), a.rsp.wrapping_sub(0x48), Operation::Subtract20, None)
        };
        let request = Request { pc, next_pc, before_rsp, after_rsp, operation }; self.captures.requests[n] = Some(request);
        self.input.services.prologue_instruction(&mut self.input.storage, request, &mut self.captures.returned[n]).map_err(Fault::Service)?;
        let b = self.captures.returned[n].ok_or(Fault::MissingReceipt)?;
        if b.pc != pc || b.next_pc != next_pc || b.rsp != after_rsp || b.stored != stored
            || b.rax != a.rax || b.rcx != a.rcx || b.rdx != a.rdx || b.r8 != a.r8 || b.r9 != a.r9 { return Err(Fault::InstructionBinding); }
        Ok(())
    }
}
