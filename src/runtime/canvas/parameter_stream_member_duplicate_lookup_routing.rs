use super::{Scope, MemberServices};
use super::super as c;
use c::object_lookup as l;
use l::member_construction as m;
use m::{duplicate as d, duplicate_call as dc};
pub trait MemberDuplicateServices: MemberServices {
    type LookupMemberDuplicateCallOwner;

    fn lookup_member_duplicate_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, request: d::CallRequest) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_read_word16(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn lookup_member_duplicate_write_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_save(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, pc: u32, register: d::native_entry::Register, returned: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_push(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, returned: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, returned: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_restore(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, pc: u32, register: d::native_return::Register, returned: &mut Option<d::native_return::Load>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, returned: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_pop(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, returned: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, returned: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_lookup_member_duplicate(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, history: dc::History, held: &mut Option<dc::Retained<Self::LookupMemberDuplicateCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: MemberDuplicateServices> l::member_call::DuplicateServices for Scope<S> {
    type MemberDuplicateCallOwner = S::LookupMemberDuplicateCallOwner;
    fn member_duplicate_call(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, request: d::CallRequest) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_call(&mut o.storage.owners, &mut o.calls, member, duplicate, request)
    }
    fn member_duplicate_read_word16(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error> {
        self.services.lookup_member_duplicate_read_word16(&mut o.storage.owners, &mut o.calls, member, duplicate, pc, address)
    }
    fn member_duplicate_write_word(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_write_word(&mut o.storage.owners, &mut o.calls, member, duplicate, pc, address, word)
    }
    fn member_duplicate_save(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, pc: u32, register: d::native_entry::Register, returned: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_save(&mut o.storage.owners, &mut o.calls, member, duplicate, pc, register, returned)
    }
    fn member_duplicate_push(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_push(&mut o.storage.owners, &mut o.calls, member, duplicate, returned)
    }
    fn member_duplicate_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_sub(&mut o.storage.owners, &mut o.calls, member, duplicate, returned)
    }
    fn member_duplicate_restore(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, pc: u32, register: d::native_return::Register, returned: &mut Option<d::native_return::Load>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_restore(&mut o.storage.owners, &mut o.calls, member, duplicate, pc, register, returned)
    }
    fn member_duplicate_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_add(&mut o.storage.owners, &mut o.calls, member, duplicate, returned)
    }
    fn member_duplicate_pop(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_pop(&mut o.storage.owners, &mut o.calls, member, duplicate, returned)
    }
    fn member_duplicate_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_ret(&mut o.storage.owners, &mut o.calls, member, duplicate, returned)
    }
    fn adopt_completed_member_duplicate(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, history: dc::History, held: &mut Option<dc::Retained<Self::MemberDuplicateCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_lookup_member_duplicate(&mut o.storage.owners, &mut o.calls, member, history, held)
    }
}
