use super::query as q;
#[path = "parameter_stream_member_query_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub actual_rbx: u64, pub child: q::native_entry::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest,
    pub body_actual: q::ActualBody, pub body: q::Captures, pub native: q::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    phase: super::Phase, attempted: Option<super::Phase>, stopped: bool, binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
fn selected_entry(site: super::CallSite) -> Option<q::Entry> {
    match site {
        super::CallSite::QueryWord => Some(q::Entry::Word567c9c),
        super::CallSite::CopySource | super::CallSite::FallbackSource => Some(q::Entry::Source56807c),
        super::CallSite::QueryContext => Some(q::Entry::Context567c68),
        super::CallSite::Allocate | super::CallSite::Duplicate => None,
    }
}
pub enum Admission<E> { NotPending, Request(super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_query_entry(mut self, actual: ActualCall) -> Result<q::native_entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old { return Err(Rejected { parent: self, actual, fault: Admission::NotPending }); }
        let request = match self.query_request() {
            Ok(request) => request, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }),
        };
        self.captures.requests[request.site.index()] = Some(request);
        if self.owners.calls[request.site.index()].is_some() {
            return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall });
        }
        if Some(actual.child.entry) != selected_entry(request.site) || actual.call_pc != request.site.pc()
            || actual.caller_rsp != request.work_rsp || actual.actual_rbx != request.required_rbx
            || actual.child.pc != request.site.target_rva()
            || actual.child.source_image_base != self.actual.source_image_base
            || actual.child.rsp != actual.caller_rsp.wrapping_sub(8)
            || request.target != actual.child.source_image_base.wrapping_add(u64::from(actual.child.pc)) {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let state = ParentState { owners: self.owners, actual: self.actual, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match q::native_entry::Prepared::from_actual_entry(q::native_entry::Input {
            services: Scope { services: self.services }, storage: state, actual: actual.child }) {
            Ok(prepared) => Ok(prepared),
            Err(e) => Err(Rejected { parent: e.input.storage.into_parent(e.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> q::native_entry::Entered<Scope<S>> {
    pub fn into_member_query_body(self) -> q::Prepared<Scope<S>> { self.into_body_with(|services, owners| (services, owners)) }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: q::native_return::History, pub phase: q::native_return::Phase,
    pub attempted: Option<q::native_return::Phase>, pub stopped: bool }
pub struct Retained<R> { pub calls: q::Calls<R>, pub body_actual: q::ActualBody, pub body: q::Captures,
    pub phase: q::Phase, pub attempted: Option<q::Phase>, pub stopped: bool, pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub retained: Retained<S::QueryCallOwner> }
pub struct EntryPreserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub entry_actual: q::native_entry::ActualEntry, pub entry: q::native_entry::Captures,
    pub attempted: bool, pub stopped: bool, pub calls: q::Calls<S::QueryCallOwner> }
impl<S: Services> q::native_entry::Parts<Scope<S>> {
    pub fn into_preserved_member(self) -> EntryPreserved<S> {
        let state = self.owners.storage; let actual = state.binding; let request = state.request;
        EntryPreserved { parent: state.into_parent(self.services.services).into_parts(), actual, request,
            entry_actual: self.actual, entry: self.captures, attempted: self.attempted, stopped: self.stopped, calls: self.owners.calls }
    }
}
fn preserve<S: Services>(body: q::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage; let actual = state.binding; let request = state.request;
    Preserved { parent: state.into_parent(body.services.services).into_parts(), actual, request,
        retained: Retained { calls: body.owners.calls, body_actual: body.actual, body: body.captures,
            phase: body.phase, attempted: body.attempted, stopped: body.stopped, native } }
}
impl<S: Services> q::Parts<Scope<S>> {
    pub fn into_preserved_member(self) -> Preserved<S> { preserve(self, None) }
}
impl<S: Services> q::native_return::Parts<Scope<S>> {
    pub fn into_preserved_member(self) -> Preserved<S> {
        let native = NativeProgress { history: q::native_return::History { actual: self.actual, captures: self.captures },
            phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>, pub held: Option<Retained<S::QueryCallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> q::native_return::Returned<Scope<S>> {
    pub fn into_member_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
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
            || actual.work_rsp != history.actual.caller_rsp || actual.rax != returned.rax {
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
        if let Err(e) = parent.services.adopt_completed_query(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::ResourcesNotAdopted });
        }
        parent.captures.query_calls[slot] = Some(history);
        parent.record_query_value(history.request.site, value); parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
