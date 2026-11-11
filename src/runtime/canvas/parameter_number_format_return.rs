use super::{Calls, Owners, facet, sentry};

pub use super::super::super::native_return::{Register, Pop, StackAdjustment};
pub trait Services: super::Services {

    fn format_restore_xmm6(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, address: u64, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn format_add_rsp140(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn format_pop_saved(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, pc: u32, register: Register, returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn format_ret1fb7eb(&mut self, o: &mut Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, f: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut Calls<Self::FinishCallOwner>, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct Input { pub actual_work_rsp: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn { pub before_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64,

    pub restored: [u64; 3], pub xmm6: [u32; 4] }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { RestoreXmm6, AdjustStack, PopSaved, Return, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub xmm6: Option<[u32; 4]>, pub adjustment: Option<StackAdjustment>,
    pub pops: [Option<Pop>; 3], pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub input: Input, pub required_rax: u64, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, StackBinding, ReturnBinding, AlreadyStopped }
pub struct Prepared<S: Services> { body: super::Parts<S>, input: Input, required_rax: u64, captures: Captures,
    phase: Phase, pop: usize, attempted: Option<(Phase, usize)>, stopped: bool }
pub struct Rejected<S: Services> { pub completed: super::NativeRestoreRequired<S>, pub input: Input }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { pub body: super::Parts<S>, pub input: Input, pub required_rax: u64, pub captures: Captures,
    pub phase: Phase, pub pop: usize, pub attempted: Option<(Phase, usize)>, pub stopped: bool }
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn history(&self) -> History { History { input: self.parts.input, required_rax: self.parts.required_rax, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_completed_body(completed: super::NativeRestoreRequired<S>, input: Input) -> Result<Self, Rejected<S>> {
        if input.actual_work_rsp != completed.parts.prefix().actual.rbp.wrapping_sub(0x80) { return Err(Rejected { completed, input }); }
        let required_rax = completed.required_rax();
        Ok(Self { body: completed.into_parts(), input, required_rax, captures: Captures::default(),
            phase: Phase::RestoreXmm6, pop: 0, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { body: self.body, input: self.input, required_rax: self.required_rax,
        captures: self.captures, phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some((self.phase, self.pop)); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::Done { Poll::Ready(Returned { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let rbp = self.body.prefix().actual.rbp;
        let r = super::memory::resources(&mut self.body.prior, &mut self.body.calls);
        let c = &mut self.captures;
        match self.phase {
            Phase::RestoreXmm6 => {
                r.services.format_restore_xmm6(r.owners, r.sentry, r.facet, r.calls, 0x1fb7da, rbp.wrapping_add(0xb0), &mut c.xmm6).map_err(Fault::Service)?;
                if c.xmm6.is_none() { return Err(Fault::MissingReceipt); }
                self.phase = Phase::AdjustStack;
            }
            Phase::AdjustStack => {
                r.services.format_add_rsp140(r.owners, r.sentry, r.facet, r.calls, 0x1fb7e1, &mut c.adjustment).map_err(Fault::Service)?;
                let a = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != self.input.actual_work_rsp || a.after != a.before.wrapping_add(0x140) { return Err(Fault::StackBinding); }
                self.phase = Phase::PopSaved;
            }
            Phase::PopSaved => {
                let registers = [Register::Rbx, Register::Rsi, Register::Rbp];
                let pcs = [0x1fb7e8, 0x1fb7e9, 0x1fb7ea];
                if self.pop >= 3 { return Err(Fault::MissingReceipt); }
                r.services.format_pop_saved(r.owners, r.sentry, r.facet, r.calls, pcs[self.pop], registers[self.pop], &mut c.pops[self.pop]).map_err(Fault::Service)?;
                let a = c.pops[self.pop].ok_or(Fault::MissingReceipt)?;
                let before = self.input.actual_work_rsp.wrapping_add(0x140 + self.pop as u64 * 8);
                if a.before != before || a.after != before.wrapping_add(8) { return Err(Fault::StackBinding); }
                self.pop += 1; if self.pop == 3 { self.phase = Phase::Return; }
            }
            Phase::Return => {
                r.services.format_ret1fb7eb(r.owners, r.sentry, r.facet, r.calls, &mut c.actual_return).map_err(Fault::Service)?;
                let a = c.actual_return.ok_or(Fault::MissingReceipt)?;
                if a.before_rsp != self.input.actual_work_rsp.wrapping_add(0x158)
                    || a.after_rsp != self.input.actual_work_rsp.wrapping_add(0x160)
                    || a.rax != self.required_rax || Some(a.xmm6) != c.xmm6 { return Err(Fault::ReturnBinding); }
                for i in 0..3 { if a.restored[i] != c.pops[i].ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); } }
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
