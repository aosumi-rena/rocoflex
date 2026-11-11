use super::{c, Scope, Services, ParentState, ActualCall};
use c::native_entry as entry;
#[derive(Clone, Copy, Debug)]
pub struct ActualEntryCall { pub call_pc: u32, pub caller_rsp: u64, pub child: entry::ActualEntry }
pub enum Admission<E> { NotPendingConstructor, Request(super::super::Fault<E>), OccupiedConstructor, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::super::Prepared<S>, pub actual: ActualEntryCall, pub fault: Admission<S::Error> }
impl<S: Services> super::super::Prepared<S> {
    pub fn bind_actual_constructor_entry(mut self, actual: ActualEntryCall) -> Result<entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::super::Phase::Construct { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingConstructor }); }
        let request = match self.constructor_request() { Ok(r) => r, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.constructor_request = Some(request);
        if self.owners.constructor.is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedConstructor }); }
        if actual.call_pc != request.call_pc || actual.caller_rsp != request.work_rsp || actual.child.pc != 0x1fbcf0
            || actual.child.source_image_base != self.actual.source_image_base || actual.child.rsp != actual.caller_rsp.wrapping_sub(8)
            || actual.child.rcx != request.rcx || actual.child.rdx != request.rdx || actual.child.r8 != u64::from(request.r8d)
            || actual.child.saved[0] != request.required_saved.rbp || actual.child.saved[2] != request.required_saved.rsi
            || actual.child.saved[4] != request.required_saved.rbx { return Err(Rejected { parent: self, actual, fault: Admission::CallBinding }); }

        let binding = ActualCall { call_pc: actual.call_pc, caller_rsp: actual.caller_rsp, child: c::ActualBody {
            pc: 0x1fbd07, source_image_base: actual.child.source_image_base, rbp: actual.child.rsp.wrapping_sub(0x28),
            object_rcx: actual.child.rcx, builder_rdx: actual.child.rdx, entry_r8: actual.child.r8,
            entry_rbx: actual.child.saved[4], entry_r14: actual.child.saved[1] } };
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved,
            phase: self.phase, attempted: self.attempted, stopped: true, binding, request };
        match entry::Prepared::from_actual_entry(entry::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor), Err(r) => Err(Rejected { parent: r.input.storage.into_parent(r.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> entry::Entered<Scope<S>> {

    pub fn into_formatter_constructor_body(self) -> c::Prepared<Scope<S>> { self.into_body_with(|s, o| (s, o)) }
}
pub struct Retained<R> { pub actual: ActualEntryCall, pub request: super::super::ConstructorRequest, pub calls: c::Calls<R>,
    pub captures: entry::Captures, pub phase: entry::Phase, pub index: usize, pub attempted: Option<(entry::Phase, usize)>, pub stopped: bool }
pub struct Preserved<S: Services> { pub parent: super::super::Parts<S>, pub retained: Retained<S::ConstructorCallOwner> }
impl<S: Services> entry::Parts<Scope<S>> {
    pub fn into_preserved_formatter(self) -> Preserved<S> {
        let state = self.owners.storage;
        let retained = Retained { actual: ActualEntryCall { call_pc: state.binding.call_pc, caller_rsp: state.binding.caller_rsp, child: self.actual },
            request: state.request, calls: self.owners.calls, captures: self.captures, phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped };
        Preserved { parent: state.into_parent(self.services.services).into_parts(), retained }
    }
}
