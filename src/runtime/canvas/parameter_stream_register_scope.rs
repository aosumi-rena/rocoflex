use super::{r, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type RegistrationAllocationOwner;

    fn registration_allocate(&mut self, o: &mut super::super::Owners<Self>, allocation: &mut Option<r::AllocationReturned<Self::RegistrationAllocationOwner>>, request: r::AllocationRequest) -> Result<(), Self::Error>;
    fn registration_read_word(&mut self, o: &mut super::super::Owners<Self>, allocation: &mut Option<r::AllocationReturned<Self::RegistrationAllocationOwner>>, request: r::ReadRequest) -> Result<u64, Self::Error>;
    fn registration_store_word(&mut self, o: &mut super::super::Owners<Self>, allocation: &mut Option<r::AllocationReturned<Self::RegistrationAllocationOwner>>, request: r::StoreRequest) -> Result<(), Self::Error>;
    fn registration_push(&mut self, o: &mut super::super::Owners<Self>, allocation: &mut Option<r::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<r::native_entry::Push>) -> Result<(), Self::Error>;
    fn registration_sub(&mut self, o: &mut super::super::Owners<Self>, allocation: &mut Option<r::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<r::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn registration_add(&mut self, o: &mut super::super::Owners<Self>, allocation: &mut Option<r::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<r::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn registration_pop(&mut self, o: &mut super::super::Owners<Self>, allocation: &mut Option<r::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<r::native_return::Pop>) -> Result<(), Self::Error>;
    fn registration_ret(&mut self, o: &mut super::super::Owners<Self>, allocation: &mut Option<r::AllocationReturned<Self::RegistrationAllocationOwner>>, returned: &mut Option<r::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_registration(&mut self, o: &mut super::super::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::RegistrationAllocationOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> r::Services for Scope<S> {
    type Storage = ParentState<S>; type AllocationOwner = S::RegistrationAllocationOwner; type Error = S::Error;
    fn allocate(&mut self, o: &mut r::Owners<Self>, request: r::AllocationRequest) -> Result<(), Self::Error> {
        self.services.registration_allocate(&mut o.storage.owners, &mut o.allocation, request)
    }
    fn read_word(&mut self, o: &mut r::Owners<Self>, request: r::ReadRequest) -> Result<u64, Self::Error> {
        self.services.registration_read_word(&mut o.storage.owners, &mut o.allocation, request)
    }
    fn store_word(&mut self, o: &mut r::Owners<Self>, request: r::StoreRequest) -> Result<(), Self::Error> {
        self.services.registration_store_word(&mut o.storage.owners, &mut o.allocation, request)
    }
}
impl<S: Services> r::native_entry::Services for Scope<S> {
    fn push_rbx_55e3d4(&mut self, o: &mut r::Owners<Self>, returned: &mut Option<r::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.registration_push(&mut o.storage.owners, &mut o.allocation, returned)
    }
    fn sub_rsp_20_55e3d6(&mut self, o: &mut r::Owners<Self>, returned: &mut Option<r::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.registration_sub(&mut o.storage.owners, &mut o.allocation, returned)
    }
}
impl<S: Services> r::native_return::Services for Scope<S> {
    fn add_rsp_20(&mut self, o: &mut r::Owners<Self>, returned: &mut Option<r::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.registration_add(&mut o.storage.owners, &mut o.allocation, returned)
    }
    fn pop_rbx(&mut self, o: &mut r::Owners<Self>, returned: &mut Option<r::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.registration_pop(&mut o.storage.owners, &mut o.allocation, returned)
    }
    fn ret_55e40b(&mut self, o: &mut r::Owners<Self>, returned: &mut Option<r::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.registration_ret(&mut o.storage.owners, &mut o.allocation, returned)
    }
}
