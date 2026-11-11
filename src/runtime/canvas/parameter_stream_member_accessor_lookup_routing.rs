use super::{Scope, MemberDuplicateCopyServices};
use super::super as c;
use c::object_lookup as l;
use l::member_construction as m;
use m::duplicate as d;
use d::copy as w;
use w::{accessor as a, accessor_call as ac};
pub trait MemberDuplicateCopyAccessorServices: MemberDuplicateCopyServices {
    type LookupMemberDuplicateCopyAccessorCallOwner;

    fn lookup_member_duplicate_copy_accessor_context(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_copy_accessor_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_copy_accessor_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_copy_accessor_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_lookup_member_duplicate_copy_accessor(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::LookupMemberDuplicateCopyAccessorCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: MemberDuplicateCopyAccessorServices> l::member_call::DuplicateCopyAccessorServices for Scope<S> {
    type MemberDuplicateCopyAccessorCallOwner = S::LookupMemberDuplicateCopyAccessorCallOwner;
    fn member_duplicate_copy_accessor_context(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_accessor_context(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, request)
    }
    fn member_duplicate_copy_accessor_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_accessor_sub(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, returned)
    }
    fn member_duplicate_copy_accessor_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_accessor_add(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, returned)
    }
    fn member_duplicate_copy_accessor_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_accessor_ret(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, returned)
    }
    fn adopt_completed_member_duplicate_copy_accessor(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::MemberDuplicateCopyAccessorCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_lookup_member_duplicate_copy_accessor(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, history, held)
    }
}
