use super::{metadata, ActualBody, AdmissionFault, Resources};
pub use super::super::initialization::base_call::entry::{Register, Xmm, Push, Subtract, XmmStore};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualEntry {
    pub state: i32, pub selected_funclet: u32, pub actual_func_info_rva: u32,
    pub actual_handler_rva: u32, pub unwind_rdx: u64, pub entry_rsp: u64,

    pub saved: [u64; 6], pub xmm6: [u32; 4], pub xmm7: [u32; 4],
}
pub trait Services: super::Services {

    fn eh_store_unwind_rdx(&mut self, pc: u32, frame: &mut Self::Frame,
        actual_address: u64, actual_rdx: u64) -> Result<(), Self::Error>;
    fn eh_entry_push(&mut self, pc: u32, frame: &mut Self::Frame, register: Register,
        returned: &mut Option<Push>) -> Result<(), Self::Error>;
    fn eh_entry_sub48(&mut self, pc: u32, frame: &mut Self::Frame,
        returned: &mut Option<Subtract>) -> Result<(), Self::Error>;
    fn eh_entry_lea_rbp80(&mut self, pc: u32, frame: &mut Self::Frame, actual_rdx: u64,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn eh_entry_save_xmm(&mut self, pc: u32, frame: &mut Self::Frame, source: Xmm,
        actual_work_rsp: u64, offset: u32, returned: &mut Option<XmmStore>) -> Result<(), Self::Error>;
}
pub enum Origin<S: Services, T> {
    Initial { resources: Resources<S, T>, normalized_pc: u32 },
    Next(super::Returned<S, T>),
}
impl<S: Services, T> Origin<S, T> {
    fn resources_mut(&mut self) -> &mut Resources<S, T> {
        match self { Self::Initial { resources, .. } => resources, Self::Next(returned) => &mut returned.parts.resources }
    }
    fn into_parts(self) -> OriginParts<S, T> {
        match self {
            Self::Initial { resources, normalized_pc } => OriginParts::Initial(StoppedInitial { resources, normalized_pc }),
            Self::Next(returned) => OriginParts::Next(returned.into_parts()),
        }
    }
}

pub enum OriginParts<S: Services, T> {
    Initial(StoppedInitial<S, T>),
    Next(super::Parts<S, T>),
}

pub struct StoppedInitial<S: Services, T> {
    resources: Resources<S, T>, normalized_pc: u32,
}
impl<S: Services, T> StoppedInitial<S, T> {
    pub fn resources(&self) -> &Resources<S, T> { &self.resources }
    pub fn normalized_pc(&self) -> u32 { self.normalized_pc }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { StoreUnwind, Push, Subtract, SetRbp, SaveXmm7, SaveXmm6, BodyRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub unwind_store_returned: bool, pub pushes: [Option<Push>; 6],
    pub subtract: Option<Subtract>, pub rbp: Option<u64>, pub xmm7: Option<XmmStore>, pub xmm6: Option<XmmStore>,
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, RegisterBinding, XmmBinding, AlreadyStopped }
pub struct Rejected<S: Services, T> { pub origin: Origin<S, T>, pub actual: ActualEntry, pub fault: AdmissionFault }
pub struct Prepared<S: Services, T> {
    origin: Origin<S, T>, actual: ActualEntry, profile: metadata::Profile, captures: Captures,
    phase: Phase, push: usize, attempted: Option<(Phase, usize)>, stopped: bool,
}
pub struct Parts<S: Services, T> {
    pub origin: OriginParts<S, T>, pub actual: ActualEntry, pub profile: metadata::Profile, pub captures: Captures,
    pub phase: Phase, pub push: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool,
}
pub struct Stopped<S: Services, T> { pub pending: Prepared<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services, T> { Pending(Prepared<S, T>), Ready(Entered<S, T>), Failed(Stopped<S, T>) }
pub struct Entered<S: Services, T> { pending: Prepared<S, T> }
pub enum BodyRejection<S: Services, T> {
    Initial(super::Rejected<S, T>),
    Next { parts: super::Parts<S, T>, actual: ActualBody, fault: AdmissionFault },
}
pub struct RejectedBody<S: Services, T> { pub history: History, pub rejected: BodyRejection<S, T> }
fn selection_matches(actual: ActualEntry, profile: metadata::Profile) -> bool {
    actual.state == profile.state && actual.selected_funclet == profile.funclet
        && actual.actual_func_info_rva == 0x7c8358 && actual.actual_handler_rva == 0x5641f4
}
impl<S: Services, T> Entered<S, T> {
    pub fn history(&self) -> History { History { actual: self.pending.actual, captures: self.pending.captures } }
    pub fn into_parts(self) -> Parts<S, T> { self.pending.into_parts() }

