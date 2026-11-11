use super::super::{metadata::Action, Phase as ParentPhase, Prepared as Parent};
pub use super::super::native_entry::{Register, Push, Subtract};

pub trait Services: super::super::Services + super::base::Services {

    fn base_entry_push(&mut self, pc: u32, frame: &mut Self::Frame, register: Register,
        returned: &mut Option<Push>) -> Result<(), Self::Error>;

    fn base_entry_sub20(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<Subtract>) -> Result<(), Self::Error>;

    fn base_entry_mov_rsi_rcx(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    pub entry_pc: u32, pub entry_rsp: u64, pub source_image_base: u64, pub rcx: u64,

    pub saved: [u64; 3],
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub pushes: [Option<Push>; 3], pub subtract: Option<Subtract>, pub rsi: Option<u64> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub input: Input, pub captures: Captures }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Push, Subtract, SetRsi, BodyRequired }
pub enum Admission { NotPendingBaseCall, MissingReceiver, ActualBinding }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, RegisterBinding, AlreadyStopped }
pub struct Rejected<S: Services, T> { pub parent: Parent<S, T>, pub input: Input, pub fault: Admission }
pub struct Prepared<S: Services, T> {
    parent: Parent<S, T>, input: Input, captures: Captures,
    phase: Phase, push: usize, attempted: Option<(Phase, usize)>, stopped: bool,
}
pub struct Parts<S: Services, T> {

    pub parent: Parent<S, T>, pub input: Input, pub captures: Captures,
    pub phase: Phase, pub push: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool,
}
pub struct Stopped<S: Services, T> { pub pending: Prepared<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services, T> { Pending(Prepared<S, T>), Ready(Entered<S, T>), Failed(Stopped<S, T>) }
pub struct Entered<S: Services, T> { pending: Prepared<S, T> }
impl<S: Services, T> Entered<S, T> {
    pub fn history(&self) -> History { History { input: self.pending.input, captures: self.pending.captures } }
    pub fn into_parts(self) -> Parts<S, T> { self.pending.into_parts() }

    pub fn into_body(self) -> Result<super::Bound<S, T>, super::RejectedBind<S, T>> {
        let history = self.history();
        let actual = super::ActualBaseBody { body_pc: 0x2b3a5a,
            source_image_base: self.pending.input.source_image_base, receiver_rcx: self.pending.input.rcx,
            saved_rsi: self.pending.captures.rsi.unwrap(), work_rsp: self.pending.captures.subtract.unwrap().after_rsp };
        self.pending.parent.bind_base_body(actual, Some(history))
    }
}
impl<S: Services, T> Prepared<S, T> {
    pub fn from_actual_entry(mut parent: Parent<S, T>, input: Input) -> Result<Self, Rejected<S, T>> {
        let already_stopped = parent.stopped;
        parent.stopped = true;
        let action = parent.stage.profile.actions.get(parent.action).copied().flatten();
        if already_stopped || parent.phase != ParentPhase::CallDirect
            || !matches!(action, Some(Action::Direct { target: 0x2b3a50, .. })) {
            return Err(Rejected { parent, input, fault: Admission::NotPendingBaseCall });
        }
        let Some(receiver) = parent.stage.captures.direct_receivers[parent.action] else {
            return Err(Rejected { parent, input, fault: Admission::MissingReceiver });
        };
        if input.entry_pc != 0x2b3a50 || input.rcx != receiver
            || input.entry_rsp != parent.stage.actual.funclet_work_rsp.wrapping_sub(8) {
            return Err(Rejected { parent, input, fault: Admission::ActualBinding });
        }
        Ok(Self { parent, input, captures: Captures::default(), phase: Phase::Push,
            push: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { parent: self.parent, input: self.input, captures: self.captures,
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
        let r = &mut self.parent.resources;
        let c = &mut self.captures;
        match self.phase {
            Phase::Push => {
                let registers = [Register::Rsi, Register::Rdi, Register::Rbx];
                if self.push >= 3 { return Err(Fault::MissingReceipt); }
                r.services.base_entry_push(0x2b3a50 + self.push as u32, &mut r.frame,
                    registers[self.push], &mut c.pushes[self.push]).map_err(Fault::Service)?;
                let actual = c.pushes[self.push].ok_or(Fault::MissingReceipt)?;
                let expected = self.input.entry_rsp.wrapping_sub(8 * self.push as u64);
                if actual.before_rsp != expected || actual.after_rsp != expected.wrapping_sub(8)
                    || actual.stored != self.input.saved[self.push] { return Err(Fault::StackBinding); }
                self.push += 1; if self.push == 3 { self.phase = Phase::Subtract; }
            }
            Phase::Subtract => {
                r.services.base_entry_sub20(&mut r.frame, &mut c.subtract).map_err(Fault::Service)?;
                let actual = c.subtract.ok_or(Fault::MissingReceipt)?;
                if actual.before_rsp != self.input.entry_rsp.wrapping_sub(0x18)
                    || actual.after_rsp != self.input.entry_rsp.wrapping_sub(0x38) { return Err(Fault::StackBinding); }
                self.phase = Phase::SetRsi;
            }
            Phase::SetRsi => {
                r.services.base_entry_mov_rsi_rcx(&mut r.frame, &mut c.rsi).map_err(Fault::Service)?;
                if c.rsi.ok_or(Fault::MissingReceipt)? != self.input.rcx { return Err(Fault::RegisterBinding); }
                self.phase = Phase::BodyRequired;
            }
            Phase::BodyRequired => {}
        }
        Ok(())
    }
}
