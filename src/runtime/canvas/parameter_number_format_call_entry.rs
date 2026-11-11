use super::{formatting as f, Services, Scope, ParentState, ActualCall};
use f::native_entry as entry;
#[derive(Clone, Copy, Debug)]
pub struct ActualEntryCall { pub call_pc: u32, pub caller_rsp: u64, pub child: entry::ActualEntry }
pub enum Admission<E> { NotPendingFormat, Request(super::super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::super::Prepared<S>, pub actual: ActualEntryCall, pub fault: Admission<S::Error> }
impl<S: Services> super::super::Prepared<S> {
    pub fn bind_actual_format_entry(mut self, actual: ActualEntryCall) -> Result<entry::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::super::Phase::Format { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingFormat }); }
        let request = match self.format_request() { Ok(r) => r,
            Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.requests[super::super::CallSite::Format.index()] = Some(request);
        if self.owners.calls[super::super::CallSite::Format.index()].is_some() {
            return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall });
        }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || actual.child.pc != 0x1fb590 || actual.child.source_image_base != self.actual.actual_image_base
            || actual.child.rsp != actual.caller_rsp.wrapping_sub(8) || actual.child.rcx != request.args.rcx
            || Some(u64::from(actual.child.r8d)) != request.args.r8 || Some(u64::from(actual.child.r9d)) != request.args.r9
            || actual.child.saved != [request.required_saved.rbp, request.required_saved.rsi, request.required_saved.rbx]
            || Some(actual.child.xmm1) != request.args.xmm1 || actual.child.xmm6 != request.required_saved.xmm6 {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }

        let binding = ActualCall { call_pc: actual.call_pc, caller_rsp: actual.caller_rsp,
            child: f::ActualBody { pc: 0x1fb5b4, source_image_base: actual.child.source_image_base,
                rbp: actual.child.rsp.wrapping_sub(0xd8), builder_rcx: actual.child.rcx, entry_rbx: actual.child.saved[2],
                precision_r8d: actual.child.r8d, flag_r9d: actual.child.r9d, value_xmm1: actual.child.xmm1 } };
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, saved: self.saved,
            phase: self.phase, attempted: self.attempted, stopped: true, binding, request };
        match entry::Prepared::from_actual_entry(entry::Input { services: Scope { services: self.services }, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor),
            Err(rejected) => Err(Rejected { parent: rejected.input.storage.into_parent(rejected.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> entry::Entered<Scope<S>> {

    pub fn into_number_format_body(self) -> f::Prepared<Scope<S>> { self.into_body_with(|s, o| (s, o)) }
}
pub struct Retained<C> { pub actual: ActualEntryCall, pub request: super::super::CallRequest,
    pub constructor: Option<f::ConstructorReturned<C>>, pub captures: entry::Captures, pub phase: entry::Phase,
    pub index: usize, pub attempted: Option<(entry::Phase, usize)>, pub stopped: bool }
pub struct Preserved<S: Services> { pub parent: super::super::Parts<S>, pub retained: Retained<S::FormatConstructorOwner> }
impl<S: Services> entry::Parts<Scope<S>> {
    pub fn into_preserved_number(self) -> Preserved<S> {
        let state = self.owners.storage;
        let retained = Retained { actual: ActualEntryCall { call_pc: state.binding.call_pc, caller_rsp: state.binding.caller_rsp, child: self.actual },
            request: state.request, constructor: self.owners.constructor, captures: self.captures, phase: self.phase,
            index: self.index, attempted: self.attempted, stopped: self.stopped };
        Preserved { parent: state.into_parent(self.services.services).into_parts(), retained }
    }
}
