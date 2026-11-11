use super::{Scope, FormatLookupServices, Resources, resources, c, format_call};
use c::object_lookup as l;
use l::{guard as g, guard_call as gc};
pub trait FormatLookupGuardServices: FormatLookupServices {
    type NumberFormatLookupGuardCallOwner; type NumberFormatLookupGuardTailOwner;

    fn number_format_constructor_lookup_guard_write_index(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_guard_read_index(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn number_format_constructor_lookup_guard_call(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_guard_mov_rax_rbx(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_guard_add_rsp(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_guard_pop_rbx(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_guard_ret(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_guard_tail(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::NumberFormatLookupGuardTailOwner>>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_guard_push_rbx(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_guard_sub_rsp(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::NumberFormatLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error>;

    fn number_adopt_completed_format_constructor_lookup_guard(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, history: gc::History, held: &mut Option<gc::Retained<Self::NumberFormatLookupGuardCallOwner, Self::NumberFormatLookupGuardTailOwner>>) -> Result<(), Self::Error>;
}
impl<S: FormatLookupGuardServices> format_call::LookupGuardServices for Scope<S> {
    type FormatLookupGuardCallOwner = S::NumberFormatLookupGuardCallOwner; type FormatLookupGuardTailOwner = S::NumberFormatLookupGuardTailOwner;
    fn format_constructor_lookup_guard_write_index(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_write_index(resources(r), ctor, lookup, call, pc, address, word)
    }
    fn format_constructor_lookup_guard_read_index(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.number_format_constructor_lookup_guard_read_index(resources(r), ctor, lookup, call, pc, address)
    }
    fn format_constructor_lookup_guard_call(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_call(resources(r), ctor, lookup, call, request)
    }
    fn format_constructor_lookup_guard_mov_rax_rbx(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_mov_rax_rbx(resources(r), ctor, lookup, call, pc, word, returned)
    }
    fn format_constructor_lookup_guard_add_rsp(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_add_rsp(resources(r), ctor, lookup, call, pc, amount, returned)
    }
    fn format_constructor_lookup_guard_pop_rbx(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_pop_rbx(resources(r), ctor, lookup, call, pc, returned)
    }
    fn format_constructor_lookup_guard_ret(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_ret(resources(r), ctor, lookup, call, pc, returned)
    }
    fn format_constructor_lookup_guard_tail(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::FormatLookupGuardTailOwner>>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_tail(resources(r), ctor, lookup, call, request, tail)
    }
    fn format_constructor_lookup_guard_push_rbx(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_push_rbx(resources(r), ctor, lookup, call, pc, returned)
    }
    fn format_constructor_lookup_guard_sub_rsp(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, call: &mut Option<g::CallReturned<Self::FormatLookupGuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_guard_sub_rsp(resources(r), ctor, lookup, call, pc, amount, returned)
    }
    fn adopt_completed_format_constructor_lookup_guard(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, history: gc::History, held: &mut Option<gc::Retained<Self::FormatLookupGuardCallOwner, Self::FormatLookupGuardTailOwner>>) -> Result<(), Self::Error> {
        self.services.number_adopt_completed_format_constructor_lookup_guard(resources(r), ctor, lookup, history, held)
    }
}

