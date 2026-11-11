use super::{attributes, function, storage, Parent};
pub use function::native_entry as leaf;

#[derive(Clone, Copy, Debug)]
pub struct ActualCall {
    pub actual_image_base: u64, pub selected_entry: function::Entry,
    pub call_pc: u32, pub caller_rsp: u64, pub entry: leaf::ActualEntry,
}
pub enum Admission { Parent(super::Admission), Prefix(leaf::CursorRejected) }
pub struct Rejected<S: attributes::Services + storage::NativeEntryServices> {
    pub parent: Parent<S>, pub actual: ActualCall, pub fault: Admission,
}
pub enum Fault<E> { Prefix(leaf::Fault<E>), MissingState, AlreadyStopped }
pub struct Bound<S: attributes::Services + storage::NativeEntryServices> {
    parent: Parent<S>, actual: ActualCall, cursor: Option<leaf::Cursor>, stopped: bool,
}
pub struct Parts<S: attributes::Services + storage::NativeEntryServices> {
    pub parent: Parent<S>, pub actual: ActualCall, pub cursor: Option<leaf::CursorParts>, pub stopped: bool,
}
pub struct Stopped<S: attributes::Services + storage::NativeEntryServices> { pub pending: Bound<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: attributes::Services + storage::NativeEntryServices> { Pending(Bound<S>), Ready(super::Bound<S>), Failed(Stopped<S>) }
impl<S: attributes::Services + storage::NativeEntryServices> super::super::Prepared<S> {
    pub fn bind_actual_selected_clone_entry(self, actual: ActualCall) -> Result<Bound<S>, Rejected<S>> {
        bind(Parent::Members(self), actual)
    }
}
impl<S: attributes::Services + storage::NativeEntryServices> attributes::Prepared<S> {
    pub fn bind_actual_selected_clone_entry(self, actual: ActualCall) -> Result<Bound<S>, Rejected<S>> {
        bind(Parent::Attributes(self), actual)
    }
}
fn bind<S: attributes::Services + storage::NativeEntryServices>(mut parent: Parent<S>, actual: ActualCall)
    -> Result<Bound<S>, Rejected<S>> {
    if parent.latch() {
        return Err(Rejected { parent, actual, fault: Admission::Parent(super::Admission::NotPendingClone) });
    }
    let s = match parent.snapshot() { Ok(s) => s, Err(fault) => return Err(Rejected { parent, actual, fault: Admission::Parent(fault) }) };
    if actual.call_pc != s.call_pc || s.target != actual.actual_image_base.wrapping_add(u64::from(actual.selected_entry.pc())) {
        return Err(Rejected { parent, actual, fault: Admission::Parent(super::Admission::TargetBinding) });
    }

    if actual.entry.rax != s.vtable || actual.entry.rcx != s.source || actual.entry.rdx != s.destination
        || actual.entry.rsp != actual.caller_rsp.wrapping_sub(8) {
        return Err(Rejected { parent, actual, fault: Admission::Parent(super::Admission::EntryBinding) });
    }
    let words = function::Words { source_word: s.source, destination_word: s.destination, actual_image_base: actual.actual_image_base };
    let cursor = match leaf::Cursor::from_actual_entry(actual.selected_entry, words, actual.entry) {
        Ok(cursor) => cursor,
        Err(error) => return Err(Rejected { parent, actual, fault: Admission::Prefix(error) }),
    };
    Ok(Bound { parent, actual, cursor: Some(cursor), stopped: false })
}
impl<S: attributes::Services + storage::NativeEntryServices> Bound<S> {
    pub fn captures(&self) -> Option<&leaf::Captures> { self.cursor.as_ref().map(leaf::Cursor::captures) }
    pub fn into_parts(self) -> Parts<S> { Parts { parent: self.parent, actual: self.actual,
        cursor: self.cursor.map(leaf::Cursor::into_parts), stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        if budget == 0 { return Poll::Pending(self); }
        let Some(cursor) = self.cursor.take() else {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        };
        let outcome = {
            let Some(r) = self.parent.resources() else {
                self.cursor = Some(cursor); self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
            };
            let mut memory = storage::Memory { services: r.services };
            let mut actual = storage::Storage { source: &mut r.source.owner, parameter: r.parameter, frame: r.frame };
            cursor.poll(&mut memory, &mut actual, budget)
        };
        match outcome {
            leaf::CursorPoll::Pending(cursor) => { self.cursor = Some(cursor); Poll::Pending(self) }
            leaf::CursorPoll::Failed(stopped) => {
                self.cursor = Some(stopped.pending); self.stopped = true;
                Poll::Failed(Stopped { pending: self, fault: Fault::Prefix(stopped.fault) })
            }
            leaf::CursorPoll::Ready(entered) => {

                let actual = super::ActualCall { actual_image_base: self.actual.actual_image_base,
                    body: super::ActualBody { entry: self.actual.selected_entry, call_pc: self.actual.call_pc,
                        caller_rsp: self.actual.caller_rsp, callee_rsp: self.actual.entry.rsp,
                        source_rcx: self.actual.entry.rcx, destination_rdx: self.actual.entry.rdx,
                        dispatch_vtable: self.actual.entry.rax, memory_entry: entered.memory_entry() } };
                Poll::Ready(super::Bound { parent: self.parent, actual, return_pc: self.actual.call_pc + 2,
                    cursor: entered.into_memory_cursor(), clone: None, captures: super::Captures::default(),
                    phase: super::CallPhase::Memory, stopped: false })
            }
        }
    }
}
