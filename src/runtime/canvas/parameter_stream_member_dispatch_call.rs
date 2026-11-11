use super::dispatch as d;
#[path = "parameter_stream_member_dispatch_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[path = "parameter_stream_member_dispatch_routing.rs"]
mod inner_routing;
pub use inner_routing::{HandlerServices, MetadataServices};
#[path = "parameter_stream_member_dispatch_cached_routing.rs"]
mod cached_routing;
pub use cached_routing::{HandlerContextServices, HandlerEncodedSlotServices};
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: d::native_entry::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest,
    pub body_actual: d::ActualBody, pub body: d::Captures, pub native: d::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    phase: super::Phase, attempted: Option<super::Phase>, stopped: bool, binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPending, Request(super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_dispatch_entry(mut self, actual: ActualCall) -> Result<d::native_entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old { return Err(Rejected { parent: self, actual, fault: Admission::NotPending }); }
        let request = match self.dispatch_request() {
            Ok(r) => r, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }),
        };
        self.captures.request = Some(request);
        if self.owners.call.is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        let a = actual.child;
        if actual.call_pc != request.pc || actual.caller_rsp != request.work_rsp || a.pc != 0x57a384
            || a.source_image_base != self.actual.source_image_base || request.target != a.source_image_base.wrapping_add(u64::from(a.pc))
            || a.rsp != actual.caller_rsp.wrapping_sub(8) || a.rax != request.required_rax
            || a.rcx != request.rcx || a.rdx != request.rdx || a.r8 != request.r8 || a.r9 != request.r9 {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }

        let state = ParentState { owners: self.owners, actual: self.actual, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match d::native_entry::Prepared::from_actual_entry(d::native_entry::Input {
            services: Scope { services: self.services }, storage: state, actual: a }) {
            Ok(p) => Ok(p),
            Err(e) => Err(Rejected { parent: e.input.storage.into_parent(e.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> d::native_entry::Entered<Scope<S>> {
    pub fn into_diagnostic_dispatch_body(self) -> d::Prepared<Scope<S>> { self.into_body_with(|s, o| (s, o)) }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: d::native_return::History, pub phase: d::native_return::Phase,
    pub attempted: Option<d::native_return::Phase>, pub stopped: bool }
pub struct Retained<R> { pub calls: d::Calls<R>, pub body_actual: d::ActualBody, pub body: d::Captures,
    pub phase: d::Phase, pub attempted: Option<d::Phase>, pub stopped: bool, pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub retained: Retained<S::DispatchCallOwner> }
pub struct EntryPreserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall, pub request: super::CallRequest,
    pub entry_actual: d::native_entry::ActualEntry, pub entry: d::native_entry::Captures,
    pub phase: d::native_entry::Phase, pub attempted: Option<d::native_entry::Phase>, pub stopped: bool, pub calls: d::Calls<S::DispatchCallOwner> }
impl<S: Services> d::native_entry::Parts<Scope<S>> {
    pub fn into_preserved_diagnostic(self) -> EntryPreserved<S> {
        let state = self.owners.storage; let actual = state.binding; let request = state.request;
        EntryPreserved { parent: state.into_parent(self.services.services).into_parts(), actual, request,
            entry_actual: self.actual, entry: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped, calls: self.owners.calls }
    }
}
fn preserve<S: Services>(body: d::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage; let actual = state.binding; let request = state.request;
    Preserved { parent: state.into_parent(body.services.services).into_parts(), actual, request,
        retained: Retained { calls: body.owners.calls, body_actual: body.actual, body: body.captures,
            phase: body.phase, attempted: body.attempted, stopped: body.stopped, native } }
}
impl<S: Services> d::Parts<Scope<S>> { pub fn into_preserved_diagnostic(self) -> Preserved<S> { preserve(self, None) } }
impl<S: Services> d::native_return::Parts<Scope<S>> {
    pub fn into_preserved_diagnostic(self) -> Preserved<S> {
        let native = NativeProgress { history: d::native_return::History { actual: self.actual, captures: self.captures },
            phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>, pub held: Option<Retained<S::DispatchCallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> d::native_return::Returned<Scope<S>> {
    pub fn into_diagnostic_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts();
        let state = parts.body.owners.storage;
        let history = History { actual: state.binding, request: state.request,
            body_actual: parts.body.actual, body: parts.body.captures, native };
        let mut held = Some(Retained { calls: parts.body.owners.calls, body_actual: parts.body.actual, body: parts.body.captures,
            phase: parts.body.phase, attempted: parts.body.attempted, stopped: parts.body.stopped,
            native: Some(NativeProgress { history: native, phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped }) });
        let mut parent = state.into_parent(parts.body.services.services); let actual = supplied.actual;
        if returned.target != parent.actual.source_image_base.wrapping_add(u64::from(history.request.resume_pc))
            || returned.after_rsp != history.actual.caller_rsp || actual.resume_pc != history.request.resume_pc
            || actual.work_rsp != history.actual.caller_rsp || actual.rax != returned.rax {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), held, fault: ReturnFault::ReturnBinding });
        }
        if parent.owners.call.is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), held, fault: ReturnFault::OccupiedCall });
        }
        parent.owners.call = Some(supplied);
        let value = match parent.record_dispatch_return(history.request) {
            Ok(value) => value,
            Err(e) => return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Parent(e) }),
        };
        if let Err(e) = parent.services.adopt_completed_dispatch(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::ResourcesNotAdopted });
        }
        parent.captures.dispatch_call = Some(history); parent.record_dispatch_value(value);
        parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
