use super::{Scope, CopyDiagnosticDispatchServices};
use super::super as p;
use p::duplicate as d;
use d::copy as w;
use w::diagnostic as a;
use a::dispatch as x;
use x::{handler as h, handler_call as hc, metadata as md, metadata_call as mc};

pub trait CopyDiagnosticDispatchHandlerServices: CopyDiagnosticDispatchServices {
    type DuplicateCopyDiagnosticHandlerCallOwner;

    fn duplicate_copy_diagnostic_handler_read_qword(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn duplicate_copy_diagnostic_handler_write_qword(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_handler_call(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, request: h::CallRequest) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_handler_save_register(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, pc: u32, register: h::native_entry::Register, out: &mut Option<h::native_entry::Store>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_handler_push_rdi_57a20f(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_entry::Push>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_handler_sub_rsp_30_57a210(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_handler_restore_register(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, pc: u32, register: h::native_return::Register, out: &mut Option<h::native_return::Load>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_handler_add_rsp_30_57a270(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_handler_pop_rdi_57a274(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_return::Pop>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_handler_ret_57a275(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate_copy_diagnostic_handler(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, history: hc::History, held: &mut Option<hc::Retained<Self::DuplicateCopyDiagnosticHandlerCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: CopyDiagnosticDispatchHandlerServices> d::copy_call::DiagnosticDispatchHandlerServices for Scope<S> {
    type CopyDiagnosticHandlerCallOwner = S::DuplicateCopyDiagnosticHandlerCallOwner;
    fn copy_diagnostic_handler_read_qword(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_read_qword(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, pc, address)
    }
    fn copy_diagnostic_handler_write_qword(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_write_qword(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, pc, address, value)
    }
    fn copy_diagnostic_handler_call(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, request: h::CallRequest) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_call(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, request)
    }
    fn copy_diagnostic_handler_save_register(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, register: h::native_entry::Register, out: &mut Option<h::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_save_register(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, pc, register, out)
    }
    fn copy_diagnostic_handler_push_rdi_57a20f(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_push_rdi_57a20f(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_handler_sub_rsp_30_57a210(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_sub_rsp_30_57a210(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_handler_restore_register(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, register: h::native_return::Register, out: &mut Option<h::native_return::Load>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_restore_register(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, pc, register, out)
    }
    fn copy_diagnostic_handler_add_rsp_30_57a270(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_add_rsp_30_57a270(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_handler_pop_rdi_57a274(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_pop_rdi_57a274(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_handler_ret_57a275(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<h::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_handler_ret_57a275(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn adopt_completed_copy_diagnostic_handler(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, history: hc::History, held: &mut Option<hc::Retained<Self::CopyDiagnosticHandlerCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_duplicate_copy_diagnostic_handler(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, history, held)
    }
}

pub trait CopyDiagnosticDispatchMetadataServices: CopyDiagnosticDispatchServices {

    fn duplicate_copy_diagnostic_metadata_save_rbx_568a1f(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<md::Store>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_metadata_push_rdi_568a10(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_entry::Push>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_metadata_sub_rsp_20_568a12(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_metadata_add_rsp_20_568a6b(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_metadata_pop_rdi_568a6f(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_return::Pop>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_metadata_ret_568a70(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate_copy_diagnostic_metadata(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, history: mc::History, held: &mut Option<mc::Retained>) -> Result<(), Self::Error>;
}
impl<S: CopyDiagnosticDispatchMetadataServices> d::copy_call::DiagnosticDispatchMetadataServices for Scope<S> {
    fn copy_diagnostic_metadata_save_rbx_568a1f(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<md::Store>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_metadata_save_rbx_568a1f(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, out)
    }
    fn copy_diagnostic_metadata_push_rdi_568a10(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_metadata_push_rdi_568a10(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, out)
    }
    fn copy_diagnostic_metadata_sub_rsp_20_568a12(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_metadata_sub_rsp_20_568a12(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, out)
    }
    fn copy_diagnostic_metadata_add_rsp_20_568a6b(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_metadata_add_rsp_20_568a6b(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, out)
    }
    fn copy_diagnostic_metadata_pop_rdi_568a6f(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_metadata_pop_rdi_568a6f(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, out)
    }
    fn copy_diagnostic_metadata_ret_568a70(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<md::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_metadata_ret_568a70(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, out)
    }
    fn adopt_completed_copy_diagnostic_metadata(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, history: mc::History, held: &mut Option<mc::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_duplicate_copy_diagnostic_metadata(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, history, held)
    }
}
