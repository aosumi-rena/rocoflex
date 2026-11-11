use super::{Scope, LookupMemberServices};
use super::super::{self as f, constructor as c};
use c::{object_lookup as l, object_lookup_call};
use l::member_construction as m;
use m::{query as q, query_call as qc};
pub trait LookupMemberQueryServices: LookupMemberServices {
    type ConstructorLookupMemberQueryCallOwner;

    fn constructor_lookup_member_query_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error>;
    fn constructor_lookup_member_query_read_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn constructor_lookup_member_query_read_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn constructor_lookup_member_query_write_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn constructor_lookup_member_query_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_query_add(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn constructor_lookup_member_query_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_constructor_lookup_member_query(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::ConstructorLookupMemberQueryCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupMemberQueryServices> object_lookup_call::MemberQueryServices for Scope<S> {
    type LookupMemberQueryCallOwner = S::ConstructorLookupMemberQueryCallOwner;
    fn lookup_member_query_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_query_call(&mut o.storage.owners, &mut o.calls, lookup, member, query, request)
    }
    fn lookup_member_query_read_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.constructor_lookup_member_query_read_word(&mut o.storage.owners, &mut o.calls, lookup, member, query, pc, address)
    }
    fn lookup_member_query_read_dword(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.constructor_lookup_member_query_read_dword(&mut o.storage.owners, &mut o.calls, lookup, member, query, pc, address)
    }
    fn lookup_member_query_write_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_query_write_word(&mut o.storage.owners, &mut o.calls, lookup, member, query, pc, address, word)
    }
    fn lookup_member_query_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_query_sub(&mut o.storage.owners, &mut o.calls, lookup, member, query, pc, returned)
    }
    fn lookup_member_query_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_query_add(&mut o.storage.owners, &mut o.calls, lookup, member, query, pc, returned)
    }
    fn lookup_member_query_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.constructor_lookup_member_query_ret(&mut o.storage.owners, &mut o.calls, lookup, member, query, pc, returned)
    }
    fn adopt_completed_lookup_member_query(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::LookupMemberQueryCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_constructor_lookup_member_query(&mut o.storage.owners, &mut o.calls, lookup, member, history, held)
    }
}
