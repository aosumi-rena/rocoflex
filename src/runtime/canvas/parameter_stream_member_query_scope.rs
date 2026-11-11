use super::{q, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type QueryCallOwner;

    fn query_call(&mut self, o: &mut super::super::Owners<Self>, calls: &mut q::Calls<Self::QueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error>;
    fn query_read_word(&mut self, o: &mut super::super::Owners<Self>, calls: &mut q::Calls<Self::QueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn query_read_dword(&mut self, o: &mut super::super::Owners<Self>, calls: &mut q::Calls<Self::QueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn query_write_word(&mut self, o: &mut super::super::Owners<Self>, calls: &mut q::Calls<Self::QueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn query_sub(&mut self, o: &mut super::super::Owners<Self>, calls: &mut q::Calls<Self::QueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn query_add(&mut self, o: &mut super::super::Owners<Self>, calls: &mut q::Calls<Self::QueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn query_ret(&mut self, o: &mut super::super::Owners<Self>, calls: &mut q::Calls<Self::QueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_query(&mut self, o: &mut super::super::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::QueryCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> q::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::QueryCallOwner; type Error = S::Error;
    fn call(&mut self, o: &mut q::Owners<Self>, request: q::CallRequest) -> Result<(), Self::Error> {
        self.services.query_call(&mut o.storage.owners, &mut o.calls, request)
    }
    fn read_word(&mut self, o: &mut q::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.query_read_word(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn read_dword(&mut self, o: &mut q::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.query_read_dword(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn write_word(&mut self, o: &mut q::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.query_write_word(&mut o.storage.owners, &mut o.calls, pc, address, word)
    }
}
impl<S: Services> q::native_entry::Services for Scope<S> {
    fn sub_rsp_28(&mut self, o: &mut q::Owners<Self>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.query_sub(&mut o.storage.owners, &mut o.calls, pc, returned)
    }
}
impl<S: Services> q::native_return::Services for Scope<S> {
    fn add_rsp_28(&mut self, o: &mut q::Owners<Self>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.query_add(&mut o.storage.owners, &mut o.calls, pc, returned)
    }
    fn ret(&mut self, o: &mut q::Owners<Self>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.query_ret(&mut o.storage.owners, &mut o.calls, pc, returned)
    }
}
