use super::{l, Scope, Services, ParentState, ActualCall};
use l::native_entry as entry;
#[derive(Clone, Copy, Debug)]
pub struct ActualEntryCall { pub call_pc: u32, pub caller_rsp: u64, pub child: entry::ActualEntry }
pub enum Admission<E> { NotPendingLookup, Request(super::super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::super::Prepared<S>, pub actual: ActualEntryCall, pub fault: Admission<S::Error> }
impl<S: Services> super::super::Prepared<S> {
    pub fn bind_actual_lookup_entry(mut self, actual: ActualEntryCall) -> Result<entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::super::Phase::GetObject { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingLookup }); }
        let request = match self.lookup_request() { Ok(r) => r, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.requests[super::super::CallSite::GetObject.index()] = Some(request);
        if self.owners.calls[super::super::CallSite::GetObject.index()].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        let saved = request.required_saved;
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp || actual.child.pc != 0x18c2d0
            || actual.child.source_image_base != self.actual.source_image_base || actual.child.rsp != actual.caller_rsp.wrapping_sub(8)
            || Some(actual.child.rcx) != request.args.rcx || actual.child.saved != [saved.rbp, saved.r14, saved.rsi, saved.rdi, saved.rbx] {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }

        let binding = ActualCall { call_pc: actual.call_pc, caller_rsp: actual.caller_rsp, child: l::ActualBody {
            pc: 0x18c2ed, source_image_base: actual.child.source_image_base, rbp: actual.child.rsp.wrapping_sub(0x48),
            holder_rcx: actual.child.rcx, entry_rbx: actual.child.saved[4], entry_rdi: actual.child.saved[3], entry_r14: actual.child.saved[1] } };
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved, phase: self.phase, index: self.index,
            attempted: self.attempted, stopped: true, binding, request };
        match entry::Prepared::from_actual_entry(entry::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor), Err(r) => Err(Rejected { parent: r.input.storage.into_parent(r.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> entry::Entered<Scope<S>> {

    pub fn into_constructor_lookup_body(self) -> l::Prepared<Scope<S>> { self.into_body_with(|s, o| (s, o)) }
}
pub struct Retained<R> { pub actual: ActualEntryCall, pub request: super::super::CallRequest, pub calls: l::Calls<R>,
    pub captures: entry::Captures, pub phase: entry::Phase, pub index: usize, pub attempted: Option<(entry::Phase, usize)>, pub stopped: bool }
pub struct Preserved<S: Services> { pub parent: super::super::Parts<S>, pub retained: Retained<S::LookupCallOwner> }
impl<S: Services> entry::Parts<Scope<S>> {
    pub fn into_preserved_constructor(self) -> Preserved<S> {
        let state = self.owners.storage;
        let retained = Retained { actual: ActualEntryCall { call_pc: state.binding.call_pc, caller_rsp: state.binding.caller_rsp, child: self.actual }, request: state.request,
            calls: self.owners.calls, captures: self.captures, phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped };
        Preserved { parent: state.into_parent(self.services.services).into_parts(), retained }
    }
}
