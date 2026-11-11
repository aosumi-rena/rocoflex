use super::constructor as c;
#[path = "parameter_number_format_constructor_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[path = "parameter_stream_lookup_constructor_routing.rs"]
mod lookup_routing;
pub use lookup_routing::{LookupServices, LookupFallbackServices, LookupRegistrationServices};
#[path = "parameter_stream_member_constructor_routing.rs"]
mod member_routing;
pub use member_routing::LookupMemberServices;
#[path = "parameter_stream_member_query_constructor_routing.rs"]
mod member_query_routing;
pub use member_query_routing::LookupMemberQueryServices;
#[path = "parameter_stream_member_duplicate_constructor_routing.rs"]
mod member_duplicate_routing;
pub use member_duplicate_routing::LookupMemberDuplicateServices;
#[path = "parameter_stream_member_copy_constructor_routing.rs"]
mod member_duplicate_copy_routing;
pub use member_duplicate_copy_routing::LookupMemberDuplicateCopyServices;
#[path = "parameter_stream_member_accessor_constructor_routing.rs"]
mod member_duplicate_copy_accessor_routing;
pub use member_duplicate_copy_accessor_routing::LookupMemberDuplicateCopyAccessorServices;
#[path = "parameter_stream_member_diagnostic_constructor_routing.rs"]
mod member_duplicate_copy_diagnostic_routing;
pub use member_duplicate_copy_diagnostic_routing::LookupMemberDuplicateCopyDiagnosticServices;
#[path = "parameter_stream_member_diagnostic_dispatch_constructor_routing.rs"]
mod diagnostic_dispatch_routing;
pub use diagnostic_dispatch_routing::LookupMemberDuplicateCopyDiagnosticDispatchServices;
#[path = "parameter_stream_member_diagnostic_children_constructor_routing.rs"]
mod diagnostic_children_routing;
pub use diagnostic_children_routing::{LookupMemberDuplicateCopyDiagnosticDispatchHandlerServices, LookupMemberDuplicateCopyDiagnosticDispatchMetadataServices};
#[path = "parameter_stream_member_diagnostic_cached_constructor_routing.rs"]
mod diagnostic_cached_routing;
pub use diagnostic_cached_routing::{LookupMemberDuplicateCopyDiagnosticDispatchHandlerContextServices, LookupMemberDuplicateCopyDiagnosticDispatchHandlerEncodedSlotServices};
#[path = "parameter_stream_guard_constructor_routing.rs"]
mod guard_routing;
pub use guard_routing::LookupGuardServices;
#[path = "parameter_stream_wrapper_constructor_routing.rs"]
mod wrapper_routing;
pub use wrapper_routing::LookupWrapperServices;
#[path = "parameter_stream_initialization_constructor_routing.rs"]
mod initialization_routing;
pub use initialization_routing::LookupInitializationServices;
#[path = "parameter_stream_resolver_constructor_routing.rs"]
mod resolver_routing;
pub use resolver_routing::{LookupResolverServices, LookupResolverEntryServices, LookupResolverProtectedServices, LookupResolverResolutionServices, LookupResolverLoadServices, LookupResolverPriorCleanupServices};
#[path = "parameter_number_format_constructor_call_entry.rs"]
pub mod native_entry;
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: c::ActualBody }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::ConstructorRequest, pub body: c::Captures, pub native: c::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    saved: Option<super::Saved>, phase: super::Phase, attempted: Option<super::Phase>, stopped: bool,
    binding: ActualCall, request: super::ConstructorRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners, actual: self.actual,
        captures: self.captures, saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPendingConstructor, Request(super::Fault<E>), OccupiedConstructor, CallBinding, BodyEntry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_constructor_body(mut self, actual: ActualCall) -> Result<c::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::Construct { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingConstructor }); }
        let request = match self.constructor_request() { Ok(r) => r, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.constructor_request = Some(request);
        if self.owners.constructor.is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedConstructor }); }
        if actual.call_pc != request.call_pc || actual.caller_rsp != request.work_rsp || actual.child.pc != 0x1fbd07
            || actual.child.source_image_base != self.actual.source_image_base || actual.child.rbp != actual.caller_rsp.wrapping_sub(0x30)
            || actual.child.object_rcx != request.rcx || actual.child.builder_rdx != request.rdx
            || actual.child.entry_r8 != u64::from(request.r8d) || actual.child.entry_rbx != request.required_saved.rbx {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match c::Prepared::from_actual_body(c::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor), Err(r) => Err(Rejected { parent: r.input.storage.into_parent(r.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: c::native_return::History, pub phase: c::native_return::Phase,
    pub pop: usize, pub attempted: Option<(c::native_return::Phase, usize)>, pub stopped: bool }
pub struct Retained<R> { pub actual: ActualCall, pub request: super::ConstructorRequest, pub calls: c::Calls<R>, pub body_actual: c::ActualBody,
    pub body: c::Captures, pub saved: Option<c::Saved>, pub phase: c::Phase, pub index: usize,
    pub attempted: Option<(c::Phase, usize)>, pub stopped: bool, pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub retained: Retained<S::ConstructorCallOwner> }
fn preserve<S: Services>(body: c::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage;
    let retained = Retained { actual: state.binding, request: state.request, calls: body.owners.calls, body_actual: body.actual, body: body.captures,
        saved: body.saved, phase: body.phase, index: body.index, attempted: body.attempted, stopped: body.stopped, native };
    Preserved { parent: state.into_parent(body.services.services).into_parts(), retained }
}
impl<S: Services> c::Parts<Scope<S>> { pub fn into_preserved_formatter(self) -> Preserved<S> { preserve(self, None) } }
impl<S: Services> c::native_return::Parts<Scope<S>> {
    pub fn into_preserved_formatter(self) -> Preserved<S> {
        let native = NativeProgress { history: c::native_return::History { input: self.input, required_rax: self.required_rax, captures: self.captures },
            phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped }; preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedConstructor, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History, pub supplied: Option<super::ConstructorReturned<S::ConstructorOwner>>,
    pub retained: Option<Retained<S::ConstructorCallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> c::native_return::Returned<Scope<S>> {

    pub fn into_formatter_parent(self, supplied: super::ConstructorReturned<S::ConstructorOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts(); let state = parts.body.owners.storage;
        let history = History { actual: state.binding, request: state.request, body: parts.body.captures, native };
        let mut held = Some(Retained { actual: state.binding, request: state.request, calls: parts.body.owners.calls, body_actual: parts.body.actual,
            body: parts.body.captures, saved: parts.body.saved, phase: parts.body.phase, index: parts.body.index, attempted: parts.body.attempted, stopped: parts.body.stopped,
            native: Some(NativeProgress { history: native, phase: parts.phase, pop: parts.pop, attempted: parts.attempted, stopped: parts.stopped }) });
        let mut parent = state.into_parent(parts.body.services.services); let actual = supplied.actual;
        if returned.target != parent.actual.source_image_base.wrapping_add(0x1fb604) || returned.after_rsp != history.actual.caller_rsp
            || native.input.actual_work_rsp != history.actual.child.rbp.wrapping_sub(0x60)
            || actual.resume_pc != 0x1fb604 || actual.work_rsp != history.actual.caller_rsp || actual.rax != returned.rax
            || [actual.saved.rbx, actual.saved.rsi, actual.saved.rbp] != [returned.restored[0], returned.restored[2], returned.restored[4]] {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::ReturnBinding });
        }

        if parent.owners.constructor.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::OccupiedConstructor }); }
        parent.owners.constructor = Some(supplied);
        if let Err(e) = parent.record_constructor_return(history.request) { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Parent(e) }); }
        if let Err(e) = parent.services.adopt_completed_constructor(&mut parent.owners, history, &mut held) { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Adoption(e) }); }
        if held.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::ResourcesNotAdopted }); }
        parent.captures.constructor_call = Some(history); parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
