use super::{a, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type AccessorCallOwner;

    fn accessor_context(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::AccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn accessor_sub(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::AccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn accessor_add(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::AccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn accessor_ret(&mut self, o: &mut super::super::Owners<Self>, call: &mut Option<a::CallReturned<Self::AccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_accessor(&mut self, o: &mut super::super::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::AccessorCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> a::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::AccessorCallOwner; type Error = S::Error;
    fn call_context(&mut self, o: &mut a::Owners<Self>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.accessor_context(&mut o.storage.owners, &mut o.call, request)
    }
}
impl<S: Services> a::native_entry::Services for Scope<S> {
    fn sub_rsp_28_578eb0(&mut self, o: &mut a::Owners<Self>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.accessor_sub(&mut o.storage.owners, &mut o.call, returned)
    }
}
impl<S: Services> a::native_return::Services for Scope<S> {
    fn add_rsp_28_578ecb(&mut self, o: &mut a::Owners<Self>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.accessor_add(&mut o.storage.owners, &mut o.call, returned)
    }
    fn ret_578ecf(&mut self, o: &mut a::Owners<Self>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.accessor_ret(&mut o.storage.owners, &mut o.call, returned)
    }
}
