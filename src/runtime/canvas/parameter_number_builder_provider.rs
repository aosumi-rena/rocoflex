use super::{number, Calls, Scope};
use super::super as digits;
use number::{builder, builder_call};
type Inner<R> = builder_call::Calls<R>;
pub trait BuilderServices: super::Services {
    type NumberBuilderCallOwner;
    fn number_builder_read_word(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn number_builder_read_dword(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn number_builder_gs58(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32) -> Result<u64, Self::Error>;
    fn number_builder_write_word(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn number_builder_write_dword(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn number_builder_zero_xmm0(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn number_builder_write_xmm0(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;
    fn number_builder_call(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, request: builder::CallRequest) -> Result<(), Self::Error>;
    fn number_builder_mov_rax(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn number_builder_add_rsp(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, returned: &mut Option<builder::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn number_builder_pop(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32, register: builder::native_return::Register, returned: &mut Option<builder::native_return::Pop>) -> Result<(), Self::Error>;
    fn number_builder_ret(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, returned: &mut Option<builder::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn number_adopt_completed_builder(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, history: builder_call::History,
        held: &mut Option<builder_call::Retained<Self::NumberBuilderCallOwner>>) -> Result<(), Self::Error>;
}
pub trait BuilderEntryServices: BuilderServices {
    fn number_builder_push(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, pc: u32,
        register: builder::native_entry::Register, returned: &mut Option<builder::native_entry::Push>) -> Result<(), Self::Error>;
    fn number_builder_sub(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, returned: &mut Option<builder::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn number_builder_rbp(&mut self, o: &mut digits::Owners<Self>, n: &mut Calls<Self::NumberCallOwner>, b: &mut Inner<Self::NumberBuilderCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;
}
impl<S: BuilderEntryServices> builder_call::native_entry::Services for Scope<S> {
    fn builder_entry_push(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32,
        register: builder::native_entry::Register, returned: &mut Option<builder::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.number_builder_push(&mut o.storage.owners, &mut o.calls, b, pc, register, returned)
    }
    fn builder_entry_sub(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, returned: &mut Option<builder::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.number_builder_sub(&mut o.storage.owners, &mut o.calls, b, returned)
    }
    fn builder_entry_rbp(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.number_builder_rbp(&mut o.storage.owners, &mut o.calls, b, returned)
    }
}
impl<S: BuilderServices> builder_call::Services for Scope<S> {
    type BuilderCallOwner = S::NumberBuilderCallOwner;
    fn builder_read_word(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.number_builder_read_word(&mut o.storage.owners, &mut o.calls, b, pc, address) }
    fn builder_read_dword(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.number_builder_read_dword(&mut o.storage.owners, &mut o.calls, b, pc, address) }
    fn builder_read_gs58(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32) -> Result<u64, Self::Error> { self.services.number_builder_gs58(&mut o.storage.owners, &mut o.calls, b, pc) }
    fn builder_write_word(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.number_builder_write_word(&mut o.storage.owners, &mut o.calls, b, pc, address, word) }
    fn builder_write_dword(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.number_builder_write_dword(&mut o.storage.owners, &mut o.calls, b, pc, address, word) }
    fn builder_zero_xmm0(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, r: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.number_builder_zero_xmm0(&mut o.storage.owners, &mut o.calls, b, pc, r) }
    fn builder_write_xmm0(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error> { self.services.number_builder_write_xmm0(&mut o.storage.owners, &mut o.calls, b, pc, address, words) }
    fn builder_call(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, request: builder::CallRequest) -> Result<(), Self::Error> { self.services.number_builder_call(&mut o.storage.owners, &mut o.calls, b, request) }
    fn builder_mov_rax(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, word: u64, r: &mut Option<u64>) -> Result<(), Self::Error> { self.services.number_builder_mov_rax(&mut o.storage.owners, &mut o.calls, b, pc, word, r) }
    fn builder_add_rsp(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, r: &mut Option<builder::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.number_builder_add_rsp(&mut o.storage.owners, &mut o.calls, b, pc, r) }
    fn builder_pop(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, pc: u32, register: builder::native_return::Register, r: &mut Option<builder::native_return::Pop>) -> Result<(), Self::Error> { self.services.number_builder_pop(&mut o.storage.owners, &mut o.calls, b, pc, register, r) }
    fn builder_ret(&mut self, o: &mut number::Owners<Self>, b: &mut Inner<Self::BuilderCallOwner>, r: &mut Option<builder::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.number_builder_ret(&mut o.storage.owners, &mut o.calls, b, r) }
    fn adopt_completed_builder(&mut self, o: &mut number::Owners<Self>, history: builder_call::History, held: &mut Option<builder_call::Retained<Self::BuilderCallOwner>>) -> Result<(), Self::Error> { self.services.number_adopt_completed_builder(&mut o.storage.owners, &mut o.calls, history, held) }
}
