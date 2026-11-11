use super::{attributes, CloneReturn, FunctionCapture, Member, Phase, Prepared};
pub use attributes::formatting::object::function;
pub use function::selected_storage as storage;

pub use attributes::formatting::object::selected_move::ActualBody;
#[path = "parameter_function_clone_entry.rs"]
pub mod native_entry;

#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub actual_image_base: u64, pub body: ActualBody }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub body: function::History, pub returned: storage::ActualReturn }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub body: Option<function::History>, pub request: Option<storage::ReturnRequest>,
    pub returned: Option<storage::ActualReturn>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallPhase { Memory, NativeReturn }
pub enum Admission { NotPendingClone, MissingState, TargetBinding, EntryBinding }

pub enum Parent<S: attributes::Services> { Members(Prepared<S>), Attributes(attributes::Prepared<S>) }
pub struct Rejected<S: attributes::Services + storage::Services> {
    pub parent: Parent<S>, pub actual: ActualCall, pub fault: Admission,
}
pub enum Fault<E> {
    Memory(function::Fault<E>), Service(E), MissingState, ReturnBinding,
    CloneBinding { expected: u64, received: u64 }, AlreadyStopped,
}
pub struct Bound<S: attributes::Services + storage::Services> {
    parent: Parent<S>, actual: ActualCall, return_pc: u32, cursor: function::Cursor,
    clone: Option<CloneReturn<S::CloneOwner>>, captures: Captures, phase: CallPhase, stopped: bool,
}
pub struct Parts<S: attributes::Services + storage::Services> {
    pub parent: Parent<S>, pub actual: ActualCall, pub cursor: function::CursorParts,
    pub clone: Option<CloneReturn<S::CloneOwner>>, pub captures: Captures, pub phase: CallPhase, pub stopped: bool,
}
pub struct Stopped<S: attributes::Services + storage::Services> { pub pending: Bound<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: attributes::Services + storage::Services> { Pending(Bound<S>), Ready(Parent<S>), Failed(Stopped<S>) }

struct Snapshot { source: u64, destination: u64, vtable: u64, target: u64, call_pc: u32 }
struct Resources<'a, S: attributes::Services> where S::Function: 'a, S::Parameter: 'a, S::Frame: 'a {
    services: &'a mut S, source: &'a mut FunctionCapture<S::Function>,
    parameter: &'a mut S::Parameter, frame: &'a mut S::Frame,
}
impl<S: attributes::Services> Parent<S> {
    fn latch(&mut self) -> bool {
        let stopped = match self { Self::Members(p) => &mut p.stopped, Self::Attributes(p) => &mut p.stopped };
        let old = *stopped; *stopped = true; old
    }
    fn snapshot(&self) -> Result<Snapshot, Admission> {
        match self {
            Self::Members(p) => {
                if p.phase != Phase::Clone { return Err(Admission::NotPendingClone); }
                let i = p.member.index();
                let destination = if p.member == Member::Third { p.captures.inline_addresses[i] }
                    else { p.captures.clone_destinations[i] };
                Ok(Snapshot {
                    source: p.sources[i].as_ref().ok_or(Admission::MissingState)?.word,
                    destination: destination.ok_or(Admission::MissingState)?,
                    vtable: p.captures.vtables[i].ok_or(Admission::MissingState)?,
                    target: p.captures.targets[i].ok_or(Admission::MissingState)?, call_pc: p.member.clone_pc(),
                })
            }
            Self::Attributes(p) => {
                if p.phase != attributes::Phase::Clone { return Err(Admission::NotPendingClone); }
                let i = p.index;
                if i > 1 { return Err(Admission::MissingState); }
                Ok(Snapshot {
                    source: p.retained.sources[i].as_ref().ok_or(Admission::MissingState)?.word,
                    destination: p.retained.captures.inline[i].ok_or(Admission::MissingState)?,
                    vtable: p.retained.captures.vtables[i].ok_or(Admission::MissingState)?,
                    target: p.retained.captures.targets[i].ok_or(Admission::MissingState)?,
                    call_pc: if i == 0 { 0x2b31fd } else { 0x2b323e },
                })
            }
        }
    }
    fn resources(&mut self) -> Option<Resources<'_, S>> {
        match self {
            Self::Members(p) => Some(Resources { services: &mut p.services,
                source: p.sources[p.member.index()].as_mut()?, parameter: &mut p.parameter, frame: &mut p.frame }),
            Self::Attributes(p) => {
                let r = &mut p.retained;
                Some(Resources { services: &mut r.parent.services, source: r.sources.get_mut(p.index)?.as_mut()?,
                    parameter: &mut r.parent.parameter, frame: &mut r.parent.frame })
            }
        }
    }
    fn accept(&mut self, cloned: CloneReturn<S::CloneOwner>, history: History) {
        match self {
            Self::Members(p) => {
                let i = p.member.index();
                p.captures.clone_destinations[i] = Some(history.body.words.destination_word);
                p.clones[i] = Some(cloned); p.captures.selected_clones[i] = Some(history);
                p.record_clone_return(); p.attempted = None; p.stopped = false;
            }
            Self::Attributes(p) => {
                let word = cloned.word; let i = p.index;
                p.retained.clones[i] = Some(cloned); p.retained.captures.selected_clones[i] = Some(history);
                p.record_clone_return(word); p.attempted = None; p.stopped = false;
            }
        }
    }
}
impl<S: attributes::Services + storage::Services> Prepared<S> {
    pub fn bind_actual_selected_clone(self, actual: ActualCall) -> Result<Bound<S>, Rejected<S>> {
        bind(Parent::Members(self), actual)
    }
}
impl<S: attributes::Services + storage::Services> attributes::Prepared<S> {
    pub fn bind_actual_selected_clone(self, actual: ActualCall) -> Result<Bound<S>, Rejected<S>> {
        bind(Parent::Attributes(self), actual)
    }
}
fn bind<S: attributes::Services + storage::Services>(mut parent: Parent<S>, actual: ActualCall) -> Result<Bound<S>, Rejected<S>> {
    if parent.latch() { return Err(Rejected { parent, actual, fault: Admission::NotPendingClone }); }
    let snapshot = match parent.snapshot() {
        Ok(snapshot) => snapshot,
        Err(fault) => return Err(Rejected { parent, actual, fault }),
    };
    let a = actual.body;
    if a.call_pc != snapshot.call_pc || snapshot.target != actual.actual_image_base.wrapping_add(u64::from(a.entry.pc())) {
        return Err(Rejected { parent, actual, fault: Admission::TargetBinding });
    }
    let words = function::Words { source_word: snapshot.source, destination_word: snapshot.destination,
        actual_image_base: actual.actual_image_base };
    if a.source_rcx != snapshot.source || a.destination_rdx != snapshot.destination
        || a.dispatch_vtable != snapshot.vtable || a.callee_rsp != a.caller_rsp.wrapping_sub(8)
        || !a.entry.matches_memory_entry(words, a.memory_entry) {
        return Err(Rejected { parent, actual, fault: Admission::EntryBinding });
    }

    Ok(Bound { parent, actual, return_pc: snapshot.call_pc + 2,
        cursor: function::Cursor::from_actual_selected_body(a.entry, words), clone: None,
        captures: Captures::default(), phase: CallPhase::Memory, stopped: false })
}
impl<S: attributes::Services + storage::Services> Bound<S> {
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn memory_captures(&self) -> &function::Captures { self.cursor.captures() }
    pub fn into_parts(self) -> Parts<S> { Parts { parent: self.parent, actual: self.actual,
        cursor: self.cursor.into_parts(), clone: self.clone, captures: self.captures, phase: self.phase, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        if budget == 0 { return Poll::Pending(self); }
        if self.phase == CallPhase::Memory {
            let outcome = {
                let Some(r) = self.parent.resources() else {
                    self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
                };
                let mut memory = storage::Memory { services: r.services };
                let mut actual = storage::Storage { source: &mut r.source.owner, parameter: r.parameter, frame: r.frame };
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
            let request = storage::ReturnRequest { entry: self.actual.body.entry, ret_pc: self.actual.body.entry.return_pc(),
                entry_rsp: self.actual.body.callee_rsp, source_word: body.words.source_word, destination_word: body.words.destination_word };
            self.captures.request = Some(request);
            let outcome = {
                let Some(r) = self.parent.resources() else {
                    self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
                };
                r.services.selected_actual_return(&mut r.source.owner, r.parameter, r.frame,
                    request, body, &mut self.clone, &mut self.captures.returned)
            };
            if let Err(error) = outcome {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::Service(error) });
            }
            let (Some(returned), Some(cloned)) = (self.captures.returned, self.clone.as_ref()) else {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
            };
            if returned.ret_pc != request.ret_pc || returned.entry_rsp != request.entry_rsp
                || returned.caller_rsp != self.actual.body.caller_rsp || returned.rax != request.destination_word
                || returned.target != self.actual.actual_image_base.wrapping_add(self.return_pc as u64) {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::ReturnBinding });
            }
            if cloned.word != request.destination_word {
                let received = cloned.word; self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault: Fault::CloneBinding { expected: request.destination_word, received } });
            }
            let Some(cloned) = self.clone.take() else {
                self.stopped = true; return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
            };
            self.parent.accept(cloned, History { actual: self.actual, body, returned });
            Poll::Ready(self.parent)
        }
    }
}
