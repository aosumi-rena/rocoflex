use super::{Scope, ConstructorServices, Resources, f, c, constructor_call};
use super::super::scope::resources;
use c::{object_lookup as l, object_lookup_call};

pub trait LookupRegistrationServices: LookupServices {
    type FormatConstructorLookupRegistrationAllocationOwner;
    fn format_constructor_lookup_registration_allocate(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::FormatConstructorLookupRegistrationAllocationOwner>>, request: l::registration::AllocationRequest) -> Result<(), Self::Error>;
    fn format_constructor_lookup_registration_read_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::FormatConstructorLookupRegistrationAllocationOwner>>, request: l::registration::ReadRequest) -> Result<u64, Self::Error>;
    fn format_constructor_lookup_registration_store_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::FormatConstructorLookupRegistrationAllocationOwner>>, request: l::registration::StoreRequest) -> Result<(), Self::Error>;
    fn format_constructor_lookup_registration_push(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::FormatConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Push>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_registration_sub(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::FormatConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_registration_add(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::FormatConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_registration_pop(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::FormatConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::Pop>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_registration_ret(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::FormatConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn adopt_completed_format_constructor_lookup_registration(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, history: l::registration_call::History, held: &mut Option<l::registration_call::Retained<Self::FormatConstructorLookupRegistrationAllocationOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupRegistrationServices> constructor_call::LookupRegistrationServices for Scope<S> {
    type ConstructorLookupRegistrationAllocationOwner = S::FormatConstructorLookupRegistrationAllocationOwner;
    fn constructor_lookup_registration_allocate(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, request: l::registration::AllocationRequest) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_registration_allocate(resources(o, None, None, None), ctor, lookup, allocation, request)
    }
    fn constructor_lookup_registration_read_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, request: l::registration::ReadRequest) -> Result<u64, Self::Error> {
        self.services.format_constructor_lookup_registration_read_word(resources(o, None, None, None), ctor, lookup, allocation, request)
    }
    fn constructor_lookup_registration_store_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, request: l::registration::StoreRequest) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_registration_store_word(resources(o, None, None, None), ctor, lookup, allocation, request)
    }
    fn constructor_lookup_registration_push(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_registration_push(resources(o, None, None, None), ctor, lookup, allocation, returned)
    }
    fn constructor_lookup_registration_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_registration_sub(resources(o, None, None, None), ctor, lookup, allocation, returned)
    }
    fn constructor_lookup_registration_add(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_registration_add(resources(o, None, None, None), ctor, lookup, allocation, returned)
    }
    fn constructor_lookup_registration_pop(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_registration_pop(resources(o, None, None, None), ctor, lookup, allocation, returned)
    }
    fn constructor_lookup_registration_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_registration_ret(resources(o, None, None, None), ctor, lookup, allocation, returned)
    }
    fn adopt_completed_constructor_lookup_registration(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, history: l::registration_call::History, held: &mut Option<l::registration_call::Retained<Self::ConstructorLookupRegistrationAllocationOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_format_constructor_lookup_registration(resources(o, None, None, None), ctor, lookup, history, held)
    }
}
pub trait LookupFallbackServices: LookupServices {
    fn format_constructor_lookup_fallback_load(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, request: l::fallback::LoadRequest,
        returned: &mut Option<l::fallback::LoadReturned>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_fallback_return(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, request: l::fallback::ReturnRequest,
        returned: &mut Option<l::fallback::ActualReturn>) -> Result<(), Self::Error>;
}
impl<S: LookupFallbackServices> constructor_call::LookupFallbackServices for Scope<S> {
    fn constructor_lookup_fallback_load(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, request: l::fallback::LoadRequest, returned: &mut Option<l::fallback::LoadReturned>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_fallback_load(resources(o, None, None, None), ctor, lookup, request, returned)
    }
    fn constructor_lookup_fallback_return(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, request: l::fallback::ReturnRequest, returned: &mut Option<l::fallback::ActualReturn>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_fallback_return(resources(o, None, None, None), ctor, lookup, request, returned)
    }
}
pub trait LookupServices: ConstructorServices {
    type FormatLookupCallOwner;
    fn format_constructor_lookup_read_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn format_constructor_lookup_read_dword(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn format_constructor_lookup_read_byte(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn format_constructor_lookup_write_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn format_constructor_lookup_write_dword(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn format_constructor_lookup_call(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, request: l::CallRequest) -> Result<(), Self::Error>;
    fn format_constructor_lookup_mov_rax(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, selected: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_add_rsp(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, returned: &mut Option<l::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_pop(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_return::Pop>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_ret(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, returned: &mut Option<l::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_push(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_entry::Push>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_sub(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, returned: &mut Option<l::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_rbp(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn adopt_completed_format_constructor_lookup(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>,
        history: object_lookup_call::History, held: &mut Option<object_lookup_call::Retained<Self::FormatLookupCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupServices> constructor_call::LookupServices for Scope<S> {
    type ConstructorLookupCallOwner = S::FormatLookupCallOwner;
    fn constructor_lookup_read_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.format_constructor_lookup_read_word(resources(o, None, None, None), ctor, lookup, pc, address) }
    fn constructor_lookup_read_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.format_constructor_lookup_read_dword(resources(o, None, None, None), ctor, lookup, pc, address) }
    fn constructor_lookup_read_byte(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> { self.services.format_constructor_lookup_read_byte(resources(o, None, None, None), ctor, lookup, pc, address) }
    fn constructor_lookup_write_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.format_constructor_lookup_write_word(resources(o, None, None, None), ctor, lookup, pc, address, word) }
    fn constructor_lookup_write_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.format_constructor_lookup_write_dword(resources(o, None, None, None), ctor, lookup, pc, address, word) }
    fn constructor_lookup_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, request: l::CallRequest) -> Result<(), Self::Error> { self.services.format_constructor_lookup_call(resources(o, None, None, None), ctor, lookup, request) }
    fn constructor_lookup_mov_rax(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, selected: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.format_constructor_lookup_mov_rax(resources(o, None, None, None), ctor, lookup, selected, returned) }
    fn constructor_lookup_add_rsp(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, returned: &mut Option<l::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.format_constructor_lookup_add_rsp(resources(o, None, None, None), ctor, lookup, returned) }
    fn constructor_lookup_pop(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_return::Pop>) -> Result<(), Self::Error> { self.services.format_constructor_lookup_pop(resources(o, None, None, None), ctor, lookup, pc, register, returned) }
    fn constructor_lookup_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, returned: &mut Option<l::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.format_constructor_lookup_ret(resources(o, None, None, None), ctor, lookup, returned) }
    fn constructor_lookup_push(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_entry::Push>) -> Result<(), Self::Error> { self.services.format_constructor_lookup_push(resources(o, None, None, None), ctor, lookup, pc, register, returned) }
    fn constructor_lookup_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, returned: &mut Option<l::native_entry::Subtract>) -> Result<(), Self::Error> { self.services.format_constructor_lookup_sub(resources(o, None, None, None), ctor, lookup, returned) }
    fn constructor_lookup_rbp(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.format_constructor_lookup_rbp(resources(o, None, None, None), ctor, lookup, returned) }
    fn adopt_completed_constructor_lookup(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>,
        history: object_lookup_call::History, held: &mut Option<object_lookup_call::Retained<Self::ConstructorLookupCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_format_constructor_lookup(resources(o, None, None, None), ctor, history, held)
    }
}

