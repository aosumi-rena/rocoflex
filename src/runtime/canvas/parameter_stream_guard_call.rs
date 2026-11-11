use super::guard as g;
#[path = "parameter_stream_guard_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[path = "parameter_stream_wrapper_guard_routing.rs"]
mod wrapper_routing;
pub use wrapper_routing::WrapperServices;
#[path = "parameter_stream_initialization_guard_routing.rs"]
mod initialization_routing;
pub use initialization_routing::InitializationServices;
#[path = "parameter_stream_resolver_guard_routing.rs"]
mod resolver_routing;
pub use resolver_routing::{ResolverServices, ResolverEntryServices, ResolverProtectedServices, ResolverResolutionServices, ResolverLoadServices, ResolverPriorCleanupServices};
#[path = "parameter_stream_guard_call_entry.rs"]
pub mod native_entry;
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: g::ActualBody }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest, pub body: g::Captures, pub native: g::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    saved: Option<super::Saved>, phase: super::Phase, attempted: Option<super::Phase>, stopped: bool,
    binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners, actual: self.actual,
        captures: self.captures, saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPendingGuard, Request(super::Fault<E>), OccupiedCall, CallBinding, BodyEntry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
fn slot(site: super::CallSite) -> Option<usize> {
    match site { super::CallSite::EnterOuter => Some(0), super::CallSite::EnterInner => Some(1),
        super::CallSite::LeaveInner => Some(2), super::CallSite::LeaveOuter => Some(3), _ => None }
}
fn expected_entry(site: super::CallSite) -> Option<g::Entry> {
    match site { super::CallSite::EnterOuter | super::CallSite::EnterInner => Some(g::Entry::Construct55e0a0),
        super::CallSite::LeaveInner | super::CallSite::LeaveOuter => Some(g::Entry::Destroy55e0dc), _ => None }
}
impl<S: Services> super::Prepared<S> {

    fn take_guard_parent(mut self, actual: ActualCall) -> Result<(Scope<S>, ParentState<S>), Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || !matches!(self.phase, super::Phase::EnterOuter | super::Phase::EnterInner | super::Phase::LeaveInner | super::Phase::LeaveOuter) {
            return Err(Rejected { parent: self, actual, fault: Admission::NotPendingGuard });
        }
        let request = match self.guard_request() { Ok(r) => r, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.requests[request.site.index()] = Some(request);
        if self.owners.calls[request.site.index()].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        let construct = actual.child.entry == g::Entry::Construct55e0a0;
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || Some(actual.child.entry) != expected_entry(request.site) || actual.child.pc != actual.child.entry.body_pc()
            || actual.child.source_image_base != self.actual.source_image_base
            || actual.child.work_rsp != actual.caller_rsp.wrapping_sub(0x30) || Some(actual.child.guard_rcx) != request.args.rcx
            || (construct && actual.child.constructor_edx.map(u64::from) != request.args.rdx) {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let state = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        Ok((Scope { services: self.services }, state))
    }

    pub fn bind_actual_guard_body(self, actual: ActualCall) -> Result<g::Prepared<Scope<S>>, Rejected<S>> {
        let (services, storage) = self.take_guard_parent(actual)?;
        match g::Prepared::from_actual_body(g::Input { services, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor), Err(r) => Err(Rejected { parent: r.input.storage.into_parent(r.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: g::native_return::History, pub phase: g::native_return::Phase,
    pub attempted: Option<g::native_return::Phase>, pub stopped: bool }
pub struct Retained<R, T> { pub actual: ActualCall, pub request: super::CallRequest,
    pub call: Option<g::CallReturned<R>>, pub tail: Option<g::native_return::TailCallReturned<T>>,
    pub body_actual: g::ActualBody, pub body: g::Captures, pub phase: g::Phase, pub attempted: Option<g::Phase>, pub stopped: bool,
    pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub retained: Retained<S::GuardCallOwner, S::GuardTailOwner> }
fn preserve<S: Services>(body: g::Parts<Scope<S>>, tail: Option<g::native_return::TailCallReturned<S::GuardTailOwner>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage;
    let retained = Retained { actual: state.binding, request: state.request, call: body.owners.call, tail,
        body_actual: body.actual, body: body.captures, phase: body.phase, attempted: body.attempted, stopped: body.stopped, native };
    Preserved { parent: state.into_parent(body.services.services).into_parts(), retained }
}
impl<S: Services> g::Parts<Scope<S>> { pub fn into_preserved_lookup(self) -> Preserved<S> { preserve(self, None, None) } }
impl<S: Services> g::native_return::Parts<Scope<S>> {
    pub fn into_preserved_lookup(self) -> Preserved<S> {
        let native = NativeProgress { history: g::native_return::History { path: self.path, captures: self.captures },
            phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, self.tail, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>, pub retained: Option<Retained<S::GuardCallOwner, S::GuardTailOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> g::native_return::Returned<Scope<S>> {

    pub fn into_lookup_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts(); let state = parts.body.owners.storage;
        let history = History { actual: state.binding, request: state.request, body: parts.body.captures, native };
        let mut held = Some(Retained { actual: state.binding, request: state.request, call: parts.body.owners.call, tail: parts.tail,
            body_actual: parts.body.actual, body: parts.body.captures, phase: parts.body.phase, attempted: parts.body.attempted, stopped: parts.body.stopped,
            native: Some(NativeProgress { history: native, phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped }) });
        let mut parent = state.into_parent(parts.body.services.services); let actual = supplied.actual;
        let index = slot(history.request.site);
        if index.is_none() || returned.target != parent.actual.source_image_base.wrapping_add(history.request.site.resume_pc() as u64)
            || returned.after_rsp != history.actual.caller_rsp || actual.resume_pc != history.request.site.resume_pc()
            || actual.work_rsp != history.actual.caller_rsp || actual.rax != returned.rax || actual.saved.rbx != returned.rbx {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::ReturnBinding });
        }
        if parent.owners.calls[history.request.site.index()].is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::OccupiedCall });
        }
        parent.owners.calls[history.request.site.index()] = Some(supplied);
        if let Err(e) = parent.record_call_return(history.request) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Parent(e) });
        }
        if let Err(e) = parent.services.adopt_completed_guard(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::ResourcesNotAdopted }); }
        if let Err(e) = parent.guard_call_returned(history.request.site) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Parent(e) });
        }
        parent.captures.guard_calls[index.unwrap()] = Some(history);
        parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
