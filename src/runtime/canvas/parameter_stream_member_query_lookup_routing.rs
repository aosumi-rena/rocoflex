use super::{Scope, MemberServices};
use super::super as c;
use c::object_lookup as l;
use l::member_construction as m;
use m::{query as q, query_call as qc};
pub trait MemberQueryServices: MemberServices {
    type LookupMemberQueryCallOwner;

    fn lookup_member_query_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error>;
    fn lookup_member_query_read_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn lookup_member_query_read_dword(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn lookup_member_query_write_word(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn lookup_member_query_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn lookup_member_query_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_member_query_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, query: &mut q::Calls<Self::LookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_lookup_member_query(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::LookupMemberQueryCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: MemberQueryServices> l::member_call::QueryServices for Scope<S> {
    type MemberQueryCallOwner = S::LookupMemberQueryCallOwner;
    fn member_query_call(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error> {
        self.services.lookup_member_query_call(&mut o.storage.owners, &mut o.calls, member, query, request)
    }
    fn member_query_read_word(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.lookup_member_query_read_word(&mut o.storage.owners, &mut o.calls, member, query, pc, address)
    }
    fn member_query_read_dword(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.lookup_member_query_read_dword(&mut o.storage.owners, &mut o.calls, member, query, pc, address)
    }
    fn member_query_write_word(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.lookup_member_query_write_word(&mut o.storage.owners, &mut o.calls, member, query, pc, address, word)
    }
    fn member_query_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.lookup_member_query_sub(&mut o.storage.owners, &mut o.calls, member, query, pc, returned)
    }
    fn member_query_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_member_query_add(&mut o.storage.owners, &mut o.calls, member, query, pc, returned)
    }
    fn member_query_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.lookup_member_query_ret(&mut o.storage.owners, &mut o.calls, member, query, pc, returned)
    }
    fn adopt_completed_member_query(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::MemberQueryCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_lookup_member_query(&mut o.storage.owners, &mut o.calls, member, history, held)
    }
}
