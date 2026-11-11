use super::{Scope, DiagnosticDispatchServices};
use super::super as parent;
use parent::copy;
use copy::{diagnostic, diagnostic_call};
use diagnostic::dispatch;
use dispatch::{handler, handler_call, metadata, metadata_call};

pub trait DiagnosticDispatchHandlerServices: DiagnosticDispatchServices {
    type CopyDiagnosticHandlerCallOwner;
    fn copy_diagnostic_handler_read_qword(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn copy_diagnostic_handler_write_qword(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn copy_diagnostic_handler_call(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, request: handler::CallRequest) -> Result<(), Self::Error>;
    fn copy_diagnostic_handler_save_register(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, register: handler::native_entry::Register, out: &mut Option<handler::native_entry::Store>) -> Result<(), Self::Error>;
    fn copy_diagnostic_handler_push_rdi_57a20f(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<handler::native_entry::Push>) -> Result<(), Self::Error>;
    fn copy_diagnostic_handler_sub_rsp_30_57a210(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<handler::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn copy_diagnostic_handler_restore_register(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, register: handler::native_return::Register, out: &mut Option<handler::native_return::Load>) -> Result<(), Self::Error>;
    fn copy_diagnostic_handler_add_rsp_30_57a270(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<handler::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn copy_diagnostic_handler_pop_rdi_57a274(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<handler::native_return::Pop>) -> Result<(), Self::Error>;
    fn copy_diagnostic_handler_ret_57a275(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<handler::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_copy_diagnostic_handler(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, history: handler_call::History, held: &mut Option<handler_call::Retained<Self::CopyDiagnosticHandlerCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: DiagnosticDispatchHandlerServices> diagnostic_call::DispatchHandlerServices for Scope<S> {
    type DiagnosticHandlerCallOwner = S::CopyDiagnosticHandlerCallOwner;
    fn diagnostic_handler_read_qword(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.copy_diagnostic_handler_read_qword(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, pc, address)
    }
    fn diagnostic_handler_write_qword(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_write_qword(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, pc, address, value)
    }
    fn diagnostic_handler_call(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, request: handler::CallRequest) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_call(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, request)
    }
    fn diagnostic_handler_save_register(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, pc: u32, register: handler::native_entry::Register, out: &mut Option<handler::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_save_register(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, pc, register, out)
    }
    fn diagnostic_handler_push_rdi_57a20f(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<handler::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_push_rdi_57a20f(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_handler_sub_rsp_30_57a210(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<handler::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_sub_rsp_30_57a210(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_handler_restore_register(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, pc: u32, register: handler::native_return::Register, out: &mut Option<handler::native_return::Load>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_restore_register(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, pc, register, out)
    }
    fn diagnostic_handler_add_rsp_30_57a270(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<handler::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_add_rsp_30_57a270(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_handler_pop_rdi_57a274(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<handler::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_pop_rdi_57a274(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_handler_ret_57a275(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<handler::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_handler_ret_57a275(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn adopt_completed_diagnostic_handler(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, history: handler_call::History, held: &mut Option<handler_call::Retained<Self::DiagnosticHandlerCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_copy_diagnostic_handler(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, history, held)
    }
}

pub trait DiagnosticDispatchMetadataServices: DiagnosticDispatchServices {
    fn copy_diagnostic_metadata_save_rbx_568a1f(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<metadata::Store>) -> Result<(), Self::Error>;
    fn copy_diagnostic_metadata_push_rdi_568a10(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_entry::Push>) -> Result<(), Self::Error>;
    fn copy_diagnostic_metadata_sub_rsp_20_568a12(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn copy_diagnostic_metadata_add_rsp_20_568a6b(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn copy_diagnostic_metadata_pop_rdi_568a6f(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_return::Pop>) -> Result<(), Self::Error>;
    fn copy_diagnostic_metadata_ret_568a70(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_copy_diagnostic_metadata(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, history: metadata_call::History, held: &mut Option<metadata_call::Retained>) -> Result<(), Self::Error>;
}
impl<S: DiagnosticDispatchMetadataServices> diagnostic_call::DispatchMetadataServices for Scope<S> {
    fn diagnostic_metadata_save_rbx_568a1f(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<metadata::Store>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_metadata_save_rbx_568a1f(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, out)
    }
    fn diagnostic_metadata_push_rdi_568a10(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_metadata_push_rdi_568a10(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, out)
    }
    fn diagnostic_metadata_sub_rsp_20_568a12(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_metadata_sub_rsp_20_568a12(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, out)
    }
    fn diagnostic_metadata_add_rsp_20_568a6b(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_metadata_add_rsp_20_568a6b(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, out)
    }
    fn diagnostic_metadata_pop_rdi_568a6f(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_metadata_pop_rdi_568a6f(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, out)
    }
    fn diagnostic_metadata_ret_568a70(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, out: &mut Option<metadata::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_metadata_ret_568a70(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, out)
    }
    fn adopt_completed_diagnostic_metadata(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, history: metadata_call::History, held: &mut Option<metadata_call::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_copy_diagnostic_metadata(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, history, held)
    }
}
