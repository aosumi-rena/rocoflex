use super::{c, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type CopyCallOwner;

    fn copy_call(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, request: c::CallRequest) -> Result<(), Self::Error>;
    fn copy_read_word16(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn copy_write_word16(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error>;
    fn copy_write_dword(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn copy_push(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, returned: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error>;
    fn copy_sub(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, returned: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn copy_add(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, returned: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn copy_pop(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, returned: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error>;
    fn copy_ret(&mut self, o: &mut super::super::Owners<Self>, calls: &mut c::Calls<Self::CopyCallOwner>, returned: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_copy(&mut self, o: &mut super::super::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::CopyCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> c::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::CopyCallOwner; type Error = S::Error;
    fn call(&mut self, o: &mut c::Owners<Self>, request: c::CallRequest) -> Result<(), Self::Error> {
        self.services.copy_call(&mut o.storage.owners, &mut o.calls, request)
    }
    fn read_word16(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64) -> Result<u16, Self::Error> {
        self.services.copy_read_word16(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn write_word16(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error> {
        self.services.copy_write_word16(&mut o.storage.owners, &mut o.calls, pc, address, word)
    }
    fn write_dword(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.copy_write_dword(&mut o.storage.owners, &mut o.calls, pc, address, word)
    }
}
impl<S: Services> c::native_entry::Services for Scope<S> {
    fn push_rbx_58e310(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.copy_push(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn sub_rsp_20_58e312(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.copy_sub(&mut o.storage.owners, &mut o.calls, returned)
    }
}
impl<S: Services> c::native_return::Services for Scope<S> {
    fn add_rsp_20_58e33d(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.copy_add(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn pop_rbx_58e341(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.copy_pop(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn ret_58e342(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.copy_ret(&mut o.storage.owners, &mut o.calls, returned)
    }
}
