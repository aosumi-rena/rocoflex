use super::{Scope, LookupServices};
use super::super::{self as f, constructor as c};
use c::{object_lookup as l, object_lookup_call};
use l::{guard as g, guard_call as gc};
pub trait LookupGuardServices: LookupServices {
    type ConstructorLookupGuardCallOwner; type ConstructorLookupGuardTailOwner;

    fn constructor_lookup_guard_write_index(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn constructor_lookup_guard_read_index(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn constructor_lookup_guard_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error>;
    fn constructor_lookup_guard_mov_rax_rbx(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn constructor_lookup_guard_add_rsp(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn constructor_lookup_guard_pop_rbx(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error>;
    fn constructor_lookup_guard_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn constructor_lookup_guard_tail(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::ConstructorLookupGuardTailOwner>>) -> Result<(), Self::Error>;
    fn constructor_lookup_guard_push_rbx(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error>;
    fn constructor_lookup_guard_sub_rsp(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error>;

    fn adopt_completed_constructor_lookup_guard(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, history: gc::History, held: &mut Option<gc::Retained<Self::ConstructorLookupGuardCallOwner, Self::ConstructorLookupGuardTailOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupGuardServices> object_lookup_call::GuardServices for Scope<S> {
    type LookupGuardCallOwner = S::ConstructorLookupGuardCallOwner; type LookupGuardTailOwner = S::ConstructorLookupGuardTailOwner;
    fn lookup_guard_write_index(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_write_index(&mut o.storage.owners, &mut o.calls, lookup, call, pc, address, word)
    }
    fn lookup_guard_read_index(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.constructor_lookup_guard_read_index(&mut o.storage.owners, &mut o.calls, lookup, call, pc, address)
    }
    fn lookup_guard_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_call(&mut o.storage.owners, &mut o.calls, lookup, call, request)
    }
    fn lookup_guard_mov_rax_rbx(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_mov_rax_rbx(&mut o.storage.owners, &mut o.calls, lookup, call, pc, word, returned)
    }
    fn lookup_guard_add_rsp(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_add_rsp(&mut o.storage.owners, &mut o.calls, lookup, call, pc, amount, returned)
    }
    fn lookup_guard_pop_rbx(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_pop_rbx(&mut o.storage.owners, &mut o.calls, lookup, call, pc, returned)
    }
    fn lookup_guard_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_ret(&mut o.storage.owners, &mut o.calls, lookup, call, pc, returned)
    }
    fn lookup_guard_tail(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_tail(&mut o.storage.owners, &mut o.calls, lookup, call, request, tail)
    }
    fn lookup_guard_push_rbx(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_push_rbx(&mut o.storage.owners, &mut o.calls, lookup, call, pc, returned)
    }
    fn lookup_guard_sub_rsp(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, call: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_guard_sub_rsp(&mut o.storage.owners, &mut o.calls, lookup, call, pc, amount, returned)
    }
    fn adopt_completed_lookup_guard(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, history: gc::History, held: &mut Option<gc::Retained<Self::LookupGuardCallOwner, Self::LookupGuardTailOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_constructor_lookup_guard(&mut o.storage.owners, &mut o.calls, lookup, history, held)
    }
}

