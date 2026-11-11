use super::{number, Calls, ParentState, Scope};
use super::super as digits;
use number::native_entry as entry;
use core::marker::PhantomData;

pub trait Services: super::Services {
    fn number_entry_push(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>,
        pc: u32, register: entry::Register, returned: &mut Option<entry::Push>) -> Result<(), Self::Error>;
    fn number_entry_sub_d8(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>,
        returned: &mut Option<entry::Subtract>) -> Result<(), Self::Error>;
    fn number_entry_lea_rbp80(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>,
        returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn number_entry_save_xmm6(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>,
        address: u64, returned: &mut Option<entry::XmmStore>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct ActualCallEntry { pub call_pc: u32, pub caller_rsp: u64, pub entry: entry::ActualEntry }

pub struct EntryState<S: Services> { parent: digits::Prepared<S>, binding: ActualCallEntry }

pub struct Driver<S: Services> { marker: PhantomData<fn() -> S> }
pub enum Admission { NotPendingFormat, MissingState, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: digits::Prepared<S>, pub actual: ActualCallEntry, pub fault: Admission }

impl<S: Services> digits::Prepared<S> {
    pub fn bind_actual_number_entry(mut self, actual: ActualCallEntry) -> Result<entry::Prepared<Driver<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != digits::Phase::Format {
            return Err(Rejected { parent: self, actual, fault: Admission::NotPendingFormat });
        }
        let Some(request) = digits::format_request(self.actual, &self.captures) else {
            return Err(Rejected { parent: self, actual, fault: Admission::MissingState });
        };
        self.captures.format_request = Some(request);
        let Some(limit) = self.captures.limit else {
            return Err(Rejected { parent: self, actual, fault: Admission::MissingState });
        };
        let a = actual.entry;
        let value = u64::from(a.xmm1[0]) | (u64::from(a.xmm1[1]) << 32);
        if actual.call_pc != request.pc || actual.caller_rsp != self.actual.rbp.wrapping_sub(0x40)
            || a.rsp != actual.caller_rsp.wrapping_sub(8) || a.pc != 0x1cae80
            || a.actual_image_base != self.actual.actual_image_base || a.rcx != request.destination_rcx
            || a.rdx != request.unchanged_rdx || a.r8d != request.digits_r8d || a.r9d != request.fourth_r9d
            || a.saved[0] != self.actual.rbp || a.saved[1] != self.actual.output_rdx
            || a.saved[3] != u64::from(limit) || value != request.value_xmm1_f64_bits {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }

        let input = entry::Input { services: Driver::<S> { marker: PhantomData },
            storage: EntryState { parent: self, binding: actual }, actual: a };
        match entry::Prepared::from_actual_entry(input) {
            Ok(cursor) => Ok(cursor),
            Err(rejected) => Err(Rejected { parent: rejected.input.storage.parent, actual, fault: Admission::Entry }),
        }
    }
}
impl<S: Services> number::Services for Driver<S> {
    type Storage = EntryState<S>;
    type CallOwner = S::NumberCallOwner;
    type Error = S::Error;
    fn read_word(&mut self, o: &mut number::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        let p = &mut o.storage.parent;
        p.services.number_read_word(&mut p.owners, &mut o.calls, pc, address)
    }
    fn write_word(&mut self, o: &mut number::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        let p = &mut o.storage.parent;
        p.services.number_write_word(&mut p.owners, &mut o.calls, pc, address, word)
    }
    fn xchg_ref_zero(&mut self, o: &mut number::Owners<Self>, pc: u32, address: u64, prior: &mut Option<u32>) -> Result<(), Self::Error> {
        let p = &mut o.storage.parent;
        p.services.number_xchg_ref_zero(&mut p.owners, &mut o.calls, pc, address, prior)
    }
    fn call(&mut self, o: &mut number::Owners<Self>, request: number::CallRequest) -> Result<(), Self::Error> {
        let p = &mut o.storage.parent;
        p.services.number_call(&mut p.owners, &mut o.calls, request)
    }
}
impl<S: Services> entry::Services for Driver<S> {
    fn entry_push(&mut self, o: &mut number::Owners<Self>, pc: u32, register: entry::Register,
        returned: &mut Option<entry::Push>) -> Result<(), Self::Error> {
        let p = &mut o.storage.parent;
        p.services.number_entry_push(&mut p.owners, &mut o.calls, pc, register, returned)
    }
    fn entry_sub_d8(&mut self, o: &mut number::Owners<Self>, returned: &mut Option<entry::Subtract>) -> Result<(), Self::Error> {
        let p = &mut o.storage.parent;
        p.services.number_entry_sub_d8(&mut p.owners, &mut o.calls, returned)
    }
    fn entry_lea_rbp80(&mut self, o: &mut number::Owners<Self>, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        let p = &mut o.storage.parent;
        p.services.number_entry_lea_rbp80(&mut p.owners, &mut o.calls, returned)
    }
    fn entry_save_xmm6(&mut self, o: &mut number::Owners<Self>, address: u64,
        returned: &mut Option<entry::XmmStore>) -> Result<(), Self::Error> {
        let p = &mut o.storage.parent;
        p.services.number_entry_save_xmm6(&mut p.owners, &mut o.calls, address, returned)
    }
}
impl<S: Services> entry::Entered<Driver<S>> {

    pub fn into_formatter_number_body(self) -> number::Prepared<Scope<S>> {
        let body = self.actual_body(); let actual = self.storage().binding;
        let binding = super::ActualCall { call_pc: actual.call_pc, caller_rsp: actual.caller_rsp,
            callee_entry_rsp: actual.entry.rsp, callee_rdx: actual.entry.rdx,
            original_rdi: actual.entry.saved[2], original_xmm6: actual.entry.xmm6, callee: body };
        self.into_body_with(|_routing, owners| {
            let p = owners.storage.parent;
            let storage = ParentState { owners: p.owners, actual: p.actual, captures: p.captures,
                phase: p.phase, attempted: p.attempted, cursor: p.cursor, remaining: p.remaining,
                stopped: true, binding };
            (Scope { services: p.services }, number::Owners { storage, calls: owners.calls })
        })
    }
}
pub struct PreservedEntry<S: Services> {
    pub parent: digits::Parts<S>, pub actual: ActualCallEntry, pub calls: Calls<S::NumberCallOwner>,
    pub captures: entry::Captures, pub phase: entry::Phase, pub index: usize,
    pub attempted: Option<(entry::Phase, usize)>, pub stopped: bool,
}
impl<S: Services> entry::Parts<Driver<S>> {

    pub fn into_preserved_formatter(self) -> PreservedEntry<S> {
        PreservedEntry { parent: self.owners.storage.parent.into_parts(), actual: self.owners.storage.binding,
            calls: self.owners.calls, captures: self.captures, phase: self.phase, index: self.index,
            attempted: self.attempted, stopped: self.stopped }
    }
}
