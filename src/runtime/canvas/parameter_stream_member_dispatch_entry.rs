pub use super::super::super::native_entry::{Push, Subtract};
#[derive(Clone, Copy, Debug)]
pub struct Store { pub address: u64, pub word: u64 }
#[derive(Clone, Copy, Debug)]
pub struct RbpAssigned { pub rsp: u64, pub before: u64, pub after: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry { pub pc: u32, pub source_image_base: u64, pub rsp: u64,
    pub rax: u64, pub rbx: u64, pub rbp: u64, pub rcx: u64, pub rdx: u64, pub r8: u64, pub r9: u64 }
pub trait Services: super::Services {

    fn save_rbx_57a384(&mut self, o: &mut super::Owners<Self>, out: &mut Option<Store>) -> Result<(), Self::Error>;
    fn push_rbp_57a389(&mut self, o: &mut super::Owners<Self>, out: &mut Option<Push>) -> Result<(), Self::Error>;
    fn mov_rbp_rsp_57a38a(&mut self, o: &mut super::Owners<Self>, out: &mut Option<RbpAssigned>) -> Result<(), Self::Error>;
    fn sub_rsp_70_57a38d(&mut self, o: &mut super::Owners<Self>, out: &mut Option<Subtract>) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { SaveRbx, PushRbp, SetRbp, Subtract, BodyRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub store: Option<Store>, pub push: Option<Push>, pub rbp: Option<RbpAssigned>, pub subtract: Option<Subtract> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StoreBinding, StackBinding, RbpBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: super::Owners<S>, actual: ActualEntry,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: super::Owners<S>,
    pub actual: ActualEntry, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn services(&self) -> &S { &self.services } pub fn owners(&self) -> &super::Owners<S> { &self.owners } }
pub struct Entered<S: Services> { parts: Parts<S> }
pub trait DirectBodyServices: Services {}
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn actual_body(&self) -> super::ActualBody { let a = self.parts.actual;
        super::ActualBody { pc: 0x57a391, source_image_base: a.source_image_base, work_rsp: a.rsp.wrapping_sub(0x78),
            rbp: self.parts.captures.rbp.unwrap().after, rbx: a.rbx, incoming_rax: a.rax, rcx: a.rcx, rdx: a.rdx, r8: a.r8, r9: a.r9 } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
    pub fn into_body(self) -> super::Prepared<S> where S: DirectBodyServices { self.into_body_with(|s, o| (s, o)) }
    pub(in super::super) fn into_body_with<T: super::Services<CallOwner = S::CallOwner>>(self,
        convert: impl FnOnce(S, super::Owners<S>) -> (T, super::Owners<T>)) -> super::Prepared<T> {
        let history = self.history(); let actual = self.actual_body();
        let (services, owners) = convert(self.parts.services, self.parts.owners);
        let mut captures = super::Captures::default(); captures.native_entry = Some(history);
        captures.rax = Some(actual.incoming_rax); captures.rbx = Some(actual.rbx);
        super::Prepared { services, owners, actual, captures, phase: super::Phase::ClearContext, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x57a384 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: super::Owners { storage: input.storage, calls: [None, None, None] },
            actual: input.actual, captures: Captures::default(), phase: Phase::SaveRbx, attempted: None, stopped: false })
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
            Phase::SaveRbx => {
                self.services.save_rbx_57a384(&mut self.owners, &mut self.captures.store).map_err(Fault::Service)?;
                let a = self.captures.store.ok_or(Fault::MissingReceipt)?;
                if a.address != self.actual.rsp.wrapping_add(8) || a.word != self.actual.rbx { return Err(Fault::StoreBinding); }
                self.phase = Phase::PushRbp;
            }
            Phase::PushRbp => {
                self.services.push_rbp_57a389(&mut self.owners, &mut self.captures.push).map_err(Fault::Service)?;
                let a = self.captures.push.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.actual.rsp || a.after_rsp != self.actual.rsp.wrapping_sub(8) || a.stored != self.actual.rbp { return Err(Fault::StackBinding); }
                self.phase = Phase::SetRbp;
            }
            Phase::SetRbp => {
                self.services.mov_rbp_rsp_57a38a(&mut self.owners, &mut self.captures.rbp).map_err(Fault::Service)?;
                let a = self.captures.rbp.ok_or(Fault::MissingReceipt)?;
                if a.rsp != self.actual.rsp.wrapping_sub(8) || a.before != self.actual.rbp || a.after != a.rsp { return Err(Fault::RbpBinding); }
                self.phase = Phase::Subtract;
            }
            Phase::Subtract => {
                self.services.sub_rsp_70_57a38d(&mut self.owners, &mut self.captures.subtract).map_err(Fault::Service)?;
                let a = self.captures.subtract.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.actual.rsp.wrapping_sub(8) || a.after_rsp != self.actual.rsp.wrapping_sub(0x78) { return Err(Fault::StackBinding); }
                self.phase = Phase::BodyRequired;
            }
            Phase::BodyRequired => {}
        }
        Ok(())
    }
}
