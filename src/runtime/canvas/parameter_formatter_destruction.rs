#[path = "../dsp/small_callable_destructor_leaf.rs"]
pub mod small_callable_destructor;
use small_callable_destructor::{source_small_callable_destructor_request as request,
    SourceSmallCallableDestructorEntry as Entry, SourceSmallCallableDestructorRequest as Request};
use super::super::super::super::super as function_owner;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualEntry {
    pub selected_rva: u32, pub rcx: u64, pub edx: u32, pub entry_rsp: u64, pub entry_rax: u64,
}
pub struct Input<S: Services> { pub function: S::Function, pub frame: S::Frame, pub actual: ActualEntry }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Branch { pub actual_edx: u32, pub next_pc: u32 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnedCall { pub entry_rsp: u64, pub caller_rsp: u64, pub actual_target: u64, pub raw_rax: u64 }
pub trait Services: function_owner::Services {

    fn callable_test_dl(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<Branch>) -> Result<(), Self::Error>;

    fn callable_mov_edx16(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<u32>) -> Result<(), Self::Error>;

    fn callable_inline_ret(&mut self, frame: &mut Self::Frame,
        returned: &mut Option<ReturnedCall>) -> Result<(), Self::Error>;

    fn callable_tail55da4c(&mut self, function: &mut Self::Function, frame: &mut Self::Frame,
        actual_rcx: u64, literal_edx: u32, actual_entry_rsp: u64,
        returned: &mut Option<ReturnedCall>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { TestDl, SetSize, Return, TailFree, Done }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub branch: Option<Branch>, pub edx: Option<u32>, pub actual_return: Option<ReturnedCall> }
pub enum Fault<E> { Service(E), MissingReceipt, BranchBinding, RegisterBinding, ReturnBinding, AlreadyStopped }
pub struct Rejected<S: Services> { pub services: S, pub input: Input<S> }
pub struct Prepared<S: Services> { services: S, input: Input<S>, cursor: Cursor }
pub struct Parts<S: Services> {
    pub services: S, pub input: Input<S>, pub request: Request, pub captures: Captures,
    pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool,
}
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(Returned<S>), Failed(Stopped<S>) }

pub struct Returned<S: Services> { parts: Parts<S> }
impl<S: Services> Returned<S> {
    pub fn captures(&self) -> &Captures { &self.parts.captures }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {
    pub fn from_actual_selected_entry(services: S, input: Input<S>) -> Result<Self, Rejected<S>> {
        let cursor = match Cursor::from_actual_selected_entry(input.actual) {
            Ok(cursor) => cursor, Err(_) => return Err(Rejected { services, input }),
        };
        Ok(Self { services, input, cursor })
    }
    pub fn phase(&self) -> Phase { self.cursor.phase() }
    pub fn captures(&self) -> &Captures { self.cursor.captures() }
    pub fn into_parts(self) -> Parts<S> {
        let cursor = self.cursor.into_parts();
        Parts { services: self.services, input: self.input, request: cursor.request, captures: cursor.captures,
            phase: cursor.phase, attempted: cursor.attempted, stopped: cursor.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        match self.cursor.poll(&mut self.services, &mut self.input.function, &mut self.input.frame, budget) {
            BorrowedPoll::Pending => Poll::Pending(self),
            BorrowedPoll::Ready => Poll::Ready(Returned { parts: self.into_parts() }),
            BorrowedPoll::Failed(fault) => Poll::Failed(Stopped { pending: self, fault }),
        }
    }
}

pub struct Cursor { actual: ActualEntry, request: Request, captures: Captures,
    phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct CursorParts { pub actual: ActualEntry, pub request: Request, pub captures: Captures,
    pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualEntry, pub request: Request, pub captures: Captures }
pub enum BorrowedPoll<E> { Pending, Ready, Failed(Fault<E>) }
impl Cursor {
    pub fn from_actual_selected_entry(actual: ActualEntry) -> Result<Self, ActualEntry> {
        if actual.selected_rva != 0x4ff0 { return Err(actual); }
        Ok(Self { actual, request: request(Entry::At4ff0, actual.rcx, actual.edx),
            captures: Captures::default(), phase: Phase::TestDl, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn history(&self) -> Option<History> {
        if self.phase == Phase::Done && !self.stopped {
            Some(History { actual: self.actual, request: self.request, captures: self.captures })
        } else { None }
    }
    pub fn into_parts(self) -> CursorParts {
        CursorParts { actual: self.actual, request: self.request, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll<S: Services>(&mut self, services: &mut S, function: &mut S::Function,
        frame: &mut S::Frame, budget: usize) -> BorrowedPoll<S::Error> {
        if self.stopped { return BorrowedPoll::Failed(Fault::AlreadyStopped); }
        let mut spent = 0usize;
        while self.phase != Phase::Done && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step(services, function, frame) {
                self.stopped = true; return BorrowedPoll::Failed(fault);
            }
            self.attempted = None;
        }
        if self.phase != Phase::Done { BorrowedPoll::Pending } else { BorrowedPoll::Ready }
    }
    fn check_return<E>(&self, unchanged_rax: bool) -> Result<(), Fault<E>> {
        let actual = self.captures.actual_return.ok_or(Fault::MissingReceipt)?;
        if actual.entry_rsp != self.actual.entry_rsp || actual.caller_rsp != self.actual.entry_rsp.wrapping_add(8)
            || (unchanged_rax && actual.raw_rax != self.actual.entry_rax) { return Err(Fault::ReturnBinding); }
        Ok(())
    }
    fn step<S: Services>(&mut self, services: &mut S, function: &mut S::Function, frame: &mut S::Frame) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::TestDl => {
                services.callable_test_dl(frame, &mut self.captures.branch).map_err(Fault::Service)?;
                let actual = self.captures.branch.ok_or(Fault::MissingReceipt)?;
                let next = match self.request { Request::ReturnRequired { ret_pc } => ret_pc,
                    Request::TailFree55da4cRequired { tail_pc, .. } => tail_pc - 5 };
                if actual.actual_edx != self.actual.edx || actual.next_pc != next { return Err(Fault::BranchBinding); }
                self.phase = match self.request { Request::ReturnRequired { .. } => Phase::Return,
                    Request::TailFree55da4cRequired { .. } => Phase::SetSize };
            }
            Phase::SetSize => {
                services.callable_mov_edx16(frame, &mut self.captures.edx).map_err(Fault::Service)?;
                let Request::TailFree55da4cRequired { edx, .. } = self.request else { return Err(Fault::MissingReceipt); };
                if self.captures.edx.ok_or(Fault::MissingReceipt)? != edx { return Err(Fault::RegisterBinding); }
                self.phase = Phase::TailFree;
            }
            Phase::Return => {
                services.callable_inline_ret(frame, &mut self.captures.actual_return).map_err(Fault::Service)?;
                self.check_return::<S::Error>(true)?; self.phase = Phase::Done;
            }
            Phase::TailFree => {
                let Request::TailFree55da4cRequired { rcx, edx, .. } = self.request else { return Err(Fault::MissingReceipt); };
                services.callable_tail55da4c(function, frame,
                    rcx, edx, self.actual.entry_rsp, &mut self.captures.actual_return).map_err(Fault::Service)?;
                self.check_return::<S::Error>(false)?; self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }
}