    pub fn into_body(self) -> Result<super::Prepared<S, T>, RejectedBody<S, T>> {
        let history = self.history();
        let a = self.pending.actual;
        let body = ActualBody { state: a.state, selected_funclet: a.selected_funclet,
            body_pc: self.pending.profile.body_pc, actual_func_info_rva: a.actual_func_info_rva,
            actual_handler_rva: a.actual_handler_rva, unwind_rdx: a.unwind_rdx,
            source_rbp: self.pending.captures.rbp.unwrap(), funclet_work_rsp: self.pending.captures.subtract.unwrap().after_rsp };
        let mut prepared = match self.pending.origin {
            Origin::Initial { resources, normalized_pc } => match super::Prepared::from_actual_frame(resources, normalized_pc, body) {
                Ok(prepared) => prepared, Err(rejected) => return Err(RejectedBody { history, rejected: BodyRejection::Initial(rejected) }),
            },
            Origin::Next(returned) => match returned.begin_actual_next(body) {
                Ok(prepared) => prepared,
                Err(rejected) => return Err(RejectedBody { history, rejected: BodyRejection::Next {
                    parts: rejected.returned.into_parts(), actual: rejected.actual, fault: rejected.fault } }),
            },
        };
        prepared.stage.native_entry = Some(history);
        Ok(prepared)
    }
}
impl<S: Services, T> Prepared<S, T> {

    pub fn from_actual_dispatch(resources: Resources<S, T>, normalized_pc: u32, actual: ActualEntry) -> Result<Self, Rejected<S, T>> {
        let origin = Origin::Initial { resources, normalized_pc };
        let state = match metadata::state_at_normalized_pc(normalized_pc) {
            None => return Err(Rejected { origin, actual, fault: AdmissionFault::PcOutsideFunction }),
            Some(-1) => return Err(Rejected { origin, actual, fault: AdmissionFault::NoLocalAction }),
            Some(state) => state,
        };
        let profile = metadata::PROFILES[state as usize];
        if actual.state != state { return Err(Rejected { origin, actual, fault: AdmissionFault::StateMismatch }); }
        if !selection_matches(actual, profile) { return Err(Rejected { origin, actual, fault: AdmissionFault::BodyBinding }); }
        Ok(Self::at(origin, actual, profile))
    }

