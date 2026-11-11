use super::{Scope, DuplicateServices};
use super::super as l;
use l::member_construction as m;
use m::duplicate as d;
use d::{copy as w, copy_call as wc};
pub trait DuplicateCopyServices: DuplicateServices {
    type MemberDuplicateCopyCallOwner;

    fn member_duplicate_copy_call(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, request: w::CallRequest) -> Result<(), Self::Error>;
    fn member_duplicate_copy_read_word16(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn member_duplicate_copy_write_word16(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error>;
    fn member_duplicate_copy_write_dword(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn member_duplicate_copy_push(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Push>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_pop(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::Pop>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, returned: &mut Option<w::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_member_duplicate_copy(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, history: wc::History, held: &mut Option<wc::Retained<Self::MemberDuplicateCopyCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: DuplicateCopyServices> m::duplicate_call::CopyServices for Scope<S> {
    type DuplicateCopyCallOwner = S::MemberDuplicateCopyCallOwner;
    fn duplicate_copy_call(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, request: w::CallRequest) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_call(&mut o.storage.owners, &mut o.calls, duplicate, copy, request)
    }
    fn duplicate_copy_read_word16(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error> {
        self.services.member_duplicate_copy_read_word16(&mut o.storage.owners, &mut o.calls, duplicate, copy, pc, address)
    }
    fn duplicate_copy_write_word16(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_write_word16(&mut o.storage.owners, &mut o.calls, duplicate, copy, pc, address, word)
    }
    fn duplicate_copy_write_dword(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_write_dword(&mut o.storage.owners, &mut o.calls, duplicate, copy, pc, address, word)
    }
    fn duplicate_copy_push(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_push(&mut o.storage.owners, &mut o.calls, duplicate, copy, returned)
    }
    fn duplicate_copy_sub(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_sub(&mut o.storage.owners, &mut o.calls, duplicate, copy, returned)
    }
    fn duplicate_copy_add(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_add(&mut o.storage.owners, &mut o.calls, duplicate, copy, returned)
    }
    fn duplicate_copy_pop(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_pop(&mut o.storage.owners, &mut o.calls, duplicate, copy, returned)
    }
    fn duplicate_copy_ret(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_ret(&mut o.storage.owners, &mut o.calls, duplicate, copy, returned)
    }
    fn adopt_completed_duplicate_copy(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, history: wc::History, held: &mut Option<wc::Retained<Self::DuplicateCopyCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_member_duplicate_copy(&mut o.storage.owners, &mut o.calls, duplicate, history, held)
    }
}
