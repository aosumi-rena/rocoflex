use super::Owners;
pub use super::native_return::Register;

pub use super::super::super::super::super::super::super::super::initialization::base_call::entry::{Push, Subtract, XmmStore};
#[derive(Clone, Copy, Debug)]
pub struct ActualEntry {
    pub pc: u32, pub actual_image_base: u64, pub rsp: u64, pub rcx: u64, pub rdx: u64,
    pub r8d: u32, pub r9d: u32,

    pub saved: [u64; 4], pub xmm1: [u32; 4], pub xmm6: [u32; 4],
}
pub trait Services: super::Services {
    fn entry_push(&mut self, owners: &mut Owners<Self>, pc: u32, register: Register,
        returned: &mut Option<Push>) -> Result<(), Self::Error>;
    fn entry_sub_d8(&mut self, owners: &mut Owners<Self>, returned: &mut Option<Subtract>) -> Result<(), Self::Error>;
    fn entry_lea_rbp80(&mut self, owners: &mut Owners<Self>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn entry_save_xmm6(&mut self, owners: &mut Owners<Self>, address: u64,
        returned: &mut Option<XmmStore>) -> Result<(), Self::Error>;
}
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Push, Subtract, SetRbp, SaveXmm6, StoreState, BodyRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub pushes: [Option<Push>; 4], pub subtract: Option<Subtract>, pub rbp: Option<u64>,
    pub xmm6: Option<XmmStore>, pub state_store_returned: bool }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, RbpBinding, XmmBinding, AlreadyStopped }
pub struct Prepared<S: Services> { services: S, owners: Owners<S>, actual: ActualEntry, captures: Captures,
    phase: Phase, index: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Entered<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub(in super::super) services: S, pub(in super::super) owners: Owners<S>, pub actual: ActualEntry, pub captures: Captures,
    pub phase: Phase, pub index: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool }
pub struct Entered<S: Services> { parts: Parts<S>, body: super::ActualBody }

pub trait DirectBodyServices: Services {}
impl<S: Services> Parts<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn owners(&self) -> &Owners<S> { &self.owners }
}
impl<S: Services> Entered<S> {
    pub fn history(&self) -> History { History { actual: self.parts.actual, captures: self.parts.captures } }
    pub fn actual_body(&self) -> super::ActualBody { self.body }
    pub fn storage(&self) -> &S::Storage { &self.parts.owners.storage }
    pub fn into_parts(self) -> Parts<S> { self.parts }
    pub fn into_body(self) -> super::Prepared<S> where S: DirectBodyServices { self.into_body_with(|s, o| (s, o)) }

    pub(in super::super) fn into_body_with<T: super::Services<CallOwner = S::CallOwner>>(self,
        convert: impl FnOnce(S, Owners<S>) -> (T, Owners<T>)) -> super::Prepared<T> {
        let history = self.history(); let actual = self.body;
        let (services, owners) = convert(self.parts.services, self.parts.owners);
        let mut captures = super::Captures::default(); captures.native_entry = Some(history);
        super::Prepared { services, owners, actual, captures, saved: None, phase: super::Phase::SaveBindings, attempted: None, stopped: false }
    }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_entry(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x1cae80 { return Err(Rejected { input }); }
        Ok(Self { services: input.services, owners: Owners { storage: input.storage,
            calls: [None, None, None, None, None, None, None] }, actual: input.actual, captures: Captures::default(),
            phase: Phase::Push, index: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { services: self.services, owners: self.owners, actual: self.actual,
        captures: self.captures, phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::BodyRequired && spent < budget {
            self.attempted = Some((self.phase, self.index)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase != Phase::BodyRequired { return Poll::Pending(self); }
        let Some(rbp) = self.captures.rbp else {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingReceipt });
        };
        let body = super::ActualBody { pc: 0x1cae9f, actual_image_base: self.actual.actual_image_base, rbp,
            output_rcx: self.actual.rcx, entry_rbx: self.actual.saved[3], precision_r8d: self.actual.r8d,
            flag_r9d: self.actual.r9d, value_xmm1: self.actual.xmm1 };
        Poll::Ready(Entered { parts: self.into_parts(), body })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let s = &mut self.services; let o = &mut self.owners; let c = &mut self.captures;
        match self.phase {
            Phase::Push => {
                let regs = [Register::Rbp, Register::Rsi, Register::Rdi, Register::Rbx];
                let pcs = [0x1cae80, 0x1cae81, 0x1cae82, 0x1cae83];
                if self.index >= 4 { return Err(Fault::MissingReceipt); }
                s.entry_push(o, pcs[self.index], regs[self.index], &mut c.pushes[self.index]).map_err(Fault::Service)?;
                let actual = c.pushes[self.index].ok_or(Fault::MissingReceipt)?;
                let before = self.actual.rsp.wrapping_sub(self.index as u64 * 8);
                if actual.before_rsp != before || actual.after_rsp != before.wrapping_sub(8)
                    || actual.stored != self.actual.saved[self.index] { return Err(Fault::StackBinding); }
                self.index += 1; if self.index == 4 { self.phase = Phase::Subtract; }
            }
            Phase::Subtract => {
                s.entry_sub_d8(o, &mut c.subtract).map_err(Fault::Service)?;
                let actual = c.subtract.ok_or(Fault::MissingReceipt)?;
                if actual.before_rsp != self.actual.rsp.wrapping_sub(0x20)
                    || actual.after_rsp != self.actual.rsp.wrapping_sub(0xf8) { return Err(Fault::StackBinding); }
                self.phase = Phase::SetRbp;
            }
            Phase::SetRbp => {
                s.entry_lea_rbp80(o, &mut c.rbp).map_err(Fault::Service)?;
                if c.rbp.ok_or(Fault::MissingReceipt)? != self.actual.rsp.wrapping_sub(0x78) { return Err(Fault::RbpBinding); }
                self.phase = Phase::SaveXmm6;
            }
            Phase::SaveXmm6 => {
                let address = c.rbp.ok_or(Fault::MissingReceipt)?.wrapping_add(0x40);
                s.entry_save_xmm6(o, address, &mut c.xmm6).map_err(Fault::Service)?;
                let actual = c.xmm6.ok_or(Fault::MissingReceipt)?;
                if actual.address != address || actual.words != self.actual.xmm6 { return Err(Fault::XmmBinding); }
                self.phase = Phase::StoreState;
            }
            Phase::StoreState => {
                s.write_word(o, 0x1cae97, c.rbp.ok_or(Fault::MissingReceipt)?.wrapping_add(0x38), u64::MAX - 1).map_err(Fault::Service)?;
                c.state_store_returned = true; self.phase = Phase::BodyRequired;
            }
            Phase::BodyRequired => {}
        }
        Ok(())
    }
}
