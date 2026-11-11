use super::{l, ParentState, Retained, History};
use super::super as c;

pub trait RegistrationServices: Services {
    type LookupRegistrationAllocationOwner;
    fn lookup_registration_allocate(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, request: l::registration::AllocationRequest) -> Result<(), Self::Error>;
    fn lookup_registration_read_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, request: l::registration::ReadRequest) -> Result<u64, Self::Error>;
    fn lookup_registration_store_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, request: l::registration::StoreRequest) -> Result<(), Self::Error>;
    fn lookup_registration_push(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Push>) -> Result<(), Self::Error>;
    fn lookup_registration_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn lookup_registration_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_registration_pop(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::Pop>) -> Result<(), Self::Error>;
    fn lookup_registration_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, allocation: &mut Option<l::registration::AllocationReturned<Self::LookupRegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn adopt_completed_lookup_registration(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, history: l::registration_call::History, held: &mut Option<l::registration_call::Retained<Self::LookupRegistrationAllocationOwner>>) -> Result<(), Self::Error>;
}
impl<S: RegistrationServices> l::registration_call::Services for Scope<S> {
    type RegistrationAllocationOwner = S::LookupRegistrationAllocationOwner;
    fn registration_allocate(&mut self, o: &mut l::Owners<Self>, allocation: &mut Option<l::registration::AllocationReturned<Self::RegistrationAllocationOwner>>, request: l::registration::AllocationRequest) -> Result<(), Self::Error> {
        self.services.lookup_registration_allocate(&mut o.storage.owners, &mut o.calls, allocation, request)
    }
    fn registration_read_word(&mut self, o: &mut l::Owners<Self>, allocation: &mut Option<l::registration::AllocationReturned<Self::RegistrationAllocationOwner>>, request: l::registration::ReadRequest) -> Result<u64, Self::Error> {
        self.services.lookup_registration_read_word(&mut o.storage.owners, &mut o.calls, allocation, request)
    }
    fn registration_store_word(&mut self, o: &mut l::Owners<Self>, allocation: &mut Option<l::registration::AllocationReturned<Self::RegistrationAllocationOwner>>, request: l::registration::StoreRequest) -> Result<(), Self::Error> {
        self.services.lookup_registration_store_word(&mut o.storage.owners, &mut o.calls, allocation, request)
    }
    fn registration_push(&mut self, o: &mut l::Owners<Self>, allocation: &mut Option<l::registration::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.lookup_registration_push(&mut o.storage.owners, &mut o.calls, allocation, returned)
    }
    fn registration_sub(&mut self, o: &mut l::Owners<Self>, allocation: &mut Option<l::registration::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.lookup_registration_sub(&mut o.storage.owners, &mut o.calls, allocation, returned)
    }
    fn registration_add(&mut self, o: &mut l::Owners<Self>, allocation: &mut Option<l::registration::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_registration_add(&mut o.storage.owners, &mut o.calls, allocation, returned)
    }
    fn registration_pop(&mut self, o: &mut l::Owners<Self>, allocation: &mut Option<l::registration::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.lookup_registration_pop(&mut o.storage.owners, &mut o.calls, allocation, returned)
    }
    fn registration_ret(&mut self, o: &mut l::Owners<Self>, allocation: &mut Option<l::registration::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<l::registration::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.lookup_registration_ret(&mut o.storage.owners, &mut o.calls, allocation, returned)
    }
    fn adopt_completed_registration(&mut self, o: &mut l::Owners<Self>, history: l::registration_call::History, held: &mut Option<l::registration_call::Retained<Self::RegistrationAllocationOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_lookup_registration(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
pub trait FallbackServices: Services {
    fn lookup_fallback_load(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, request: l::fallback::LoadRequest,
        returned: &mut Option<l::fallback::LoadReturned>) -> Result<(), Self::Error>;
    fn lookup_fallback_return(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, request: l::fallback::ReturnRequest,
        returned: &mut Option<l::fallback::ActualReturn>) -> Result<(), Self::Error>;
}
impl<S: FallbackServices> l::fallback_call::Services for Scope<S> {
    fn fallback_load_current(&mut self, o: &mut l::Owners<Self>, request: l::fallback::LoadRequest, returned: &mut Option<l::fallback::LoadReturned>) -> Result<(), Self::Error> {
        self.services.lookup_fallback_load(&mut o.storage.owners, &mut o.calls, request, returned)
    }
    fn fallback_actual_return(&mut self, o: &mut l::Owners<Self>, request: l::fallback::ReturnRequest, returned: &mut Option<l::fallback::ActualReturn>) -> Result<(), Self::Error> {
        self.services.lookup_fallback_return(&mut o.storage.owners, &mut o.calls, request, returned)
    }
}
pub trait Services: c::Services {
    type LookupCallOwner;

    fn lookup_read_word(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn lookup_read_dword(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn lookup_read_byte(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn lookup_write_word(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn lookup_write_dword(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn lookup_call(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, request: l::CallRequest) -> Result<(), Self::Error>;
    fn lookup_mov_rax(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, selected: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn lookup_add_rsp(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, returned: &mut Option<l::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_pop(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_return::Pop>) -> Result<(), Self::Error>;
    fn lookup_ret(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, returned: &mut Option<l::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn lookup_push(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_entry::Push>) -> Result<(), Self::Error>;
    fn lookup_sub(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, returned: &mut Option<l::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn lookup_rbp(&mut self, o: &mut c::Owners<Self>, calls: &mut l::Calls<Self::LookupCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn adopt_completed_lookup(&mut self, o: &mut c::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::LookupCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> l::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::LookupCallOwner; type Error = S::Error;
    fn read_word(&mut self, o: &mut l::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.lookup_read_word(&mut o.storage.owners, &mut o.calls, pc, address) }
    fn read_dword(&mut self, o: &mut l::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.lookup_read_dword(&mut o.storage.owners, &mut o.calls, pc, address) }
    fn read_byte(&mut self, o: &mut l::Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error> { self.services.lookup_read_byte(&mut o.storage.owners, &mut o.calls, pc, address) }
    fn write_word(&mut self, o: &mut l::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.lookup_write_word(&mut o.storage.owners, &mut o.calls, pc, address, word) }
    fn write_dword(&mut self, o: &mut l::Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.lookup_write_dword(&mut o.storage.owners, &mut o.calls, pc, address, word) }
    fn call(&mut self, o: &mut l::Owners<Self>, request: l::CallRequest) -> Result<(), Self::Error> { self.services.lookup_call(&mut o.storage.owners, &mut o.calls, request) }
}
impl<S: Services> l::native_return::Services for Scope<S> {
    fn mov_rax_r14(&mut self, o: &mut l::Owners<Self>, selected: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.lookup_mov_rax(&mut o.storage.owners, &mut o.calls, selected, returned) }
    fn add_rsp_a0(&mut self, o: &mut l::Owners<Self>, returned: &mut Option<l::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.lookup_add_rsp(&mut o.storage.owners, &mut o.calls, returned) }
    fn pop_saved(&mut self, o: &mut l::Owners<Self>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_return::Pop>) -> Result<(), Self::Error> { self.services.lookup_pop(&mut o.storage.owners, &mut o.calls, pc, register, returned) }
    fn ret_18c426(&mut self, o: &mut l::Owners<Self>, returned: &mut Option<l::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.lookup_ret(&mut o.storage.owners, &mut o.calls, returned) }
}
impl<S: Services> l::native_entry::Services for Scope<S> {
    fn lookup_push(&mut self, o: &mut l::Owners<Self>, pc: u32, register: l::native_return::Register, returned: &mut Option<l::native_entry::Push>) -> Result<(), Self::Error> { self.services.lookup_push(&mut o.storage.owners, &mut o.calls, pc, register, returned) }
    fn lookup_sub_a0(&mut self, o: &mut l::Owners<Self>, returned: &mut Option<l::native_entry::Subtract>) -> Result<(), Self::Error> { self.services.lookup_sub(&mut o.storage.owners, &mut o.calls, returned) }
    fn lookup_lea_rbp80(&mut self, o: &mut l::Owners<Self>, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.lookup_rbp(&mut o.storage.owners, &mut o.calls, returned) }
}

