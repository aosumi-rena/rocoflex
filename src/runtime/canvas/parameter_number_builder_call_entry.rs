use super::super as number;
use super::builder;
use builder::native_entry as entry;
use core::marker::PhantomData;
pub trait Services: super::Services {
    fn builder_entry_push(&mut self, owners: &mut number::Owners<Self>, calls: &mut super::Calls<Self::BuilderCallOwner>, pc: u32,
        register: entry::Register, returned: &mut Option<entry::Push>) -> Result<(), Self::Error>;
    fn builder_entry_sub(&mut self, owners: &mut number::Owners<Self>, calls: &mut super::Calls<Self::BuilderCallOwner>, returned: &mut Option<entry::Subtract>) -> Result<(), Self::Error>;
    fn builder_entry_rbp(&mut self, owners: &mut number::Owners<Self>, calls: &mut super::Calls<Self::BuilderCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct ActualCallEntry { pub call_pc: u32, pub caller_rsp: u64, pub child: entry::ActualEntry }
pub struct EntryState<S: Services> { parent: number::Prepared<S>, binding: ActualCallEntry, request: number::CallRequest }

pub struct Driver<S: Services> { marker: PhantomData<fn() -> S> }
pub enum Admission<E> { NotPendingBuilder, Request(number::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: number::Prepared<S>, pub actual: ActualCallEntry, pub fault: Admission<S::Error> }
impl<S: Services> number::Prepared<S> {
    pub fn bind_actual_builder_entry(mut self, actual: ActualCallEntry) -> Result<entry::Prepared<Driver<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != number::Phase::Builder { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingBuilder }); }
        let request = match self.builder_request() { Ok(r) => r,
            Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }) };
        self.captures.requests[number::CallSite::Builder.index()] = Some(request);
        if self.owners.calls[number::CallSite::Builder.index()].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        let a = actual.child; let saved = request.required_saved;
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || a.pc != 0x1fb420 || a.rsp != actual.caller_rsp.wrapping_sub(8)
            || a.source_image_base != self.actual.actual_image_base || a.rcx != request.args.rcx || Some(a.rdx) != request.args.rdx
            || a.saved != [saved.rbp, saved.rsi, saved.rdi, saved.rbx] {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let input = entry::Input { services: Driver::<S> { marker: PhantomData }, storage: EntryState { parent: self, binding: actual, request }, actual: a };
        match entry::Prepared::from_actual_entry(input) {
            Ok(cursor) => Ok(cursor),
            Err(rejected) => Err(Rejected { parent: rejected.input.storage.parent, actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> entry::Services for Driver<S> {
    type Storage = EntryState<S>; type CallOwner = S::BuilderCallOwner; type Error = S::Error;
    fn push_saved(&mut self, state: &mut Self::Storage, calls: &mut entry::Calls<Self::CallOwner>, pc: u32,
        register: entry::Register, returned: &mut Option<entry::Push>) -> Result<(), Self::Error> {
        state.parent.services.builder_entry_push(&mut state.parent.owners, calls, pc, register, returned)
    }
    fn sub_rsp38(&mut self, state: &mut Self::Storage, calls: &mut entry::Calls<Self::CallOwner>, returned: &mut Option<entry::Subtract>) -> Result<(), Self::Error> {
        state.parent.services.builder_entry_sub(&mut state.parent.owners, calls, returned)
    }
    fn lea_rbp30(&mut self, state: &mut Self::Storage, calls: &mut entry::Calls<Self::CallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        state.parent.services.builder_entry_rbp(&mut state.parent.owners, calls, returned)
    }
    fn write_state(&mut self, state: &mut Self::Storage, calls: &mut entry::Calls<Self::CallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        state.parent.services.builder_write_word(&mut state.parent.owners, calls, pc, address, word)
    }
}
impl<S: Services> entry::Entered<Driver<S>> {
    pub fn into_number_builder_body(self) -> builder::Prepared<super::Scope<S>> {
        let a = self.storage().binding; let request = self.storage().request;
        let binding = super::ActualCall { call_pc: a.call_pc, caller_rsp: a.caller_rsp, child: self.actual_body() };
        self.into_body_with(|_routing, state| {
            let p = state.parent;
            let storage = super::ParentState { owners: p.owners, actual: p.actual, captures: p.captures, saved: p.saved,
                phase: p.phase, attempted: p.attempted, stopped: true, binding, request };
            (super::Scope { services: p.services }, storage)
        })
    }
}
pub struct Preserved<S: Services> { pub parent: number::Parts<S>, pub actual: ActualCallEntry, pub request: number::CallRequest,
    pub calls: super::Calls<S::BuilderCallOwner>, pub captures: entry::Captures, pub phase: entry::Phase,
    pub index: usize, pub attempted: Option<(entry::Phase, usize)>, pub stopped: bool }
impl<S: Services> entry::Parts<Driver<S>> {

    pub fn into_preserved_number(self) -> Preserved<S> {
        Preserved { parent: self.storage.parent.into_parts(), actual: self.storage.binding, request: self.storage.request,
            calls: self.calls, captures: self.captures, phase: self.phase, index: self.index, attempted: self.attempted, stopped: self.stopped }
    }
}
