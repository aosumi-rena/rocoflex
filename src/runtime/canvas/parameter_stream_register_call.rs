use super::registration as r;
#[path = "parameter_stream_register_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: r::native_entry::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest,
    pub body_actual: r::ActualBody, pub body: r::Captures, pub native: r::native_return::History }
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
    pub fn bind_actual_registration_entry(mut self, actual: ActualCall) -> Result<r::native_entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::Register { return Err(Rejected { parent: self, actual, fault: Admission::NotPending }); }
        let request = match self.registration_request() {
            Ok(request) => request, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }),
        };
        self.captures.requests[request.site.index()] = Some(request);
        if self.owners.calls[request.site.index()].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || !matches!(request.target, super::Target::DirectRva(0x55e3d4)) || actual.child.pc != 0x55e3d4
            || actual.child.source_image_base != self.actual.source_image_base || actual.child.rsp != actual.caller_rsp.wrapping_sub(8)
            || Some(actual.child.rcx) != request.args.rcx || actual.child.saved_rbx != request.required_saved.rbx {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let state = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match r::native_entry::Prepared::from_actual_entry(r::native_entry::Input { services: Scope { services: self.services }, storage: state, actual: actual.child }) {
            Ok(prepared) => Ok(prepared),
            Err(e) => Err(Rejected { parent: e.input.storage.into_parent(e.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> r::native_entry::Entered<Scope<S>> {
    pub fn into_registration_body(self) -> r::Prepared<Scope<S>> { self.into_body_with(|services, owners| (services, owners)) }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: r::native_return::History, pub phase: r::native_return::Phase,
    pub attempted: Option<r::native_return::Phase>, pub stopped: bool }
pub struct Retained<R> { pub allocation: Option<r::AllocationReturned<R>>, pub body_actual: r::ActualBody,
    pub body: r::Captures, pub phase: r::Phase, pub attempted: Option<r::Phase>, pub stopped: bool,
    pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub retained: Retained<S::RegistrationAllocationOwner> }
pub struct EntryPreserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall,
    pub request: super::CallRequest, pub entry_actual: r::native_entry::ActualEntry, pub entry: r::native_entry::Captures,
    pub phase: r::native_entry::Phase, pub attempted: Option<r::native_entry::Phase>, pub stopped: bool,
    pub allocation: Option<r::AllocationReturned<S::RegistrationAllocationOwner>> }
impl<S: Services> r::native_entry::Parts<Scope<S>> {
    pub fn into_preserved_lookup(self) -> EntryPreserved<S> {
        let state = self.owners.storage; let actual = state.binding; let request = state.request;
        EntryPreserved { parent: state.into_parent(self.services.services).into_parts(), actual, request,
            entry_actual: self.actual, entry: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped, allocation: self.owners.allocation }
    }
}
fn preserve<S: Services>(body: r::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage; let actual = state.binding; let request = state.request;
    Preserved { parent: state.into_parent(body.services.services).into_parts(), actual, request,
        retained: Retained { allocation: body.owners.allocation, body_actual: body.actual, body: body.captures,
            phase: body.phase, attempted: body.attempted, stopped: body.stopped, native } }
}
impl<S: Services> r::Parts<Scope<S>> { pub fn into_preserved_lookup(self) -> Preserved<S> { preserve(self, None) } }
impl<S: Services> r::native_return::Parts<Scope<S>> {
    pub fn into_preserved_lookup(self) -> Preserved<S> {
        let native = NativeProgress { history: r::native_return::History { actual: self.actual, captures: self.captures },
            phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>, pub held: Option<Retained<S::RegistrationAllocationOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> r::native_return::Returned<Scope<S>> {
    pub fn into_lookup_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts(); let state = parts.body.owners.storage;
        let history = History { actual: state.binding, request: state.request, body_actual: parts.body.actual, body: parts.body.captures, native };
        let mut held = Some(Retained { allocation: parts.body.owners.allocation, body_actual: parts.body.actual, body: parts.body.captures,
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
        if let Err(e) = parent.services.adopt_completed_registration(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, held, fault: ReturnFault::ResourcesNotAdopted }); }
        parent.captures.registration_call = Some(history);
        parent.record_registration_return(); parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
