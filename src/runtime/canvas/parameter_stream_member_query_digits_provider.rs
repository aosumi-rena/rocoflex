use super::{Scope, FormatLookupMemberServices, Resources, resources, c, format_call};
use c::object_lookup as l;
use l::member_construction as m;
use m::{query as q, query_call as qc};
pub trait FormatLookupMemberQueryServices: FormatLookupMemberServices {
    type NumberFormatConstructorLookupMemberQueryCallOwner;

    fn number_format_constructor_lookup_member_query_call(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::NumberFormatConstructorLookupMemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_query_read_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::NumberFormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn number_format_constructor_lookup_member_query_read_dword(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::NumberFormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn number_format_constructor_lookup_member_query_write_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::NumberFormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_query_sub(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::NumberFormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_query_add(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::NumberFormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_query_ret(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::NumberFormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_number_format_constructor_lookup_member_query(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::NumberFormatConstructorLookupMemberQueryCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: FormatLookupMemberQueryServices> format_call::LookupMemberQueryServices for Scope<S> {
    type FormatConstructorLookupMemberQueryCallOwner = S::NumberFormatConstructorLookupMemberQueryCallOwner;
    fn format_constructor_lookup_member_query_call(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, request: q::CallRequest) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_query_call(resources(r), ctor, lookup, member, query, request)
    }
    fn format_constructor_lookup_member_query_read_word(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.number_format_constructor_lookup_member_query_read_word(resources(r), ctor, lookup, member, query, pc, address)
    }
    fn format_constructor_lookup_member_query_read_dword(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.number_format_constructor_lookup_member_query_read_dword(resources(r), ctor, lookup, member, query, pc, address)
    }
    fn format_constructor_lookup_member_query_write_word(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_query_write_word(resources(r), ctor, lookup, member, query, pc, address, word)
    }
    fn format_constructor_lookup_member_query_sub(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_query_sub(resources(r), ctor, lookup, member, query, pc, returned)
    }
    fn format_constructor_lookup_member_query_add(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_query_add(resources(r), ctor, lookup, member, query, pc, returned)
    }
    fn format_constructor_lookup_member_query_ret(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, query: &mut q::Calls<Self::FormatConstructorLookupMemberQueryCallOwner>, pc: u32, returned: &mut Option<q::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_query_ret(resources(r), ctor, lookup, member, query, pc, returned)
    }
    fn adopt_completed_format_constructor_lookup_member_query(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, history: qc::History, held: &mut Option<qc::Retained<Self::FormatConstructorLookupMemberQueryCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_number_format_constructor_lookup_member_query(resources(r), ctor, lookup, member, history, held)
    }
}
