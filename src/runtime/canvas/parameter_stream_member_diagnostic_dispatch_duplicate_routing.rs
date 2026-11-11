use super::{Scope, CopyDiagnosticServices};
use super::super as p;
use p::duplicate as d;
use d::copy as w;
use w::diagnostic as a;
use a::{dispatch as x, dispatch_call as xc};

pub trait CopyDiagnosticDispatchServices: CopyDiagnosticServices {
    type DuplicateCopyDiagnosticDispatchCallOwner;

    fn duplicate_copy_diagnostic_dispatch_and_qword_zero(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<x::AndQword>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_and_dword_fd(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<x::AndDword>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_read_byte(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_read_dword(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_read_qword(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_write_byte(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_write_dword(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_write_qword(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_load_movups(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_store_movdqu(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_call(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, request: x::CallRequest) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_save_rbx_57a384(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_entry::Store>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_push_rbp_57a389(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_entry::Push>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_mov_rbp_rsp_57a38a(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_entry::RbpAssigned>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_sub_rsp_70_57a38d(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_restore_rbx_57a411(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_return::Load>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_add_rsp_70_57a419(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_pop_rbp_57a41d(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_return::Pop>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch_ret_57a41e(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate_copy_diagnostic_dispatch(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, history: xc::History, held: &mut Option<xc::Retained<Self::DuplicateCopyDiagnosticDispatchCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: CopyDiagnosticDispatchServices> d::copy_call::DiagnosticDispatchServices for Scope<S> {
    type CopyDiagnosticDispatchCallOwner = S::DuplicateCopyDiagnosticDispatchCallOwner;
    fn copy_diagnostic_dispatch_and_qword_zero(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<x::AndQword>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_and_qword_zero(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address, out)
    }
    fn copy_diagnostic_dispatch_and_dword_fd(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, out: &mut Option<x::AndDword>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_and_dword_fd(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address, out)
    }
    fn copy_diagnostic_dispatch_read_byte(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_read_byte(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address)
    }
    fn copy_diagnostic_dispatch_read_dword(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_read_dword(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address)
    }
    fn copy_diagnostic_dispatch_read_qword(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_read_qword(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address)
    }
    fn copy_diagnostic_dispatch_write_byte(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u8) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_write_byte(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address, value)
    }
    fn copy_diagnostic_dispatch_write_dword(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u32) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_write_dword(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address, value)
    }
    fn copy_diagnostic_dispatch_write_qword(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_write_qword(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address, value)
    }
    fn copy_diagnostic_dispatch_load_movups(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_load_movups(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address)
    }
    fn copy_diagnostic_dispatch_store_movdqu(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, pc: u32, address: u64, value: [u32; 4]) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_store_movdqu(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, pc, address, value)
    }
    fn copy_diagnostic_dispatch_call(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, request: x::CallRequest) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_call(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, request)
    }
    fn copy_diagnostic_dispatch_save_rbx_57a384(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_save_rbx_57a384(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, out)
    }
    fn copy_diagnostic_dispatch_push_rbp_57a389(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_push_rbp_57a389(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, out)
    }
    fn copy_diagnostic_dispatch_mov_rbp_rsp_57a38a(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_entry::RbpAssigned>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_mov_rbp_rsp_57a38a(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, out)
    }
    fn copy_diagnostic_dispatch_sub_rsp_70_57a38d(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_sub_rsp_70_57a38d(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, out)
    }
    fn copy_diagnostic_dispatch_restore_rbx_57a411(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_return::Load>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_restore_rbx_57a411(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, out)
    }
    fn copy_diagnostic_dispatch_add_rsp_70_57a419(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_add_rsp_70_57a419(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, out)
    }
    fn copy_diagnostic_dispatch_pop_rbp_57a41d(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_pop_rbp_57a41d(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, out)
    }
    fn copy_diagnostic_dispatch_ret_57a41e(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, calls: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<x::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch_ret_57a41e(&mut o.storage.owners, &mut o.calls, copy, diagnostic, calls, out)
    }
    fn adopt_completed_copy_diagnostic_dispatch(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, history: xc::History, held: &mut Option<xc::Retained<Self::CopyDiagnosticDispatchCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_duplicate_copy_diagnostic_dispatch(&mut o.storage.owners, &mut o.calls, copy, diagnostic, history, held)
    }
}
