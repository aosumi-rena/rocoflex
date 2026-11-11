pub use super::native_return::Register;
pub use super::super::native_entry::Push;
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry {
    pub pc: u32, pub rsp: u64, pub rcx: u64, pub rdx: u64, pub r8d: u32,

    pub saved: [u64; 4],
}
pub trait Services: Sized {
    type Storage;
    type Error;
    fn push_saved(&mut self, storage: &mut Self::Storage, pc: u32, register: Register,
        returned: &mut Option<Push>) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub pushes: [Option<Push>; 4] }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { PushSaved, BodyRequired }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, storage: S::Storage, actual: ActualEntry,
    captures: Captures, index: usize, attempted: Option<usize>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) storage: S::Storage,
    pub actual: ActualEntry, pub captures: Captures, pub index: usize, pub attempted: Option<usize>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn storage(&self) -> &S::Storage { &self.storage }
}
pub struct Entered<S: Services> { parts: Parts<S> }
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn actual_body_rsp(&self) -> u64 { self.parts.actual.rsp.wrapping_sub(0x20) }
    pub fn storage(&self) -> &S::Storage { &self.parts.storage }
    pub fn into_parts(self) -> Parts<S> { self.parts }

    pub(in super::super) fn into_body_with<T: super::Services>(self,
        convert: impl FnOnce(S, S::Storage) -> (T, T::Storage)) -> super::Prepared<T> {
        let history = self.history(); let a = self.parts.actual;
        let (services, storage) = convert(self.parts.services, self.parts.storage);
        let mut captures = super::Captures::default();
        captures.native_entry = Some(history); captures.source_cursor = a.rdx; captures.remaining_r8d = a.r8d;
        super::Prepared { services, storage, actual: super::ActualBody { pc: 0x1fb214,
            output_slot_rcx: a.rcx, source_rdx: a.rdx, count_r8d: a.r8d }, captures,
            phase: super::Phase::InitialBudget, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x1fb210 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, storage: input.storage, actual: input.actual,
            captures: Captures::default(), index: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { if self.index == 4 { Phase::BodyRequired } else { Phase::PushSaved } }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, storage: self.storage, actual: self.actual,
        captures: self.captures, index: self.index, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.index < 4 && spent < budget {
            self.attempted = Some(self.index); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.index == 4 { Poll::Ready(Entered { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let registers = [Register::Rsi, Register::Rdi, Register::Rbp, Register::Rbx];
        let pc = 0x1fb210 + self.index as u32;
        let register = *registers.get(self.index).ok_or(Fault::MissingReceipt)?;
        self.services.push_saved(&mut self.storage, pc, register, &mut self.captures.pushes[self.index]).map_err(Fault::Service)?;
        let actual = self.captures.pushes[self.index].ok_or(Fault::MissingReceipt)?;
        let before = self.actual.rsp.wrapping_sub(self.index as u64 * 8);
        if actual.before_rsp != before || actual.after_rsp != before.wrapping_sub(8)
            || actual.stored != self.actual.saved[self.index] { return Err(Fault::StackBinding); }
        self.index += 1; Ok(())
    }
}
