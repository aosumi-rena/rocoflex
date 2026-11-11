use super::{Scope, CopyDiagnosticDispatchHandlerServices};
use super::super as p;
use p::duplicate as d;
use d::copy as w;
use w::diagnostic as a;
use a::dispatch as x;
use x::handler as h;
use h::{context as ctx, context_call as cc, encoded_slot as slot, encoded_slot_call as sc};

pub trait CopyDiagnosticDispatchHandlerContextServices: CopyDiagnosticDispatchHandlerServices {

    fn duplicate_copy_diagnostic_context_push_rdi_57a2d6(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_entry::Push>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_context_sub_rsp_20_57a2d7(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_context_mov_rax_rsi_57a328(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_return::MoveRax>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_context_add_rsp_20_57a330(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_context_pop_rdi_57a334(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_return::Pop>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_context_ret_57a335(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate_copy_diagnostic_context(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, history: cc::History, held: &mut Option<cc::Retained>) -> Result<(), Self::Error>;
}
impl<S: CopyDiagnosticDispatchHandlerContextServices> d::copy_call::DiagnosticDispatchHandlerContextServices for Scope<S> {
    fn copy_diagnostic_context_push_rdi_57a2d6(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_context_push_rdi_57a2d6(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_context_sub_rsp_20_57a2d7(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_context_sub_rsp_20_57a2d7(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_context_mov_rax_rsi_57a328(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_return::MoveRax>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_context_mov_rax_rsi_57a328(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_context_add_rsp_20_57a330(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_context_add_rsp_20_57a330(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_context_pop_rdi_57a334(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_context_pop_rdi_57a334(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_context_ret_57a335(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<ctx::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_context_ret_57a335(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn adopt_completed_copy_diagnostic_context(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, history: cc::History, held: &mut Option<cc::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_duplicate_copy_diagnostic_context(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, history, held)
    }
}

pub trait CopyDiagnosticDispatchHandlerEncodedSlotServices: CopyDiagnosticDispatchHandlerServices {

    fn duplicate_copy_diagnostic_slot_read_byte(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn duplicate_copy_diagnostic_slot_push_rdi_57a342(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_entry::Push>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_slot_sub_rsp_20_57a343(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_slot_add_rsp_20_57a37e(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_slot_pop_rdi_57a382(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_return::Pop>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_slot_ret_57a383(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate_copy_diagnostic_slot(&mut self, o: &mut p::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::DuplicateCopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DuplicateCopyDiagnosticHandlerCallOwner>, history: sc::History, held: &mut Option<sc::Retained>) -> Result<(), Self::Error>;
}
impl<S: CopyDiagnosticDispatchHandlerEncodedSlotServices> d::copy_call::DiagnosticDispatchHandlerEncodedSlotServices for Scope<S> {
    fn copy_diagnostic_slot_read_byte(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.duplicate_copy_diagnostic_slot_read_byte(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, pc, address)
    }
    fn copy_diagnostic_slot_push_rdi_57a342(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_slot_push_rdi_57a342(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_slot_sub_rsp_20_57a343(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_slot_sub_rsp_20_57a343(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_slot_add_rsp_20_57a37e(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_slot_add_rsp_20_57a37e(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_slot_pop_rdi_57a382(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_slot_pop_rdi_57a382(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn copy_diagnostic_slot_ret_57a383(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, out: &mut Option<slot::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_slot_ret_57a383(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, out)
    }
    fn adopt_completed_copy_diagnostic_slot(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, diagnostic: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, dispatch: &mut x::Calls<Self::CopyDiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::CopyDiagnosticHandlerCallOwner>, history: sc::History, held: &mut Option<sc::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_duplicate_copy_diagnostic_slot(&mut o.storage.owners, &mut o.calls, copy, diagnostic, dispatch, calls, history, held)
    }
}
