use super::{number, Calls, Scope};
use super::super as digits;
use number::{format_call, formatting as f};
use f::{sentry, facet, finish, native_entry as entry};
use finish::native_return as ret;
#[path = "parameter_number_format_constructor_provider.rs"]
mod constructor;
pub use constructor::FormatLookupMemberQueryServices;
pub use constructor::FormatLookupMemberDuplicateServices;
pub use constructor::FormatLookupMemberDuplicateCopyServices;
pub use constructor::FormatLookupMemberDuplicateCopyAccessorServices;
pub use constructor::FormatLookupMemberDuplicateCopyDiagnosticServices;
pub use constructor::FormatLookupMemberDuplicateCopyDiagnosticDispatchServices;
pub use constructor::{FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerServices, FormatLookupMemberDuplicateCopyDiagnosticDispatchMetadataServices};
pub use constructor::{FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerContextServices, FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerEncodedSlotServices};
pub use constructor::{FormatConstructorServices, FormatLookupServices, FormatLookupFallbackServices, FormatLookupRegistrationServices, FormatLookupMemberServices, FormatLookupGuardServices, FormatLookupWrapperServices, FormatLookupInitializationServices, FormatLookupResolverServices, FormatLookupResolverEntryServices, FormatLookupResolverProtectedServices, FormatLookupResolverResolutionServices, FormatLookupResolverLoadServices, FormatLookupResolverPriorCleanupServices};
pub struct Resources<'a, S: FormatServices> {
    pub formatter: &'a mut digits::Owners<S>, pub number: &'a mut Calls<S::NumberCallOwner>,
    pub constructor: &'a mut Option<f::ConstructorReturned<S::NumberFormatConstructorOwner>>,
    pub sentry: Option<&'a mut sentry::Calls<S::NumberFormatSentryOwner>>,
    pub facet: Option<&'a mut facet::Calls<S::NumberFormatFacetOwner>>,
    pub finish: Option<&'a mut finish::Calls<S::NumberFormatFinishOwner>>,
}
pub trait FormatServices: super::Services {
    type NumberFormatConstructorOwner; type NumberFormatSentryOwner; type NumberFormatFacetOwner; type NumberFormatFinishOwner;

