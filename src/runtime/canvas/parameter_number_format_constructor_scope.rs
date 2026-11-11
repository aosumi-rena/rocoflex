use super::{c, ParentState, History, Retained};
use super::super as f;
pub trait Services: f::Services {
    type ConstructorCallOwner;

    fn constructor_read_word(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn constructor_read_dword(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn constructor_write_word(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn constructor_write_dword(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn constructor_write_byte(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error>;
    fn constructor_zero_xmm0(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn constructor_write_xmm0(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error>;
    fn constructor_call(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, request: c::CallRequest) -> Result<(), Self::Error>;
    fn constructor_nop(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, next_pc: &mut Option<u32>) -> Result<(), Self::Error>;
    fn constructor_add_rsp(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, returned: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn constructor_pop(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error>;
    fn constructor_ret(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, returned: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn constructor_push(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error>;
    fn constructor_sub(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, returned: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn constructor_rbp(&mut self, o: &mut f::Owners<Self>, calls: &mut c::Calls<Self::ConstructorCallOwner>, returned: &mut Option<u64>) -> Result<(), Self::Error>;

    fn adopt_completed_constructor(&mut self, o: &mut f::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::ConstructorCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> c::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::ConstructorCallOwner; type Error = S::Error;
    fn read_word(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> { self.services.constructor_read_word(&mut o.storage.owners, &mut o.calls, pc, address) }
    fn read_dword(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error> { self.services.constructor_read_dword(&mut o.storage.owners, &mut o.calls, pc, address) }
    fn write_word(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> { self.services.constructor_write_word(&mut o.storage.owners, &mut o.calls, pc, address, word) }
    fn write_dword(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> { self.services.constructor_write_dword(&mut o.storage.owners, &mut o.calls, pc, address, word) }
    fn write_byte(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64, byte: u8) -> Result<(), Self::Error> { self.services.constructor_write_byte(&mut o.storage.owners, &mut o.calls, pc, address, byte) }
    fn zero_xmm0(&mut self, o: &mut c::Owners<Self>, pc: u32, returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error> { self.services.constructor_zero_xmm0(&mut o.storage.owners, &mut o.calls, pc, returned) }
    fn write_xmm0(&mut self, o: &mut c::Owners<Self>, pc: u32, address: u64, words: [u32; 4]) -> Result<(), Self::Error> { self.services.constructor_write_xmm0(&mut o.storage.owners, &mut o.calls, pc, address, words) }
    fn call(&mut self, o: &mut c::Owners<Self>, request: c::CallRequest) -> Result<(), Self::Error> { self.services.constructor_call(&mut o.storage.owners, &mut o.calls, request) }
}
impl<S: Services> c::native_return::Services for Scope<S> {
    fn nop_1fbdc6(&mut self, o: &mut c::Owners<Self>, next_pc: &mut Option<u32>) -> Result<(), Self::Error> { self.services.constructor_nop(&mut o.storage.owners, &mut o.calls, next_pc) }
    fn add_rsp60(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<c::native_return::StackAdjustment>) -> Result<(), Self::Error> { self.services.constructor_add_rsp(&mut o.storage.owners, &mut o.calls, returned) }
    fn pop_saved(&mut self, o: &mut c::Owners<Self>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_return::Pop>) -> Result<(), Self::Error> { self.services.constructor_pop(&mut o.storage.owners, &mut o.calls, pc, register, returned) }
    fn ret_1fbdd1(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<c::native_return::ActualReturn>) -> Result<(), Self::Error> { self.services.constructor_ret(&mut o.storage.owners, &mut o.calls, returned) }
}
impl<S: Services> c::native_entry::Services for Scope<S> {
    fn constructor_push(&mut self, o: &mut c::Owners<Self>, pc: u32, register: c::native_return::Register, returned: &mut Option<c::native_entry::Push>) -> Result<(), Self::Error> { self.services.constructor_push(&mut o.storage.owners, &mut o.calls, pc, register, returned) }
    fn constructor_sub60(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<c::native_entry::Subtract>) -> Result<(), Self::Error> { self.services.constructor_sub(&mut o.storage.owners, &mut o.calls, returned) }
    fn constructor_lea_rbp60(&mut self, o: &mut c::Owners<Self>, returned: &mut Option<u64>) -> Result<(), Self::Error> { self.services.constructor_rbp(&mut o.storage.owners, &mut o.calls, returned) }
}

