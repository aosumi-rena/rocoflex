use super::{number, f, FormatServices, Resources, Scope, resources};
use f::{constructor as c, constructor_call};
use number::format_call;
#[path = "parameter_stream_lookup_digits_provider.rs"]
mod lookup;
pub use lookup::{FormatLookupServices, FormatLookupFallbackServices, FormatLookupRegistrationServices};
#[path = "parameter_stream_member_digits_provider.rs"]
mod member;
pub use member::FormatLookupMemberServices;
#[path = "parameter_stream_member_query_digits_provider.rs"]
mod member_query;
pub use member_query::FormatLookupMemberQueryServices;
#[path = "parameter_stream_member_duplicate_digits_provider.rs"]
mod member_duplicate;
pub use member_duplicate::FormatLookupMemberDuplicateServices;
#[path = "parameter_stream_member_copy_digits_provider.rs"]
mod member_duplicate_copy;
pub use member_duplicate_copy::FormatLookupMemberDuplicateCopyServices;
#[path = "parameter_stream_member_accessor_digits_provider.rs"]
mod member_duplicate_copy_accessor;
pub use member_duplicate_copy_accessor::FormatLookupMemberDuplicateCopyAccessorServices;
#[path = "parameter_stream_member_diagnostic_digits_provider.rs"]
mod member_duplicate_copy_diagnostic;
pub use member_duplicate_copy_diagnostic::FormatLookupMemberDuplicateCopyDiagnosticServices;
#[path = "parameter_stream_member_diagnostic_dispatch_digits_provider.rs"]
mod diagnostic_dispatch_routing;
pub use diagnostic_dispatch_routing::FormatLookupMemberDuplicateCopyDiagnosticDispatchServices;
#[path = "parameter_stream_member_diagnostic_children_digits_provider.rs"]
mod diagnostic_children_routing;
pub use diagnostic_children_routing::{FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerServices, FormatLookupMemberDuplicateCopyDiagnosticDispatchMetadataServices};
#[path = "parameter_stream_member_diagnostic_cached_digits_provider.rs"]
mod diagnostic_cached_routing;
pub use diagnostic_cached_routing::{FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerContextServices, FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerEncodedSlotServices};
#[path = "parameter_stream_guard_digits_provider.rs"]
mod guard;
pub use guard::FormatLookupGuardServices;
#[path = "parameter_stream_wrapper_digits_provider.rs"]
mod wrapper;
pub use wrapper::FormatLookupWrapperServices;
#[path = "parameter_stream_initialization_digits_provider.rs"]
mod initialization;
pub use initialization::FormatLookupInitializationServices;
#[path = "parameter_stream_resolver_digits_provider.rs"]
mod resolver;
pub use resolver::{FormatLookupResolverServices, FormatLookupResolverEntryServices, FormatLookupResolverProtectedServices, FormatLookupResolverResolutionServices, FormatLookupResolverLoadServices, FormatLookupResolverPriorCleanupServices};
pub trait FormatConstructorServices: FormatServices {
    type NumberFormatConstructorCallOwner;

    fn number_format_constructor_read_word(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn number_format_constructor_read_dword(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn number_format_constructor_write_word(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn number_format_constructor_write_dword(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn number_format_constructor_write_byte(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
    fn number_format_constructor_zero_xmm0(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn number_format_constructor_write_xmm0(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;
    fn number_format_constructor_call(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, request: c::CallRequest) -> Result<(), Self::Error>;
    fn number_format_constructor_nop(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, next_pc: &mut Option<u32>) -> Result<(), Self::Error>;
    fn number_format_constructor_add_rsp(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, returned: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn number_format_constructor_pop(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error>;
    fn number_format_constructor_ret(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, returned: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn number_format_constructor_push(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error>;
    fn number_format_constructor_sub(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, returned: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn number_format_constructor_rbp(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::NumberFormatConstructorCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn number_adopt_completed_formatter_constructor(&mut self, r: Resources<'_, Self>, history: constructor_call::History,
        held: &mut Option<constructor_call::Retained<Self::NumberFormatConstructorCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: FormatConstructorServices> format_call::ConstructorServices for Scope<S> {
    type FormatConstructorCallOwner = S::NumberFormatConstructorCallOwner;
    fn format_constructor_read_word(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.number_format_constructor_read_word(resources(r), calls, pc, address) }
    fn format_constructor_read_dword(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.number_format_constructor_read_dword(resources(r), calls, pc, address) }
    fn format_constructor_write_word(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.number_format_constructor_write_word(resources(r), calls, pc, address, word) }
    fn format_constructor_write_dword(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.number_format_constructor_write_dword(resources(r), calls, pc, address, word) }
    fn format_constructor_write_byte(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error> { self.services.number_format_constructor_write_byte(resources(r), calls, pc, address, byte) }
    fn format_constructor_zero_xmm0(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.number_format_constructor_zero_xmm0(resources(r), calls, pc, returned) }
    fn format_constructor_write_xmm0(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error> { self.services.number_format_constructor_write_xmm0(resources(r), calls, pc, address, words) }
    fn format_constructor_call(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, request: c::CallRequest) -> Result<(), Self::Error> { self.services.number_format_constructor_call(resources(r), calls, request) }
    fn format_constructor_nop(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, next_pc: &mut Option<u32>) -> Result<(), Self::Error> { self.services.number_format_constructor_nop(resources(r), calls, next_pc) }
    fn format_constructor_add_rsp(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, returned: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.number_format_constructor_add_rsp(resources(r), calls, returned) }
    fn format_constructor_pop(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error> { self.services.number_format_constructor_pop(resources(r), calls, pc, register, returned) }
    fn format_constructor_ret(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, returned: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.number_format_constructor_ret(resources(r), calls, returned) }
    fn format_constructor_push(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error> { self.services.number_format_constructor_push(resources(r), calls, pc, register, returned) }
    fn format_constructor_sub(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, returned: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error> { self.services.number_format_constructor_sub(resources(r), calls, returned) }
    fn format_constructor_rbp(&mut self, r: format_call::Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.number_format_constructor_rbp(resources(r), calls, returned) }
    fn adopt_completed_formatter_constructor(&mut self, r: format_call::Resources<'_, Self>, history: constructor_call::History,
        held: &mut Option<constructor_call::Retained<Self::FormatConstructorCallOwner>>) -> Result<(), Self::Error> {
        self.services.number_adopt_completed_formatter_constructor(resources(r), history, held)
    }
}

