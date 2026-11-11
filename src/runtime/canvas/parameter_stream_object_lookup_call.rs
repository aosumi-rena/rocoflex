use super::object_lookup as l;
#[path = "parameter_stream_object_lookup_scope.rs"]
mod scope;
pub use scope::{Scope, Services, FallbackServices, RegistrationServices};
#[path = "parameter_stream_member_lookup_routing.rs"]
mod member_routing;
pub use member_routing::MemberServices;
#[path = "parameter_stream_member_query_lookup_routing.rs"]
mod member_query_routing;
pub use member_query_routing::MemberQueryServices;
#[path = "parameter_stream_member_duplicate_lookup_routing.rs"]
mod member_duplicate_routing;
pub use member_duplicate_routing::MemberDuplicateServices;
#[path = "parameter_stream_member_copy_lookup_routing.rs"]
mod member_duplicate_copy_routing;
pub use member_duplicate_copy_routing::MemberDuplicateCopyServices;
#[path = "parameter_stream_member_accessor_lookup_routing.rs"]
mod member_duplicate_copy_accessor_routing;
pub use member_duplicate_copy_accessor_routing::MemberDuplicateCopyAccessorServices;
#[path = "parameter_stream_member_diagnostic_lookup_routing.rs"]
mod member_duplicate_copy_diagnostic_routing;
pub use member_duplicate_copy_diagnostic_routing::MemberDuplicateCopyDiagnosticServices;
#[path = "parameter_stream_member_diagnostic_dispatch_lookup_routing.rs"]
mod diagnostic_dispatch_routing;
pub use diagnostic_dispatch_routing::MemberDuplicateCopyDiagnosticDispatchServices;
#[path = "parameter_stream_member_diagnostic_children_lookup_routing.rs"]
mod diagnostic_children_routing;
pub use diagnostic_children_routing::{MemberDuplicateCopyDiagnosticDispatchHandlerServices, MemberDuplicateCopyDiagnosticDispatchMetadataServices};
#[path = "parameter_stream_member_diagnostic_cached_lookup_routing.rs"]
mod diagnostic_cached_routing;
pub use diagnostic_cached_routing::{MemberDuplicateCopyDiagnosticDispatchHandlerContextServices, MemberDuplicateCopyDiagnosticDispatchHandlerEncodedSlotServices};
#[path = "parameter_stream_guard_lookup_routing.rs"]
mod guard_routing;
pub use guard_routing::GuardServices;
#[path = "parameter_stream_wrapper_lookup_routing.rs"]
mod wrapper_routing;
pub use wrapper_routing::WrapperServices;
#[path = "parameter_stream_initialization_lookup_routing.rs"]
mod initialization_routing;
pub use initialization_routing::InitializationServices;
#[path = "parameter_stream_resolver_lookup_routing.rs"]
mod resolver_routing;
pub use resolver_routing::{ResolverServices, ResolverEntryServices, ResolverProtectedServices, ResolverResolutionServices, ResolverLoadServices, ResolverPriorCleanupServices};
#[path = "parameter_stream_object_lookup_call_entry.rs"]
pub mod native_entry;
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: l::ActualBody }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest, pub body: l::Captures, pub native: l::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    saved: Option<super::Saved>, phase: super::Phase, index: usize, attempted: Option<(super::Phase, usize)>, stopped: bool,
    binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners, actual: self.actual,
        captures: self.captures, saved: self.saved, phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPendingLookup, Request(super::Fault<E>), OccupiedCall, CallBinding, BodyEntry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_lookup_body(mut self, actual: ActualCall) -> Result<l::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::GetObject { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingLookup }); }
        let request = match self.lookup_request() { Ok(r) => r, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.requests[super::CallSite::GetObject.index()] = Some(request);
        if self.owners.calls[super::CallSite::GetObject.index()].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp || actual.child.pc != 0x18c2ed
            || actual.child.source_image_base != self.actual.source_image_base || actual.child.rbp != actual.caller_rsp.wrapping_sub(0x50)
            || Some(actual.child.holder_rcx) != request.args.rcx || actual.child.entry_rbx != request.required_saved.rbx
            || actual.child.entry_rdi != request.required_saved.rdi || actual.child.entry_r14 != request.required_saved.r14 {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved, phase: self.phase, index: self.index,
            attempted: self.attempted, stopped: true, binding: actual, request };
        match l::Prepared::from_actual_body(l::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor), Err(r) => Err(Rejected { parent: r.input.storage.into_parent(r.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: l::native_return::History, pub phase: l::native_return::Phase, pub pop: usize,
    pub attempted: Option<(l::native_return::Phase, usize)>, pub stopped: bool }
pub struct Retained<R> { pub actual: ActualCall, pub request: super::CallRequest, pub calls: l::Calls<R>, pub body_actual: l::ActualBody,
    pub body: l::Captures, pub saved: Option<l::Saved>, pub phase: l::Phase, pub attempted: Option<l::Phase>, pub stopped: bool, pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub retained: Retained<S::LookupCallOwner> }
fn preserve<S: Services>(body: l::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage;
    let retained = Retained { actual: state.binding, request: state.request, calls: body.owners.calls, body_actual: body.actual, body: body.captures,
        saved: body.saved, phase: body.phase, attempted: body.attempted, stopped: body.stopped, native };
    Preserved { parent: state.into_parent(body.services.services).into_parts(), retained }
}
impl<S: Services> l::Parts<Scope<S>> { pub fn into_preserved_constructor(self) -> Preserved<S> { preserve(self, None) } }
impl<S: Services> l::native_return::Parts<Scope<S>> {
    pub fn into_preserved_constructor(self) -> Preserved<S> { let native = NativeProgress { history: l::native_return::History { input: self.input, captures: self.captures },
        phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped }; preserve(self.body, Some(native)) }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History, pub supplied: Option<super::CallReturned<S::CallOwner>>,
    pub retained: Option<Retained<S::LookupCallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> l::native_return::Returned<Scope<S>> {
    pub fn into_constructor_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts(); let state = parts.body.owners.storage;
        let history = History { actual: state.binding, request: state.request, body: parts.body.captures, native };
        let mut held = Some(Retained { actual: state.binding, request: state.request, calls: parts.body.owners.calls, body_actual: parts.body.actual, body: parts.body.captures,
            saved: parts.body.saved, phase: parts.body.phase, attempted: parts.body.attempted, stopped: parts.body.stopped,
            native: Some(NativeProgress { history: native, phase: parts.phase, pop: parts.pop, attempted: parts.attempted, stopped: parts.stopped }) });
        let mut parent = state.into_parent(parts.body.services.services); let actual = supplied.actual;
        if returned.target != parent.actual.source_image_base.wrapping_add(0x1fbd6c) || returned.after_rsp != history.actual.caller_rsp
            || native.input.actual_work_rsp != history.actual.child.rbp.wrapping_sub(0x80)
            || actual.resume_pc != 0x1fbd6c || actual.work_rsp != history.actual.caller_rsp || actual.rax != returned.rax
            || [actual.saved.rbx, actual.saved.rdi, actual.saved.rsi, actual.saved.r14, actual.saved.rbp] != returned.restored {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::ReturnBinding });
        }
        if parent.owners.calls[super::CallSite::GetObject.index()].is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::OccupiedCall }); }
        parent.owners.calls[super::CallSite::GetObject.index()] = Some(supplied);
        let word = match parent.record_call_return(history.request) { Ok(w) => w, Err(e) => return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Parent(e) }) };
        parent.record_lookup_value(word);
        if let Err(e) = parent.services.adopt_completed_lookup(&mut parent.owners, history, &mut held) { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Adoption(e) }); }
        if held.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::ResourcesNotAdopted }); }
        parent.captures.object_lookup_call = Some(history); parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
