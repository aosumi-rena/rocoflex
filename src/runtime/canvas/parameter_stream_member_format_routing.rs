use super::{Scope, LookupServices, Resources, f, c, constructor_call};
use super::super::scope::resources;
use c::object_lookup as l;
use l::member_construction as m;
pub trait LookupMemberServices: LookupServices {
    type FormatConstructorLookupMemberCallOwner;

    fn format_constructor_lookup_member_call(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_read_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn format_constructor_lookup_member_write_word(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_write_dword(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_read_vector(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error>;
    fn format_constructor_lookup_member_write_vector(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_and_zero(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_push(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_sub(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_move_rax(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_add(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_pop(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error>;
    fn format_constructor_lookup_member_ret(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, member: &mut m::Calls<Self::FormatConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn adopt_completed_format_constructor_lookup_member(&mut self, r: Resources<'_, Self>, ctor: &mut c::Calls<Self::FormatConstructorCallOwner>, lookup: &mut l::Calls<Self::FormatLookupCallOwner>, history: l::member_call::History, held: &mut Option<l::member_call::Retained<Self::FormatConstructorLookupMemberCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: LookupMemberServices> constructor_call::LookupMemberServices for Scope<S> {
    type ConstructorLookupMemberCallOwner = S::FormatConstructorLookupMemberCallOwner;
    fn constructor_lookup_member_call(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_call(resources(o, None, None, None), ctor, lookup, member, request)
    }
    fn constructor_lookup_member_read_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.format_constructor_lookup_member_read_word(resources(o, None, None, None), ctor, lookup, member, pc, address)
    }
    fn constructor_lookup_member_write_word(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_write_word(resources(o, None, None, None), ctor, lookup, member, pc, address, word)
    }
    fn constructor_lookup_member_write_dword(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_write_dword(resources(o, None, None, None), ctor, lookup, member, pc, address, word)
    }
    fn constructor_lookup_member_read_vector(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error> {
        self.services.format_constructor_lookup_member_read_vector(resources(o, None, None, None), ctor, lookup, member, pc, address)
    }
    fn constructor_lookup_member_write_vector(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_write_vector(resources(o, None, None, None), ctor, lookup, member, pc, address, bits)
    }
    fn constructor_lookup_member_and_zero(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_and_zero(resources(o, None, None, None), ctor, lookup, member, pc, address, returned)
    }
    fn constructor_lookup_member_push(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_push(resources(o, None, None, None), ctor, lookup, member, returned)
    }
    fn constructor_lookup_member_sub(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_sub(resources(o, None, None, None), ctor, lookup, member, returned)
    }
    fn constructor_lookup_member_move_rax(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_move_rax(resources(o, None, None, None), ctor, lookup, member, receiver, returned)
    }
    fn constructor_lookup_member_add(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_add(resources(o, None, None, None), ctor, lookup, member, returned)
    }
    fn constructor_lookup_member_pop(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_pop(resources(o, None, None, None), ctor, lookup, member, returned)
    }
    fn constructor_lookup_member_ret(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, member: &mut m::Calls<Self::ConstructorLookupMemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.format_constructor_lookup_member_ret(resources(o, None, None, None), ctor, lookup, member, returned)
    }
    fn adopt_completed_constructor_lookup_member(&mut self, o: &mut f::Owners<Self>, ctor: &mut c::Calls<Self::ConstructorCallOwner>, lookup: &mut l::Calls<Self::ConstructorLookupCallOwner>, history: l::member_call::History, held: &mut Option<l::member_call::Retained<Self::ConstructorLookupMemberCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_format_constructor_lookup_member(resources(o, None, None, None), ctor, lookup, history, held)
    }
}
