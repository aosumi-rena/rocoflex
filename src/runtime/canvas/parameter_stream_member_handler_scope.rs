use super::{h, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type HandlerCallOwner;

    fn handler_read_qword(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn handler_write_qword(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn handler_call(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, request: h::CallRequest) -> Result<(), Self::Error>;
    fn handler_save_register(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, register: h::native_entry::Register, out: &mut Option<h::native_entry::Store>) -> Result<(), Self::Error>;
    fn handler_push_rdi_57a20f(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_entry::Push>) -> Result<(), Self::Error>;
    fn handler_sub_rsp_30_57a210(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn handler_restore_register(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, register: h::native_return::Register, out: &mut Option<h::native_return::Load>) -> Result<(), Self::Error>;
    fn handler_add_rsp_30_57a270(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn handler_pop_rdi_57a274(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_return::Pop>) -> Result<(), Self::Error>;
    fn handler_ret_57a275(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_handler(&mut self, parent: &mut super::super::Owners<Self>, history: History, held: &mut Option<Retained<Self::HandlerCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> h::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::HandlerCallOwner; type Error = S::Error;
    fn read_qword(&mut self, o: &mut h::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.handler_read_qword(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn write_qword(&mut self, o: &mut h::Owners<Self>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error> {
        self.services.handler_write_qword(&mut o.storage.owners, &mut o.calls, pc, address, value)
    }
    fn call(&mut self, o: &mut h::Owners<Self>, request: h::CallRequest) -> Result<(), Self::Error> {
        self.services.handler_call(&mut o.storage.owners, &mut o.calls, request)
    }
}
impl<S: Services> h::native_entry::Services for Scope<S> {
    fn save_register(&mut self, o: &mut h::Owners<Self>, pc: u32, register: h::native_entry::Register, out: &mut Option<h::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.handler_save_register(&mut o.storage.owners, &mut o.calls, pc, register, out)
    }
    fn push_rdi_57a20f(&mut self, o: &mut h::Owners<Self>, out: &mut Option<h::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.handler_push_rdi_57a20f(&mut o.storage.owners, &mut o.calls, out)
    }
    fn sub_rsp_30_57a210(&mut self, o: &mut h::Owners<Self>, out: &mut Option<h::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.handler_sub_rsp_30_57a210(&mut o.storage.owners, &mut o.calls, out)
    }
}
impl<S: Services> h::native_return::Services for Scope<S> {
    fn restore_register(&mut self, o: &mut h::Owners<Self>, pc: u32, register: h::native_return::Register, out: &mut Option<h::native_return::Load>) -> Result<(), Self::Error> {
        self.services.handler_restore_register(&mut o.storage.owners, &mut o.calls, pc, register, out)
    }
    fn add_rsp_30_57a270(&mut self, o: &mut h::Owners<Self>, out: &mut Option<h::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.handler_add_rsp_30_57a270(&mut o.storage.owners, &mut o.calls, out)
    }
    fn pop_rdi_57a274(&mut self, o: &mut h::Owners<Self>, out: &mut Option<h::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.handler_pop_rdi_57a274(&mut o.storage.owners, &mut o.calls, out)
    }
    fn ret_57a275(&mut self, o: &mut h::Owners<Self>, out: &mut Option<h::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.handler_ret_57a275(&mut o.storage.owners, &mut o.calls, out)
    }
}
