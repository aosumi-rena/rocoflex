use super::{m, ParentState, History, Retained};
pub trait Services: super::super::Services {

    fn metadata_read_qword(&mut self, parent: &mut super::super::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn metadata_save_rbx_568a1f(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<m::Store>) -> Result<(), Self::Error>;
    fn metadata_push_rdi_568a10(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error>;
    fn metadata_sub_rsp_20_568a12(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn metadata_add_rsp_20_568a6b(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn metadata_pop_rdi_568a6f(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error>;
    fn metadata_ret_568a70(&mut self, parent: &mut super::super::Owners<Self>, out: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_metadata(&mut self, parent: &mut super::super::Owners<Self>, history: History, held: &mut Option<Retained>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> m::Services for Scope<S> {
    type Storage = ParentState<S>; type Error = S::Error;
    fn read_qword(&mut self, o: &mut m::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.metadata_read_qword(&mut o.storage.owners, pc, address)
    }
    fn save_rbx_568a1f(&mut self, o: &mut m::Owners<Self>, out: &mut Option<m::Store>) -> Result<(), Self::Error> {
        self.services.metadata_save_rbx_568a1f(&mut o.storage.owners, out)
    }
}
impl<S: Services> m::native_entry::Services for Scope<S> {
    fn push_rdi_568a10(&mut self, o: &mut m::Owners<Self>, out: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.metadata_push_rdi_568a10(&mut o.storage.owners, out)
    }
    fn sub_rsp_20_568a12(&mut self, o: &mut m::Owners<Self>, out: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.metadata_sub_rsp_20_568a12(&mut o.storage.owners, out)
    }
}
impl<S: Services> m::native_return::Services for Scope<S> {
    fn add_rsp_20_568a6b(&mut self, o: &mut m::Owners<Self>, out: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.metadata_add_rsp_20_568a6b(&mut o.storage.owners, out)
    }
    fn pop_rdi_568a6f(&mut self, o: &mut m::Owners<Self>, out: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.metadata_pop_rdi_568a6f(&mut o.storage.owners, out)
    }
    fn ret_568a70(&mut self, o: &mut m::Owners<Self>, out: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.metadata_ret_568a70(&mut o.storage.owners, out)
    }
}
