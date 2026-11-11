use super::{Scope, DuplicateCopyServices};
use super::super as l;
use l::member_construction as m;
use m::duplicate as d;
use d::copy as w;
use w::{accessor as a, accessor_call as ac};
pub trait DuplicateCopyAccessorServices: DuplicateCopyServices {
    type MemberDuplicateCopyAccessorCallOwner;

    fn member_duplicate_copy_accessor_context(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn member_duplicate_copy_accessor_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_accessor_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_accessor_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_member_duplicate_copy_accessor(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::MemberDuplicateCopyAccessorCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: DuplicateCopyAccessorServices> m::duplicate_call::CopyAccessorServices for Scope<S> {
    type DuplicateCopyAccessorCallOwner = S::MemberDuplicateCopyAccessorCallOwner;
    fn duplicate_copy_accessor_context(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_accessor_context(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, request)
    }
    fn duplicate_copy_accessor_sub(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_accessor_sub(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, returned)
    }
    fn duplicate_copy_accessor_add(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_accessor_add(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, returned)
    }
    fn duplicate_copy_accessor_ret(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_accessor_ret(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, returned)
    }
    fn adopt_completed_duplicate_copy_accessor(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::DuplicateCopyAccessorCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_member_duplicate_copy_accessor(&mut o.storage.owners, &mut o.calls, duplicate, copy, history, held)
    }
}
