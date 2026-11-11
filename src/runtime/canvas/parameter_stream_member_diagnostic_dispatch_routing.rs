use super::{Scope, Services};
use super::super as p;
use p::diagnostic as a;
use a::{dispatch as d, dispatch_call as dc};
pub trait DispatchServices: Services {
    type DiagnosticDispatchCallOwner;

    fn diagnostic_dispatch_and_qword_zero(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<d::AndQword>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_and_dword_fd(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<d::AndDword>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_read_byte(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn diagnostic_dispatch_read_dword(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn diagnostic_dispatch_read_qword(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn diagnostic_dispatch_write_byte(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_write_dword(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_write_qword(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_load_movups(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error>;
    fn diagnostic_dispatch_store_movdqu(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_call(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, request: d::CallRequest) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_save_rbx_57a384(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_push_rbp_57a389(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_mov_rbp_rsp_57a38a(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<d::native_entry::RbpAssigned>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_sub_rsp_70_57a38d(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_restore_rbx_57a411(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<d::native_return::Load>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_add_rsp_70_57a419(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_pop_rbp_57a41d(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch_ret_57a41e(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_diagnostic_dispatch(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, history: dc::History, held: &mut Option<dc::Retained<Self::DiagnosticDispatchCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: DispatchServices> dc::Services for Scope<S> {
    type DispatchCallOwner = S::DiagnosticDispatchCallOwner;
    fn dispatch_and_qword_zero(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, out: &mut Option<d::AndQword>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_and_qword_zero(&mut o.storage.owners, &mut o.call, calls, pc, address, out)
    }
    fn dispatch_and_dword_fd(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, out: &mut Option<d::AndDword>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_and_dword_fd(&mut o.storage.owners, &mut o.call, calls, pc, address, out)
    }
    fn dispatch_read_byte(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.diagnostic_dispatch_read_byte(&mut o.storage.owners, &mut o.call, calls, pc, address)
    }
    fn dispatch_read_dword(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.diagnostic_dispatch_read_dword(&mut o.storage.owners, &mut o.call, calls, pc, address)
    }
    fn dispatch_read_qword(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.diagnostic_dispatch_read_qword(&mut o.storage.owners, &mut o.call, calls, pc, address)
    }
    fn dispatch_write_byte(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_write_byte(&mut o.storage.owners, &mut o.call, calls, pc, address, value)
    }
    fn dispatch_write_dword(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_write_dword(&mut o.storage.owners, &mut o.call, calls, pc, address, value)
    }
    fn dispatch_write_qword(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_write_qword(&mut o.storage.owners, &mut o.call, calls, pc, address, value)
    }
    fn dispatch_load_movups(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error> {
        self.services.diagnostic_dispatch_load_movups(&mut o.storage.owners, &mut o.call, calls, pc, address)
    }
    fn dispatch_store_movdqu(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_store_movdqu(&mut o.storage.owners, &mut o.call, calls, pc, address, value)
    }
    fn dispatch_call(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, request: d::CallRequest) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_call(&mut o.storage.owners, &mut o.call, calls, request)
    }
    fn dispatch_save_rbx_57a384(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_save_rbx_57a384(&mut o.storage.owners, &mut o.call, calls, out)
    }
    fn dispatch_push_rbp_57a389(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_push_rbp_57a389(&mut o.storage.owners, &mut o.call, calls, out)
    }
    fn dispatch_mov_rbp_rsp_57a38a(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_entry::RbpAssigned>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_mov_rbp_rsp_57a38a(&mut o.storage.owners, &mut o.call, calls, out)
    }
    fn dispatch_sub_rsp_70_57a38d(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_sub_rsp_70_57a38d(&mut o.storage.owners, &mut o.call, calls, out)
    }
    fn dispatch_restore_rbx_57a411(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_return::Load>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_restore_rbx_57a411(&mut o.storage.owners, &mut o.call, calls, out)
    }
    fn dispatch_add_rsp_70_57a419(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_add_rsp_70_57a419(&mut o.storage.owners, &mut o.call, calls, out)
    }
    fn dispatch_pop_rbp_57a41d(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_pop_rbp_57a41d(&mut o.storage.owners, &mut o.call, calls, out)
    }
    fn dispatch_ret_57a41e(&mut self, o: &mut a::Owners<Self>, calls: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch_ret_57a41e(&mut o.storage.owners, &mut o.call, calls, out)
    }
    fn adopt_completed_dispatch(&mut self, o: &mut a::Owners<Self>, history: dc::History, held: &mut Option<dc::Retained<Self::DispatchCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_diagnostic_dispatch(&mut o.storage.owners, &mut o.call, history, held)
    }
}
