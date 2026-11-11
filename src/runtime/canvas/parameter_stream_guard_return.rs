pub use super::super::native_return::{Pop, StackAdjustment};
use super::Entry;
pub trait Services: super::Services {
    type TailOwner;
    fn mov_rax_rbx(&mut self, o: &mut super::Owners<Self>, pc: u32, required_rbx: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn add_rsp(&mut self, o: &mut super::Owners<Self>, pc: u32, amount: u64, returned: &mut Option<StackAdjustment>) -> Result<(), Self::Error>;
    fn pop_rbx(&mut self, o: &mut super::Owners<Self>, pc: u32, returned: &mut Option<Pop>) -> Result<(), Self::Error>;
    fn ret(&mut self, o: &mut super::Owners<Self>, pc: u32, returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;

    fn tail_57a7fc(&mut self, o: &mut super::Owners<Self>, request: TailRequest, held: &mut Option<TailCallReturned<Self::TailOwner>>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path { ConstructNormal, DestroyNormal, DestroyZeroTail }
#[derive(Clone, Copy, Debug)]
pub struct TailRequest { pub source_pc: u32, pub target_rva: u32, pub entry_rsp: u64, pub rcx: u64, pub rax: u64 }
#[derive(Clone, Copy, Debug)]
pub struct ActualReturn {

    pub entry_rsp: u64, pub after_rsp: u64, pub target: u64, pub rax: u64, pub rbx: u64 }
pub struct TailCallReturned<R> { pub actual: ActualReturn, pub owner: R }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { MoveRax, AdjustStack, PopRbx, Return, Tail, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub wrapper_tail: Option<super::wrapper_call::History>, pub rax: Option<u64>, pub adjustment: Option<StackAdjustment>, pub pop: Option<Pop>,
    pub tail_request: Option<TailRequest>, pub actual_return: Option<ActualReturn> }
#[derive(Clone, Copy, Debug)]
pub struct History { pub path: Path, pub captures: Captures }
pub enum Fault<E> { Service(E), MissingReceipt, RaxBinding, StackBinding, ReturnBinding, OccupiedTail, AlreadyStopped }
pub struct Prepared<S: Services> { body: super::Parts<S>, path: Path, tail: Option<TailCallReturned<S::TailOwner>>,
    captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Parts<S: Services> { pub body: super::Parts<S>, pub path: Path, pub(in super::super) tail: Option<TailCallReturned<S::TailOwner>>,
    pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> { pub fn tail(&self) -> Option<&TailCallReturned<S::TailOwner>> { self.tail.as_ref() } }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn actual_return(&self) -> &ActualReturn { self.parts.captures.actual_return.as_ref().unwrap() }
    pub fn history(&self) -> History { History { path: self.parts.path, captures: self.parts.captures } }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_normal(completed: super::NormalRequired<S>) -> Self {
        let body = completed.into_parts();
        let (path, phase) = match body.actual.entry { Entry::Construct55e0a0 => (Path::ConstructNormal, Phase::MoveRax), Entry::Destroy55e0dc => (Path::DestroyNormal, Phase::AdjustStack) };
        let mut captures = Captures::default(); if path == Path::DestroyNormal { captures.rax = body.captures.rax; }
        Self { body, path, tail: None, captures, phase, attempted: None, stopped: false }
    }
    pub fn from_tail(completed: super::TailRestoreRequired<S>) -> Self {
        let body = completed.into_parts(); let mut captures = Captures::default(); captures.rax = body.captures.rax;
        Self { body, path: Path::DestroyZeroTail, tail: None, captures, phase: Phase::AdjustStack, attempted: None, stopped: false }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { body: self.body, path: self.path, tail: self.tail, captures: self.captures,
        phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub(super) fn suspend_for_wrapper(mut self) -> Result<Parts<S>, Parts<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.path != Path::DestroyZeroTail || self.phase != Phase::Tail { return Err(self.into_parts()); }
        Ok(self.into_parts())
    }

    pub(super) fn from_selected_parts(parts: Parts<S>) -> Self {
        Self { body: parts.body, path: parts.path, tail: parts.tail, captures: parts.captures,
            phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped }
    }
    pub(super) fn record_selected_tail(&mut self, request: TailRequest) -> Result<(), Fault<S::Error>> {
        let a = self.tail.as_ref().ok_or(Fault::MissingReceipt)?.actual; self.captures.actual_return = Some(a);
        if a.entry_rsp != request.entry_rsp || a.after_rsp != self.body.actual.work_rsp.wrapping_add(0x30) { return Err(Fault::ReturnBinding); }
        self.phase = Phase::Done; Ok(())
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); } let mut spent = 0;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); } self.attempted = None;
        }
        if self.phase == Phase::Done { Poll::Ready(Returned { parts: self.into_parts() }) } else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let work = self.body.actual.work_rsp; let s = &mut self.body.services; let o = &mut self.body.owners; let c = &mut self.captures;
        match self.phase {
            Phase::MoveRax => {
                let word = self.body.captures.bound_rbx.ok_or(Fault::MissingReceipt)?; s.mov_rax_rbx(o, 0x55e0d2, word, &mut c.rax).map_err(Fault::Service)?;
                if c.rax.ok_or(Fault::MissingReceipt)? != word { return Err(Fault::RaxBinding); } self.phase = Phase::AdjustStack;
            }
            Phase::AdjustStack => {
                let (pc, amount) = match self.path { Path::ConstructNormal => (0x55e0d5, 0x20), Path::DestroyNormal => (0x55e109, 0x28), Path::DestroyZeroTail => (0x55e0e7, 0x28) };
                s.add_rsp(o, pc, amount, &mut c.adjustment).map_err(Fault::Service)?; let a = c.adjustment.ok_or(Fault::MissingReceipt)?;
                if a.before != work || a.after != work.wrapping_add(amount) { return Err(Fault::StackBinding); }
                self.phase = match self.path { Path::ConstructNormal => Phase::PopRbx, Path::DestroyNormal => Phase::Return, Path::DestroyZeroTail => Phase::Tail };
            }
            Phase::PopRbx => {
                s.pop_rbx(o, 0x55e0d9, &mut c.pop).map_err(Fault::Service)?; let a = c.pop.ok_or(Fault::MissingReceipt)?;
                if a.before != work.wrapping_add(0x20) || a.after != work.wrapping_add(0x28) { return Err(Fault::StackBinding); } self.phase = Phase::Return;
            }
            Phase::Return => {
                let pc = if self.path == Path::ConstructNormal { 0x55e0da } else { 0x55e10d };
                s.ret(o, pc, &mut c.actual_return).map_err(Fault::Service)?; let a = c.actual_return.ok_or(Fault::MissingReceipt)?;
                if a.entry_rsp != work.wrapping_add(0x28) || a.after_rsp != work.wrapping_add(0x30) || Some(a.rax) != c.rax { return Err(Fault::ReturnBinding); }
                if self.path == Path::ConstructNormal && a.rbx != c.pop.ok_or(Fault::MissingReceipt)?.value { return Err(Fault::ReturnBinding); } self.phase = Phase::Done;
            }
            Phase::Tail => {
                if self.tail.is_some() { return Err(Fault::OccupiedTail); }
                let rax = c.rax.ok_or(Fault::MissingReceipt)?; if rax != 0 { return Err(Fault::RaxBinding); }
                let request = TailRequest { source_pc: 0x55e0eb, target_rva: 0x57a7fc, entry_rsp: work.wrapping_add(0x28), rcx: self.body.actual.guard_rcx, rax };
                c.tail_request = Some(request); s.tail_57a7fc(o, request, &mut self.tail).map_err(Fault::Service)?;

                self.record_selected_tail(request)?;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
