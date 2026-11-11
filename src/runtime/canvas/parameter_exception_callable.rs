use super::{metadata::Action, Phase, Prepared};
pub use super::super::functions::attributes::formatting::object::function::destruction as leaf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualCall { pub source_image_base: u64, pub call_pc: u32, pub entry: leaf::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub leaf: leaf::History }
pub enum Admission { NotPendingMemberCall, MissingState, TargetBinding, EntryBinding }
pub struct Rejected<S: super::Services + leaf::Services, T> {
    pub pending: Prepared<S, T>, pub actual: ActualCall, pub fault: Admission,
}
pub enum Fault<E> { Leaf(leaf::Fault<E>), MissingState, ReturnBinding, AlreadyStopped }
pub struct Bound<S: super::Services + leaf::Services, T> {
    parent: Prepared<S, T>, actual: ActualCall, cursor: leaf::Cursor, stopped: bool,
}
pub struct Parts<S: super::Services + leaf::Services, T> {

    pub parent: Prepared<S, T>, pub actual: ActualCall, pub cursor: leaf::CursorParts, pub stopped: bool,
}
pub struct Stopped<S: super::Services + leaf::Services, T> { pub pending: Bound<S, T>, pub fault: Fault<S::Error> }
pub enum Poll<S: super::Services + leaf::Services, T> {
    Pending(Bound<S, T>), Ready(Prepared<S, T>), Failed(Stopped<S, T>),
}
impl<S: super::Services + leaf::Services, T> Prepared<S, T> {

    pub fn bind_actual_member_4ff0(mut self, actual: ActualCall) -> Result<Bound<S, T>, Rejected<S, T>> {
        let already_stopped = self.stopped;
        self.stopped = true;
        let action = self.stage.profile.actions.get(self.action).copied().flatten();
        let Some(Action::Member(member)) = action else {
            return Err(Rejected { pending: self, actual, fault: Admission::NotPendingMemberCall });
        };
        if already_stopped || self.phase != Phase::CallMember {
            return Err(Rejected { pending: self, actual, fault: Admission::NotPendingMemberCall });
        }
        let i = self.action;
        let Some(function) = self.stage.functions[i].as_ref() else {
            return Err(Rejected { pending: self, actual, fault: Admission::MissingState });
        };
        let Some(delete_storage) = self.stage.captures.delete_storage[i] else {
            return Err(Rejected { pending: self, actual, fault: Admission::MissingState });
        };
        if actual.call_pc != member.call_pc
            || self.stage.captures.targets[i] != Some(actual.source_image_base.wrapping_add(0x4ff0)) {
            return Err(Rejected { pending: self, actual, fault: Admission::TargetBinding });
        }
        if actual.entry.selected_rva != 0x4ff0 || actual.entry.rcx != function.word
            || actual.entry.edx & 0xff != u32::from(delete_storage)
            || actual.entry.entry_rsp != self.stage.actual.funclet_work_rsp.wrapping_sub(8)
            || self.stage.captures.vtables[i] != Some(actual.entry.entry_rax) {
            return Err(Rejected { pending: self, actual, fault: Admission::EntryBinding });
        }
        let cursor = match leaf::Cursor::from_actual_selected_entry(actual.entry) {
            Ok(cursor) => cursor,
            Err(_) => return Err(Rejected { pending: self, actual, fault: Admission::EntryBinding }),
        };
        Ok(Bound { parent: self, actual, cursor, stopped: false })
    }
}
impl<S: super::Services + leaf::Services, T> Bound<S, T> {
    pub fn captures(&self) -> &leaf::Captures { self.cursor.captures() }
    pub fn into_parts(self) -> Parts<S, T> {
        Parts { parent: self.parent, actual: self.actual, cursor: self.cursor.into_parts(), stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S, T> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let i = self.parent.action;
        let Some(function) = self.parent.stage.functions[i].as_mut() else {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        };
        let r = &mut self.parent.resources;
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

                if returned.caller_rsp != self.parent.stage.actual.funclet_work_rsp
                    || returned.actual_target != self.actual.source_image_base.wrapping_add(self.actual.call_pc as u64).wrapping_add(3) {
                    self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::ReturnBinding });
                }
                self.parent.stage.selected_callable[i] = Some(History { actual: self.actual, leaf: history });
                self.parent.stage.captures.member_call_returned[i] = true;
                self.parent.phase = Phase::ReloadAfterCall;
                self.parent.attempted = None;
                self.parent.stopped = false;
                Poll::Ready(self.parent)
            }
        }
    }
}