    pub fn from_actual_next(returned: super::Returned<S, T>, actual: ActualEntry) -> Result<Self, Rejected<S, T>> {
        let next = returned.next_required_state();
        if next < 0 { return Err(Rejected { origin: Origin::Next(returned), actual, fault: AdmissionFault::NoNextLocalState }); }
        let profile = metadata::PROFILES[next as usize];
        if !selection_matches(actual, profile) || actual.unwind_rdx != returned.parts.stage.actual.unwind_rdx {
            return Err(Rejected { origin: Origin::Next(returned), actual, fault: AdmissionFault::BodyBinding });
        }
        if returned.parts.previous_len >= 3 {
            return Err(Rejected { origin: Origin::Next(returned), actual, fault: AdmissionFault::HistoryFull });
        }
        Ok(Self::at(Origin::Next(returned), actual, profile))
    }
    fn at(origin: Origin<S, T>, actual: ActualEntry, profile: metadata::Profile) -> Self {
        Self { origin, actual, profile, captures: Captures::default(), phase: Phase::StoreUnwind,
            push: 0, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { origin: self.origin.into_parts(), actual: self.actual, profile: self.profile, captures: self.captures,
            phase: self.phase, push: self.push, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S, T> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::BodyRequired && spent < budget {
            self.attempted = Some((self.phase, self.push)); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::BodyRequired { return Poll::Pending(self); }
        Poll::Ready(Entered { pending: self })
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let a = self.actual;
        let start = self.profile.funclet;
        let r = self.origin.resources_mut();
        let c = &mut self.captures;
        match self.phase {
            Phase::StoreUnwind => {
                r.services.eh_store_unwind_rdx(start, &mut r.frame, a.entry_rsp.wrapping_add(0x10), a.unwind_rdx).map_err(Fault::Service)?;
                c.unwind_store_returned = true; self.phase = Phase::Push;
            }
            Phase::Push => {
                let registers = [Register::Rbp, Register::R15, Register::R14, Register::Rsi, Register::Rdi, Register::Rbx];
                let offsets = [5, 6, 8, 10, 11, 12];
                if self.push >= 6 { return Err(Fault::MissingReceipt); }
                r.services.eh_entry_push(start + offsets[self.push], &mut r.frame, registers[self.push], &mut c.pushes[self.push]).map_err(Fault::Service)?;
                let actual = c.pushes[self.push].ok_or(Fault::MissingReceipt)?;
                let expected = a.entry_rsp.wrapping_sub(8 * self.push as u64);
                if actual.before_rsp != expected || actual.after_rsp != expected.wrapping_sub(8)
                    || actual.stored != a.saved[self.push] { return Err(Fault::StackBinding); }
                self.push += 1; if self.push == 6 { self.phase = Phase::Subtract; }
            }
            Phase::Subtract => {
                r.services.eh_entry_sub48(start + 13, &mut r.frame, &mut c.subtract).map_err(Fault::Service)?;
                let actual = c.subtract.ok_or(Fault::MissingReceipt)?;
                if actual.before_rsp != a.entry_rsp.wrapping_sub(0x30) || actual.after_rsp != a.entry_rsp.wrapping_sub(0x78) {
                    return Err(Fault::StackBinding);
                }
                self.phase = Phase::SetRbp;
            }
            Phase::SetRbp => {
                r.services.eh_entry_lea_rbp80(start + 17, &mut r.frame, a.unwind_rdx, &mut c.rbp).map_err(Fault::Service)?;
                if c.rbp.ok_or(Fault::MissingReceipt)? != a.unwind_rdx.wrapping_add(0x80) { return Err(Fault::RegisterBinding); }
                self.phase = Phase::SaveXmm7;
            }
            Phase::SaveXmm7 | Phase::SaveXmm6 => {
                let work = c.subtract.ok_or(Fault::MissingReceipt)?.after_rsp;
                let (pc, offset, source, expected, held) = if self.phase == Phase::SaveXmm7 {
                    (start + 24, 0x20, Xmm::Xmm7, a.xmm7, &mut c.xmm7)
                } else { (start + 29, 0x30, Xmm::Xmm6, a.xmm6, &mut c.xmm6) };
                r.services.eh_entry_save_xmm(pc, &mut r.frame, source, work, offset, held).map_err(Fault::Service)?;
                let actual = held.ok_or(Fault::MissingReceipt)?;
                if actual.address != work.wrapping_add(offset as u64) || actual.words != expected { return Err(Fault::XmmBinding); }
                self.phase = if self.phase == Phase::SaveXmm7 { Phase::SaveXmm6 } else { Phase::BodyRequired };
            }
            Phase::BodyRequired => {}
        }
        Ok(())
    }
}
