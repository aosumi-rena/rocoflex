use super::{Scope, FormatLookupServices, Resources, resources, c, format_call};
use c::object_lookup as l;
use l::member_construction as m;
pub trait FormatLookupMemberServices: FormatLookupServices {
    type NumberFormatConstructorLookupMemberCallOwner;

    fn number_format_constructor_lookup_member_call(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_read_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn number_format_constructor_lookup_member_write_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_write_dword(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_read_vector(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error>;
    fn number_format_constructor_lookup_member_write_vector(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_and_zero(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_push(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_sub(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_move_rax(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_add(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_pop(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error>;
    fn number_format_constructor_lookup_member_ret(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, member: &mut m::Calls<Self::NumberFormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn number_adopt_completed_format_constructor_lookup_member(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, lookup: &mut l::Calls<Self::NumberFormatLookupCallOwner>, history: l::member_call::History, held: &mut Option<l::member_call::Retained<Self::NumberFormatConstructorLookupMemberCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: FormatLookupMemberServices> format_call::LookupMemberServices for Scope<S> {
    type FormatConstructorLookupMemberCallOwner = S::NumberFormatConstructorLookupMemberCallOwner;
    fn format_constructor_lookup_member_call(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_call(resources(r), ctor, lookup, member, request)
    }
    fn format_constructor_lookup_member_read_word(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.number_format_constructor_lookup_member_read_word(resources(r), ctor, lookup, member, pc, address)
    }
    fn format_constructor_lookup_member_write_word(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_write_word(resources(r), ctor, lookup, member, pc, address, word)
    }
    fn format_constructor_lookup_member_write_dword(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_write_dword(resources(r), ctor, lookup, member, pc, address, word)
    }
    fn format_constructor_lookup_member_read_vector(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error> {
        self.services.number_format_constructor_lookup_member_read_vector(resources(r), ctor, lookup, member, pc, address)
    }
    fn format_constructor_lookup_member_write_vector(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_write_vector(resources(r), ctor, lookup, member, pc, address, bits)
    }
    fn format_constructor_lookup_member_and_zero(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_and_zero(resources(r), ctor, lookup, member, pc, address, returned)
    }
    fn format_constructor_lookup_member_push(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_push(resources(r), ctor, lookup, member, returned)
    }
    fn format_constructor_lookup_member_sub(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_sub(resources(r), ctor, lookup, member, returned)
    }
    fn format_constructor_lookup_member_move_rax(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_move_rax(resources(r), ctor, lookup, member, receiver, returned)
    }
    fn format_constructor_lookup_member_add(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_add(resources(r), ctor, lookup, member, returned)
    }
    fn format_constructor_lookup_member_pop(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_pop(resources(r), ctor, lookup, member, returned)
    }
    fn format_constructor_lookup_member_ret(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.number_format_constructor_lookup_member_ret(resources(r), ctor, lookup, member, returned)
    }
    fn adopt_completed_format_constructor_lookup_member(&mut self, r: format_call::Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, history: l::member_call::History, held: &mut Option<l::member_call::Retained<Self::FormatConstructorLookupMemberCallOwner>>) -> Result<(), Self::Error> {
        self.services.number_adopt_completed_format_constructor_lookup_member(resources(r), ctor, lookup, history, held)
    }
}
