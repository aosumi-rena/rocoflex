use super::{g, Scope, Services, ActualCall};
use g::native_entry as entry;
#[derive(Clone, Copy, Debug)]
pub struct ActualEntryCall { pub call_pc: u32, pub caller_rsp: u64, pub child: entry::ActualEntry }
pub enum Admission<E> { Parent(super::Admission<E>), CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::super::Prepared<S>, pub actual: ActualEntryCall, pub fault: Admission<S::Error> }
impl<S: Services> super::super::Prepared<S> {
    pub fn bind_actual_guard_entry(mut self, actual: ActualEntryCall) -> Result<entry::Prepared<Scope<S>>, Rejected<S>> {

        let old = self.stopped; self.stopped = true;
        let construct = actual.child.entry == g::Entry::Construct55e0a0;
        let entry_pc = if construct { 0x55e0a0 } else { 0x55e0dc };
        if actual.child.pc != entry_pc || actual.child.rsp != actual.caller_rsp.wrapping_sub(8)
            || (construct && (actual.child.constructor_saved_rbx != self.saved.map(|s| s.rbx) || actual.child.constructor_saved_rbx.is_none())) {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }

        self.stopped = old;
        let binding = ActualCall { call_pc: actual.call_pc, caller_rsp: actual.caller_rsp, child: g::ActualBody {
            entry: actual.child.entry, pc: actual.child.entry.body_pc(), source_image_base: actual.child.source_image_base,
            work_rsp: actual.child.rsp.wrapping_sub(0x28), guard_rcx: actual.child.rcx, constructor_edx: actual.child.constructor_edx } };
        let (services, storage) = match self.take_guard_parent(binding) {
            Ok(p) => p, Err(r) => return Err(Rejected { parent: r.parent, actual, fault: Admission::Parent(r.fault) }),
        };
        match entry::Prepared::from_actual_entry(entry::Input { services, storage, actual: actual.child }) {
            Ok(cursor) => Ok(cursor), Err(r) => Err(Rejected { parent: r.input.storage.into_parent(r.input.services.services), actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> entry::Entered<Scope<S>> {
    pub fn into_lookup_guard_body(self) -> g::Prepared<Scope<S>> { self.into_body_with(|s, o| (s, o)) }
}
pub struct Retained<R> { pub actual: ActualEntryCall, pub request: super::super::CallRequest,
    pub call: Option<g::CallReturned<R>>, pub captures: entry::Captures, pub phase: entry::Phase,
    pub attempted: Option<entry::Phase>, pub stopped: bool }
pub struct Preserved<S: Services> { pub parent: super::super::Parts<S>, pub retained: Retained<S::GuardCallOwner> }
impl<S: Services> entry::Parts<Scope<S>> {
    pub fn into_preserved_lookup(self) -> Preserved<S> {
        let state = self.owners.storage;
        let retained = Retained { actual: ActualEntryCall { call_pc: state.binding.call_pc, caller_rsp: state.binding.caller_rsp, child: self.actual },
            request: state.request, call: self.owners.call, captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        Preserved { parent: state.into_parent(self.services.services).into_parts(), retained }
    }
}
