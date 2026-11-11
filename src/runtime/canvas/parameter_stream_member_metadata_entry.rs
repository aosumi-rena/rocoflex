pub use super::super::native_entry::{Push, Subtract};
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub pc: u32, pub source_image_base: u64, pub rsp: u64, pub rcx: u64, pub incoming_rax: u64, pub rbx: u64, pub rbp: u64, pub saved_rdi: u64 }
pub trait Services: super::Services {

    fn push_rdi_568a10(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<Push>) -> Result<(), Self::Error>;
    fn sub_rsp_20_568a12(&mut self, owners: &mut super::Owners<Self>, returned: &mut Option<Subtract>) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { PushRdi, Subtract, BodyRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub push: Option<Push>, pub subtract: Option<Subtract> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: super::Owners<S>, actual: ActualEntry,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: super::Owners<S>,
    pub actual: ActualEntry, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn owners(&self) -> &super::Owners<S> { &self.owners }
}
pub struct Entered<S: Services> { parts: Parts<S> }

pub trait DirectBodyServices: Services {}
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn actual_body(&self) -> super::ActualBody { super::ActualBody { pc: 0x568a16,
        source_image_base: self.parts.actual.source_image_base, work_rsp: self.parts.actual.rsp.wrapping_sub(0x28), metadata_rcx: self.parts.actual.rcx, incoming_rax: self.parts.actual.incoming_rax, rbx: self.parts.actual.rbx, rbp: self.parts.actual.rbp } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
    pub fn into_body(self) -> super::Prepared<S> where S: DirectBodyServices { self.into_body_with(|services, owners| (services, owners)) }
    pub(in super::super) fn into_body_with<T: super::Services>(self,
        convert: impl FnOnce(S, super::Owners<S>) -> (T, super::Owners<T>)) -> super::Prepared<T> {
        let history = self.history(); let actual = self.actual_body();
        let (services, owners) = convert(self.parts.services, self.parts.owners);
        let mut captures = super::Captures::default(); captures.native_entry = Some(history);
        super::Prepared { services, owners, actual, captures, phase: super::Phase::Compare, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x568a10 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: super::Owners { storage: input.storage }, actual: input.actual,
            captures: Captures::default(), phase: Phase::PushRdi, attempted: None, stopped: false })
    }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::BodyRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::BodyRequired { Poll::Ready(Entered { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::PushRdi => {
                self.services.push_rdi_568a10(&mut self.owners, &mut self.captures.push).map_err(Fault::Service)?;
                let a = self.captures.push.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.actual.rsp || a.after_rsp != self.actual.rsp.wrapping_sub(8) || a.stored != self.actual.saved_rdi { return Err(Fault::StackBinding); }
                self.phase = Phase::Subtract;
            }
            Phase::Subtract => {
                self.services.sub_rsp_20_568a12(&mut self.owners, &mut self.captures.subtract).map_err(Fault::Service)?;
                let a = self.captures.subtract.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.actual.rsp.wrapping_sub(8) || a.after_rsp != self.actual.rsp.wrapping_sub(0x28) { return Err(Fault::StackBinding); }
                self.phase = Phase::BodyRequired;
            }
            Phase::BodyRequired => {}
        }
        Ok(())
    }
}
