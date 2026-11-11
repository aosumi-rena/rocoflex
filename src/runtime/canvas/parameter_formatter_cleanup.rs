use super::{Cleanup, Phase, Prepared, Services};
pub use super::function::destruction as leaf;

pub use super::super::super::retirement::selected_callable::{ActualCall, History};

pub enum Admission { NotPendingCleanup, MissingState, TargetBinding, EntryBinding }
pub struct Rejected<S: Services + leaf::Services> { pub parent: Prepared<S>, pub actual: ActualCall, pub fault: Admission }
pub enum Fault<E> { Leaf(leaf::Fault<E>), MissingState, ReturnBinding, AlreadyStopped }
pub struct Bound<S: Services + leaf::Services> {
    parent: Prepared<S>, actual: ActualCall, kind: Cleanup, cursor: leaf::Cursor, stopped: bool,
}
pub struct Parts<S: Services + leaf::Services> {
    pub parent: Prepared<S>, pub actual: ActualCall, pub kind: Cleanup, pub cursor: leaf::CursorParts, pub stopped: bool,
}
pub struct Stopped<S: Services + leaf::Services> { pub pending: Bound<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services + leaf::Services> { Pending(Bound<S>), Ready(Prepared<S>), Failed(Stopped<S>) }
impl<S: Services + leaf::Services> Prepared<S> {

    pub fn bind_actual_cleanup_4ff0(mut self, actual: ActualCall) -> Result<Bound<S>, Rejected<S>> {
        let already_stopped = self.stopped;
        self.stopped = true;
        if already_stopped || self.phase != Phase::CleanupCall {
            return Err(Rejected { parent: self, actual, fault: Admission::NotPendingCleanup });
        }
        let Some(kind) = self.cleanup else {
            return Err(Rejected { parent: self, actual, fault: Admission::MissingState });
        };
        let i = kind.index();
        let Some(function) = self.history.functions[kind.function()].as_ref() else {
            return Err(Rejected { parent: self, actual, fault: Admission::MissingState });
        };
        let Some(delete_storage) = self.history.captures.cleanup_delete[i] else {
            return Err(Rejected { parent: self, actual, fault: Admission::MissingState });
        };
        if actual.source_image_base != self.history.actual_image_base || actual.call_pc != kind.pc()
            || self.history.captures.cleanup_targets[i] != Some(actual.source_image_base.wrapping_add(0x4ff0)) {
            return Err(Rejected { parent: self, actual, fault: Admission::TargetBinding });
        }
        if actual.entry.selected_rva != 0x4ff0 || actual.entry.rcx != function.word
            || actual.entry.edx & 0xff != u32::from(delete_storage)
            || self.history.captures.cleanup_vtables[i] != Some(actual.entry.entry_rax)
            || actual.entry.entry_rsp != actual.caller_rsp.wrapping_sub(8) {
            return Err(Rejected { parent: self, actual, fault: Admission::EntryBinding });
        }
        let cursor = match leaf::Cursor::from_actual_selected_entry(actual.entry) {
            Ok(cursor) => cursor, Err(_) => return Err(Rejected { parent: self, actual, fault: Admission::EntryBinding }),
        };
        Ok(Bound { parent: self, actual, kind, cursor, stopped: false })
    }
}
impl<S: Services + leaf::Services> Bound<S> {
    pub fn captures(&self) -> &leaf::Captures { self.cursor.captures() }
    pub fn into_parts(self) -> Parts<S> {
        Parts { parent: self.parent, actual: self.actual, kind: self.kind, cursor: self.cursor.into_parts(), stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let Some(function) = self.parent.history.functions[self.kind.function()].as_mut() else {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        };
        let r = &mut self.parent.retained.parent;
        match self.cursor.poll(&mut r.services, &mut function.owner, &mut r.frame, budget) {
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
                let i = self.kind.index();
                self.parent.history.captures.selected_cleanup[i] = Some(History { actual: self.actual, leaf: history });
                self.parent.history.captures.cleanup_returned[i] = true;
                self.parent.cleanup = None;
                self.parent.phase = self.kind.after_call();
                self.parent.attempted = None; self.parent.stopped = false;
                Poll::Ready(self.parent)
            }
        }
    }
}
