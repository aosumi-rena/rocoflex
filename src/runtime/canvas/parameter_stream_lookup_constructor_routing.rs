use super::{Scope, Services};
use super::super::{self as f, constructor as c};
use c::{object_lookup as l, object_lookup_call};

pub trait LookupRegistrationServices: LookupServices {
    type ConstructorLookupRegistrationAllocationOwner;
    fn constructor_lookup_registration_allocate(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, request: l::registration::AllocationRequest) -> Result<(), Self::Error>;
    fn constructor_lookup_registration_read_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, request: l::registration::ReadRequest) -> Result<u64, Self::Error>;
    fn constructor_lookup_registration_store_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, request: l::registration::StoreRequest) -> Result<(), Self::Error>;
    fn constructor_lookup_registration_push(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Push>) -> Result<(), Self::Error>;
    fn constructor_lookup_registration_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn constructor_lookup_registration_add(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn constructor_lookup_registration_pop(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::Pop>) -> Result<(), Self::Error>;
    fn constructor_lookup_registration_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::ConstructorLookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn adopt_completed_constructor_lookup_registration(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, history: l::registration_call::History, held: &mut Option<l::registration_call::Retained<Self::ConstructorLookupRegistrationAllocationOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupRegistrationServices> object_lookup_call::RegistrationServices for Scope<S> {
    type LookupRegistrationAllocationOwner = S::ConstructorLookupRegistrationAllocationOwner;
    fn lookup_registration_allocate(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, request: l::registration::AllocationRequest) -> Result<(), Self::Error> {
        self.services.constructor_lookup_registration_allocate(&mut o.storage.owners, &mut o.calls, lookup, allocation, request)
    }
    fn lookup_registration_read_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, request: l::registration::ReadRequest) -> Result<u64, Self::Error> {
        self.services.constructor_lookup_registration_read_word(&mut o.storage.owners, &mut o.calls, lookup, allocation, request)
    }
    fn lookup_registration_store_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, request: l::registration::StoreRequest) -> Result<(), Self::Error> {
        self.services.constructor_lookup_registration_store_word(&mut o.storage.owners, &mut o.calls, lookup, allocation, request)
    }
    fn lookup_registration_push(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_registration_push(&mut o.storage.owners, &mut o.calls, lookup, allocation, returned)
    }
    fn lookup_registration_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_registration_sub(&mut o.storage.owners, &mut o.calls, lookup, allocation, returned)
    }
    fn lookup_registration_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_registration_add(&mut o.storage.owners, &mut o.calls, lookup, allocation, returned)
    }
    fn lookup_registration_pop(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_registration_pop(&mut o.storage.owners, &mut o.calls, lookup, allocation, returned)
    }
    fn lookup_registration_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_registration_ret(&mut o.storage.owners, &mut o.calls, lookup, allocation, returned)
    }
    fn adopt_completed_lookup_registration(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, history: l::registration_call::History, held: &mut Option<l::registration_call::Retained<Self::LookupRegistrationAllocationOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_constructor_lookup_registration(&mut o.storage.owners, &mut o.calls, lookup, history, held)
    }
}
pub trait LookupFallbackServices: LookupServices {
    fn constructor_lookup_fallback_load(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, request: l::fallback::LoadRequest,
        returned: &mut Option<l::fallback::LoadReturned>) -> Result<(), Self::Error>;
    fn constructor_lookup_fallback_return(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, request: l::fallback::ReturnRequest,
        returned: &mut Option<l::fallback::ActualReturn>) -> Result<(), Self::Error>;
}
impl<S: LookupFallbackServices> object_lookup_call::FallbackServices for Scope<S> {
    fn lookup_fallback_load(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, request: l::fallback::LoadRequest, returned: &mut Option<l::fallback::LoadReturned>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_fallback_load(&mut o.storage.owners, &mut o.calls, lookup, request, returned)
    }
    fn lookup_fallback_return(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, request: l::fallback::ReturnRequest, returned: &mut Option<l::fallback::ActualReturn>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_fallback_return(&mut o.storage.owners, &mut o.calls, lookup, request, returned)
    }
}
pub trait LookupServices: Services {
    type ConstructorLookupCallOwner;
    fn constructor_lookup_read_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn constructor_lookup_read_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn constructor_lookup_read_byte(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn constructor_lookup_write_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn constructor_lookup_write_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn constructor_lookup_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, request: l::CallRequest) -> Result<(), Self::Error>;
    fn constructor_lookup_mov_rax(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, selected: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn constructor_lookup_add_rsp(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, returned: &mut Option<l::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn constructor_lookup_pop(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_return::Pop>) -> Result<(), Self::Error>;
    fn constructor_lookup_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, returned: &mut Option<l::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn constructor_lookup_push(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_entry::Push>) -> Result<(), Self::Error>;
    fn constructor_lookup_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, returned: &mut Option<l::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn constructor_lookup_rbp(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn adopt_completed_constructor_lookup(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>,
        history: object_lookup_call::History, held: &mut Option<object_lookup_call::Retained<Self::ConstructorLookupCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupServices> object_lookup_call::Services for Scope<S> {
    type LookupCallOwner = S::ConstructorLookupCallOwner;
    fn lookup_read_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.constructor_lookup_read_word(&mut o.storage.owners, &mut o.calls, lookup, pc, address) }
    fn lookup_read_dword(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.constructor_lookup_read_dword(&mut o.storage.owners, &mut o.calls, lookup, pc, address) }
    fn lookup_read_byte(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> { self.services.constructor_lookup_read_byte(&mut o.storage.owners, &mut o.calls, lookup, pc, address) }
    fn lookup_write_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.constructor_lookup_write_word(&mut o.storage.owners, &mut o.calls, lookup, pc, address, word) }
    fn lookup_write_dword(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.constructor_lookup_write_dword(&mut o.storage.owners, &mut o.calls, lookup, pc, address, word) }
    fn lookup_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, request: l::CallRequest) -> Result<(), Self::Error> { self.services.constructor_lookup_call(&mut o.storage.owners, &mut o.calls, lookup, request) }
    fn lookup_mov_rax(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, selected: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.constructor_lookup_mov_rax(&mut o.storage.owners, &mut o.calls, lookup, selected, returned) }
    fn lookup_add_rsp(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, returned: &mut Option<l::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.constructor_lookup_add_rsp(&mut o.storage.owners, &mut o.calls, lookup, returned) }
    fn lookup_pop(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_return::Pop>) -> Result<(), Self::Error> { self.services.constructor_lookup_pop(&mut o.storage.owners, &mut o.calls, lookup, pc, register, returned) }
    fn lookup_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, returned: &mut Option<l::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.constructor_lookup_ret(&mut o.storage.owners, &mut o.calls, lookup, returned) }
    fn lookup_push(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_entry::Push>) -> Result<(), Self::Error> { self.services.constructor_lookup_push(&mut o.storage.owners, &mut o.calls, lookup, pc, register, returned) }
    fn lookup_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, returned: &mut Option<l::native_entry::Subtract>) -> Result<(), Self::Error> { self.services.constructor_lookup_sub(&mut o.storage.owners, &mut o.calls, lookup, returned) }
    fn lookup_rbp(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.constructor_lookup_rbp(&mut o.storage.owners, &mut o.calls, lookup, returned) }
    fn adopt_completed_lookup(&mut self, o: &mut c::Owners<Self>, history: object_lookup_call::History,
        held: &mut Option<object_lookup_call::Retained<Self::LookupCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_constructor_lookup(&mut o.storage.owners, &mut o.calls, history, held)
    }
}

