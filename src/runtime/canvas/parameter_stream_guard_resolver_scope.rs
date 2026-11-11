use super::{r, i, ParentState, Retained, History};
pub trait Services: i::Services {

    fn resolver_read_word(&mut self, o: &mut i::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn resolver_read_dword(&mut self, o: &mut i::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn resolver_xchg_word(&mut self, o: &mut i::Owners<Self>, pc: u32, address: u64, word: u64, returned: &mut Option<r::ExchangeReturned>) -> Result<(), Self::Error>;
    fn resolver_restore_word(&mut self, o: &mut i::Owners<Self>, pc: u32, register: r::native_return::Register, address: u64, returned: &mut Option<r::native_return::Restored>) -> Result<(), Self::Error>;
    fn resolver_add_rsp20(&mut self, o: &mut i::Owners<Self>, returned: &mut Option<r::StackAdjustment>) -> Result<(), Self::Error>;
    fn resolver_pop(&mut self, o: &mut i::Owners<Self>, pc: u32, register: r::native_return::Register, returned: &mut Option<r::Pop>) -> Result<(), Self::Error>;
    fn resolver_ret(&mut self, o: &mut i::Owners<Self>, returned: &mut Option<r::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_resolver(&mut self, o: &mut i::Owners<Self>, history: History, held: &mut Option<Retained>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> r::Services for Scope<S> {
    type Storage = ParentState<S>; type Error = S::Error;
    fn read_word(&mut self, o: &mut Self::Storage, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.resolver_read_word(&mut o.owners, pc, address)
    }
    fn read_dword(&mut self, o: &mut Self::Storage, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.resolver_read_dword(&mut o.owners, pc, address)
    }
    fn xchg_word(&mut self, o: &mut Self::Storage, pc: u32, address: u64, word: u64, returned: &mut Option<r::ExchangeReturned>) -> Result<(), Self::Error> {
        self.services.resolver_xchg_word(&mut o.owners, pc, address, word, returned)
    }
}
impl<S: Services> r::native_return::Services for Scope<S> {
    fn restore_word(&mut self, o: &mut Self::Storage, pc: u32, register: r::native_return::Register, address: u64, returned: &mut Option<r::native_return::Restored>) -> Result<(), Self::Error> {
        self.services.resolver_restore_word(&mut o.owners, pc, register, address, returned)
    }
    fn add_rsp20(&mut self, o: &mut Self::Storage, returned: &mut Option<r::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.resolver_add_rsp20(&mut o.owners, returned)
    }
    fn pop(&mut self, o: &mut Self::Storage, pc: u32, register: r::native_return::Register, returned: &mut Option<r::Pop>) -> Result<(), Self::Error> {
        self.services.resolver_pop(&mut o.owners, pc, register, returned)
    }
    fn ret(&mut self, o: &mut Self::Storage, returned: &mut Option<r::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.resolver_ret(&mut o.owners, returned)
    }
}
