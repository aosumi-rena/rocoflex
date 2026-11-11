use super::{Scope, LookupMemberDuplicateServices};
use super::super::{self as f, constructor as c};
use c::{object_lookup as l, object_lookup_call};
use l::member_construction as m;
use m::duplicate as d;
use d::{copy as w, copy_call as wc};
pub trait LookupMemberDuplicateCopyServices: LookupMemberDuplicateServices {
    type ConstructorLookupMemberDuplicateCopyCallOwner;

    fn constructor_lookup_member_duplicate_copy_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, request: w::CallRequest) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_read_word16(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn constructor_lookup_member_duplicate_copy_write_word16(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_write_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_push(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Push>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_add(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_pop(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::Pop>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_duplicate_copy_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::ConstructorLookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_constructor_lookup_member_duplicate_copy(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, duplicate: &mut d::Calls<Self::ConstructorLookupMemberDuplicateCallOwner>, history: wc::History, held: &mut Option<wc::Retained<Self::ConstructorLookupMemberDuplicateCopyCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupMemberDuplicateCopyServices> object_lookup_call::MemberDuplicateCopyServices for Scope<S> {
    type LookupMemberDuplicateCopyCallOwner = S::ConstructorLookupMemberDuplicateCopyCallOwner;
    fn lookup_member_duplicate_copy_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, request: w::CallRequest) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_call(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, request)
    }
    fn lookup_member_duplicate_copy_read_word16(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_read_word16(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, pc, address)
    }
    fn lookup_member_duplicate_copy_write_word16(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_write_word16(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, pc, address, word)
    }
    fn lookup_member_duplicate_copy_write_dword(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_write_dword(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, pc, address, word)
    }
    fn lookup_member_duplicate_copy_push(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_push(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, returned)
    }
    fn lookup_member_duplicate_copy_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_sub(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, returned)
    }
    fn lookup_member_duplicate_copy_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_add(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, returned)
    }
    fn lookup_member_duplicate_copy_pop(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_pop(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, returned)
    }
    fn lookup_member_duplicate_copy_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_duplicate_copy_ret(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, copy, returned)
    }
    fn adopt_completed_lookup_member_duplicate_copy(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, history: wc::History, held: &mut Option<wc::Retained<Self::LookupMemberDuplicateCopyCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_constructor_lookup_member_duplicate_copy(&mut o.storage.owners, &mut o.calls, lookup, member, duplicate, history, held)
    }
}
