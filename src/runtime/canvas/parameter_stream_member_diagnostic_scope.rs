use super::{a, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type DiagnosticCallOwner;

    fn diagnostic_and(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error>;
    fn diagnostic_dispatch(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn diagnostic_sub(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn diagnostic_add(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn diagnostic_ret(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_diagnostic(&mut self, o: &mut super::super::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::DiagnosticCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> a::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::DiagnosticCallOwner; type Error = S::Error;
    fn and_stack_qword_zero(&mut self, o: &mut a::Owners<Self>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error> {
        self.services.diagnostic_and(&mut o.storage.owners, &mut o.call, pc, address, returned)
    }
    fn call_dispatch(&mut self, o: &mut a::Owners<Self>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.diagnostic_dispatch(&mut o.storage.owners, &mut o.call, request)
    }
}
impl<S: Services> a::native_entry::Services for Scope<S> {
    fn sub_rsp_38_57a004(&mut self, o: &mut a::Owners<Self>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.diagnostic_sub(&mut o.storage.owners, &mut o.call, returned)
    }
}
impl<S: Services> a::native_return::Services for Scope<S> {
    fn add_rsp_38_57a01d(&mut self, o: &mut a::Owners<Self>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.diagnostic_add(&mut o.storage.owners, &mut o.call, returned)
    }
    fn ret_57a021(&mut self, o: &mut a::Owners<Self>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.diagnostic_ret(&mut o.storage.owners, &mut o.call, returned)
    }
}
