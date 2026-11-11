use super::{function, storage, Kind, Prepared, Services};
pub use function::native_entry as leaf;

#[derive(Clone, Copy, Debug)]
pub struct ActualCall {
    pub selected_entry: function::Entry, pub call_pc: u32, pub caller_rsp: u64,

    pub dispatch_vtable: u64, pub entry: leaf::ActualEntry,
}
pub enum Admission { Parent(super::Admission), Prefix(leaf::CursorRejected) }
pub struct Rejected<S: Services + storage::NativeEntryServices> { pub parent: Prepared<S>, pub actual: ActualCall, pub fault: Admission }
pub enum Fault<E> { Prefix(leaf::Fault<E>), MissingState, AlreadyStopped }
pub struct Bound<S: Services + storage::NativeEntryServices> {
    parent: Prepared<S>, actual: ActualCall, kind: Kind, cursor: Option<leaf::Cursor>, stopped: bool,
}
pub struct Parts<S: Services + storage::NativeEntryServices> {
    pub parent: Prepared<S>, pub actual: ActualCall, pub kind: Kind,
    pub cursor: Option<leaf::CursorParts>, pub stopped: bool,
}
pub struct Stopped<S: Services + storage::NativeEntryServices> { pub pending: Bound<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services + storage::NativeEntryServices> { Pending(Bound<S>), Ready(super::Bound<S>), Failed(Stopped<S>) }
impl<S: Services + storage::NativeEntryServices> Prepared<S> {
    pub fn bind_actual_selected_move_entry(mut self, actual: ActualCall) -> Result<Bound<S>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old { return Err(Rejected { parent: self, actual, fault: Admission::Parent(super::Admission::NotPendingMove) }); }
        let s = match self.move_snapshot() {
            Ok(s) => s, Err(fault) => return Err(Rejected { parent: self, actual, fault: Admission::Parent(fault) }),
        };
        if actual.call_pc != s.kind.call_pc() || s.target != self.history.actual_image_base.wrapping_add(u64::from(actual.selected_entry.pc())) {
            return Err(Rejected { parent: self, actual, fault: Admission::Parent(super::Admission::TargetBinding) });
        }
        if actual.dispatch_vtable != s.vtable || actual.entry.rcx != s.source || actual.entry.rdx != s.destination
            || actual.entry.rsp != actual.caller_rsp.wrapping_sub(8)
            || (s.kind == Kind::Old && actual.entry.rax != s.vtable) {
            return Err(Rejected { parent: self, actual, fault: Admission::Parent(super::Admission::EntryBinding) });
        }
        let words = function::Words { source_word: s.source, destination_word: s.destination,
            actual_image_base: self.history.actual_image_base };
        let cursor = match leaf::Cursor::from_actual_entry(actual.selected_entry, words, actual.entry) {
            Ok(cursor) => cursor, Err(error) => return Err(Rejected { parent: self, actual, fault: Admission::Prefix(error) }),
        };
        Ok(Bound { parent: self, actual, kind: s.kind, cursor: Some(cursor), stopped: false })
    }
}
impl<S: Services + storage::NativeEntryServices> Bound<S> {
    pub fn captures(&self) -> Option<&leaf::Captures> { self.cursor.as_ref().map(leaf::Cursor::captures) }
    pub fn into_parts(self) -> Parts<S> { Parts { parent: self.parent, actual: self.actual, kind: self.kind,
        cursor: self.cursor.map(leaf::Cursor::into_parts), stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        if budget == 0 { return Poll::Pending(self); }
        let Some(cursor) = self.cursor.take() else {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        };
        let outcome = {
            let Some(source) = self.parent.history.functions[self.kind.function()].as_mut() else {
                self.cursor = Some(cursor); self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
            };
            let r = &mut self.parent.retained.parent;
            let mut memory = storage::Memory { services: &mut r.services };
            let mut actual = storage::Storage { source: &mut source.owner, parameter: &mut r.parameter, frame: &mut r.frame };
            cursor.poll(&mut memory, &mut actual, budget)
        };
        match outcome {
            leaf::CursorPoll::Pending(cursor) => { self.cursor = Some(cursor); Poll::Pending(self) }
            leaf::CursorPoll::Failed(stopped) => {
                self.cursor = Some(stopped.pending); self.stopped = true;
                Poll::Failed(Stopped { pending: self, fault: Fault::Prefix(stopped.fault) })
            }
            leaf::CursorPoll::Ready(entered) => {
                let actual = super::ActualBody { entry: self.actual.selected_entry, call_pc: self.actual.call_pc,
                    caller_rsp: self.actual.caller_rsp, callee_rsp: self.actual.entry.rsp,
                    source_rcx: self.actual.entry.rcx, destination_rdx: self.actual.entry.rdx,
                    dispatch_vtable: self.actual.dispatch_vtable, memory_entry: entered.memory_entry() };
                Poll::Ready(super::Bound { parent: self.parent, actual, kind: self.kind,
                    cursor: entered.into_memory_cursor(), clone: None, captures: super::Captures::default(),
                    phase: super::CallPhase::Memory, stopped: false })
            }
        }
    }
}
