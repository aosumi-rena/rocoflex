use super::{Scope, Services};
use super::super as w;
use w::dispatch as d;
use d::{handler as h, handler_call as hc, metadata as m, metadata_call as mc};
pub trait HandlerServices: Services {
    type DispatchHandlerCallOwner;

    fn dispatch_handler_read_qword(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn dispatch_handler_write_qword(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error>;
    fn dispatch_handler_call(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, request: h::CallRequest) -> Result<(), Self::Error>;
    fn dispatch_handler_save_register(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, pc: u32, register: h::native_entry::Register, out: &mut Option<h::native_entry::Store>) -> Result<(), Self::Error>;
    fn dispatch_handler_push_rdi_57a20f(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<h::native_entry::Push>) -> Result<(), Self::Error>;
    fn dispatch_handler_sub_rsp_30_57a210(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<h::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn dispatch_handler_restore_register(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, pc: u32, register: h::native_return::Register, out: &mut Option<h::native_return::Load>) -> Result<(), Self::Error>;
    fn dispatch_handler_add_rsp_30_57a270(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<h::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn dispatch_handler_pop_rdi_57a274(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<h::native_return::Pop>) -> Result<(), Self::Error>;
    fn dispatch_handler_ret_57a275(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, calls: &mut h::Calls<Self::DispatchHandlerCallOwner>, out: &mut Option<h::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_dispatch_handler(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, history: hc::History, held: &mut Option<hc::Retained<Self::DispatchHandlerCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: HandlerServices> hc::Services for Scope<S> {
    type HandlerCallOwner = S::DispatchHandlerCallOwner;
    fn handler_read_qword(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.dispatch_handler_read_qword(&mut o.storage.owners, &mut o.calls, calls, pc, address)
    }
    fn handler_write_qword(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, address: u64, value: u64) -> Result<(), Self::Error> {
        self.services.dispatch_handler_write_qword(&mut o.storage.owners, &mut o.calls, calls, pc, address, value)
    }
    fn handler_call(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, request: h::CallRequest) -> Result<(), Self::Error> {
        self.services.dispatch_handler_call(&mut o.storage.owners, &mut o.calls, calls, request)
    }
    fn handler_save_register(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, register: h::native_entry::Register, out: &mut Option<h::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_save_register(&mut o.storage.owners, &mut o.calls, calls, pc, register, out)
    }
    fn handler_push_rdi_57a20f(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_push_rdi_57a20f(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_sub_rsp_30_57a210(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_sub_rsp_30_57a210(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_restore_register(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, register: h::native_return::Register, out: &mut Option<h::native_return::Load>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_restore_register(&mut o.storage.owners, &mut o.calls, calls, pc, register, out)
    }
    fn handler_add_rsp_30_57a270(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_add_rsp_30_57a270(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_pop_rdi_57a274(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_pop_rdi_57a274(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn handler_ret_57a275(&mut self, o: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<h::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.dispatch_handler_ret_57a275(&mut o.storage.owners, &mut o.calls, calls, out)
    }
    fn adopt_completed_handler(&mut self, o: &mut d::Owners<Self>, history: hc::History, held: &mut Option<hc::Retained<Self::HandlerCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_dispatch_handler(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
pub trait MetadataServices: Services {

    fn dispatch_metadata_read_qword(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn dispatch_metadata_save_rbx_568a1f(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<m::Store>) -> Result<(), Self::Error>;
    fn dispatch_metadata_push_rdi_568a10(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error>;
    fn dispatch_metadata_sub_rsp_20_568a12(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn dispatch_metadata_add_rsp_20_568a6b(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn dispatch_metadata_pop_rdi_568a6f(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error>;
    fn dispatch_metadata_ret_568a70(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, out: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_dispatch_metadata(&mut self, parent: &mut w::Owners<Self>, dispatch: &mut d::Calls<Self::DispatchCallOwner>, history: mc::History, held: &mut Option<mc::Retained>) -> Result<(), Self::Error>;
}
impl<S: MetadataServices> mc::Services for Scope<S> {
    fn metadata_read_qword(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.dispatch_metadata_read_qword(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn metadata_save_rbx_568a1f(&mut self, o: &mut d::Owners<Self>, out: &mut Option<m::Store>) -> Result<(), Self::Error> {
        self.services.dispatch_metadata_save_rbx_568a1f(&mut o.storage.owners, &mut o.calls, out)
    }
    fn metadata_push_rdi_568a10(&mut self, o: &mut d::Owners<Self>, out: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.dispatch_metadata_push_rdi_568a10(&mut o.storage.owners, &mut o.calls, out)
    }
    fn metadata_sub_rsp_20_568a12(&mut self, o: &mut d::Owners<Self>, out: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.dispatch_metadata_sub_rsp_20_568a12(&mut o.storage.owners, &mut o.calls, out)
    }
    fn metadata_add_rsp_20_568a6b(&mut self, o: &mut d::Owners<Self>, out: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.dispatch_metadata_add_rsp_20_568a6b(&mut o.storage.owners, &mut o.calls, out)
    }
    fn metadata_pop_rdi_568a6f(&mut self, o: &mut d::Owners<Self>, out: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.dispatch_metadata_pop_rdi_568a6f(&mut o.storage.owners, &mut o.calls, out)
    }
    fn metadata_ret_568a70(&mut self, o: &mut d::Owners<Self>, out: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.dispatch_metadata_ret_568a70(&mut o.storage.owners, &mut o.calls, out)
    }
    fn adopt_completed_metadata(&mut self, o: &mut d::Owners<Self>, history: mc::History, held: &mut Option<mc::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_dispatch_metadata(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
