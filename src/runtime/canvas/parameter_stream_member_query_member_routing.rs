use super::{Scope, Services as MemberServices};
use super::super as l;
use l::member_construction as m;
use m::{query as q, query_call as qc};
pub trait QueryServices: MemberServices {
    type MemberQueryCallOwner;

    fn member_query_call(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error>;
    fn member_query_read_word(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn member_query_read_dword(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn member_query_write_word(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn member_query_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn member_query_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn member_query_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, query: &mut q::Calls<Self::MemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_member_query(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::MemberQueryCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: QueryServices> qc::Services for Scope<S> {
    type QueryCallOwner = S::MemberQueryCallOwner;
    fn query_call(&mut self, o: &mut m::Owners<Self>, query: &mut q::Calls<Self::QueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error> {
        self.services.member_query_call(&mut o.storage.owners, &mut o.calls, query, request)
    }
    fn query_read_word(&mut self, o: &mut m::Owners<Self>, query: &mut q::Calls<Self::QueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.member_query_read_word(&mut o.storage.owners, &mut o.calls, query, pc, address)
    }
    fn query_read_dword(&mut self, o: &mut m::Owners<Self>, query: &mut q::Calls<Self::QueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.member_query_read_dword(&mut o.storage.owners, &mut o.calls, query, pc, address)
    }
    fn query_write_word(&mut self, o: &mut m::Owners<Self>, query: &mut q::Calls<Self::QueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.member_query_write_word(&mut o.storage.owners, &mut o.calls, query, pc, address, word)
    }
    fn query_sub(&mut self, o: &mut m::Owners<Self>, query: &mut q::Calls<Self::QueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.member_query_sub(&mut o.storage.owners, &mut o.calls, query, pc, returned)
    }
    fn query_add(&mut self, o: &mut m::Owners<Self>, query: &mut q::Calls<Self::QueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.member_query_add(&mut o.storage.owners, &mut o.calls, query, pc, returned)
    }
    fn query_ret(&mut self, o: &mut m::Owners<Self>, query: &mut q::Calls<Self::QueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.member_query_ret(&mut o.storage.owners, &mut o.calls, query, pc, returned)
    }
    fn adopt_completed_query(&mut self, o: &mut m::Owners<Self>, history: qc::History, held: &mut Option<qc::Retained<Self::QueryCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_member_query(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
