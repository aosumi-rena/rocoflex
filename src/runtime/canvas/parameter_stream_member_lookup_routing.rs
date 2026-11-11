use super::{Scope, Services as LookupServices};
use super::super as c;
use c::object_lookup as l;
use l::member_construction as m;
pub trait MemberServices: LookupServices {
    type LookupMemberCallOwner;

    fn lookup_member_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error>;
    fn lookup_member_read_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn lookup_member_write_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn lookup_member_write_dword(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn lookup_member_read_vector(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error>;
    fn lookup_member_write_vector(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error>;
    fn lookup_member_and_zero(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error>;
    fn lookup_member_push(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error>;
    fn lookup_member_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn lookup_member_move_rax(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn lookup_member_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_member_pop(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error>;
    fn lookup_member_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn adopt_completed_lookup_member(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, history: l::member_call::History, held: &mut Option<l::member_call::Retained<Self::LookupMemberCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: MemberServices> l::member_call::Services for Scope<S> {
    type MemberCallOwner = S::LookupMemberCallOwner;
    fn member_call(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error> {
        self.services.lookup_member_call(&mut o.storage.owners, &mut o.calls, calls, request)
    }
    fn member_read_word(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.lookup_member_read_word(&mut o.storage.owners, &mut o.calls, calls, pc, address)
    }
    fn member_write_word(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.lookup_member_write_word(&mut o.storage.owners, &mut o.calls, calls, pc, address, word)
    }
    fn member_write_dword(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.lookup_member_write_dword(&mut o.storage.owners, &mut o.calls, calls, pc, address, word)
    }
    fn member_read_vector(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error> {
        self.services.lookup_member_read_vector(&mut o.storage.owners, &mut o.calls, calls, pc, address)
    }
    fn member_write_vector(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error> {
        self.services.lookup_member_write_vector(&mut o.storage.owners, &mut o.calls, calls, pc, address, bits)
    }
    fn member_and_zero(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error> {
        self.services.lookup_member_and_zero(&mut o.storage.owners, &mut o.calls, calls, pc, address, returned)
    }
    fn member_push(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.lookup_member_push(&mut o.storage.owners, &mut o.calls, calls, returned)
    }
    fn member_sub(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.lookup_member_sub(&mut o.storage.owners, &mut o.calls, calls, returned)
    }
    fn member_move_rax(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.lookup_member_move_rax(&mut o.storage.owners, &mut o.calls, calls, receiver, returned)
    }
    fn member_add(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_member_add(&mut o.storage.owners, &mut o.calls, calls, returned)
    }
    fn member_pop(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.lookup_member_pop(&mut o.storage.owners, &mut o.calls, calls, returned)
    }
    fn member_ret(&mut self, o: &mut l::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.lookup_member_ret(&mut o.storage.owners, &mut o.calls, calls, returned)
    }
    fn adopt_completed_member(&mut self, o: &mut l::Owners<Self>, history: l::member_call::History, held: &mut Option<l::member_call::Retained<Self::MemberCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_lookup_member(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
