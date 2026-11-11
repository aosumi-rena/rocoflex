use super::{Scope, DiagnosticDispatchHandlerServices};
use super::super as parent;
use parent::copy;
use copy::{diagnostic, diagnostic_call};
use diagnostic::dispatch;
use dispatch::handler;
use handler::{context, context_call, encoded_slot, encoded_slot_call};

pub trait DiagnosticDispatchHandlerContextServices: DiagnosticDispatchHandlerServices {
    fn copy_diagnostic_context_push_rdi_57a2d6(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<context::native_entry::Push>) -> Result<(), Self::Error>;
    fn copy_diagnostic_context_sub_rsp_20_57a2d7(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<context::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn copy_diagnostic_context_mov_rax_rsi_57a328(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<context::native_return::MoveRax>) -> Result<(), Self::Error>;
    fn copy_diagnostic_context_add_rsp_20_57a330(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<context::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn copy_diagnostic_context_pop_rdi_57a334(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<context::native_return::Pop>) -> Result<(), Self::Error>;
    fn copy_diagnostic_context_ret_57a335(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<context::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_copy_diagnostic_context(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, history: context_call::History, held: &mut Option<context_call::Retained>) -> Result<(), Self::Error>;
}
impl<S: DiagnosticDispatchHandlerContextServices> diagnostic_call::DispatchHandlerContextServices for Scope<S> {
    fn diagnostic_context_push_rdi_57a2d6(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<context::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_context_push_rdi_57a2d6(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_context_sub_rsp_20_57a2d7(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<context::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_context_sub_rsp_20_57a2d7(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_context_mov_rax_rsi_57a328(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<context::native_return::MoveRax>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_context_mov_rax_rsi_57a328(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_context_add_rsp_20_57a330(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<context::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_context_add_rsp_20_57a330(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_context_pop_rdi_57a334(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<context::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_context_pop_rdi_57a334(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_context_ret_57a335(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<context::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_context_ret_57a335(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn adopt_completed_diagnostic_context(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, history: context_call::History, held: &mut Option<context_call::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_copy_diagnostic_context(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, history, held)
    }
}

pub trait DiagnosticDispatchHandlerEncodedSlotServices: DiagnosticDispatchHandlerServices {
    fn copy_diagnostic_slot_read_byte(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn copy_diagnostic_slot_push_rdi_57a342(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_entry::Push>) -> Result<(), Self::Error>;
    fn copy_diagnostic_slot_sub_rsp_20_57a343(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn copy_diagnostic_slot_add_rsp_20_57a37e(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn copy_diagnostic_slot_pop_rdi_57a382(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_return::Pop>) -> Result<(), Self::Error>;
    fn copy_diagnostic_slot_ret_57a383(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_copy_diagnostic_slot(&mut self, o: &mut parent::Owners<Self>, copy: &mut copy::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<diagnostic::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::CopyDiagnosticHandlerCallOwner>, history: encoded_slot_call::History, held: &mut Option<encoded_slot_call::Retained>) -> Result<(), Self::Error>;
}
impl<S: DiagnosticDispatchHandlerEncodedSlotServices> diagnostic_call::DispatchHandlerEncodedSlotServices for Scope<S> {
    fn diagnostic_slot_read_byte(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.copy_diagnostic_slot_read_byte(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, pc, address)
    }
    fn diagnostic_slot_push_rdi_57a342(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_slot_push_rdi_57a342(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_slot_sub_rsp_20_57a343(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_slot_sub_rsp_20_57a343(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_slot_add_rsp_20_57a37e(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_slot_add_rsp_20_57a37e(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_slot_pop_rdi_57a382(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_slot_pop_rdi_57a382(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn diagnostic_slot_ret_57a383(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<encoded_slot::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_slot_ret_57a383(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, out)
    }
    fn adopt_completed_diagnostic_slot(&mut self, o: &mut copy::Owners<Self>, diagnostic: &mut Option<diagnostic::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut dispatch::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut handler::Calls<Self::DiagnosticHandlerCallOwner>, history: encoded_slot_call::History, held: &mut Option<encoded_slot_call::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_copy_diagnostic_slot(&mut o.storage.owners, &mut o.calls, diagnostic, dispatch, calls, history, held)
    }
}
