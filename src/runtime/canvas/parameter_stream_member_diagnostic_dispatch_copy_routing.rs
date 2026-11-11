use super::{Scope, DiagnosticServices};
use super::super as parent;
use parent::copy;
use copy::{diagnostic, diagnostic_call};
use diagnostic::{dispatch, dispatch_call};

pub trait DiagnosticDispatchServices: DiagnosticServices {
    type CopyDiagnosticDispatchCallOwner;

    fn copy_diagnostic_dispatch_and_qword_zero(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<dispatch::AndQword>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_and_dword_fd(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<dispatch::AndDword>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_read_byte(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn copy_diagnostic_dispatch_read_dword(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn copy_diagnostic_dispatch_read_qword(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn copy_diagnostic_dispatch_write_byte(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_write_dword(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_write_qword(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_load_movups(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error>;
    fn copy_diagnostic_dispatch_store_movdqu(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_call(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, request: dispatch::CallRequest) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_save_rbx_57a384(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_entry::Store>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_push_rbp_57a389(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_entry::Push>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_mov_rbp_rsp_57a38a(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_entry::RbpAssigned>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_sub_rsp_70_57a38d(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_restore_rbx_57a411(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_return::Load>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_add_rsp_70_57a419(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_pop_rbp_57a41d(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_return::Pop>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch_ret_57a41e(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_copy_diagnostic_dispatch(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, history: dispatch_call::History, held: &mut Option<dispatch_call::Retained<Self::CopyDiagnosticDispatchCallOwner>>) -> Result<(), Self::Error>;
}

impl<S: DiagnosticDispatchServices> diagnostic_call::DispatchServices for Scope<S> {
    type DiagnosticDispatchCallOwner = S::CopyDiagnosticDispatchCallOwner;
    fn diagnostic_dispatch_and_qword_zero(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<dispatch::AndQword>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_and_qword_zero(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address, out)
    }
    fn diagnostic_dispatch_and_dword_fd(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<dispatch::AndDword>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_and_dword_fd(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address, out)
    }
    fn diagnostic_dispatch_read_byte(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.copy_diagnostic_dispatch_read_byte(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address)
    }
    fn diagnostic_dispatch_read_dword(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.copy_diagnostic_dispatch_read_dword(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address)
    }
    fn diagnostic_dispatch_read_qword(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.copy_diagnostic_dispatch_read_qword(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address)
    }
    fn diagnostic_dispatch_write_byte(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_write_byte(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address, value)
    }
    fn diagnostic_dispatch_write_dword(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_write_dword(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address, value)
    }
    fn diagnostic_dispatch_write_qword(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_write_qword(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address, value)
    }
    fn diagnostic_dispatch_load_movups(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error> {
        self.services.copy_diagnostic_dispatch_load_movups(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address)
    }
    fn diagnostic_dispatch_store_movdqu(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_store_movdqu(&mut o.storage.owners, &mut o.calls, diagnostic, calls, pc, address, value)
    }
    fn diagnostic_dispatch_call(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, request: dispatch::CallRequest) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_call(&mut o.storage.owners, &mut o.calls, diagnostic, calls, request)
    }
    fn diagnostic_dispatch_save_rbx_57a384(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_save_rbx_57a384(&mut o.storage.owners, &mut o.calls, diagnostic, calls, out)
    }
    fn diagnostic_dispatch_push_rbp_57a389(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_push_rbp_57a389(&mut o.storage.owners, &mut o.calls, diagnostic, calls, out)
    }
    fn diagnostic_dispatch_mov_rbp_rsp_57a38a(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_entry::RbpAssigned>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_mov_rbp_rsp_57a38a(&mut o.storage.owners, &mut o.calls, diagnostic, calls, out)
    }
    fn diagnostic_dispatch_sub_rsp_70_57a38d(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_sub_rsp_70_57a38d(&mut o.storage.owners, &mut o.calls, diagnostic, calls, out)
    }
    fn diagnostic_dispatch_restore_rbx_57a411(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_return::Load>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_restore_rbx_57a411(&mut o.storage.owners, &mut o.calls, diagnostic, calls, out)
    }
    fn diagnostic_dispatch_add_rsp_70_57a419(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_add_rsp_70_57a419(&mut o.storage.owners, &mut o.calls, diagnostic, calls, out)
    }
    fn diagnostic_dispatch_pop_rbp_57a41d(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_pop_rbp_57a41d(&mut o.storage.owners, &mut o.calls, diagnostic, calls, out)
    }
    fn diagnostic_dispatch_ret_57a41e(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, calls: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<dispatch::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch_ret_57a41e(&mut o.storage.owners, &mut o.calls, diagnostic, calls, out)
    }
    fn adopt_completed_diagnostic_dispatch(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, history: dispatch_call::History, held: &mut Option<dispatch_call::Retained<Self::DiagnosticDispatchCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_copy_diagnostic_dispatch(&mut o.storage.owners, &mut o.calls, diagnostic, history, held)
    }
}
