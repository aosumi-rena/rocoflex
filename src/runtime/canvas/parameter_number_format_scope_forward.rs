use super::{f, sentry, facet, finish, native_entry, ret, resources, Scope, Services};
impl<S: Services> f::Services for Scope<S> {
    type Storage = super::ParentState<S>; type ConstructorOwner = S::FormatConstructorOwner; type Error = S::Error;
    fn read_word(&mut self, o: &mut f::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.format_read_word(resources(o, None, None, None), pc, address) }
    fn read_dword(&mut self, o: &mut f::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.format_read_dword(resources(o, None, None, None), pc, address) }
    fn write_word(&mut self, o: &mut f::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.format_write_word(resources(o, None, None, None), pc, address, word) }
    fn write_dword(&mut self, o: &mut f::Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.format_write_dword(resources(o, None, None, None), pc, address, word) }
    fn zero_xmm0(&mut self, o: &mut f::Owners<Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.format_zero_xmm0(resources(o, None, None, None), pc, returned) }
    fn write_xmm0(&mut self, o: &mut f::Owners<Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error> { self.services.format_write_128(resources(o, None, None, None), pc, address, words) }
    fn or_dword(&mut self, o: &mut f::Owners<Self>, pc: u32, address: u64, mask: u32, returned: &mut Option<f::OrReturned>) -> Result<(), Self::Error> { self.services.format_or_dword(resources(o, None, None, None), pc, address, mask, returned) }
    fn construct_1fbcf0(&mut self, o: &mut f::Owners<Self>, request: f::ConstructorRequest) -> Result<(), Self::Error> { self.services.format_construct(resources(o, None, None, None), request) }
}
impl<S: Services> sentry::Services for Scope<S> {
    type SentryCallOwner = S::FormatSentryOwner;
    fn sentry_read_word(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.format_read_word(resources(o, Some(s), None, None), pc, address) }
    fn sentry_read_dword(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.format_read_dword(resources(o, Some(s), None, None), pc, address) }
    fn sentry_write_word(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.format_write_word(resources(o, Some(s), None, None), pc, address, word) }
    fn sentry_write_dword(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.format_write_dword(resources(o, Some(s), None, None), pc, address, word) }
    fn sentry_write_byte(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error> { self.services.format_write_byte(resources(o, Some(s), None, None), pc, address, byte) }
    fn sentry_call(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, request: sentry::CallRequest) -> Result<(), Self::Error> { self.services.format_sentry_call(resources(o, Some(s), None, None), request) }
}
impl<S: Services> facet::Services for Scope<S> {
    type FacetCallOwner = S::FormatFacetOwner;
    fn facet_read_word(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut facet::Calls<Self::FacetCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.format_read_word(resources(o, Some(s), Some(c), None), pc, address) }
    fn facet_read_dword(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut facet::Calls<Self::FacetCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.format_read_dword(resources(o, Some(s), Some(c), None), pc, address) }
    fn facet_read_byte(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut facet::Calls<Self::FacetCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error> { self.services.format_read_byte(resources(o, Some(s), Some(c), None), pc, address) }
    fn facet_write_word(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut facet::Calls<Self::FacetCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.format_write_word(resources(o, Some(s), Some(c), None), pc, address, word) }
    fn facet_write_dword(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut facet::Calls<Self::FacetCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.format_write_dword(resources(o, Some(s), Some(c), None), pc, address, word) }
    fn facet_write_byte(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut facet::Calls<Self::FacetCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error> { self.services.format_write_byte(resources(o, Some(s), Some(c), None), pc, address, byte) }
    fn facet_call(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, c: &mut facet::Calls<Self::FacetCallOwner>, request: facet::CallRequest) -> Result<(), Self::Error> { self.services.format_facet_call(resources(o, Some(s), Some(c), None), request) }
}
impl<S: Services> finish::Services for Scope<S> {
    type FinishCallOwner = S::FormatFinishOwner;
    fn finish_read_word(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.format_read_word(resources(o, Some(s), b, Some(c)), pc, address) }
    fn finish_read_dword(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.format_read_dword(resources(o, Some(s), b, Some(c)), pc, address) }
    fn finish_read_128(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, address: u64) -> Result<[u32; 4], Self::Error> { self.services.format_read_128(resources(o, Some(s), b, Some(c)), pc, address) }
    fn finish_write_word(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.format_write_word(resources(o, Some(s), b, Some(c)), pc, address, word) }
    fn finish_write_dword(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.format_write_dword(resources(o, Some(s), b, Some(c)), pc, address, word) }
    fn finish_write_128(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error> { self.services.format_write_128(resources(o, Some(s), b, Some(c)), pc, address, words) }
    fn finish_call(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, request: finish::CallRequest) -> Result<(), Self::Error> { self.services.format_finish_call(resources(o, Some(s), b, Some(c)), request) }
}
impl<S: Services> ret::Services for Scope<S> {
    fn format_restore_xmm6(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, address: u64, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.format_restore_xmm6(resources(o, Some(s), b, Some(c)), pc, address, returned) }
    fn format_add_rsp140(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, returned: &mut Option<ret::StackAdjustment>) -> Result<(), Self::Error> { self.services.format_add_rsp(resources(o, Some(s), b, Some(c)), pc, returned) }
    fn format_pop_saved(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, pc: u32, register: ret::Register, returned: &mut Option<ret::Pop>) -> Result<(), Self::Error> { self.services.format_pop(resources(o, Some(s), b, Some(c)), pc, register, returned) }
    fn format_ret1fb7eb(&mut self, o: &mut f::Owners<Self>, s: &mut sentry::Calls<Self::SentryCallOwner>, b: Option<&mut facet::Calls<Self::FacetCallOwner>>, c: &mut finish::Calls<Self::FinishCallOwner>, returned: &mut Option<ret::ActualReturn>) -> Result<(), Self::Error> { self.services.format_ret(resources(o, Some(s), b, Some(c)), returned) }
}
impl<S: Services> native_entry::Services for Scope<S> {
    fn format_entry_push(&mut self, o: &mut f::Owners<Self>, pc: u32, register: ret::Register, returned: &mut Option<native_entry::Push>) -> Result<(), Self::Error> { self.services.format_entry_push(resources(o, None, None, None), pc, register, returned) }
    fn format_entry_sub140(&mut self, o: &mut f::Owners<Self>, returned: &mut Option<native_entry::Subtract>) -> Result<(), Self::Error> { self.services.format_entry_sub(resources(o, None, None, None), returned) }
    fn format_entry_lea_rbp80(&mut self, o: &mut f::Owners<Self>, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.format_entry_rbp(resources(o, None, None, None), returned) }
    fn format_entry_save_xmm6(&mut self, o: &mut f::Owners<Self>, address: u64, returned: &mut Option<native_entry::XmmStore>) -> Result<(), Self::Error> { self.services.format_entry_save6(resources(o, None, None, None), address, returned) }
}

