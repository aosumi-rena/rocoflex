use super::{Scope, DispatchHandlerServices};
use super::super as p;
use p::diagnostic as a;
use a::{dispatch as d, dispatch_call as dc};
use d::handler as h;
use h::{context as c, context_call as cc, encoded_slot as e, encoded_slot_call as ec};
pub trait DispatchHandlerContextServices: DispatchHandlerServices {
    fn diagnostic_context_push_rdi_57a2d6(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error>;
    fn diagnostic_context_sub_rsp_20_57a2d7(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn diagnostic_context_mov_rax_rsi_57a328(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<c::native_return::MoveRax>) -> Result<(), Self::Error>;
    fn diagnostic_context_add_rsp_20_57a330(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn diagnostic_context_pop_rdi_57a334(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error>;
    fn diagnostic_context_ret_57a335(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_diagnostic_context(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, history: cc::History, held: &mut Option<cc::Retained>) -> Result<(), Self::Error>;
}
impl<S: DispatchHandlerContextServices> dc::HandlerContextServices for Scope<S> {
    fn dispatch_context_push_rdi_57a2d6(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.diagnostic_context_push_rdi_57a2d6(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_context_sub_rsp_20_57a2d7(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.diagnostic_context_sub_rsp_20_57a2d7(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_context_mov_rax_rsi_57a328(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_return::MoveRax>) -> Result<(), Self::Error> {
        self.services.diagnostic_context_mov_rax_rsi_57a328(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_context_add_rsp_20_57a330(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.diagnostic_context_add_rsp_20_57a330(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_context_pop_rdi_57a334(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.diagnostic_context_pop_rdi_57a334(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_context_ret_57a335(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.diagnostic_context_ret_57a335(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn adopt_completed_dispatch_context(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, history: cc::History, held: &mut Option<cc::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_diagnostic_context(&mut o.storage.owners, &mut o.call, dispatch, calls, history, held)
    }
}
pub trait DispatchHandlerEncodedSlotServices: DispatchHandlerServices {
    fn diagnostic_slot_read_byte(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn diagnostic_slot_push_rdi_57a342(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<e::native_entry::Push>) -> Result<(), Self::Error>;
    fn diagnostic_slot_sub_rsp_20_57a343(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<e::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn diagnostic_slot_add_rsp_20_57a37e(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<e::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn diagnostic_slot_pop_rdi_57a382(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<e::native_return::Pop>) -> Result<(), Self::Error>;
    fn diagnostic_slot_ret_57a383(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, out: &mut Option<e::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_diagnostic_slot(&mut self, parent: &mut p::Owners<Self>, diagnostic: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, dispatch: &mut d::Calls<Self::DiagnosticDispatchCallOwner>, calls: &mut h::Calls<Self::DiagnosticHandlerCallOwner>, history: ec::History, held: &mut Option<ec::Retained>) -> Result<(), Self::Error>;
}
impl<S: DispatchHandlerEncodedSlotServices> dc::HandlerEncodedSlotServices for Scope<S> {
    fn dispatch_slot_read_byte(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.diagnostic_slot_read_byte(&mut o.storage.owners, &mut o.call, dispatch, calls, pc, address)
    }
    fn dispatch_slot_push_rdi_57a342(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.diagnostic_slot_push_rdi_57a342(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_slot_sub_rsp_20_57a343(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.diagnostic_slot_sub_rsp_20_57a343(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_slot_add_rsp_20_57a37e(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.diagnostic_slot_add_rsp_20_57a37e(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_slot_pop_rdi_57a382(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.diagnostic_slot_pop_rdi_57a382(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn dispatch_slot_ret_57a383(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.diagnostic_slot_ret_57a383(&mut o.storage.owners, &mut o.call, dispatch, calls, out)
    }
    fn adopt_completed_dispatch_slot(&mut self, o: &mut a::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, history: ec::History, held: &mut Option<ec::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_diagnostic_slot(&mut o.storage.owners, &mut o.call, dispatch, calls, history, held)
    }
}
