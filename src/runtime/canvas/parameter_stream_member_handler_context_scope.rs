use super::{c, ParentState, History, Retained};
pub trait Services: super::super::Services {
    fn context_read(&mut self, parent: &mut super::super::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn context_save_register(&mut self, parent: &mut super::super::Owners<Self>, pc: u32, register: c::native_entry::Register, out: &mut Option<c::native_entry::Store>) -> Result<(), Self::Error>;
    fn context_push_rdi_57a2d6(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error>;
    fn context_sub_rsp_20_57a2d7(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn context_restore_register(&mut self, parent: &mut super::super::Owners<Self>, pc: u32, register: c::native_return::Register, out: &mut Option<c::native_return::Load>) -> Result<(), Self::Error>;
    fn context_mov_rax_rsi_57a328(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<c::native_return::MoveRax>) -> Result<(), Self::Error>;
    fn context_add_rsp_20_57a330(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn context_pop_rdi_57a334(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error>;
    fn context_ret_57a335(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_context(&mut self, parent: &mut super::super::Owners<Self>, history: History, held: &mut Option<Retained>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> c::Services for Scope<S> {
    type Storage = ParentState<S>; type Error = S::Error;
    fn read_context(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.context_read(&mut o.storage.owners, pc, address) }
}
impl<S: Services> c::native_entry::Services for Scope<S> {
    fn save_register(&mut self, o: &mut c::Owners<Self>, pc: u32, register: c::native_entry::Register, out: &mut Option<c::native_entry::Store>) -> Result<(), Self::Error> { self.services.context_save_register(&mut o.storage.owners, pc, register, out) }
    fn push_rdi_57a2d6(&mut self, o: &mut c::Owners<Self>, out: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error> { self.services.context_push_rdi_57a2d6(&mut o.storage.owners, out) }
    fn sub_rsp_20_57a2d7(&mut self, o: &mut c::Owners<Self>, out: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error> { self.services.context_sub_rsp_20_57a2d7(&mut o.storage.owners, out) }
}
impl<S: Services> c::native_return::Services for Scope<S> {
    fn restore_register(&mut self, o: &mut c::Owners<Self>, pc: u32, register: c::native_return::Register, out: &mut Option<c::native_return::Load>) -> Result<(), Self::Error> { self.services.context_restore_register(&mut o.storage.owners, pc, register, out) }
    fn mov_rax_rsi_57a328(&mut self, o: &mut c::Owners<Self>, out: &mut Option<c::native_return::MoveRax>) -> Result<(), Self::Error> { self.services.context_mov_rax_rsi_57a328(&mut o.storage.owners, out) }
    fn add_rsp_20_57a330(&mut self, o: &mut c::Owners<Self>, out: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.context_add_rsp_20_57a330(&mut o.storage.owners, out) }
    fn pop_rdi_57a334(&mut self, o: &mut c::Owners<Self>, out: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error> { self.services.context_pop_rdi_57a334(&mut o.storage.owners, out) }
    fn ret_57a335(&mut self, o: &mut c::Owners<Self>, out: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.context_ret_57a335(&mut o.storage.owners, out) }
}
