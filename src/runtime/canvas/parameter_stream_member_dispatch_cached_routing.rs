use super::{Scope, HandlerServices};
use super::super as w;
use w::dispatch as d;
use d::{handler as h, handler_call as hc};
use h::{context as c, context_call as cc, encoded_slot as e, encoded_slot_call as ec};
pub trait HandlerContextServices: HandlerServices {
    fn dispatch_context_push_rdi_57a2d6(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error>;
    fn dispatch_context_sub_rsp_20_57a2d7(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn dispatch_context_mov_rax_rsi_57a328(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_return::MoveRax>) -> Result<(), Self::Error>;
    fn dispatch_context_add_rsp_20_57a330(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn dispatch_context_pop_rdi_57a334(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error>;
    fn dispatch_context_ret_57a335(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_dispatch_context(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, history: cc::History, held: &mut Option<cc::Retained>) -> Result<(), Self::Error>;
}
impl<S: HandlerContextServices> hc::ContextServices for Scope<S> {
    fn handler_context_push_rdi_57a2d6(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.dispatch_context_push_rdi_57a2d6(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_context_sub_rsp_20_57a2d7(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.dispatch_context_sub_rsp_20_57a2d7(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_context_mov_rax_rsi_57a328(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_return::MoveRax>) -> Result<(), Self::Error> {
        self.services.dispatch_context_mov_rax_rsi_57a328(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_context_add_rsp_20_57a330(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.dispatch_context_add_rsp_20_57a330(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_context_pop_rdi_57a334(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.dispatch_context_pop_rdi_57a334(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_context_ret_57a335(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.dispatch_context_ret_57a335(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn adopt_completed_handler_context(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, history: cc::History, held: &mut Option<cc::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_dispatch_context(&mut o.storage.owners, &mut o.calls, calls, history, held)
    }
}
pub trait HandlerEncodedSlotServices: HandlerServices {
    fn dispatch_slot_read_byte(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn dispatch_slot_push_rdi_57a342(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_entry::Push>) -> Result<(), Self::Error>;
    fn dispatch_slot_sub_rsp_20_57a343(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn dispatch_slot_add_rsp_20_57a37e(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn dispatch_slot_pop_rdi_57a382(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_return::Pop>) -> Result<(), Self::Error>;
    fn dispatch_slot_ret_57a383(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<e::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_dispatch_slot(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, history: ec::History, held: &mut Option<ec::Retained>) -> Result<(), Self::Error>;
}
impl<S: HandlerEncodedSlotServices> hc::EncodedSlotServices for Scope<S> {
    fn handler_slot_read_byte(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.dispatch_slot_read_byte(&mut o.storage.owners, &mut o.calls, calls, pc, address)
    }
    fn handler_slot_read_qword(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.dispatch_handler_read_qword(&mut o.storage.owners, &mut o.calls, calls, pc, address)
    }
    fn handler_slot_save_register(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, register: e::native_entry::Register, out: &mut Option<e::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_save_register(&mut o.storage.owners, &mut o.calls, calls, pc, register, out)
    }
    fn handler_slot_push_rdi_57a342(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.dispatch_slot_push_rdi_57a342(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_slot_sub_rsp_20_57a343(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.dispatch_slot_sub_rsp_20_57a343(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_slot_restore_register(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, register: e::native_return::Register, out: &mut Option<e::native_return::Load>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_restore_register(&mut o.storage.owners, &mut o.calls, calls, pc, register, out)
    }
    fn handler_slot_add_rsp_20_57a37e(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.dispatch_slot_add_rsp_20_57a37e(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_slot_pop_rdi_57a382(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.dispatch_slot_pop_rdi_57a382(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_slot_ret_57a383(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.dispatch_slot_ret_57a383(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn adopt_completed_handler_slot(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, history: ec::History, held: &mut Option<ec::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_dispatch_slot(&mut o.storage.owners, &mut o.calls, calls, history, held)
    }
}
