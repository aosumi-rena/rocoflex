use super::{Scope, LookupServices, Resources, f, c, constructor_call};
use super::super::scope::resources;
use c::object_lookup as l;
use l::{guard as g, guard_call as gc};
pub trait LookupGuardServices: LookupServices {
    type FormatLookupGuardCallOwner; type FormatLookupGuardTailOwner;

    fn format_constructor_lookup_guard_write_index(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn format_constructor_lookup_guard_read_index(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn format_constructor_lookup_guard_call(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error>;
    fn format_constructor_lookup_guard_mov_rax_rbx(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_guard_add_rsp(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_guard_pop_rbx(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_guard_ret(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_guard_tail(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::FormatLookupGuardTailOwner>>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_guard_push_rbx(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_guard_sub_rsp(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error>;

    fn adopt_completed_format_constructor_lookup_guard(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, history: gc::History, held: &mut Option<gc::Retained<Self::FormatLookupGuardCallOwner, Self::FormatLookupGuardTailOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupGuardServices> constructor_call::LookupGuardServices for Scope<S> {
    type ConstructorLookupGuardCallOwner = S::FormatLookupGuardCallOwner; type ConstructorLookupGuardTailOwner = S::FormatLookupGuardTailOwner;
    fn constructor_lookup_guard_write_index(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_write_index(resources(o, None, None, None), ctor, lookup, call, pc, address, word)
    }
    fn constructor_lookup_guard_read_index(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.format_constructor_lookup_guard_read_index(resources(o, None, None, None), ctor, lookup, call, pc, address)
    }
    fn constructor_lookup_guard_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_call(resources(o, None, None, None), ctor, lookup, call, request)
    }
    fn constructor_lookup_guard_mov_rax_rbx(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_mov_rax_rbx(resources(o, None, None, None), ctor, lookup, call, pc, word, returned)
    }
    fn constructor_lookup_guard_add_rsp(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_add_rsp(resources(o, None, None, None), ctor, lookup, call, pc, amount, returned)
    }
    fn constructor_lookup_guard_pop_rbx(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_pop_rbx(resources(o, None, None, None), ctor, lookup, call, pc, returned)
    }
    fn constructor_lookup_guard_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_ret(resources(o, None, None, None), ctor, lookup, call, pc, returned)
    }
    fn constructor_lookup_guard_tail(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::ConstructorLookupGuardTailOwner>>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_tail(resources(o, None, None, None), ctor, lookup, call, request, tail)
    }
    fn constructor_lookup_guard_push_rbx(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_push_rbx(resources(o, None, None, None), ctor, lookup, call, pc, returned)
    }
    fn constructor_lookup_guard_sub_rsp(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, call: &mut Option<g::CallReturned<Self::ConstructorLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_guard_sub_rsp(resources(o, None, None, None), ctor, lookup, call, pc, amount, returned)
    }
    fn adopt_completed_constructor_lookup_guard(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, history: gc::History, held: &mut Option<gc::Retained<Self::ConstructorLookupGuardCallOwner, Self::ConstructorLookupGuardTailOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_format_constructor_lookup_guard(resources(o, None, None, None), ctor, lookup, history, held)
    }
}

