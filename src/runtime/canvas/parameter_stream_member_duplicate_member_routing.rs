use super::{Scope, Services as MemberServices};
use super::super as l;
use l::member_construction as m;
use m::{duplicate as d, duplicate_call as dc};
pub trait DuplicateServices: MemberServices {
    type MemberDuplicateCallOwner;

    fn member_duplicate_call(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, request: d::CallRequest) -> Result<(), Self::Error>;
    fn member_duplicate_read_word16(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn member_duplicate_write_word(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn member_duplicate_save(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, pc: u32, register: d::native_entry::Register, returned: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error>;
    fn member_duplicate_push(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error>;
    fn member_duplicate_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn member_duplicate_restore(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, pc: u32, register: d::native_return::Register, returned: &mut Option<d::native_return::Load>) -> Result<(), Self::Error>;
    fn member_duplicate_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn member_duplicate_pop(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error>;
    fn member_duplicate_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, returned: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_member_duplicate(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, history: dc::History, held: &mut Option<dc::Retained<Self::MemberDuplicateCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: DuplicateServices> dc::Services for Scope<S> {
    type DuplicateCallOwner = S::MemberDuplicateCallOwner;
    fn duplicate_call(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, request: d::CallRequest) -> Result<(), Self::Error> {
        self.services.member_duplicate_call(&mut o.storage.owners, &mut o.calls, duplicate, request)
    }
    fn duplicate_read_word16(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error> {
        self.services.member_duplicate_read_word16(&mut o.storage.owners, &mut o.calls, duplicate, pc, address)
    }
    fn duplicate_write_word(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.member_duplicate_write_word(&mut o.storage.owners, &mut o.calls, duplicate, pc, address, word)
    }
    fn duplicate_save(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, pc: u32, register: d::native_entry::Register, returned: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.member_duplicate_save(&mut o.storage.owners, &mut o.calls, duplicate, pc, register, returned)
    }
    fn duplicate_push(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.member_duplicate_push(&mut o.storage.owners, &mut o.calls, duplicate, returned)
    }
    fn duplicate_sub(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.member_duplicate_sub(&mut o.storage.owners, &mut o.calls, duplicate, returned)
    }
    fn duplicate_restore(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, pc: u32, register: d::native_return::Register, returned: &mut Option<d::native_return::Load>) -> Result<(), Self::Error> {
        self.services.member_duplicate_restore(&mut o.storage.owners, &mut o.calls, duplicate, pc, register, returned)
    }
    fn duplicate_add(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.member_duplicate_add(&mut o.storage.owners, &mut o.calls, duplicate, returned)
    }
    fn duplicate_pop(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.member_duplicate_pop(&mut o.storage.owners, &mut o.calls, duplicate, returned)
    }
    fn duplicate_ret(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.member_duplicate_ret(&mut o.storage.owners, &mut o.calls, duplicate, returned)
    }
    fn adopt_completed_duplicate(&mut self, o: &mut m::Owners<Self>, history: dc::History, held: &mut Option<dc::Retained<Self::DuplicateCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_member_duplicate(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
