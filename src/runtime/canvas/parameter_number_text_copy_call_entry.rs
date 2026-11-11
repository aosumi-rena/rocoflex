use super::super as number;
use super::copy;
use copy::native_entry as entry;
use core::marker::PhantomData;
pub trait Services: super::Services {
    fn text_copy_push(&mut self, owners: &mut number::Owners<Self>, pc: u32, register: entry::Register,
        returned: &mut Option<entry::Push>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct ActualCallEntry { pub call_pc: u32, pub caller_rsp: u64, pub child: entry::ActualEntry }

pub struct EntryState<S: Services> { parent: number::Prepared<S>, binding: ActualCallEntry, request: number::CallRequest }
pub struct Driver<S: Services> { marker: PhantomData<fn() -> S> }
pub enum Admission<E> { NotPendingCopy, Request(number::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: number::Prepared<S>, pub actual: ActualCallEntry, pub fault: Admission<S::Error> }
impl<S: Services> number::Prepared<S> {
    pub fn bind_actual_text_copy_entry(mut self, actual: ActualCallEntry) -> Result<entry::Prepared<Driver<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != number::Phase::Copy { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingCopy }); }
        let request = match self.copy_request() {
            Ok(request) => request,
            Err(error) => return Err(Rejected { parent: self, actual, fault: Admission::Request(error) }),
        };
        self.captures.requests[number::CallSite::Copy.index()] = Some(request);
        if self.owners.calls[number::CallSite::Copy.index()].is_some() {
            return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall });
        }
        let saved = request.required_saved; let a = actual.child;
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || a.pc != 0x1fb210 || a.rsp != actual.caller_rsp.wrapping_sub(8)
            || a.rcx != request.args.rcx || Some(a.rdx) != request.args.rdx || Some(u64::from(a.r8d)) != request.args.r8
            || a.saved != [saved.rsi, saved.rdi, saved.rbp, saved.rbx] {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let input = entry::Input { services: Driver::<S> { marker: PhantomData },
            storage: EntryState { parent: self, binding: actual, request }, actual: a };
        match entry::Prepared::from_actual_entry(input) {
            Ok(cursor) => Ok(cursor),
            Err(rejected) => Err(Rejected { parent: rejected.input.storage.parent, actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> entry::Services for Driver<S> {
    type Storage = EntryState<S>;
    type Error = S::Error;
    fn push_saved(&mut self, state: &mut Self::Storage, pc: u32, register: entry::Register,
        returned: &mut Option<entry::Push>) -> Result<(), Self::Error> {
        state.parent.services.text_copy_push(&mut state.parent.owners, pc, register, returned)
    }
}
impl<S: Services> entry::Entered<Driver<S>> {

    pub fn into_number_text_copy_body(self) -> copy::Prepared<super::Scope<S>> {
        let a = self.storage().binding; let request = self.storage().request;
        let binding = super::ActualCall { call_pc: a.call_pc, caller_rsp: a.caller_rsp,
            child_body_rsp: self.actual_body_rsp(), child: copy::ActualBody { pc: 0x1fb214,
                output_slot_rcx: a.child.rcx, source_rdx: a.child.rdx, count_r8d: a.child.r8d } };
        self.into_body_with(|_routing, state| {
            let p = state.parent;
            let storage = super::ParentState { owners: p.owners, actual: p.actual, captures: p.captures,
                saved: p.saved, phase: p.phase, attempted: p.attempted, stopped: true, binding, request };
            (super::Scope { services: p.services }, storage)
        })
    }
}
pub struct Preserved<S: Services> { pub parent: number::Parts<S>, pub actual: ActualCallEntry,
    pub request: number::CallRequest, pub captures: entry::Captures, pub index: usize,
    pub attempted: Option<usize>, pub stopped: bool }
impl<S: Services> entry::Parts<Driver<S>> {

    pub fn into_preserved_number(self) -> Preserved<S> {
        Preserved { parent: self.storage.parent.into_parts(), actual: self.storage.binding,
            request: self.storage.request, captures: self.captures, index: self.index,
            attempted: self.attempted, stopped: self.stopped }
    }
}
