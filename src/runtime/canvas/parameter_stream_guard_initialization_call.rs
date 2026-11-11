use super::initialization as i;
#[path = "parameter_stream_guard_initialization_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[path = "parameter_stream_resolver_initialization_routing.rs"]
mod resolver_routing;
pub use resolver_routing::{ResolverServices, ResolverEntryServices, ResolverProtectedServices, ResolverResolutionServices, ResolverLoadServices, ResolverPriorCleanupServices, ResolverResources};
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub source_pc: u32, pub child: i::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest, pub initialization: i::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualEntry, captures: super::Captures,
    live_rsp: u64, live_rcx: u64, live_rax: u64, phase: super::Phase, attempted: Option<super::Phase>, stopped: bool,
    binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners, actual: self.actual,
        captures: self.captures, live_rsp: self.live_rsp, live_rcx: self.live_rcx, live_rax: self.live_rax,
        phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPending, Request(super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_initialization(mut self, actual: ActualCall) -> Result<i::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::Call { return Err(Rejected { parent: self, actual, fault: Admission::NotPending }); }
        let request = match self.initialization_request() { Ok(r) => r, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.call_request = Some(request);
        if self.owners.call.is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        if actual.source_pc != request.source_pc || actual.child.pc != request.target_rva
            || actual.child.source_image_base != self.actual.source_image_base || actual.child.rsp != request.work_rsp.wrapping_sub(8)
            || actual.child.rax != self.live_rax {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, live_rsp: self.live_rsp,
            live_rcx: self.live_rcx, live_rax: self.live_rax, phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match i::Prepared::from_actual_entry(i::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(p) => Ok(p), Err(e) => Err(Rejected { parent: e.input.storage.into_parent(e.input.services.services), actual, fault: Admission::Entry }) }
    }
}
pub struct Retained<C, T> { pub actual: ActualCall, pub request: super::CallRequest, pub calls: i::Calls<C>, pub tail: Option<i::TailReturned<T>>,
    pub history: i::History, pub rax: u64, pub phase: i::Phase, pub row: usize, pub argument: usize,
    pub attempted: Option<(i::Phase, usize, usize)>, pub stopped: bool }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub retained: Retained<S::InitializationCallOwner, S::InitializationTailOwner> }
impl<S: Services> i::Parts<Scope<S>> {
    pub fn into_preserved_wrapper(self) -> Preserved<S> {
        let state = self.owners.storage;
        let retained = Retained { actual: state.binding, request: state.request, calls: self.owners.calls, tail: self.owners.tail,
            history: i::History { actual: self.actual, captures: self.captures }, rax: self.rax, phase: self.phase, row: self.row,
            argument: self.argument, attempted: self.attempted, stopped: self.stopped };
        Preserved { parent: state.into_parent(self.services.services).into_parts(), retained }
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub supplied: Option<super::CallReturned<S::CallOwner>>,
    pub retained: Option<Retained<S::InitializationCallOwner, S::InitializationTailOwner>>, pub history: History, pub fault: ReturnFault<S::Error> }
impl<S: Services> i::Returned<Scope<S>> {
    pub fn into_wrapper_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let actual = *self.actual_return(); let parts = self.into_parts(); let state = parts.owners.storage;
        let history = History { actual: state.binding, request: state.request, initialization: i::History { actual: parts.actual, captures: parts.captures } };
        let mut held = Some(Retained { actual: state.binding, request: state.request, calls: parts.owners.calls, tail: parts.owners.tail,
            history: history.initialization, rax: parts.rax, phase: parts.phase, row: parts.row, argument: parts.argument,
            attempted: parts.attempted, stopped: parts.stopped });
        let mut parent = state.into_parent(parts.services.services); let a = supplied.actual;
        if actual.target != parent.actual.source_image_base.wrapping_add(history.request.expected_resume_pc as u64)
            || actual.after_rsp != history.request.work_rsp || a.resume_pc != history.request.expected_resume_pc
            || a.work_rsp != history.request.work_rsp || a.rax != actual.rax {
            return Err(ReturnRejected { parent: parent.into_parts(), supplied: Some(supplied), retained: held, history, fault: ReturnFault::ReturnBinding });
        }
        if parent.owners.call.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), supplied: Some(supplied), retained: held, history, fault: ReturnFault::OccupiedCall }); }
        parent.owners.call = Some(supplied);
        if let Err(e) = parent.record_initialization_return(history.request) { return Err(ReturnRejected {
            parent: parent.into_parts(), supplied: None, retained: held, history, fault: ReturnFault::Parent(e) }); }
        if let Err(e) = parent.services.adopt_completed_initialization(&mut parent.owners, history, &mut held) { return Err(ReturnRejected {
            parent: parent.into_parts(), supplied: None, retained: held, history, fault: ReturnFault::Adoption(e) }); }
        if held.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), supplied: None, retained: held, history, fault: ReturnFault::ResourcesNotAdopted }); }
        parent.captures.initialization_call = Some(history); parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
