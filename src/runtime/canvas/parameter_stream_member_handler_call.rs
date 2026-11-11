use super::handler as h;
#[path = "parameter_stream_member_handler_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[path = "parameter_stream_member_handler_slot_routing.rs"]
mod slot_routing;
pub use slot_routing::EncodedSlotServices;
#[path = "parameter_stream_member_handler_context_routing.rs"]
mod context_routing;
pub use context_routing::ContextServices;
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: h::native_entry::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest,
    pub body_actual: h::ActualBody, pub body: h::Captures, pub native: h::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    phase: super::Phase, attempted: Option<super::Phase>, stopped: bool, binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPending, Request(super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_handler_entry(mut self, actual: ActualCall) -> Result<h::native_entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old { return Err(Rejected { parent: self, actual, fault: Admission::NotPending }); }
        let request = match self.handler_request() {
            Ok(r) => r, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }),
        };
        let slot = request.site.index(); self.captures.requests[slot] = Some(request);
        if self.owners.calls[slot].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        let a = actual.child;
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp || a.pc != 0x57a200
            || a.source_image_base != self.actual.source_image_base || request.target != a.source_image_base.wrapping_add(u64::from(a.pc))
            || a.rsp != actual.caller_rsp.wrapping_sub(8) || a.rax != request.required_rax || a.rcx != request.rcx
            || Some(a.rdx) != request.rdx || Some(a.r8) != request.r8 || Some(a.r9) != request.r9
            || a.saved.rbx != request.required_rbx || a.saved.rbp != request.required_rbp {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }

        let state = ParentState { owners: self.owners, actual: self.actual, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match h::native_entry::Prepared::from_actual_entry(h::native_entry::Input {
            services: Scope { services: self.services }, storage: state, actual: a }) {
            Ok(p) => Ok(p),
            Err(e) => Err(Rejected { parent: e.input.storage.into_parent(e.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> h::native_entry::Entered<Scope<S>> {
    pub fn into_dispatch_handler_body(self) -> h::Prepared<Scope<S>> { self.into_body_with(|s, o| (s, o)) }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: h::native_return::History, pub phase: h::native_return::Phase,
    pub attempted: Option<h::native_return::Phase>, pub stopped: bool }
pub struct Retained<R> { pub calls: h::Calls<R>, pub body_actual: h::ActualBody, pub body: h::Captures,
    pub phase: h::Phase, pub attempted: Option<h::Phase>, pub stopped: bool, pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub retained: Retained<S::HandlerCallOwner> }
pub struct EntryPreserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall, pub request: super::CallRequest,
    pub entry_actual: h::native_entry::ActualEntry, pub entry: h::native_entry::Captures,
    pub phase: h::native_entry::Phase, pub attempted: Option<h::native_entry::Phase>, pub stopped: bool, pub calls: h::Calls<S::HandlerCallOwner> }
impl<S: Services> h::native_entry::Parts<Scope<S>> {
    pub fn into_preserved_dispatch(self) -> EntryPreserved<S> {
        let state = self.owners.storage; let actual = state.binding; let request = state.request;
        EntryPreserved { parent: state.into_parent(self.services.services).into_parts(), actual, request,
            entry_actual: self.actual, entry: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped, calls: self.owners.calls }
    }
}
fn preserve<S: Services>(body: h::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage; let actual = state.binding; let request = state.request;
    Preserved { parent: state.into_parent(body.services.services).into_parts(), actual, request,
        retained: Retained { calls: body.owners.calls, body_actual: body.actual, body: body.captures,
            phase: body.phase, attempted: body.attempted, stopped: body.stopped, native } }
}
impl<S: Services> h::Parts<Scope<S>> { pub fn into_preserved_dispatch(self) -> Preserved<S> { preserve(self, None) } }
impl<S: Services> h::BreakpointRequired<Scope<S>> { pub fn into_preserved_dispatch(self) -> Preserved<S> { preserve(self.into_parts(), None) } }
impl<S: Services> h::native_return::Parts<Scope<S>> {
    pub fn into_preserved_dispatch(self) -> Preserved<S> {
        let native = NativeProgress { history: h::native_return::History { actual: self.actual, captures: self.captures },
            phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>, pub held: Option<Retained<S::HandlerCallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> h::native_return::Returned<Scope<S>> {
    pub fn into_dispatch_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts();
        let state = parts.body.owners.storage;
        let history = History { actual: state.binding, request: state.request,
            body_actual: parts.body.actual, body: parts.body.captures, native };
        let mut held = Some(Retained { calls: parts.body.owners.calls, body_actual: parts.body.actual, body: parts.body.captures,
            phase: parts.body.phase, attempted: parts.body.attempted, stopped: parts.body.stopped,
            native: Some(NativeProgress { history: native, phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped }) });
        let mut parent = state.into_parent(parts.body.services.services); let actual = supplied.actual;
        if returned.target != parent.actual.source_image_base.wrapping_add(u64::from(history.request.site.resume()))
            || returned.after_rsp != history.actual.caller_rsp || actual.resume_pc != history.request.site.resume()
            || actual.work_rsp != history.actual.caller_rsp || actual.rax != returned.rax
            || actual.rbx != returned.saved.rbx || actual.rbp != returned.saved.rbp {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), held, fault: ReturnFault::ReturnBinding });
        }
        let slot = history.request.site.index();
        if parent.owners.calls[slot].is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), held, fault: ReturnFault::OccupiedCall });
        }
        parent.owners.calls[slot] = Some(supplied);
        let value = match parent.record_call_return(history.request) {
            Ok(value) => value,
            Err(e) => return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Parent(e) }),
        };
        if let Err(e) = parent.services.adopt_completed_handler(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::ResourcesNotAdopted });
        }
        parent.captures.handler_call = Some(history); parent.record_handler_value(value);
        parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
