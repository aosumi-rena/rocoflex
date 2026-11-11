use super::member_construction as m;
#[path = "parameter_stream_member_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[path = "parameter_stream_member_query_member_routing.rs"]
mod query_routing;
pub use query_routing::QueryServices;
#[path = "parameter_stream_member_duplicate_member_routing.rs"]
mod duplicate_routing;
pub use duplicate_routing::DuplicateServices;
#[path = "parameter_stream_member_copy_member_routing.rs"]
mod duplicate_copy_routing;
pub use duplicate_copy_routing::DuplicateCopyServices;
#[path = "parameter_stream_member_accessor_member_routing.rs"]
mod duplicate_copy_accessor_routing;
pub use duplicate_copy_accessor_routing::DuplicateCopyAccessorServices;
#[path = "parameter_stream_member_diagnostic_member_routing.rs"]
mod duplicate_copy_diagnostic_routing;
pub use duplicate_copy_diagnostic_routing::DuplicateCopyDiagnosticServices;
#[path = "parameter_stream_member_diagnostic_dispatch_member_routing.rs"]
mod diagnostic_dispatch_routing;
pub use diagnostic_dispatch_routing::DuplicateCopyDiagnosticDispatchServices;
#[path = "parameter_stream_member_diagnostic_children_member_routing.rs"]
mod diagnostic_children_routing;
pub use diagnostic_children_routing::{DuplicateCopyDiagnosticDispatchHandlerServices, DuplicateCopyDiagnosticDispatchMetadataServices};
#[path = "parameter_stream_member_diagnostic_cached_member_routing.rs"]
mod diagnostic_cached_routing;
pub use diagnostic_cached_routing::{DuplicateCopyDiagnosticDispatchHandlerContextServices, DuplicateCopyDiagnosticDispatchHandlerEncodedSlotServices};
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: m::native_entry::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest,
    pub body_actual: m::ActualBody, pub body: m::Captures, pub native: m::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    saved: Option<super::Saved>, phase: super::Phase, attempted: Option<super::Phase>, stopped: bool,
    binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPending, Request(super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_member_entry(mut self, actual: ActualCall) -> Result<m::native_entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::ConstructMember { return Err(Rejected { parent: self, actual, fault: Admission::NotPending }); }
        let request = match self.member_request() {
            Ok(request) => request, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }),
        };
        self.captures.requests[request.site.index()] = Some(request);
        if self.owners.calls[request.site.index()].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || !matches!(request.target, super::Target::DirectRva(0x5612f0)) || actual.child.pc != 0x5612f0
            || actual.child.source_image_base != self.actual.source_image_base || actual.child.rsp != actual.caller_rsp.wrapping_sub(8)
            || Some(actual.child.rcx) != request.args.rcx || actual.child.saved_rbx != request.required_saved.rbx {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let state = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match m::native_entry::Prepared::from_actual_entry(m::native_entry::Input { services: Scope { services: self.services }, storage: state, actual: actual.child }) {
            Ok(prepared) => Ok(prepared),
            Err(e) => Err(Rejected { parent: e.input.storage.into_parent(e.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> m::native_entry::Entered<Scope<S>> {
    pub fn into_member_body(self) -> m::Prepared<Scope<S>> { self.into_body_with(|services, owners| (services, owners)) }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: m::native_return::History, pub phase: m::native_return::Phase,
    pub attempted: Option<m::native_return::Phase>, pub stopped: bool }
pub struct Retained<R> { pub calls: m::Calls<R>, pub body_actual: m::ActualBody,
    pub body: m::Captures, pub phase: m::Phase, pub attempted: Option<m::Phase>, pub stopped: bool,
    pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub retained: Retained<S::MemberCallOwner> }
pub struct EntryPreserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub entry_actual: m::native_entry::ActualEntry, pub entry: m::native_entry::Captures,
    pub phase: m::native_entry::Phase, pub attempted: Option<m::native_entry::Phase>, pub stopped: bool,
    pub calls: m::Calls<S::MemberCallOwner> }
impl<S: Services> m::native_entry::Parts<Scope<S>> {
    pub fn into_preserved_lookup(self) -> EntryPreserved<S> {
        let state = self.owners.storage; let actual = state.binding; let request = state.request;
        EntryPreserved { parent: state.into_parent(self.services.services).into_parts(), actual, request,
            entry_actual: self.actual, entry: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped, calls: self.owners.calls }
    }
}
fn preserve<S: Services>(body: m::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage; let actual = state.binding; let request = state.request;
    Preserved { parent: state.into_parent(body.services.services).into_parts(), actual, request,
        retained: Retained { calls: body.owners.calls, body_actual: body.actual, body: body.captures,
            phase: body.phase, attempted: body.attempted, stopped: body.stopped, native } }
}
impl<S: Services> m::Parts<Scope<S>> { pub fn into_preserved_lookup(self) -> Preserved<S> { preserve(self, None) } }
impl<S: Services> m::native_return::Parts<Scope<S>> {
    pub fn into_preserved_lookup(self) -> Preserved<S> {
        let native = NativeProgress { history: m::native_return::History { actual: self.actual, captures: self.captures },
            phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>, pub held: Option<Retained<S::MemberCallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> m::native_return::Returned<Scope<S>> {
    pub fn into_lookup_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts(); let state = parts.body.owners.storage;
        let history = History { actual: state.binding, request: state.request, body_actual: parts.body.actual, body: parts.body.captures, native };
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
        if parent.owners.calls[slot].is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), held, fault: ReturnFault::OccupiedCall }); }
        parent.owners.calls[slot] = Some(supplied);
        if let Err(e) = parent.record_call_return(history.request) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Parent(e) });
        }
        if let Err(e) = parent.services.adopt_completed_member(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::ResourcesNotAdopted }); }
        parent.captures.member_call = Some(history);
        parent.record_member_return(); parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
