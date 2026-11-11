use super::{Scope, Resources, Services};
use super::scope::resources;
use super::formatting::{self as f, constructor as c, constructor_call};
#[path = "parameter_stream_lookup_format_routing.rs"]
mod lookup;
pub use lookup::{LookupServices, LookupFallbackServices, LookupRegistrationServices};
#[path = "parameter_stream_member_format_routing.rs"]
mod member;
pub use member::LookupMemberServices;
#[path = "parameter_stream_member_query_format_routing.rs"]
mod member_query;
pub use member_query::LookupMemberQueryServices;
#[path = "parameter_stream_member_duplicate_format_routing.rs"]
mod member_duplicate;
pub use member_duplicate::LookupMemberDuplicateServices;
#[path = "parameter_stream_member_copy_format_routing.rs"]
mod member_duplicate_copy;
pub use member_duplicate_copy::LookupMemberDuplicateCopyServices;
#[path = "parameter_stream_member_accessor_format_routing.rs"]
mod member_duplicate_copy_accessor;
pub use member_duplicate_copy_accessor::LookupMemberDuplicateCopyAccessorServices;
#[path = "parameter_stream_member_diagnostic_format_routing.rs"]
mod member_duplicate_copy_diagnostic;
pub use member_duplicate_copy_diagnostic::LookupMemberDuplicateCopyDiagnosticServices;
#[path = "parameter_stream_member_diagnostic_dispatch_format_routing.rs"]
mod diagnostic_dispatch_routing;
pub use diagnostic_dispatch_routing::LookupMemberDuplicateCopyDiagnosticDispatchServices;
#[path = "parameter_stream_member_diagnostic_children_format_routing.rs"]
mod diagnostic_children_routing;
pub use diagnostic_children_routing::{LookupMemberDuplicateCopyDiagnosticDispatchHandlerServices, LookupMemberDuplicateCopyDiagnosticDispatchMetadataServices};
#[path = "parameter_stream_member_diagnostic_cached_format_routing.rs"]
mod diagnostic_cached_routing;
pub use diagnostic_cached_routing::{LookupMemberDuplicateCopyDiagnosticDispatchHandlerContextServices, LookupMemberDuplicateCopyDiagnosticDispatchHandlerEncodedSlotServices};
#[path = "parameter_stream_guard_format_routing.rs"]
mod guard;
pub use guard::LookupGuardServices;
#[path = "parameter_stream_wrapper_format_routing.rs"]
mod wrapper;
pub use wrapper::LookupWrapperServices;
#[path = "parameter_stream_initialization_format_routing.rs"]
mod initialization;
pub use initialization::LookupInitializationServices;
#[path = "parameter_stream_resolver_format_routing.rs"]
mod resolver;
pub use resolver::{LookupResolverServices, LookupResolverEntryServices, LookupResolverProtectedServices, LookupResolverResolutionServices, LookupResolverLoadServices, LookupResolverPriorCleanupServices};
pub trait ConstructorServices: Services {
    type FormatConstructorCallOwner;

    fn format_constructor_read_word(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn format_constructor_read_dword(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn format_constructor_write_word(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn format_constructor_write_dword(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn format_constructor_write_byte(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
    fn format_constructor_zero_xmm0(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn format_constructor_write_xmm0(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;
    fn format_constructor_call(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, request: c::CallRequest) -> Result<(), Self::Error>;
    fn format_constructor_nop(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, next_pc: &mut Option<u32>) -> Result<(), Self::Error>;
    fn format_constructor_add_rsp(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, returned: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn format_constructor_pop(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error>;
    fn format_constructor_ret(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, returned: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn format_constructor_push(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error>;
    fn format_constructor_sub(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, returned: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn format_constructor_rbp(&mut self, r: Resources<'_, Self>, calls: &mut c::Calls<Self::FormatConstructorCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn adopt_completed_formatter_constructor(&mut self, r: Resources<'_, Self>, history: constructor_call::History,
        held: &mut Option<constructor_call::Retained<Self::FormatConstructorCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: ConstructorServices> constructor_call::Services for Scope<S> {
    type ConstructorCallOwner = S::FormatConstructorCallOwner;
    fn constructor_read_word(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.format_constructor_read_word(resources(o, None, None, None), calls, pc, address) }
    fn constructor_read_dword(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.format_constructor_read_dword(resources(o, None, None, None), calls, pc, address) }
    fn constructor_write_word(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.format_constructor_write_word(resources(o, None, None, None), calls, pc, address, word) }
    fn constructor_write_dword(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.format_constructor_write_dword(resources(o, None, None, None), calls, pc, address, word) }
    fn constructor_write_byte(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error> { self.services.format_constructor_write_byte(resources(o, None, None, None), calls, pc, address, byte) }
    fn constructor_zero_xmm0(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.format_constructor_zero_xmm0(resources(o, None, None, None), calls, pc, returned) }
    fn constructor_write_xmm0(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error> { self.services.format_constructor_write_xmm0(resources(o, None, None, None), calls, pc, address, words) }
    fn constructor_call(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, request: c::CallRequest) -> Result<(), Self::Error> { self.services.format_constructor_call(resources(o, None, None, None), calls, request) }
    fn constructor_nop(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, next_pc: &mut Option<u32>) -> Result<(), Self::Error> { self.services.format_constructor_nop(resources(o, None, None, None), calls, next_pc) }
    fn constructor_add_rsp(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, returned: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.format_constructor_add_rsp(resources(o, None, None, None), calls, returned) }
    fn constructor_pop(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error> { self.services.format_constructor_pop(resources(o, None, None, None), calls, pc, register, returned) }
    fn constructor_ret(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, returned: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.format_constructor_ret(resources(o, None, None, None), calls, returned) }
    fn constructor_push(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error> { self.services.format_constructor_push(resources(o, None, None, None), calls, pc, register, returned) }
    fn constructor_sub(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, returned: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error> { self.services.format_constructor_sub(resources(o, None, None, None), calls, returned) }
    fn constructor_rbp(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.format_constructor_rbp(resources(o, None, None, None), calls, returned) }
    fn adopt_completed_constructor(&mut self, o: &mut f::Owners<Self>, history: constructor_call::History,
        held: &mut Option<constructor_call::Retained<Self::ConstructorCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_formatter_constructor(resources(o, None, None, None), history, held)
    }
}

