pub use super::super::native_entry::Subtract;
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub entry: super::Entry, pub pc: u32, pub source_image_base: u64, pub rsp: u64 }
pub trait Services: super::Services {
    fn sub_rsp_28(&mut self, owners: &mut super::Owners<Self>, pc: u32, returned: &mut Option<Subtract>) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub subtract: Option<Subtract> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: super::Owners<S>, actual: ActualEntry,
    captures: Captures, attempted: bool, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: super::Owners<S>,
    pub actual: ActualEntry, pub captures: Captures, pub attempted: bool, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &super::Owners<S> { &self.owners } }
pub struct Entered<S: Services> { parts: Parts<S> }
pub trait DirectBodyServices: Services {}
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn actual_body(&self) -> super::ActualBody { super::ActualBody { entry: self.parts.actual.entry,
        pc: self.parts.actual.entry.body_pc(), source_image_base: self.parts.actual.source_image_base, work_rsp: self.parts.actual.rsp.wrapping_sub(0x28) } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
    pub fn into_body(self) -> super::Prepared<S> where S: DirectBodyServices { self.into_body_with(|services, owners| (services, owners)) }
    pub(in super::super) fn into_body_with<T: super::Services<CallOwner = S::CallOwner>>(self,
        convert: impl FnOnce(S, super::Owners<S>) -> (T, super::Owners<T>)) -> super::Prepared<T> {
        let history = self.history(); let actual = self.actual_body();
        let (services, owners) = convert(self.parts.services, self.parts.owners);
        let mut captures = super::Captures::default(); captures.native_entry = Some(history);
        super::Prepared { services, owners, actual, captures, phase: super::Phase::Context, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != input.actual.entry.entry_pc() { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: super::Owners { storage: input.storage, calls: [None, None] },
            actual: input.actual, captures: Captures::default(), attempted: false, stopped: false })
    }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        if budget == 0 { return Poll::Pending(self); }
        self.attempted = true;
        if let Err(e) = self.services.sub_rsp_28(&mut self.owners, self.actual.pc, &mut self.captures.subtract) {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::Service(e) });
        }
        let actual = match self.captures.subtract { Some(actual) => actual,
            None => { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingReceipt }); } };
        if actual.before_rsp != self.actual.rsp || actual.after_rsp != self.actual.rsp.wrapping_sub(0x28) {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::StackBinding });
        }
        Poll::Ready(Entered { parts: self.into_parts() })
    }
}
