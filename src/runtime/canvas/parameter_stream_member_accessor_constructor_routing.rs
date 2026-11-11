use super::{Scope, LookupMemberDuplicateCopyServices};
use super::super::{self as f, constructor as c};
use c::{object_lookup as l, object_lookup_call};
use l::member_construction as m;
use m::duplicate as d;
use d::copy as w;
use w::{accessor as a, accessor_call as ac};
pub trait LookupMemberDuplicateCopyAccessorServices: LookupMemberDuplicateCopyServices {
    type ConstructorLookupMemberDuplicateCopyAccessorCallOwner;

    fn constructor_lookup_member_duplicate_copy_accessor_context(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::ConstructorLookupMemberDuplicateCopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_accessor_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::ConstructorLookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_accessor_add(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::ConstructorLookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_accessor_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::ConstructorLookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_constructor_lookup_member_duplicate_copy_accessor(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::ConstructorLookupMemberDuplicateCopyAccessorCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupMemberDuplicateCopyAccessorServices> object_lookup_call::MemberDuplicateCopyAccessorServices for Scope<S> {
    type LookupMemberDuplicateCopyAccessorCallOwner = S::ConstructorLookupMemberDuplicateCopyAccessorCallOwner;
    fn lookup_member_duplicate_copy_accessor_context(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyAccessorCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_accessor_context(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, call, request)
    }
    fn lookup_member_duplicate_copy_accessor_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_accessor_sub(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, call, returned)
    }
    fn lookup_member_duplicate_copy_accessor_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_accessor_add(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, call, returned)
    }
    fn lookup_member_duplicate_copy_accessor_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyAccessorCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_accessor_ret(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, call, returned)
    }
    fn adopt_completed_lookup_member_duplicate_copy_accessor(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::LookupMemberDuplicateCopyAccessorCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_constructor_lookup_member_duplicate_copy_accessor(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, history, held)
    }
}
