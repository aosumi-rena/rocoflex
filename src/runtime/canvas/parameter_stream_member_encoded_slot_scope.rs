use super::{e, ParentState, History, Retained};
pub trait Services: super::super::Services {
    fn slot_read_byte(&mut self, parent: &mut super::super::Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn slot_read_qword(&mut self, parent: &mut super::super::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn slot_save_register(&mut self, parent: &mut super::super::Owners<Self>, pc: u32, register: e::native_entry::Register, out: &mut Option<e::native_entry::Store>) -> Result<(), Self::Error>;
    fn slot_push_rdi_57a342(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<e::native_entry::Push>) -> Result<(), Self::Error>;
    fn slot_sub_rsp_20_57a343(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<e::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn slot_restore_register(&mut self, parent: &mut super::super::Owners<Self>, pc: u32, register: e::native_return::Register, out: &mut Option<e::native_return::Load>) -> Result<(), Self::Error>;
    fn slot_add_rsp_20_57a37e(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<e::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn slot_pop_rdi_57a382(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<e::native_return::Pop>) -> Result<(), Self::Error>;
    fn slot_ret_57a383(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<e::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_encoded_slot(&mut self, parent: &mut super::super::Owners<Self>, history: History, held: &mut Option<Retained>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> e::Services for Scope<S> {
    type Storage = ParentState<S>; type Error = S::Error;
    fn read_byte(&mut self, o: &mut e::Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error> { self.services.slot_read_byte(&mut o.storage.owners, pc, address) }
    fn read_qword(&mut self, o: &mut e::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.slot_read_qword(&mut o.storage.owners, pc, address) }
}
impl<S: Services> e::native_entry::Services for Scope<S> {
    fn save_register(&mut self, o: &mut e::Owners<Self>, pc: u32, register: e::native_entry::Register, out: &mut Option<e::native_entry::Store>) -> Result<(), Self::Error> { self.services.slot_save_register(&mut o.storage.owners, pc, register, out) }
    fn push_rdi_57a342(&mut self, o: &mut e::Owners<Self>, out: &mut Option<e::native_entry::Push>) -> Result<(), Self::Error> { self.services.slot_push_rdi_57a342(&mut o.storage.owners, out) }
    fn sub_rsp_20_57a343(&mut self, o: &mut e::Owners<Self>, out: &mut Option<e::native_entry::Subtract>) -> Result<(), Self::Error> { self.services.slot_sub_rsp_20_57a343(&mut o.storage.owners, out) }
}
impl<S: Services> e::native_return::Services for Scope<S> {
    fn restore_register(&mut self, o: &mut e::Owners<Self>, pc: u32, register: e::native_return::Register, out: &mut Option<e::native_return::Load>) -> Result<(), Self::Error> { self.services.slot_restore_register(&mut o.storage.owners, pc, register, out) }
    fn add_rsp_20_57a37e(&mut self, o: &mut e::Owners<Self>, out: &mut Option<e::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.slot_add_rsp_20_57a37e(&mut o.storage.owners, out) }
    fn pop_rdi_57a382(&mut self, o: &mut e::Owners<Self>, out: &mut Option<e::native_return::Pop>) -> Result<(), Self::Error> { self.services.slot_pop_rdi_57a382(&mut o.storage.owners, out) }
    fn ret_57a383(&mut self, o: &mut e::Owners<Self>, out: &mut Option<e::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.slot_ret_57a383(&mut o.storage.owners, out) }
}
