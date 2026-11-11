use super::{Scope, Services as CopyServices};
use super::super as d;
use d::copy as w;
use w::{accessor as a, accessor_call as ac};
pub trait AccessorServices: CopyServices {
    type CopyAccessorCallOwner;

    fn copy_accessor_context(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn copy_accessor_sub(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn copy_accessor_add(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn copy_accessor_ret(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_copy_accessor(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::CopyAccessorCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: AccessorServices> ac::Services for Scope<S> {
    type AccessorCallOwner = S::CopyAccessorCallOwner;
    fn accessor_context(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::AccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.copy_accessor_context(&mut o.storage.owners, &mut o.calls, call, request)
    }
    fn accessor_sub(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::AccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.copy_accessor_sub(&mut o.storage.owners, &mut o.calls, call, returned)
    }
    fn accessor_add(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::AccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.copy_accessor_add(&mut o.storage.owners, &mut o.calls, call, returned)
    }
    fn accessor_ret(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::AccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.copy_accessor_ret(&mut o.storage.owners, &mut o.calls, call, returned)
    }
    fn adopt_completed_accessor(&mut self, o: &mut w::Owners<Self>, history: ac::History, held: &mut Option<ac::Retained<Self::AccessorCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_copy_accessor(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
