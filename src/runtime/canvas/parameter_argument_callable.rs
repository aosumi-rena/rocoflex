use super::{Caller, Captures, FunctionCapture, Phase, Services};
pub use super::super::formatting::object::function::destruction as leaf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualCall {
    pub source_image_base: u64, pub call_pc: u32, pub caller_rsp: u64, pub entry: leaf::ActualEntry,
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub leaf: leaf::History }

pub enum Parent<S: Services + leaf::Services, T> {
    Constructor(super::Prepared<S>), Standalone(super::standalone::Prepared<S, T>),
}
pub enum Admission { NotPendingCall, MissingState, TargetBinding, EntryBinding }
pub struct Rejected<S: Services + leaf::Services, T> { pub parent: Parent<S, T>, pub actual: ActualCall, pub fault: Admission }
pub enum Fault<E> { Leaf(leaf::Fault<E>), MissingState, ReturnBinding, AlreadyStopped }
pub struct Bound<S: Services + leaf::Services, T> {
    parent: Parent<S, T>, actual: ActualCall, cursor: leaf::Cursor, stopped: bool,
}
pub struct Parts<S: Services + leaf::Services, T> {
    pub parent: Parent<S, T>, pub actual: ActualCall, pub cursor: leaf::CursorParts, pub stopped: bool,
}
pub struct Stopped<S: Services + leaf::Services, T> { pub pending: Bound<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services + leaf::Services, T> {
    Pending(Bound<S, T>), Ready(Parent<S, T>), Failed(Stopped<S, T>),
}
#[derive(Clone, Copy)]
struct Snapshot {
    phase: Phase, call_pc: Option<u32>, word: Option<u64>, vtable: Option<u64>,
    target: Option<u64>, delete_storage: Option<bool>,
}
fn snapshot<F>(index: usize, phase: Phase, caller: Caller, functions: &[Option<FunctionCapture<F>>; 3], c: &Captures) -> Snapshot {
    Snapshot { phase, call_pc: caller.call_pc(index), word: functions.get(index).and_then(|f| f.as_ref().map(|f| f.word)),
        vtable: c.vtables.get(index).copied().flatten(), target: c.targets.get(index).copied().flatten(),
        delete_storage: c.delete_storage.get(index).copied().flatten() }
}
impl<S: Services + leaf::Services, T> Parent<S, T> {
    fn stop(&mut self) -> bool {
        let stopped = match self { Self::Constructor(p) => &mut p.stopped, Self::Standalone(p) => &mut p.stopped };
        let old = *stopped; *stopped = true; old
    }
    fn snapshot(&self) -> Snapshot {
        match self {
            Self::Constructor(p) => snapshot(p.index, p.phase, Caller::Constructor326f, &p.owner.current_functions, &p.owner.captures),
            Self::Standalone(p) => snapshot(p.index, p.phase, Caller::Standalone39d8, &p.functions, &p.captures),
        }
    }
    fn poll_leaf(&mut self, cursor: &mut leaf::Cursor, budget: usize) -> Result<leaf::BorrowedPoll<S::Error>, ()> {
        match self {
            Self::Constructor(p) => {
                let f = p.owner.current_functions.get_mut(p.index).and_then(Option::as_mut).ok_or(())?;
                let r = &mut p.owner.constructor.parent;
                Ok(cursor.poll(&mut r.services, &mut f.owner, &mut r.frame, budget))
            }
            Self::Standalone(p) => {
                let f = p.functions.get_mut(p.index).and_then(Option::as_mut).ok_or(())?;
                Ok(cursor.poll(&mut p.input.services, &mut f.owner, &mut p.input.frame, budget))
            }
        }
    }
    fn accept_return(&mut self, history: History) {
        let (c, index, phase, attempted, stopped) = match self {
            Self::Constructor(p) => (&mut p.owner.captures, p.index, &mut p.phase, &mut p.attempted, &mut p.stopped),
            Self::Standalone(p) => (&mut p.captures, p.index, &mut p.phase, &mut p.attempted, &mut p.stopped),
        };
        c.selected_callable[index] = Some(history); c.call_returned[index] = true;
        *phase = Phase::ClearSlot; *attempted = None; *stopped = false;
    }
}
impl<S: Services + leaf::Services> super::Prepared<S> {
    pub fn bind_actual_argument_4ff0(self, actual: ActualCall) -> Result<Bound<S, ()>, Rejected<S, ()>> {
        bind(Parent::Constructor(self), actual)
    }
}
impl<S: Services + leaf::Services, T> super::standalone::Prepared<S, T> {
    pub fn bind_actual_argument_4ff0(self, actual: ActualCall) -> Result<Bound<S, T>, Rejected<S, T>> {
        bind(Parent::Standalone(self), actual)
    }
}
fn bind<S: Services + leaf::Services, T>(mut parent: Parent<S, T>, actual: ActualCall) -> Result<Bound<S, T>, Rejected<S, T>> {
    let already_stopped = parent.stop();
    let s = parent.snapshot();
    if already_stopped || s.phase != Phase::Call {
        return Err(Rejected { parent, actual, fault: Admission::NotPendingCall });
    }
    let (Some(word), Some(vtable), Some(target), Some(delete_storage), Some(call_pc)) =
        (s.word, s.vtable, s.target, s.delete_storage, s.call_pc) else {
        return Err(Rejected { parent, actual, fault: Admission::MissingState });
    };
    if actual.call_pc != call_pc || target != actual.source_image_base.wrapping_add(0x4ff0) {
        return Err(Rejected { parent, actual, fault: Admission::TargetBinding });
    }
    if actual.entry.selected_rva != 0x4ff0 || actual.entry.rcx != word || actual.entry.entry_rax != vtable
        || actual.entry.edx & 0xff != u32::from(delete_storage)
        || actual.entry.entry_rsp != actual.caller_rsp.wrapping_sub(8) {
        return Err(Rejected { parent, actual, fault: Admission::EntryBinding });
    }
    let cursor = match leaf::Cursor::from_actual_selected_entry(actual.entry) {
        Ok(cursor) => cursor, Err(_) => return Err(Rejected { parent, actual, fault: Admission::EntryBinding }),
    };
    Ok(Bound { parent, actual, cursor, stopped: false })
}
impl<S: Services + leaf::Services, T> Bound<S, T> {
    pub fn captures(&self) -> &leaf::Captures { self.cursor.captures() }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { parent: self.parent, actual: self.actual, cursor: self.cursor.into_parts(), stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S, T> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let outcome = match self.parent.poll_leaf(&mut self.cursor, budget) {
            Ok(outcome) => outcome,
            Err(()) => { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState }); }
        };
        match outcome {
            leaf::BorrowedPoll::Pending => Poll::Pending(self),
            leaf::BorrowedPoll::Failed(error) => {
                self.stopped = true; Poll::Failed(Stopped { pending: self, fault: Fault::Leaf(error) })
            }
            leaf::BorrowedPoll::Ready => {
                let Some(history) = self.cursor.history() else {
                    self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
                };
                let Some(returned) = history.captures.actual_return else {
                    self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
                };

                if returned.caller_rsp != self.actual.caller_rsp
                    || returned.actual_target != self.actual.source_image_base.wrapping_add(self.actual.call_pc as u64).wrapping_add(3) {
                    self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::ReturnBinding });
                }
                self.parent.accept_return(History { actual: self.actual, leaf: history });
                Poll::Ready(self.parent)
            }
        }
    }
}
