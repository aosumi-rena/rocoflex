use super::{function, CloneReturn, Phase, Prepared, Services};
pub use function::selected_storage as storage;
#[path = "parameter_formatter_move_entry.rs"]
pub mod native_entry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Old, New }
impl Kind {
    fn index(self) -> usize { match self { Self::Old => 0, Self::New => 1 } }
    fn function(self) -> usize { match self { Self::Old => 0, Self::New => 5 } }
    fn call_pc(self) -> u32 { match self { Self::Old => 0x2b3579, Self::New => 0x2b35c6 } }
    fn return_pc(self) -> u32 { match self { Self::Old => 0x2b357c, Self::New => 0x2b35ca } }
    fn after_call(self) -> Phase { match self { Self::Old => Phase::StoreOldMoveResult, Self::New => Phase::StoreNewMember } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualBody {
    pub entry: function::Entry, pub call_pc: u32, pub caller_rsp: u64, pub callee_rsp: u64,

    pub source_rcx: u64, pub destination_rdx: u64, pub dispatch_vtable: u64,
    pub memory_entry: function::ActualMemoryEntry,
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualBody, pub body: function::History, pub returned: storage::ActualReturn }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures { pub body: Option<function::History>, pub request: Option<storage::ReturnRequest>, pub returned: Option<storage::ActualReturn> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallPhase { Memory, NativeReturn }
pub enum Admission { NotPendingMove, MissingState, TargetBinding, EntryBinding }
pub struct Rejected<S: Services + storage::Services> { pub parent: Prepared<S>, pub actual: ActualBody, pub fault: Admission }
pub enum Fault<E> { Memory(function::Fault<E>), Service(E), MissingState, ReturnBinding, CloneBinding { expected: u64, received: u64 }, AlreadyStopped }
pub struct Bound<S: Services + storage::Services> {
    parent: Prepared<S>, actual: ActualBody, kind: Kind, cursor: function::Cursor,
    clone: Option<CloneReturn<S::CloneOwner>>, captures: Captures, phase: CallPhase, stopped: bool,
}
pub struct Parts<S: Services + storage::Services> {
    pub parent: Prepared<S>, pub actual: ActualBody, pub kind: Kind, pub cursor: function::CursorParts,
    pub clone: Option<CloneReturn<S::CloneOwner>>, pub captures: Captures, pub phase: CallPhase, pub stopped: bool,
}
pub struct Stopped<S: Services + storage::Services> { pub pending: Bound<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services + storage::Services> { Pending(Bound<S>), Ready(Prepared<S>), Failed(Stopped<S>) }
struct Snapshot { kind: Kind, source: u64, destination: u64, vtable: u64, target: u64 }
impl<S: Services + storage::Services> Prepared<S> {
    fn move_snapshot(&self) -> Result<Snapshot, Admission> {
        let kind = match self.phase { Phase::MoveOld => Kind::Old, Phase::MoveNew => Kind::New,
            _ => return Err(Admission::NotPendingMove) };
        let i = kind.index();
        Ok(Snapshot { kind, source: self.history.functions[kind.function()].as_ref().ok_or(Admission::MissingState)?.word,
            destination: self.history.captures.move_destinations[i].ok_or(Admission::MissingState)?,
            vtable: match kind { Kind::Old => self.history.captures.old_move_vtable,
                Kind::New => self.history.captures.new_move_vtable }.ok_or(Admission::MissingState)?,
            target: self.history.captures.move_targets[i].ok_or(Admission::MissingState)? })
    }
    pub fn bind_actual_selected_move(mut self, actual: ActualBody) -> Result<Bound<S>, Rejected<S>> {
        let already_stopped = self.stopped;
        self.stopped = true;
        if already_stopped { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingMove }); }
        let s = match self.move_snapshot() {
            Ok(s) => s, Err(fault) => return Err(Rejected { parent: self, actual, fault }),
        };
        if actual.call_pc != s.kind.call_pc() || s.target != self.history.actual_image_base.wrapping_add(u64::from(actual.entry.pc())) {
            return Err(Rejected { parent: self, actual, fault: Admission::TargetBinding });
        }
        let words = function::Words { source_word: s.source, destination_word: s.destination,
            actual_image_base: self.history.actual_image_base };
        if actual.source_rcx != s.source || actual.destination_rdx != s.destination
            || s.vtable != actual.dispatch_vtable || actual.callee_rsp != actual.caller_rsp.wrapping_sub(8)
            || !actual.entry.matches_memory_entry(words, actual.memory_entry) {
            return Err(Rejected { parent: self, actual, fault: Admission::EntryBinding });
        }
        let cursor = function::Cursor::from_actual_selected_body(actual.entry, words);
        Ok(Bound { parent: self, actual, kind: s.kind, cursor, clone: None, captures: Captures::default(), phase: CallPhase::Memory, stopped: false })
    }
}
impl<S: Services + storage::Services> Bound<S> {
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn memory_captures(&self) -> &function::Captures { self.cursor.captures() }
    pub fn into_parts(self) -> Parts<S> { Parts { parent: self.parent, actual: self.actual, kind: self.kind,
        cursor: self.cursor.into_parts(), clone: self.clone, captures: self.captures, phase: self.phase, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        if budget == 0 { return Poll::Pending(self); }
        let Some(source) = self.parent.history.functions[self.kind.function()].as_mut() else {
            self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        };
        let r = &mut self.parent.retained.parent;
        if self.phase == CallPhase::Memory {
            let outcome = {
                let mut memory = storage::Memory { services: &mut r.services };
                let mut actual = storage::Storage { source: &mut source.owner, parameter: &mut r.parameter, frame: &mut r.frame };
                self.cursor.poll(&mut memory, &mut actual, budget)
            };
            match outcome {
                function::BorrowedPoll::Pending => Poll::Pending(self),
                function::BorrowedPoll::Failed(error) => {
                    self.stopped = true; Poll::Failed(Stopped { pending: self, fault: Fault::Memory(error) })
                }
                function::BorrowedPoll::Ready => {
                    self.captures.body = self.cursor.history();
                    if self.captures.body.is_none() {
                        self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
                    }
                    self.phase = CallPhase::NativeReturn; Poll::Pending(self)
                }
            }
        } else {
            let Some(body) = self.captures.body else {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
            };
            let request = storage::ReturnRequest { entry: self.actual.entry, ret_pc: self.actual.entry.return_pc(),
                entry_rsp: self.actual.callee_rsp, source_word: body.words.source_word, destination_word: body.words.destination_word };
            self.captures.request = Some(request);
            if let Err(error) = r.services.selected_actual_return(&mut source.owner, &mut r.parameter, &mut r.frame,
                request, body, &mut self.clone, &mut self.captures.returned) {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::Service(error) });
            }
            let (Some(returned), Some(cloned)) = (self.captures.returned, self.clone.as_ref()) else {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
            };
            if returned.ret_pc != request.ret_pc || returned.entry_rsp != request.entry_rsp || returned.caller_rsp != self.actual.caller_rsp
                || returned.rax != request.destination_word
                || returned.target != self.parent.history.actual_image_base.wrapping_add(self.kind.return_pc() as u64) {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::ReturnBinding });
            }
            if cloned.word != request.destination_word {
                let received = cloned.word; self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault: Fault::CloneBinding { expected: request.destination_word, received } });
            }
            let i = self.kind.index();
            self.parent.history.moves[i] = self.clone;
            self.parent.history.captures.selected_moves[i] = Some(History { actual: self.actual, body, returned });
            self.parent.history.captures.move_returned[i] = true;
            self.parent.phase = self.kind.after_call(); self.parent.attempted = None; self.parent.stopped = false;
            Poll::Ready(self.parent)
        }
    }
}
