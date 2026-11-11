use super::{l, Scope, Services};
use super::super as c;
use l::{guard as g, guard_call as gc};
pub trait GuardServices: Services {
    type LookupGuardCallOwner; type LookupGuardTailOwner;

    fn lookup_guard_write_index(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn lookup_guard_read_index(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn lookup_guard_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error>;
    fn lookup_guard_mov_rax_rbx(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn lookup_guard_add_rsp(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_guard_pop_rbx(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error>;
    fn lookup_guard_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn lookup_guard_tail(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>) -> Result<(), Self::Error>;
    fn lookup_guard_push_rbx(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error>;
    fn lookup_guard_sub_rsp(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error>;

    fn adopt_completed_lookup_guard(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, history: gc::History, held: &mut Option<gc::Retained<Self::LookupGuardCallOwner, Self::LookupGuardTailOwner>>) -> Result<(), Self::Error>;
}
impl<S: GuardServices> gc::Services for Scope<S> {
    type GuardCallOwner = S::LookupGuardCallOwner; type GuardTailOwner = S::LookupGuardTailOwner;
    fn guard_write_index(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.lookup_guard_write_index(&mut o.storage.owners, &mut o.calls, call, pc, address, word)
    }
    fn guard_read_index(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.lookup_guard_read_index(&mut o.storage.owners, &mut o.calls, call, pc, address)
    }
    fn guard_call(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error> {
        self.services.lookup_guard_call(&mut o.storage.owners, &mut o.calls, call, request)
    }
    fn guard_mov_rax_rbx(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.lookup_guard_mov_rax_rbx(&mut o.storage.owners, &mut o.calls, call, pc, word, returned)
    }
    fn guard_add_rsp(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_guard_add_rsp(&mut o.storage.owners, &mut o.calls, call, pc, amount, returned)
    }
    fn guard_pop_rbx(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.lookup_guard_pop_rbx(&mut o.storage.owners, &mut o.calls, call, pc, returned)
    }
    fn guard_ret(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.lookup_guard_ret(&mut o.storage.owners, &mut o.calls, call, pc, returned)
    }
    fn guard_tail(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>) -> Result<(), Self::Error> {
        self.services.lookup_guard_tail(&mut o.storage.owners, &mut o.calls, call, request, tail)
    }
    fn guard_push_rbx(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.lookup_guard_push_rbx(&mut o.storage.owners, &mut o.calls, call, pc, returned)
    }
    fn guard_sub_rsp(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.lookup_guard_sub_rsp(&mut o.storage.owners, &mut o.calls, call, pc, amount, returned)
    }
    fn adopt_completed_guard(&mut self, o: &mut l::Owners<Self>, history: gc::History, held: &mut Option<gc::Retained<Self::GuardCallOwner, Self::GuardTailOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_lookup_guard(&mut o.storage.owners, &mut o.calls, history, held)
    }
}

