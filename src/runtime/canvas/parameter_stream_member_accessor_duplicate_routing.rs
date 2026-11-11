use super::{Scope, CopyServices};
use super::super as m;
use m::duplicate as d;
use d::copy as w;
use w::{accessor as a, accessor_call as ac};
pub trait CopyAccessorServices: CopyServices {
    type DuplicateCopyAccessorCallOwner;

    fn duplicate_copy_accessor_context(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn duplicate_copy_accessor_sub(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_copy_accessor_add(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_copy_accessor_ret(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate_copy_accessor(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::DuplicateCopyAccessorCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: CopyAccessorServices> d::copy_call::AccessorServices for Scope<S> {
    type CopyAccessorCallOwner = S::DuplicateCopyAccessorCallOwner;
    fn copy_accessor_context(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.duplicate_copy_accessor_context(&mut o.storage.owners, &mut o.calls, copy, call, request)
    }
    fn copy_accessor_sub(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_accessor_sub(&mut o.storage.owners, &mut o.calls, copy, call, returned)
    }
    fn copy_accessor_add(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_accessor_add(&mut o.storage.owners, &mut o.calls, copy, call, returned)
    }
    fn copy_accessor_ret(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_accessor_ret(&mut o.storage.owners, &mut o.calls, copy, call, returned)
    }
    fn adopt_completed_copy_accessor(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::CopyAccessorCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_duplicate_copy_accessor(&mut o.storage.owners, &mut o.calls, copy, history, held)
    }
}
