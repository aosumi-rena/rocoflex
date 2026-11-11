use super::{formatting as f, ParentState, Retained, History};
use f::{sentry, facet, finish, native_entry};
use finish::native_return as ret;
use super::super as number;
#[path = "parameter_number_format_scope_forward.rs"]
mod forward;

pub struct Resources<'a, S: Services> {
    pub parent: &'a mut number::Owners<S>,
    pub constructor: &'a mut Option<f::ConstructorReturned<S::FormatConstructorOwner>>,
    pub sentry: Option<&'a mut sentry::Calls<S::FormatSentryOwner>>,
    pub facet: Option<&'a mut facet::Calls<S::FormatFacetOwner>>,
    pub finish: Option<&'a mut finish::Calls<S::FormatFinishOwner>>,
}
pub trait Services: number::Services {
    type FormatConstructorOwner; type FormatSentryOwner; type FormatFacetOwner; type FormatFinishOwner;

    fn format_read_word(&mut self, r: Resources<'_, Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn format_read_dword(&mut self, r: Resources<'_, Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn format_read_byte(&mut self, r: Resources<'_, Self>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn format_read_128(&mut self, r: Resources<'_, Self>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error>;
    fn format_write_word(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn format_write_dword(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn format_write_byte(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
    fn format_write_128(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;
    fn format_zero_xmm0(&mut self, r: Resources<'_, Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn format_or_dword(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, mask: u32, returned: &mut Option<f::OrReturned>) -> Result<(), Self::Error>;
    fn format_construct(&mut self, r: Resources<'_, Self>, request: f::ConstructorRequest) -> Result<(), Self::Error>;
    fn format_sentry_call(&mut self, r: Resources<'_, Self>, request: sentry::CallRequest) -> Result<(), Self::Error>;
    fn format_facet_call(&mut self, r: Resources<'_, Self>, request: facet::CallRequest) -> Result<(), Self::Error>;
    fn format_finish_call(&mut self, r: Resources<'_, Self>, request: finish::CallRequest) -> Result<(), Self::Error>;
    fn format_restore_xmm6(&mut self, r: Resources<'_, Self>, pc: u32, address: u64, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn format_add_rsp(&mut self, r: Resources<'_, Self>, pc: u32, returned: &mut Option<ret::StackAdjustment>) -> Result<(), Self::Error>;
    fn format_pop(&mut self, r: Resources<'_, Self>, pc: u32, register: ret::Register, returned: &mut Option<ret::Pop>) -> Result<(), Self::Error>;
    fn format_ret(&mut self, r: Resources<'_, Self>, returned: &mut Option<ret::ActualReturn>) -> Result<(), Self::Error>;
    fn format_entry_push(&mut self, r: Resources<'_, Self>, pc: u32, register: ret::Register, returned: &mut Option<native_entry::Push>) -> Result<(), Self::Error>;
    fn format_entry_sub(&mut self, r: Resources<'_, Self>, returned: &mut Option<native_entry::Subtract>) -> Result<(), Self::Error>;
    fn format_entry_rbp(&mut self, r: Resources<'_, Self>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn format_entry_save6(&mut self, r: Resources<'_, Self>, address: u64, returned: &mut Option<native_entry::XmmStore>) -> Result<(), Self::Error>;

    fn adopt_completed_format(&mut self, owners: &mut number::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::FormatConstructorOwner, Self::FormatSentryOwner, Self::FormatFacetOwner, Self::FormatFinishOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
pub(super) fn resources<'a, S: Services>(o: &'a mut f::Owners<Scope<S>>,
    sentry: Option<&'a mut sentry::Calls<S::FormatSentryOwner>>,
    facet: Option<&'a mut facet::Calls<S::FormatFacetOwner>>,
    finish: Option<&'a mut finish::Calls<S::FormatFinishOwner>>) -> Resources<'a, S> {
    Resources { parent: &mut o.storage.owners, constructor: &mut o.constructor, sentry, facet, finish }
}
