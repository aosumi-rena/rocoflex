use super::copy as c;
#[path = "parameter_stream_member_copy_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[path = "parameter_stream_member_accessor_copy_routing.rs"]
mod accessor_routing;
pub use accessor_routing::AccessorServices;
#[path = "parameter_stream_member_diagnostic_copy_routing.rs"]
mod diagnostic_routing;
pub use diagnostic_routing::DiagnosticServices;
#[path = "parameter_stream_member_diagnostic_dispatch_copy_routing.rs"]
mod diagnostic_dispatch_routing;
pub use diagnostic_dispatch_routing::DiagnosticDispatchServices;
#[path = "parameter_stream_member_diagnostic_children_copy_routing.rs"]
mod diagnostic_children_routing;
pub use diagnostic_children_routing::{DiagnosticDispatchHandlerServices, DiagnosticDispatchMetadataServices};
#[path = "parameter_stream_member_diagnostic_cached_copy_routing.rs"]
mod diagnostic_cached_routing;
pub use diagnostic_cached_routing::{DiagnosticDispatchHandlerContextServices, DiagnosticDispatchHandlerEncodedSlotServices};
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: c::native_entry::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest,
    pub body_actual: c::ActualBody, pub body: c::Captures, pub native: c::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    phase: super::Phase, attempted: Option<super::Phase>, stopped: bool, binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPending, Request(super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_copy_entry(mut self, actual: ActualCall) -> Result<c::native_entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old { return Err(Rejected { parent: self, actual, fault: Admission::NotPending }); }
        let request = match self.copy_request() {
            Ok(request) => request, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }),
        };
        let slot = request.site.index(); self.captures.requests[slot] = Some(request);
        if self.owners.calls[slot].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp || actual.child.pc != 0x58e310
            || actual.child.source_image_base != self.actual.source_image_base
            || request.target != actual.child.source_image_base.wrapping_add(u64::from(actual.child.pc))
            || actual.child.rsp != actual.caller_rsp.wrapping_sub(8)
            || Some(actual.child.destination_rcx) != request.args.rcx || Some(actual.child.capacity_rdx) != request.args.rdx
            || Some(actual.child.source_r8) != request.args.r8 || actual.child.saved_rbx != request.required_saved.rbx {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let state = ParentState { owners: self.owners, actual: self.actual, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match c::native_entry::Prepared::from_actual_entry(c::native_entry::Input {
            services: Scope { services: self.services }, storage: state, actual: actual.child }) {
            Ok(prepared) => Ok(prepared),
            Err(e) => Err(Rejected { parent: e.input.storage.into_parent(e.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> c::native_entry::Entered<Scope<S>> {
    pub fn into_duplicate_copy_body(self) -> c::Prepared<Scope<S>> { self.into_body_with(|s, o| (s, o)) }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: c::native_return::History, pub phase: c::native_return::Phase,
    pub attempted: Option<c::native_return::Phase>, pub stopped: bool }
pub struct Retained<R> { pub calls: c::Calls<R>, pub body_actual: c::ActualBody, pub body: c::Captures,
    pub phase: c::Phase, pub attempted: Option<c::Phase>, pub stopped: bool, pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub retained: Retained<S::CopyCallOwner> }
pub struct EntryPreserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall, pub request: super::CallRequest,
    pub entry_actual: c::native_entry::ActualEntry, pub entry: c::native_entry::Captures, pub phase: c::native_entry::Phase,
    pub attempted: Option<c::native_entry::Phase>, pub stopped: bool, pub calls: c::Calls<S::CopyCallOwner> }
impl<S: Services> c::native_entry::Parts<Scope<S>> {
    pub fn into_preserved_duplicate(self) -> EntryPreserved<S> {
        let state = self.owners.storage; let actual = state.binding; let request = state.request;
        EntryPreserved { parent: state.into_parent(self.services.services).into_parts(), actual, request,
            entry_actual: self.actual, entry: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped, calls: self.owners.calls }
    }
}
fn preserve<S: Services>(body: c::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage; let actual = state.binding; let request = state.request;
    Preserved { parent: state.into_parent(body.services.services).into_parts(), actual, request,
        retained: Retained { calls: body.owners.calls, body_actual: body.actual, body: body.captures,
            phase: body.phase, attempted: body.attempted, stopped: body.stopped, native } }
}
impl<S: Services> c::Parts<Scope<S>> {
    pub fn into_preserved_duplicate(self) -> Preserved<S> { preserve(self, None) }
}
impl<S: Services> c::native_return::Parts<Scope<S>> {
    pub fn into_preserved_duplicate(self) -> Preserved<S> {
        let native = NativeProgress { history: c::native_return::History { actual: self.actual, captures: self.captures },
            phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>, pub held: Option<Retained<S::CopyCallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> c::native_return::Returned<Scope<S>> {
    pub fn into_duplicate_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts();
        let state = parts.body.owners.storage;
        let history = History { actual: state.binding, request: state.request,
            body_actual: parts.body.actual, body: parts.body.captures, native };
        let mut held = Some(Retained { calls: parts.body.owners.calls, body_actual: parts.body.actual, body: parts.body.captures,
            phase: parts.body.phase, attempted: parts.body.attempted, stopped: parts.body.stopped,
            native: Some(NativeProgress { history: native, phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped }) });
        let mut parent = state.into_parent(parts.body.services.services); let actual = supplied.actual;
        if returned.target != parent.actual.source_image_base.wrapping_add(u64::from(history.request.site.resume_pc()))
            || returned.after_rsp != history.actual.caller_rsp || actual.resume_pc != history.request.site.resume_pc()
            || actual.work_rsp != history.actual.caller_rsp || actual.rax != returned.rax || actual.saved.rbx != returned.rbx {
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
        if let Err(e) = parent.services.adopt_completed_copy(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::ResourcesNotAdopted });
        }
        parent.captures.copy_call = Some(history); parent.record_copy_value(value);
        parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
