use super::{Entry, Owners};
pub use super::super::native_entry::{Push, Subtract};
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub entry: Entry, pub pc: u32, pub source_image_base: u64, pub rsp: u64, pub rcx: u64,

    pub constructor_edx: Option<u32>, pub constructor_saved_rbx: Option<u64> }
pub trait Services: super::Services {

    fn push_rbx(&mut self, o: &mut Owners<Self>, pc: u32, returned: &mut Option<Push>) -> Result<(), Self::Error>;
    fn sub_rsp(&mut self, o: &mut Owners<Self>, pc: u32, amount: u64, returned: &mut Option<Subtract>) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Push, Subtract, BodyRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub push: Option<Push>, pub subtract: Option<Subtract> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualEntry, captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>, pub actual: ActualEntry,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &Owners<S> { &self.owners } }
pub struct Entered<S: Services> { parts: Parts<S> }

pub trait DirectBodyServices: Services {}
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn actual_body(&self) -> super::ActualBody { super::ActualBody { entry: self.parts.actual.entry, pc: self.parts.actual.entry.body_pc(),
        source_image_base: self.parts.actual.source_image_base, work_rsp: self.parts.actual.rsp.wrapping_sub(0x28), guard_rcx: self.parts.actual.rcx,
        constructor_edx: self.parts.actual.constructor_edx } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
    pub fn into_body(self) -> super::Prepared<S> where S: DirectBodyServices { self.into_body_with(|s, o| (s, o)) }
    pub(in super::super) fn into_body_with<T: super::Services<CallOwner = S::CallOwner>>(self,
        convert: impl FnOnce(S, Owners<S>) -> (T, Owners<T>)) -> super::Prepared<T> {
        let history = self.history(); let actual = self.actual_body(); let (services, owners) = convert(self.parts.services, self.parts.owners);
        let mut captures = super::Captures::default(); captures.native_entry = Some(history);
        let phase = match actual.entry { Entry::Construct55e0a0 => super::Phase::StoreConstructorIndex, Entry::Destroy55e0dc => super::Phase::ReadDestructorIndex };
        super::Prepared { services, owners, actual, captures, phase, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        let expected = match input.actual.entry { Entry::Construct55e0a0 => 0x55e0a0, Entry::Destroy55e0dc => 0x55e0dc };
        if input.actual.pc != expected || (input.actual.entry == Entry::Construct55e0a0 && (input.actual.constructor_edx.is_none() || input.actual.constructor_saved_rbx.is_none())) { return Err(Rejected { input }); }
        let phase = if input.actual.entry == Entry::Construct55e0a0 { Phase::Push } else { Phase::Subtract };
        Ok(Self { services: input.services, owners: Owners { storage: input.storage, call: None }, actual: input.actual,
            captures: Captures::default(), phase, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); } let mut spent = 0;
        while self.phase != Phase::BodyRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase == Phase::BodyRequired { Poll::Ready(Entered { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::Push => {
                self.services.push_rbx(&mut self.owners, 0x55e0a0, &mut self.captures.push).map_err(Fault::Service)?;
                let a = self.captures.push.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.actual.rsp || a.after_rsp != self.actual.rsp.wrapping_sub(8) || Some(a.stored) != self.actual.constructor_saved_rbx { return Err(Fault::StackBinding); } self.phase = Phase::Subtract;
            }
            Phase::Subtract => {
                let (pc, amount, before) = match self.actual.entry { Entry::Construct55e0a0 => (0x55e0a2, 0x20, self.actual.rsp.wrapping_sub(8)), Entry::Destroy55e0dc => (0x55e0dc, 0x28, self.actual.rsp) };
                self.services.sub_rsp(&mut self.owners, pc, amount, &mut self.captures.subtract).map_err(Fault::Service)?;
                let a = self.captures.subtract.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != before || a.after_rsp != self.actual.rsp.wrapping_sub(0x28) { return Err(Fault::StackBinding); } self.phase = Phase::BodyRequired;
            }
            Phase::BodyRequired => {}
        }
        Ok(())
    }
}
