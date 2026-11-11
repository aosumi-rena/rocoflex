use super::{Scope, Services};
use super::super as d;
use d::handler as h;
use h::{context as c, context_call as cc};
pub trait ContextServices: Services {
    fn handler_context_push_rdi_57a2d6(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error>;
    fn handler_context_sub_rsp_20_57a2d7(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn handler_context_mov_rax_rsi_57a328(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_return::MoveRax>) -> Result<(), Self::Error>;
    fn handler_context_add_rsp_20_57a330(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn handler_context_pop_rdi_57a334(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error>;
    fn handler_context_ret_57a335(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_handler_context(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, history: cc::History, held: &mut Option<cc::Retained>) -> Result<(), Self::Error>;
}
impl<S: ContextServices> cc::Services for Scope<S> {
    fn context_read(&mut self, o: &mut h::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.handler_read_qword(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn context_save_register(&mut self, o: &mut h::Owners<Self>, pc: u32, register: c::native_entry::Register, out: &mut Option<c::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.handler_save_register(&mut o.storage.owners, &mut o.calls, pc, register, out)
    }
    fn context_push_rdi_57a2d6(&mut self, o: &mut h::Owners<Self>, out: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.handler_context_push_rdi_57a2d6(&mut o.storage.owners, &mut o.calls, out)
    }
    fn context_sub_rsp_20_57a2d7(&mut self, o: &mut h::Owners<Self>, out: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.handler_context_sub_rsp_20_57a2d7(&mut o.storage.owners, &mut o.calls, out)
    }
    fn context_restore_register(&mut self, o: &mut h::Owners<Self>, pc: u32, register: c::native_return::Register, out: &mut Option<c::native_return::Load>) -> Result<(), Self::Error> {
        self.services.handler_restore_register(&mut o.storage.owners, &mut o.calls, pc, register, out)
    }
    fn context_mov_rax_rsi_57a328(&mut self, o: &mut h::Owners<Self>, out: &mut Option<c::native_return::MoveRax>) -> Result<(), Self::Error> {
        self.services.handler_context_mov_rax_rsi_57a328(&mut o.storage.owners, &mut o.calls, out)
    }
    fn context_add_rsp_20_57a330(&mut self, o: &mut h::Owners<Self>, out: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.handler_context_add_rsp_20_57a330(&mut o.storage.owners, &mut o.calls, out)
    }
    fn context_pop_rdi_57a334(&mut self, o: &mut h::Owners<Self>, out: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.handler_context_pop_rdi_57a334(&mut o.storage.owners, &mut o.calls, out)
    }
    fn context_ret_57a335(&mut self, o: &mut h::Owners<Self>, out: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.handler_context_ret_57a335(&mut o.storage.owners, &mut o.calls, out)
    }
    fn adopt_completed_context(&mut self, o: &mut h::Owners<Self>, history: cc::History, held: &mut Option<cc::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_handler_context(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