    fn number_format_read_word(&mut self, r: Resources<'_, Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn number_format_read_dword(&mut self, r: Resources<'_, Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn number_format_read_byte(&mut self, r: Resources<'_, Self>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn number_format_read_128(&mut self, r: Resources<'_, Self>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error>;
    fn number_format_write_word(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn number_format_write_dword(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn number_format_write_byte(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
    fn number_format_write_128(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;
    fn number_format_zero_xmm0(&mut self, r: Resources<'_, Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn number_format_or_dword(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, mask: u32, returned: &mut Option<f::OrReturned>) -> Result<(), Self::Error>;
    fn number_format_construct(&mut self, r: Resources<'_, Self>, request: f::ConstructorRequest) -> Result<(), Self::Error>;
    fn number_format_sentry_call(&mut self, r: Resources<'_, Self>, request: sentry::CallRequest) -> Result<(), Self::Error>;
    fn number_format_facet_call(&mut self, r: Resources<'_, Self>, request: facet::CallRequest) -> Result<(), Self::Error>;
    fn number_format_finish_call(&mut self, r: Resources<'_, Self>, request: finish::CallRequest) -> Result<(), Self::Error>;
    fn number_format_restore_xmm6(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn number_format_add_rsp(&mut self, r: Resources<'_, Self>, pc: u32, returned: &mut Option<ret::StackAdjustment>) -> Result<(), Self::Error>;
    fn number_format_pop(&mut self, r: Resources<'_, Self>, pc: u32, register: ret::Register, returned: &mut Option<ret::Pop>) -> Result<(), Self::Error>;
    fn number_format_ret(&mut self, r: Resources<'_, Self>, returned: &mut Option<ret::ActualReturn>) -> Result<(), Self::Error>;
    fn number_format_entry_push(&mut self, r: Resources<'_, Self>, pc: u32, register: ret::Register, returned: &mut Option<entry::Push>) -> Result<(), Self::Error>;
    fn number_format_entry_sub(&mut self, r: Resources<'_, Self>, returned: &mut Option<entry::Subtract>) -> Result<(), Self::Error>;
    fn number_format_entry_rbp(&mut self, r: Resources<'_, Self>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn number_format_entry_save6(&mut self, r: Resources<'_, Self>, address: u64, returned: &mut Option<entry::XmmStore>) -> Result<(), Self::Error>;

    fn number_adopt_completed_format(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>,
        history: format_call::History, held: &mut Option<format_call::Retained<Self::NumberFormatConstructorOwner, Self::NumberFormatSentryOwner, Self::NumberFormatFacetOwner, Self::NumberFormatFinishOwner>>) -> Result<(), Self::Error>;
}
fn resources<'a, S: FormatServices>(r: format_call::Resources<'a, Scope<S>>) -> Resources<'a, S> {
    Resources { formatter: &mut r.parent.storage.owners, number: &mut r.parent.calls,
        constructor: r.constructor, sentry: r.sentry, facet: r.facet, finish: r.finish }
}
impl<S: FormatServices> format_call::Services for Scope<S> {
    type FormatConstructorOwner = S::NumberFormatConstructorOwner;
    type FormatSentryOwner = S::NumberFormatSentryOwner;
    type FormatFacetOwner = S::NumberFormatFacetOwner;
    type FormatFinishOwner = S::NumberFormatFinishOwner;
    fn format_read_word(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.number_format_read_word(resources(r), pc, address) }
    fn format_read_dword(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.number_format_read_dword(resources(r), pc, address) }
    fn format_read_byte(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64) -> Result<u8, Self::Error> { self.services.number_format_read_byte(resources(r), pc, address) }
    fn format_read_128(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error> { self.services.number_format_read_128(resources(r), pc, address) }
    fn format_write_word(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.number_format_write_word(resources(r), pc, address, word) }
    fn format_write_dword(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.number_format_write_dword(resources(r), pc, address, word) }
    fn format_write_byte(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error> { self.services.number_format_write_byte(resources(r), pc, address, byte) }
    fn format_write_128(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error> { self.services.number_format_write_128(resources(r), pc, address, words) }
    fn format_zero_xmm0(&mut self, r: format_call::Resources<'_, Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.number_format_zero_xmm0(resources(r), pc, returned) }
    fn format_or_dword(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64, mask: u32, returned: &mut Option<f::OrReturned>) -> Result<(), Self::Error> { self.services.number_format_or_dword(resources(r), pc, address, mask, returned) }
    fn format_construct(&mut self, r: format_call::Resources<'_, Self>, request: f::ConstructorRequest) -> Result<(), Self::Error> { self.services.number_format_construct(resources(r), request) }
    fn format_sentry_call(&mut self, r: format_call::Resources<'_, Self>, request: sentry::CallRequest) -> Result<(), Self::Error> { self.services.number_format_sentry_call(resources(r), request) }
    fn format_facet_call(&mut self, r: format_call::Resources<'_, Self>, request: facet::CallRequest) -> Result<(), Self::Error> { self.services.number_format_facet_call(resources(r), request) }
    fn format_finish_call(&mut self, r: format_call::Resources<'_, Self>, request: finish::CallRequest) -> Result<(), Self::Error> { self.services.number_format_finish_call(resources(r), request) }
    fn format_restore_xmm6(&mut self, r: format_call::Resources<'_, Self>, pc: u32, address: u64, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.number_format_restore_xmm6(resources(r), pc, address, returned) }
    fn format_add_rsp(&mut self, r: format_call::Resources<'_, Self>, pc: u32, returned: &mut Option<ret::StackAdjustment>) -> Result<(), Self::Error> { self.services.number_format_add_rsp(resources(r), pc, returned) }
    fn format_pop(&mut self, r: format_call::Resources<'_, Self>, pc: u32, register: ret::Register, returned: &mut Option<ret::Pop>) -> Result<(), Self::Error> { self.services.number_format_pop(resources(r), pc, register, returned) }
    fn format_ret(&mut self, r: format_call::Resources<'_, Self>, returned: &mut Option<ret::ActualReturn>) -> Result<(), Self::Error> { self.services.number_format_ret(resources(r), returned) }
    fn format_entry_push(&mut self, r: format_call::Resources<'_, Self>, pc: u32, register: ret::Register, returned: &mut Option<entry::Push>) -> Result<(), Self::Error> { self.services.number_format_entry_push(resources(r), pc, register, returned) }
    fn format_entry_sub(&mut self, r: format_call::Resources<'_, Self>, returned: &mut Option<entry::Subtract>) -> Result<(), Self::Error> { self.services.number_format_entry_sub(resources(r), returned) }
    fn format_entry_rbp(&mut self, r: format_call::Resources<'_, Self>, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.number_format_entry_rbp(resources(r), returned) }
    fn format_entry_save6(&mut self, r: format_call::Resources<'_, Self>, address: u64, returned: &mut Option<entry::XmmStore>) -> Result<(), Self::Error> { self.services.number_format_entry_save6(resources(r), address, returned) }
    fn adopt_completed_format(&mut self, o: &mut number::Owners<Self>, history: format_call::History,
        held: &mut Option<format_call::Retained<Self::FormatConstructorOwner, Self::FormatSentryOwner, Self::FormatFacetOwner, Self::FormatFinishOwner>>) -> Result<(), Self::Error> {
        self.services.number_adopt_completed_format(&mut o.storage.owners, &mut o.calls, history, held)
    }
}

