use super::formatting;
use formatting::{sentry, facet, finish};
use finish::native_return as ret;
#[path = "parameter_number_format_scope.rs"]
mod scope;
#[path = "parameter_number_format_core.rs"]
mod retained;
#[path = "parameter_number_format_constructor_routing.rs"]
mod constructor_routing;
pub use constructor_routing::LookupMemberQueryServices;
pub use constructor_routing::LookupMemberDuplicateServices;
pub use constructor_routing::LookupMemberDuplicateCopyServices;
pub use constructor_routing::LookupMemberDuplicateCopyAccessorServices;
pub use constructor_routing::LookupMemberDuplicateCopyDiagnosticServices;
pub use constructor_routing::LookupMemberDuplicateCopyDiagnosticDispatchServices;
pub use constructor_routing::{LookupMemberDuplicateCopyDiagnosticDispatchHandlerServices, LookupMemberDuplicateCopyDiagnosticDispatchMetadataServices};
pub use constructor_routing::{LookupMemberDuplicateCopyDiagnosticDispatchHandlerContextServices, LookupMemberDuplicateCopyDiagnosticDispatchHandlerEncodedSlotServices};
pub use constructor_routing::{ConstructorServices, LookupServices, LookupFallbackServices, LookupRegistrationServices, LookupMemberServices, LookupGuardServices, LookupWrapperServices, LookupInitializationServices, LookupResolverServices, LookupResolverEntryServices, LookupResolverProtectedServices, LookupResolverResolutionServices, LookupResolverLoadServices, LookupResolverPriorCleanupServices};
#[path = "parameter_number_format_call_entry.rs"]
pub mod native_entry;
pub use scope::{Scope, Services, Resources};
pub use retained::{Retained, Preserved, Stage, BodyRetained, NativeProgress};
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: formatting::ActualBody }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest, pub body: formatting::Captures,
    pub sentry: sentry::Captures, pub facet: Option<facet::Captures>, pub finish: finish::Captures, pub native: ret::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody,
    captures: super::Captures, saved: Option<super::Nonvolatile>, phase: super::Phase,
    attempted: Option<super::Phase>, stopped: bool, binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPendingFormat, Request(super::Fault<E>), OccupiedCall, CallBinding, BodyEntry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_format_body(mut self, actual: ActualCall) -> Result<formatting::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::Format { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingFormat }); }
        let request = match self.format_request() { Ok(r) => r,
            Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.requests[super::CallSite::Format.index()] = Some(request);
        if self.owners.calls[super::CallSite::Format.index()].is_some() {
            return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall });
        }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || actual.child.pc != 0x1fb5b4 || actual.child.source_image_base != self.actual.actual_image_base
            || actual.child.rbp != actual.caller_rsp.wrapping_sub(0xe0)
            || actual.child.builder_rcx != request.args.rcx || actual.child.entry_rbx != request.required_saved.rbx
            || Some(u64::from(actual.child.precision_r8d)) != request.args.r8
            || Some(u64::from(actual.child.flag_r9d)) != request.args.r9 || Some(actual.child.value_xmm1) != request.args.xmm1 {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match formatting::Prepared::from_actual_body(formatting::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor),
            Err(rejected) => Err(Rejected { parent: rejected.input.storage.into_parent(rejected.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>,
    pub retained: Option<Retained<S::FormatConstructorOwner, S::FormatSentryOwner, S::FormatFacetOwner, S::FormatFinishOwner>>,
    pub fault: ReturnFault<S::Error> }
impl<S: Services> ret::Returned<Scope<S>> {

    pub fn into_number_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts();
        let prefix = parts.body.prefix(); let state = &prefix.owners.storage;
        let history = History { actual: state.binding, request: state.request, body: prefix.captures,
            sentry: parts.body.sentry().captures, facet: parts.body.facet().map(|p| p.captures), finish: parts.body.captures, native };
        let preserved = retained::preserve_native(parts); let p = preserved.parent;
        let mut parent = super::Prepared { services: p.services, owners: p.owners, actual: p.actual, captures: p.captures,
            saved: p.saved, phase: p.phase, attempted: p.attempted, stopped: true };
        let mut held = Some(preserved.retained); let actual = supplied.actual;
        if returned.target != parent.actual.actual_image_base.wrapping_add(0x1caeca)
            || returned.after_rsp != history.actual.caller_rsp || native.input.actual_work_rsp != history.actual.child.rbp.wrapping_sub(0x80)
            || actual.rax != returned.rax || actual.resume_pc != 0x1caeca || actual.work_rsp != history.actual.caller_rsp
            || [actual.saved.rbx, actual.saved.rsi, actual.saved.rbp] != returned.restored || actual.saved.xmm6 != returned.xmm6 {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::ReturnBinding });
        }
        if parent.owners.calls[super::CallSite::Format.index()].is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::OccupiedCall });
        }
        parent.owners.calls[super::CallSite::Format.index()] = Some(supplied);
        let length = match parent.record_call_return(history.request) { Ok(word) => word,
            Err(e) => return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Parent(e) }) };
        if let Err(e) = parent.record_format_value(length) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Parent(e) });
        }
        if let Err(e) = parent.services.adopt_completed_format(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::ResourcesNotAdopted }); }
        parent.captures.format_call = Some(history); parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
