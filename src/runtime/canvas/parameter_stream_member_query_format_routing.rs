use super::{Scope, LookupMemberServices, Resources, f, c, constructor_call};
use super::super::scope::resources;
use c::object_lookup as l;
use l::member_construction as m;
use m::{query as q, query_call as qc};
pub trait LookupMemberQueryServices: LookupMemberServices {
    type FormatConstructorLookupMemberQueryCallOwner;

    fn format_constructor_lookup_member_query_call(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_query_read_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn format_constructor_lookup_member_query_read_dword(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn format_constructor_lookup_member_query_write_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_query_sub(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_query_add(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_query_ret(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_format_constructor_lookup_member_query(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::FormatConstructorLookupMemberQueryCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupMemberQueryServices> constructor_call::LookupMemberQueryServices for Scope<S> {
    type ConstructorLookupMemberQueryCallOwner = S::FormatConstructorLookupMemberQueryCallOwner;
    fn constructor_lookup_member_query_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_query_call(resources(o, None, None, None), ctor, lookup, member, query, request)
    }
    fn constructor_lookup_member_query_read_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.format_constructor_lookup_member_query_read_word(resources(o, None, None, None), ctor, lookup, member, query, pc, address)
    }
    fn constructor_lookup_member_query_read_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.format_constructor_lookup_member_query_read_dword(resources(o, None, None, None), ctor, lookup, member, query, pc, address)
    }
    fn constructor_lookup_member_query_write_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_query_write_word(resources(o, None, None, None), ctor, lookup, member, query, pc, address, word)
    }
    fn constructor_lookup_member_query_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_query_sub(resources(o, None, None, None), ctor, lookup, member, query, pc, returned)
    }
    fn constructor_lookup_member_query_add(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_query_add(resources(o, None, None, None), ctor, lookup, member, query, pc, returned)
    }
    fn constructor_lookup_member_query_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::ConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_query_ret(resources(o, None, None, None), ctor, lookup, member, query, pc, returned)
    }
    fn adopt_completed_constructor_lookup_member_query(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::ConstructorLookupMemberQueryCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_format_constructor_lookup_member_query(resources(o, None, None, None), ctor, lookup, member, history, held)
    }
}
