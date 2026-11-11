use super::text_copy as copy;
use super::{CallRequest, CallReturned, CallSite, Nonvolatile, Owners};
#[path = "parameter_number_text_copy_call_entry.rs"]
pub mod native_entry;
pub trait Services: super::Services {
    fn text_copy_read_byte(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn text_copy_write_byte(&mut self, owners: &mut Owners<Self>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
    fn text_copy_pop(&mut self, owners: &mut Owners<Self>, pc: u32, register: copy::native_return::Register,
        returned: &mut Option<copy::native_return::Pop>) -> Result<(), Self::Error>;
    fn text_copy_ret(&mut self, owners: &mut Owners<Self>, returned: &mut Option<copy::native_return::ActualReturn>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child_body_rsp: u64, pub child: copy::ActualBody }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: CallRequest,
    pub body: copy::Captures, pub native: copy::native_return::History }
pub struct ParentState<S: Services> { owners: Owners<S>, actual: super::ActualBody,
    captures: super::Captures, saved: Option<Nonvolatile>, phase: super::Phase,
    attempted: Option<super::Phase>, stopped: bool, binding: ActualCall, request: CallRequest }
pub struct Scope<S: Services> { services: S }
impl<S: Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, saved: self.saved, phase: self.phase,
        attempted: self.attempted, stopped: self.stopped } }
}
pub enum Admission<E> { NotPendingCopy, Request(super::Fault<E>), CallBinding, OccupiedCall, BodyEntry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_text_copy_body(mut self, actual: ActualCall) -> Result<copy::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::Copy { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingCopy }); }
        let request = match self.copy_request() {
            Ok(request) => request,
            Err(error) => return Err(Rejected { parent: self, actual, fault: Admission::Request(error) }),
        };
        self.captures.requests[CallSite::Copy.index()] = Some(request);
        if self.owners.calls[CallSite::Copy.index()].is_some() {
            return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall });
        }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || actual.child_body_rsp != actual.caller_rsp.wrapping_sub(0x28)
            || actual.child.output_slot_rcx != request.args.rcx || Some(actual.child.source_rdx) != request.args.rdx
            || Some(u64::from(actual.child.count_r8d)) != request.args.r8 || actual.child.pc != 0x1fb214 {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures,
            saved: self.saved, phase: self.phase, attempted: self.attempted, stopped: true, binding: actual, request };
        match copy::Prepared::from_actual_body(copy::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor),
            Err(rejected) => Err(Rejected { parent: rejected.input.storage.into_parent(rejected.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
impl<S: Services> copy::Services for Scope<S> {
    type Storage = ParentState<S>;
    type Error = S::Error;
    fn read_byte(&mut self, state: &mut Self::Storage, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.text_copy_read_byte(&mut state.owners, pc, address)
    }
    fn read_word(&mut self, state: &mut Self::Storage, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.read_word(&mut state.owners, pc, address)
    }
    fn write_word(&mut self, state: &mut Self::Storage, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.write_word(&mut state.owners, pc, address, word)
    }
    fn write_byte(&mut self, state: &mut Self::Storage, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error> {
        self.services.text_copy_write_byte(&mut state.owners, pc, address, byte)
    }
}
impl<S: Services> copy::native_return::Services for Scope<S> {
    fn pop_saved(&mut self, state: &mut Self::Storage, pc: u32, register: copy::native_return::Register,
        returned: &mut Option<copy::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.text_copy_pop(&mut state.owners, pc, register, returned)
    }
    fn ret_1fb416(&mut self, state: &mut Self::Storage, returned: &mut Option<copy::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.text_copy_ret(&mut state.owners, returned)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: copy::native_return::History, pub phase: copy::native_return::Phase,
    pub pop: usize, pub attempted: Option<(copy::native_return::Phase, usize)>, pub stopped: bool }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall, pub request: CallRequest,
    pub body_actual: copy::ActualBody, pub body: copy::Captures, pub phase: copy::Phase,
    pub attempted: Option<copy::Phase>, pub stopped: bool, pub native: Option<NativeProgress> }
fn preserve<S: Services>(body: copy::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.storage;
    Preserved { actual: state.binding, request: state.request, body_actual: body.actual,
        body: body.captures, phase: body.phase, attempted: body.attempted, stopped: body.stopped, native,
        parent: state.into_parent(body.services.services).into_parts() }
}
impl<S: Services> copy::Parts<Scope<S>> {
    pub fn into_preserved_number(self) -> Preserved<S> { preserve(self, None) }
}
impl<S: Services> copy::native_return::Parts<Scope<S>> {
    pub fn into_preserved_number(self) -> Preserved<S> {
        let native = NativeProgress { history: copy::native_return::History { input: self.input, captures: self.captures },
            phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(native))
    }
}
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, Parent(super::Fault<E>) }
pub struct ReturnRejected<S: Services> { pub parent: super::Parts<S>, pub history: History,
    pub supplied: Option<CallReturned<S::CallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> copy::native_return::Returned<Scope<S>> {

    pub fn into_number_parent(self, supplied: CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return(); let native = self.history(); let parts = self.into_parts();
        let state = parts.body.storage;
        let history = History { actual: state.binding, request: state.request, body: parts.body.captures, native };
        let binding = state.binding; let request = state.request;
        let mut parent = state.into_parent(parts.body.services.services);
        let actual = supplied.actual;
        if native.input.actual_body_rsp != binding.child_body_rsp
            || returned.target != parent.actual.actual_image_base.wrapping_add(0x1caf43)
            || returned.after_rsp != binding.caller_rsp || actual.rax != returned.rax
            || actual.resume_pc != 0x1caf43 || actual.work_rsp != binding.caller_rsp
            || [actual.saved.rbx, actual.saved.rbp, actual.saved.rdi, actual.saved.rsi] != returned.restored {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), fault: ReturnFault::ReturnBinding });
        }
        if parent.owners.calls[CallSite::Copy.index()].is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: Some(supplied), fault: ReturnFault::OccupiedCall });
        }
        parent.owners.calls[CallSite::Copy.index()] = Some(supplied);
        if let Err(error) = parent.record_call_return(request) {
            return Err(ReturnRejected { parent: parent.into_parts(), history, supplied: None, fault: ReturnFault::Parent(error) });
        }
        parent.captures.text_copy_call = Some(history); parent.phase = super::Phase::NativeReturnRequired;
        parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
