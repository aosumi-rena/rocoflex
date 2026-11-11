use super::builder;
#[path = "parameter_number_builder_scope.rs"]
mod scope;
pub use scope::{Scope, Services};
#[path = "parameter_number_builder_call_entry.rs"]
pub mod native_entry;
pub type Calls<R> = [Option<builder::CallReturned<R>>; 7];
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: builder::ActualBody }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: super::CallRequest,
    pub body: builder::Captures, pub native: builder::native_return::History }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody,
    captures: super::Captures, saved: Option<super::Nonvolatile>, phase: super::Phase,
    attempted: Option<super::Phase>, stopped: bool, binding: ActualCall, request: super::CallRequest }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPendingBuilder, Request(super::Fault<E>), OccupiedCall, CallBinding, BodyEntry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_builder_body(mut self, actual: ActualCall) -> Result<builder::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::Builder { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingBuilder }); }
        let request = match self.builder_request() { Ok(r) => r,
            Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.requests[super::CallSite::Builder.index()] = Some(request);
        if self.owners.calls[super::CallSite::Builder.index()].is_some() {
            return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall });
        }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || actual.child.pc != 0x1fb435 || actual.child.source_image_base != self.actual.actual_image_base
            || actual.child.rbp != actual.caller_rsp.wrapping_sub(0x30)
            || actual.child.object_rcx != request.args.rcx || Some(actual.child.source_rdx) != request.args.rdx
            || actual.child.entry_rdi != request.required_saved.rdi {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved,
            phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match builder::Prepared::from_actual_body(builder::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor),
            Err(rejected) => Err(Rejected { parent: rejected.input.storage.into_parent(rejected.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: builder::native_return::History, pub phase: builder::native_return::Phase,
    pub pop: usize, pub attempted: Option<(builder::native_return::Phase, usize)>, pub stopped: bool }
pub struct Retained<R> { pub actual: ActualCall, pub request: super::CallRequest, pub calls: Calls<R>,
    pub body_actual: builder::ActualBody, pub body: builder::Captures, pub saved: Option<builder::Saved>,
    pub phase: builder::Phase, pub index: usize, pub attempted: Option<(builder::Phase, usize)>, pub stopped: bool,
    pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub retained: Retained<S::BuilderCallOwner> }
fn preserve<S: Services>(body: builder::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage;
    let retained = Retained { actual: state.binding, request: state.request, calls: body.owners.calls, body_actual: body.actual,
        body: body.captures, saved: body.saved, phase: body.phase, index: body.index, attempted: body.attempted, stopped: body.stopped, native };
    Preserved { parent: state.into_parent(body.services.services).into_parts(), retained }
}
impl<S: Services> builder::Parts<Scope<S>> {
    pub fn into_preserved_number(self) -> Preserved<S> { preserve(self, None) }
}
impl<S: Services> builder::native_return::Parts<Scope<S>> {
    pub fn into_preserved_number(self) -> Preserved<S> {
        let native = NativeProgress { history: builder::native_return::History { input: self.input, captures: self.captures },
            phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<super::CallReturned<S::CallOwner>>, pub retained: Option<Retained<S::BuilderCallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> builder::native_return::Returned<Scope<S>> {

    pub fn into_number_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts();
        let state = parts.body.owners.storage; let binding = state.binding; let request = state.request;
        let history = History { actual: binding, request, body: parts.body.captures, native };
        let mut held = Some(Retained { actual: binding, request, calls: parts.body.owners.calls,
            body_actual: parts.body.actual, body: parts.body.captures, saved: parts.body.saved, phase: parts.body.phase,
            index: parts.body.index, attempted: parts.body.attempted, stopped: parts.body.stopped,
            native: Some(NativeProgress { history: native, phase: parts.phase, pop: parts.pop, attempted: parts.attempted, stopped: parts.stopped }) });
        let mut parent = state.into_parent(parts.body.services.services); let actual = supplied.actual;
        if returned.target != parent.actual.actual_image_base.wrapping_add(0x1caeb8)
            || returned.after_rsp != binding.caller_rsp || native.input.actual_work_rsp != binding.child.rbp.wrapping_sub(0x30)
            || actual.rax != returned.rax || actual.work_rsp != binding.caller_rsp || actual.resume_pc != 0x1caeb8
            || [actual.saved.rbx, actual.saved.rdi, actual.saved.rsi, actual.saved.rbp] != returned.restored {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::ReturnBinding });
        }
        if parent.owners.calls[super::CallSite::Builder.index()].is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), retained: held, fault: ReturnFault::OccupiedCall });
        }
        parent.owners.calls[super::CallSite::Builder.index()] = Some(supplied);
        if let Err(e) = parent.record_call_return(request) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Parent(e) });
        }
        if let Err(e) = parent.services.adopt_completed_builder(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, retained: held, fault: ReturnFault::ResourcesNotAdopted });
        }
        parent.captures.builder_call = Some(history); parent.phase = super::Phase::Format;
        parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
