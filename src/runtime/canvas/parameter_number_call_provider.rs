use super::{number, Calls, History, ParentState, Retained};
use super::super as digits;
pub trait Services: digits::Services {
    type NumberCallOwner;
    fn number_read_word(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn number_write_word(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn number_xchg_ref_zero(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, address: u64, prior: &mut Option<u32>) -> Result<(), Self::Error>;

    fn number_call(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, request: number::CallRequest) -> Result<(), Self::Error>;
    fn number_mov_rax_rsi(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, output: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn number_restore_xmm6(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, address: u64, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn number_add_rsp_d8(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, returned: &mut Option<number::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn number_pop(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, destination: number::native_return::Register, returned: &mut Option<number::native_return::Pop>) -> Result<(), Self::Error>;
    fn number_ret(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, returned: &mut Option<number::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_number(&mut self, owners: &mut digits::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::NumberCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }

pub trait TextCopyServices: Services {
    fn number_copy_read_byte(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, address: u64) -> Result<u8, Self::Error>;
    fn number_copy_write_byte(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
    fn number_copy_pop(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32,
        register: number::text_copy::native_return::Register, returned: &mut Option<number::text_copy::native_return::Pop>) -> Result<(), Self::Error>;
    fn number_copy_ret(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>,
        returned: &mut Option<number::text_copy::native_return::ActualReturn>) -> Result<(), Self::Error>;
}
pub trait TextCopyEntryServices: TextCopyServices {
    fn number_copy_push(&mut self, owners: &mut digits::Owners<Self>, calls: &mut Calls<Self::NumberCallOwner>, pc: u32,
        register: number::text_copy::native_entry::Register, returned: &mut Option<number::text_copy::native_entry::Push>) -> Result<(), Self::Error>;
}
impl<S: TextCopyEntryServices> number::text_copy_call::native_entry::Services for Scope<S> {
    fn text_copy_push(&mut self, owners: &mut number::Owners<Self>, pc: u32,
        register: number::text_copy::native_entry::Register, returned: &mut Option<number::text_copy::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.number_copy_push(&mut owners.storage.owners, &mut owners.calls, pc, register, returned)
    }
}
impl<S: TextCopyServices> number::text_copy_call::Services for Scope<S> {
    fn text_copy_read_byte(&mut self, owners: &mut number::Owners<Self>, pc: u32, address: u64) -> Result<u8, Self::Error> {
        self.services.number_copy_read_byte(&mut owners.storage.owners, &mut owners.calls, pc, address)
    }
    fn text_copy_write_byte(&mut self, owners: &mut number::Owners<Self>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error> {
        self.services.number_copy_write_byte(&mut owners.storage.owners, &mut owners.calls, pc, address, byte)
    }
    fn text_copy_pop(&mut self, owners: &mut number::Owners<Self>, pc: u32, register: number::text_copy::native_return::Register,
        returned: &mut Option<number::text_copy::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.number_copy_pop(&mut owners.storage.owners, &mut owners.calls, pc, register, returned)
    }
    fn text_copy_ret(&mut self, owners: &mut number::Owners<Self>, returned: &mut Option<number::text_copy::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.number_copy_ret(&mut owners.storage.owners, &mut owners.calls, returned)
    }
}
impl<S: Services> number::Services for Scope<S> {
    type Storage = ParentState<S>;
    type CallOwner = S::NumberCallOwner;
    type Error = S::Error;
    fn read_word(&mut self, owners: &mut number::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.number_read_word(&mut owners.storage.owners, &mut owners.calls, pc, address)
    }
    fn write_word(&mut self, owners: &mut number::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.number_write_word(&mut owners.storage.owners, &mut owners.calls, pc, address, word)
    }
    fn xchg_ref_zero(&mut self, owners: &mut number::Owners<Self>, pc: u32, address: u64, prior: &mut Option<u32>) -> Result<(), Self::Error> {
        self.services.number_xchg_ref_zero(&mut owners.storage.owners, &mut owners.calls, pc, address, prior)
    }
    fn call(&mut self, owners: &mut number::Owners<Self>, request: number::CallRequest) -> Result<(), Self::Error> {
        self.services.number_call(&mut owners.storage.owners, &mut owners.calls, request)
    }
}
impl<S: Services> number::native_return::Services for Scope<S> {
    fn mov_rax_rsi(&mut self, owners: &mut number::Owners<Self>, pc: u32, output: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.number_mov_rax_rsi(&mut owners.storage.owners, &mut owners.calls, pc, output, returned)
    }
    fn restore_xmm6(&mut self, owners: &mut number::Owners<Self>, pc: u32, address: u64, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> {
        self.services.number_restore_xmm6(&mut owners.storage.owners, &mut owners.calls, pc, address, returned)
    }
    fn add_rsp_d8(&mut self, owners: &mut number::Owners<Self>, pc: u32, returned: &mut Option<number::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.number_add_rsp_d8(&mut owners.storage.owners, &mut owners.calls, pc, returned)
    }
    fn pop_saved(&mut self, owners: &mut number::Owners<Self>, pc: u32, destination: number::native_return::Register, returned: &mut Option<number::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.number_pop(&mut owners.storage.owners, &mut owners.calls, pc, destination, returned)
    }
    fn ret1caf55(&mut self, owners: &mut number::Owners<Self>, returned: &mut Option<number::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.number_ret(&mut owners.storage.owners, &mut owners.calls, returned)
    }
}
