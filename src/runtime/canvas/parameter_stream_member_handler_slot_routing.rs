use super::{Scope, Services};
use super::super as d;
use d::handler as h;
use h::{encoded_slot as e, encoded_slot_call as ec};
pub trait EncodedSlotServices: Services {
    fn handler_slot_read_byte(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn handler_slot_read_qword(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn handler_slot_save_register(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, register: e::native_entry::Register, out: &mut Option<e::native_entry::Store>) -> Result<(), Self::Error>;
    fn handler_slot_push_rdi_57a342(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_entry::Push>) -> Result<(), Self::Error>;
    fn handler_slot_sub_rsp_20_57a343(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn handler_slot_restore_register(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, pc: u32, register: e::native_return::Register, out: &mut Option<e::native_return::Load>) -> Result<(), Self::Error>;
    fn handler_slot_add_rsp_20_57a37e(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn handler_slot_pop_rdi_57a382(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_return::Pop>) -> Result<(), Self::Error>;
    fn handler_slot_ret_57a383(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, out: &mut Option<e::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_handler_slot(&mut self, parent: &mut d::Owners<Self>, calls: &mut h::Calls<Self::HandlerCallOwner>, history: ec::History, held: &mut Option<ec::Retained>) -> Result<(), Self::Error>;
}
impl<S: EncodedSlotServices> ec::Services for Scope<S> {
    fn slot_read_byte(&mut self, o: &mut h::Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.handler_slot_read_byte(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn slot_read_qword(&mut self, o: &mut h::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.handler_slot_read_qword(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn slot_save_register(&mut self, o: &mut h::Owners<Self>, pc: u32, register: e::native_entry::Register, out: &mut Option<e::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.handler_slot_save_register(&mut o.storage.owners, &mut o.calls, pc, register, out)
    }
    fn slot_push_rdi_57a342(&mut self, o: &mut h::Owners<Self>, out: &mut Option<e::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.handler_slot_push_rdi_57a342(&mut o.storage.owners, &mut o.calls, out)
    }
    fn slot_sub_rsp_20_57a343(&mut self, o: &mut h::Owners<Self>, out: &mut Option<e::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.handler_slot_sub_rsp_20_57a343(&mut o.storage.owners, &mut o.calls, out)
    }
    fn slot_restore_register(&mut self, o: &mut h::Owners<Self>, pc: u32, register: e::native_return::Register, out: &mut Option<e::native_return::Load>) -> Result<(), Self::Error> {
        self.services.handler_slot_restore_register(&mut o.storage.owners, &mut o.calls, pc, register, out)
    }
    fn slot_add_rsp_20_57a37e(&mut self, o: &mut h::Owners<Self>, out: &mut Option<e::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.handler_slot_add_rsp_20_57a37e(&mut o.storage.owners, &mut o.calls, out)
    }
    fn slot_pop_rdi_57a382(&mut self, o: &mut h::Owners<Self>, out: &mut Option<e::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.handler_slot_pop_rdi_57a382(&mut o.storage.owners, &mut o.calls, out)
    }
    fn slot_ret_57a383(&mut self, o: &mut h::Owners<Self>, out: &mut Option<e::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.handler_slot_ret_57a383(&mut o.storage.owners, &mut o.calls, out)
    }
    fn adopt_completed_encoded_slot(&mut self, o: &mut h::Owners<Self>, history: ec::History, held: &mut Option<ec::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_completed_handler_slot(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
