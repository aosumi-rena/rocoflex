use super as r;
use super::super as i;
#[path = "parameter_stream_guard_resolver_scope.rs"]
mod scope;
#[path = "parameter_stream_guard_resolver_finish.rs"]
mod finish;
#[path = "parameter_stream_guard_resolver_call_entry.rs"]
pub mod native_entry;
#[path = "parameter_stream_guard_resolver_protected_scope.rs"]
pub mod protected_scope;
pub use scope::{Scope, Services};
pub use finish::{Supplied, ReturnFault, ReturnRejected};
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub source_pc: u32, pub child: r::ActualBody }
#[derive(Clone, Copy, Debug)]
pub enum Request { Call(i::CallRequest), Tail(i::TailRequest) }
impl Request {
    fn entry_rsp(self) -> u64 { match self { Self::Call(a) => a.entry_rsp, Self::Tail(a) => a.entry_rsp } }
    fn pc(self) -> u32 { match self { Self::Call(a) => a.pc, Self::Tail(a) => a.pc } }
    fn arguments(self) -> i::Arguments { match self { Self::Call(a) => a.arguments, Self::Tail(a) => a.arguments } }
    fn index(self) -> usize { match self { Self::Call(a) => a.index, Self::Tail(_) => 10 } }
}
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: Request, pub body: r::Captures, pub native_return: r::native_return::History }
pub struct ParentState<S: Services> { owners: i::Owners<S>, actual: i::ActualEntry, captures: i::Captures,
    rax: u64, phase: i::Phase, row: usize, argument: usize, attempted: Option<(i::Phase, usize, usize)>,
    binding: ActualCall, request: Request }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> i::Prepared<S> {
        i::Prepared { services, owners: self.owners, actual: self.actual, captures: self.captures,
            rax: self.rax, phase: self.phase, row: self.row, argument: self.argument, attempted: self.attempted, stopped: true }
    }
}
pub enum Admission<E> { NotPending, Request(i::Fault<E>), CallBinding, BodyEntry }
pub struct Rejected<S: Services> { pub parent: i::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> i::Prepared<S> {
    fn suspend_resolver(&mut self) -> Result<Request, Admission<S::Error>> {
        let old = self.stopped; self.stopped = true;
        if old || !matches!(self.phase, i::Phase::Call | i::Phase::Tail) { return Err(Admission::NotPending); }
        let request = match self.phase {
            i::Phase::Call => Request::Call(self.resolver_call_request().map_err(Admission::Request)?),
            _ => Request::Tail(self.resolver_tail_request().map_err(Admission::Request)?),
        };
        match request { Request::Call(a) => self.captures.call_requests[a.index] = Some(a), Request::Tail(a) => self.captures.tail_request = Some(a) }
        Ok(request)
    }
    fn into_resolver_scope(self, binding: ActualCall, request: Request) -> (Scope<S>, ParentState<S>) {
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures,
            rax: self.rax, phase: self.phase, row: self.row, argument: self.argument, attempted: self.attempted, binding, request };
        (Scope { services: self.services }, storage)
    }

    pub fn bind_actual_resolver_body(mut self, actual: ActualCall) -> Result<r::Prepared<Scope<S>>, Rejected<S>> {
        let request = match self.suspend_resolver() { Ok(a) => a, Err(fault) => return Err(Rejected { parent: self, actual, fault }) };
        let args = request.arguments(); let body = actual.child;

        if actual.source_pc != request.pc() || body.pc != 0x58ee20 || body.source_image_base != self.actual.source_image_base
            || body.work_rsp != request.entry_rsp().wrapping_sub(0x48) || u64::from(body.ecx) != args.rcx
            || body.rdx != args.rdx || body.r8 != args.r8 || body.r9 != args.r9 || body.entry_rax != self.rax {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let (services, storage) = self.into_resolver_scope(actual, request);
        match r::Prepared::from_actual_body(r::Input { services, storage, actual: body }) {
            Ok(p) => Ok(p), Err(e) => Err(Rejected { parent: e.input.storage.into_parent(e.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Stage { Body { phase: r::Phase, attempted: Option<r::Phase>, stopped: bool },
    NativeReturn { phase: r::native_return::Phase, index: usize, attempted: Option<(r::native_return::Phase, usize)>, stopped: bool } }

pub struct Retained { pub actual: ActualCall, pub request: Request, pub body: r::Captures,
    pub stage: Stage, pub native_return: Option<r::native_return::History> }
pub struct Preserved<S: Services> { pub parent: i::Parts<S>, pub retained: Retained }
fn preserve<S: Services>(parts: r::Parts<Scope<S>>, stage: Stage, native_return: Option<r::native_return::History>) -> Preserved<S> {
    let state = parts.storage;
    let retained = Retained { actual: state.binding, request: state.request, body: parts.captures, stage, native_return };
    Preserved { parent: state.into_parent(parts.services.services).into_parts(), retained }
}
impl<S: Services> r::Parts<Scope<S>> {
    pub fn into_preserved_initializer(self) -> Preserved<S> {
        let stage = Stage::Body { phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        preserve(self, stage, None)
    }
}
impl<S: Services> r::native_return::Parts<Scope<S>> {
    pub fn into_preserved_initializer(self) -> Preserved<S> {
        let stage = Stage::NativeReturn { phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped };
        let history = r::native_return::History { work_rsp: self.body.actual.work_rsp, captures: self.captures };
        preserve(self.body, stage, Some(history))
    }
}
