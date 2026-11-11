pub use super::super::native_entry::{Push, Subtract};
pub use super::native_return::Register;
pub type Calls<R> = super::super::builder_call::Calls<R>;
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub pc: u32, pub source_image_base: u64, pub rsp: u64, pub rcx: u64, pub rdx: u64,

    pub saved: [u64; 4] }
pub trait Services: Sized {
    type Storage; type CallOwner; type Error;
    fn push_saved(&mut self, storage: &mut Self::Storage, calls: &mut Calls<Self::CallOwner>, pc: u32, register: Register,
        returned: &mut Option<Push>) -> Result<(), Self::Error>;
    fn sub_rsp38(&mut self, storage: &mut Self::Storage, calls: &mut Calls<Self::CallOwner>, returned: &mut Option<Subtract>) -> Result<(), Self::Error>;
    fn lea_rbp30(&mut self, storage: &mut Self::Storage, calls: &mut Calls<Self::CallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn write_state(&mut self, storage: &mut Self::Storage, calls: &mut Calls<Self::CallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { PushSaved, Subtract, SetRbp, StoreState, BodyRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub pushes: [Option<Push>; 4], pub subtract: Option<Subtract>, pub rbp: Option<u64>, pub state_store_returned: bool }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, RbpBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, storage: S::Storage, calls: Calls<S::CallOwner>, actual: ActualEntry,
    captures: Captures, phase: Phase, index: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) storage: S::Storage,
    pub(in super::super) calls: Calls<S::CallOwner>, pub actual: ActualEntry, pub captures: Captures,
    pub phase: Phase, pub index: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn storage(&self) -> &S::Storage { &self.storage }
    pub fn calls(&self) -> &Calls<S::CallOwner> { &self.calls }
}
pub struct Entered<S: Services> { parts: Parts<S> }
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn storage(&self) -> &S::Storage { &self.parts.storage }
    pub fn actual_body(&self) -> super::ActualBody { super::ActualBody { pc: 0x1fb435,
        source_image_base: self.parts.actual.source_image_base, rbp: self.parts.captures.rbp.unwrap(),
        object_rcx: self.parts.actual.rcx, source_rdx: self.parts.actual.rdx, entry_rdi: self.parts.actual.saved[2] } }
    pub fn into_parts(self) -> Parts<S> { self.parts }

    pub(in super::super) fn into_body_with<T: super::Services<CallOwner = S::CallOwner>>(self,
        convert: impl FnOnce(S, S::Storage) -> (T, T::Storage)) -> super::Prepared<T> {
        let history = self.history(); let actual = self.actual_body();
        let (services, storage) = convert(self.parts.services, self.parts.storage);
        let mut captures = super::Captures::default(); captures.native_entry = Some(history);
        super::Prepared { services, owners: super::Owners { storage, calls: self.parts.calls }, actual, captures,
            saved: None, phase: super::Phase::Bind, index: 0, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x1fb420 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, storage: input.storage, calls: [None, None, None, None, None, None, None],
            actual: input.actual, captures: Captures::default(), phase: Phase::PushSaved, index: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, storage: self.storage, calls: self.calls,
        actual: self.actual, captures: self.captures, phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::BodyRequired && spent < budget {
            self.attempted = Some((self.phase, self.index)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::BodyRequired { Poll::Ready(Entered { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::PushSaved => {
                let registers = [Register::Rbp, Register::Rsi, Register::Rdi, Register::Rbx];
                let register = *registers.get(self.index).ok_or(Fault::MissingReceipt)?;
                self.services.push_saved(&mut self.storage, &mut self.calls, 0x1fb420 + self.index as u32,
                    register, &mut self.captures.pushes[self.index]).map_err(Fault::Service)?;
                let a = self.captures.pushes[self.index].ok_or(Fault::MissingReceipt)?;
                let before = self.actual.rsp.wrapping_sub(self.index as u64 * 8);
                if a.before_rsp != before || a.after_rsp != before.wrapping_sub(8) || a.stored != self.actual.saved[self.index] { return Err(Fault::StackBinding); }
                self.index += 1; if self.index == 4 { self.phase = Phase::Subtract; }
            }
            Phase::Subtract => {
                self.services.sub_rsp38(&mut self.storage, &mut self.calls, &mut self.captures.subtract).map_err(Fault::Service)?;
                let a = self.captures.subtract.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.actual.rsp.wrapping_sub(0x20) || a.after_rsp != self.actual.rsp.wrapping_sub(0x58) { return Err(Fault::StackBinding); }
                self.phase = Phase::SetRbp;
            }
            Phase::SetRbp => {
                self.services.lea_rbp30(&mut self.storage, &mut self.calls, &mut self.captures.rbp).map_err(Fault::Service)?;
                if self.captures.rbp.ok_or(Fault::MissingReceipt)? != self.actual.rsp.wrapping_sub(0x28) { return Err(Fault::RbpBinding); }
                self.phase = Phase::StoreState;
            }
            Phase::StoreState => {
                self.services.write_state(&mut self.storage, &mut self.calls, 0x1fb42d, self.captures.rbp.ok_or(Fault::MissingReceipt)?, u64::MAX - 1).map_err(Fault::Service)?;
                self.captures.state_store_returned = true; self.phase = Phase::BodyRequired;
            }
            Phase::BodyRequired => {}
        }
        Ok(())
    }
}
