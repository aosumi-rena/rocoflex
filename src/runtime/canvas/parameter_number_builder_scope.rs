use super::{builder, Calls, History, ParentState, Retained};
use super::super as number;
pub trait Services: number::Services {
    type BuilderCallOwner;
    fn builder_read_word(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn builder_read_dword(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn builder_read_gs58(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32) -> Result<u64, Self::Error>;
    fn builder_write_word(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn builder_write_dword(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn builder_zero_xmm0(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn builder_write_xmm0(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;
    fn builder_call(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, request: builder::CallRequest) -> Result<(), Self::Error>;
    fn builder_mov_rax(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn builder_add_rsp(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, returned: &mut Option<builder::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn builder_pop(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, pc: u32, register: builder::native_return::Register, returned: &mut Option<builder::native_return::Pop>) -> Result<(), Self::Error>;
    fn builder_ret(&mut self, owners: &mut number::Owners<Self>, calls: &mut Calls<Self::BuilderCallOwner>, returned: &mut Option<builder::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_builder(&mut self, owners: &mut number::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::BuilderCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> builder::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::BuilderCallOwner; type Error = S::Error;
    fn read_word(&mut self, o: &mut builder::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.builder_read_word(&mut o.storage.owners, &mut o.calls, pc, address) }
    fn read_dword(&mut self, o: &mut builder::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.builder_read_dword(&mut o.storage.owners, &mut o.calls, pc, address) }
    fn read_gs58(&mut self, o: &mut builder::Owners<Self>, pc: u32) -> Result<u64, Self::Error> { self.services.builder_read_gs58(&mut o.storage.owners, &mut o.calls, pc) }
    fn write_word(&mut self, o: &mut builder::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.builder_write_word(&mut o.storage.owners, &mut o.calls, pc, address, word) }
    fn write_dword(&mut self, o: &mut builder::Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.builder_write_dword(&mut o.storage.owners, &mut o.calls, pc, address, word) }
    fn zero_xmm0(&mut self, o: &mut builder::Owners<Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.builder_zero_xmm0(&mut o.storage.owners, &mut o.calls, pc, returned) }
    fn write_xmm0(&mut self, o: &mut builder::Owners<Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error> { self.services.builder_write_xmm0(&mut o.storage.owners, &mut o.calls, pc, address, words) }
    fn call(&mut self, o: &mut builder::Owners<Self>, request: builder::CallRequest) -> Result<(), Self::Error> { self.services.builder_call(&mut o.storage.owners, &mut o.calls, request) }
}
impl<S: Services> builder::native_return::Services for Scope<S> {
    fn mov_rax_rbx(&mut self, o: &mut builder::Owners<Self>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.builder_mov_rax(&mut o.storage.owners, &mut o.calls, pc, word, returned) }
    fn add_rsp38(&mut self, o: &mut builder::Owners<Self>, pc: u32, returned: &mut Option<builder::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.builder_add_rsp(&mut o.storage.owners, &mut o.calls, pc, returned) }
    fn pop_saved(&mut self, o: &mut builder::Owners<Self>, pc: u32, register: builder::native_return::Register, returned: &mut Option<builder::native_return::Pop>) -> Result<(), Self::Error> { self.services.builder_pop(&mut o.storage.owners, &mut o.calls, pc, register, returned) }
    fn ret_1fb4f2(&mut self, o: &mut builder::Owners<Self>, returned: &mut Option<builder::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.builder_ret(&mut o.storage.owners, &mut o.calls, returned) }
}
