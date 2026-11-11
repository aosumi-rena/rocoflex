use super::{Scope, LookupServices};
use super::super::{self as f, constructor as c};
use c::{object_lookup as l, object_lookup_call};
use l::member_construction as m;
pub trait LookupMemberServices: LookupServices {
    type ConstructorLookupMemberCallOwner;

    fn constructor_lookup_member_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error>;
    fn constructor_lookup_member_read_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn constructor_lookup_member_write_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn constructor_lookup_member_write_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn constructor_lookup_member_read_vector(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error>;
    fn constructor_lookup_member_write_vector(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error>;
    fn constructor_lookup_member_and_zero(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_push(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_move_rax(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_add(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_pop(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn adopt_completed_constructor_lookup_member(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, history: l::member_call::History, held: &mut Option<l::member_call::Retained<Self::ConstructorLookupMemberCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupMemberServices> object_lookup_call::MemberServices for Scope<S> {
    type LookupMemberCallOwner = S::ConstructorLookupMemberCallOwner;
    fn lookup_member_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_call(&mut o.storage.owners, &mut o.calls, lookup, member, request)
    }
    fn lookup_member_read_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.constructor_lookup_member_read_word(&mut o.storage.owners, &mut o.calls, lookup, member, pc, address)
    }
    fn lookup_member_write_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_write_word(&mut o.storage.owners, &mut o.calls, lookup, member, pc, address, word)
    }
    fn lookup_member_write_dword(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_write_dword(&mut o.storage.owners, &mut o.calls, lookup, member, pc, address, word)
    }
    fn lookup_member_read_vector(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error> {
        self.services.constructor_lookup_member_read_vector(&mut o.storage.owners, &mut o.calls, lookup, member, pc, address)
    }
    fn lookup_member_write_vector(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_write_vector(&mut o.storage.owners, &mut o.calls, lookup, member, pc, address, bits)
    }
    fn lookup_member_and_zero(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_and_zero(&mut o.storage.owners, &mut o.calls, lookup, member, pc, address, returned)
    }
    fn lookup_member_push(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_push(&mut o.storage.owners, &mut o.calls, lookup, member, returned)
    }
    fn lookup_member_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_sub(&mut o.storage.owners, &mut o.calls, lookup, member, returned)
    }
    fn lookup_member_move_rax(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_move_rax(&mut o.storage.owners, &mut o.calls, lookup, member, receiver, returned)
    }
    fn lookup_member_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_add(&mut o.storage.owners, &mut o.calls, lookup, member, returned)
    }
    fn lookup_member_pop(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_pop(&mut o.storage.owners, &mut o.calls, lookup, member, returned)
    }
    fn lookup_member_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_ret(&mut o.storage.owners, &mut o.calls, lookup, member, returned)
    }
    fn adopt_completed_lookup_member(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, history: l::member_call::History, held: &mut Option<l::member_call::Retained<Self::LookupMemberCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_constructor_lookup_member(&mut o.storage.owners, &mut o.calls, lookup, history, held)
    }
}
