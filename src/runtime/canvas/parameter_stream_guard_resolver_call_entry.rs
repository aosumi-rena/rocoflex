use super::{r, i, Scope, Request, ActualCall, Admission};
use r::native_entry as e;
pub trait Services: super::Services {
    fn resolver_prologue_instruction(&mut self, o: &mut i::Owners<Self>, request: e::Request,
        returned: &mut Option<e::InstructionReturned>) -> Result<(), Self::Error>;
}
impl<S: Services> e::Services for Scope<S> {
    fn prologue_instruction(&mut self, o: &mut Self::Storage, request: e::Request,
        returned: &mut Option<e::InstructionReturned>) -> Result<(), Self::Error> {
        self.services.resolver_prologue_instruction(&mut o.owners, request, returned)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ActualEntryCall { pub source_pc: u32, pub child: e::ActualEntry }
pub struct Rejected<S: Services> { pub parent: i::Prepared<S>, pub actual: ActualEntryCall, pub fault: Admission<S::Error> }
impl<S: Services> i::Prepared<S> {
    pub fn bind_actual_resolver_entry(mut self, actual: ActualEntryCall) -> Result<e::Prepared<Scope<S>>, Rejected<S>> {
        let request = match self.suspend_resolver() { Ok(a) => a, Err(fault) => return Err(Rejected { parent: self, actual, fault }) };
        let a = actual.child; let args = request.arguments();
        if actual.source_pc != request.pc() || a.pc != 0x58ee04 || a.source_image_base != self.actual.source_image_base
            || a.rsp != request.entry_rsp() || a.rax != self.rax || a.rcx != args.rcx || a.rdx != args.rdx || a.r8 != args.r8 || a.r9 != args.r9 {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }

        let binding = ActualCall { source_pc: actual.source_pc, child: r::ActualBody { pc: 0x58ee20,
            source_image_base: a.source_image_base, work_rsp: a.rsp.wrapping_sub(0x48), ecx: a.rcx as u32,
            rdx: a.rdx, r8: a.r8, r9: a.r9, entry_rax: a.rax } };
        let (services, storage) = self.into_resolver_scope(binding, request);
        match e::Prepared::from_actual_entry(e::Input { services, storage, actual: a }) {
            Ok(p) => Ok(p), Err(f) => Err(Rejected { parent: f.input.storage.into_parent(f.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
pub struct Retained { pub actual: ActualEntryCall, pub request: Request, pub captures: e::Captures,
    pub index: usize, pub attempted: Option<usize>, pub stopped: bool }
pub struct Preserved<S: Services> { pub parent: i::Parts<S>, pub retained: Retained }
impl<S: Services> e::Parts<Scope<S>> {
    pub fn into_preserved_initializer(self) -> Preserved<S> {
        let state = self.storage;
        let retained = Retained { actual: ActualEntryCall { source_pc: state.binding.source_pc, child: self.actual },
            request: state.request, captures: self.captures, index: self.index, attempted: self.attempted, stopped: self.stopped };
        Preserved { parent: state.into_parent(self.services.services).into_parts(), retained }
    }
}
impl<S: Services> e::Entered<Scope<S>> {

    pub fn into_initializer_resolver_body(self) -> r::Prepared<Scope<S>> { self.into_checked_body() }
}
