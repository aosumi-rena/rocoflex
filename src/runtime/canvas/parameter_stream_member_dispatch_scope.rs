use super::{d, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type DispatchCallOwner;

    fn dispatch_and_qword_zero(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, out: &mut Option<d::AndQword>) -> Result<(), Self::Error>;
    fn dispatch_and_dword_fd(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, out: &mut Option<d::AndDword>) -> Result<(), Self::Error>;
    fn dispatch_read_byte(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn dispatch_read_dword(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn dispatch_read_qword(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn dispatch_write_byte(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error>;
    fn dispatch_write_dword(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error>;
    fn dispatch_write_qword(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn dispatch_load_movups(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error>;
    fn dispatch_store_movdqu(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error>;
    fn dispatch_call(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, request: d::CallRequest) -> Result<(), Self::Error>;
    fn dispatch_save_rbx_57a384(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error>;
    fn dispatch_push_rbp_57a389(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error>;
    fn dispatch_mov_rbp_rsp_57a38a(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_entry::RbpAssigned>) -> Result<(), Self::Error>;
    fn dispatch_sub_rsp_70_57a38d(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn dispatch_restore_rbx_57a411(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_return::Load>) -> Result<(), Self::Error>;
    fn dispatch_add_rsp_70_57a419(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn dispatch_pop_rbp_57a41d(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error>;
    fn dispatch_ret_57a41e(&mut self, parent: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_dispatch(&mut self, parent: &mut super::super::Owners<Self>, history: History, held: &mut Option<Retained<Self::DispatchCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> d::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::DispatchCallOwner; type Error = S::Error;
    fn and_qword_zero(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64, out: &mut Option<d::AndQword>) -> Result<(), Self::Error> {
        self.services.dispatch_and_qword_zero(&mut o.storage.owners, &mut o.calls, pc, address, out)
    }
    fn and_dword_fd(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64, out: &mut Option<d::AndDword>) -> Result<(), Self::Error> {
        self.services.dispatch_and_dword_fd(&mut o.storage.owners, &mut o.calls, pc, address, out)
    }
    fn read_byte(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.dispatch_read_byte(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn read_dword(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.dispatch_read_dword(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn read_qword(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.dispatch_read_qword(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn write_byte(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error> {
        self.services.dispatch_write_byte(&mut o.storage.owners, &mut o.calls, pc, address, value)
    }
    fn write_dword(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error> {
        self.services.dispatch_write_dword(&mut o.storage.owners, &mut o.calls, pc, address, value)
    }
    fn write_qword(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error> {
        self.services.dispatch_write_qword(&mut o.storage.owners, &mut o.calls, pc, address, value)
    }
    fn load_movups(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error> {
        self.services.dispatch_load_movups(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn store_movdqu(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error> {
        self.services.dispatch_store_movdqu(&mut o.storage.owners, &mut o.calls, pc, address, value)
    }
    fn call(&mut self, o: &mut d::Owners<Self>, request: d::CallRequest) -> Result<(), Self::Error> {
        self.services.dispatch_call(&mut o.storage.owners, &mut o.calls, request)
    }
}
impl<S: Services> d::native_entry::Services for Scope<S> {
    fn save_rbx_57a384(&mut self, o: &mut d::Owners<Self>, out: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.dispatch_save_rbx_57a384(&mut o.storage.owners, &mut o.calls, out)
    }
    fn push_rbp_57a389(&mut self, o: &mut d::Owners<Self>, out: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.dispatch_push_rbp_57a389(&mut o.storage.owners, &mut o.calls, out)
    }
    fn mov_rbp_rsp_57a38a(&mut self, o: &mut d::Owners<Self>, out: &mut Option<d::native_entry::RbpAssigned>) -> Result<(), Self::Error> {
        self.services.dispatch_mov_rbp_rsp_57a38a(&mut o.storage.owners, &mut o.calls, out)
    }
    fn sub_rsp_70_57a38d(&mut self, o: &mut d::Owners<Self>, out: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.dispatch_sub_rsp_70_57a38d(&mut o.storage.owners, &mut o.calls, out)
    }
}
impl<S: Services> d::native_return::Services for Scope<S> {
    fn restore_rbx_57a411(&mut self, o: &mut d::Owners<Self>, out: &mut Option<d::native_return::Load>) -> Result<(), Self::Error> {
        self.services.dispatch_restore_rbx_57a411(&mut o.storage.owners, &mut o.calls, out)
    }
    fn add_rsp_70_57a419(&mut self, o: &mut d::Owners<Self>, out: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.dispatch_add_rsp_70_57a419(&mut o.storage.owners, &mut o.calls, out)
    }
    fn pop_rbp_57a41d(&mut self, o: &mut d::Owners<Self>, out: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.dispatch_pop_rbp_57a41d(&mut o.storage.owners, &mut o.calls, out)
    }
    fn ret_57a41e(&mut self, o: &mut d::Owners<Self>, out: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.dispatch_ret_57a41e(&mut o.storage.owners, &mut o.calls, out)
    }
}
