use super::{g, ParentState, History, Retained};
use super::super as l;
pub trait Services: l::Services {
    type GuardCallOwner; type GuardTailOwner;

    fn guard_write_index(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn guard_read_index(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn guard_call(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, request: g::CallRequest) -> Result<(), Self::Error>;
    fn guard_mov_rax_rbx(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn guard_add_rsp(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn guard_pop_rbx(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error>;
    fn guard_ret(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn guard_tail(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, request: g::native_return::TailRequest,
        tail: &mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>) -> Result<(), Self::Error>;
    fn guard_push_rbx(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error>;
    fn guard_sub_rsp(&mut self, o: &mut l::Owners<Self>, call: &mut Option<g::CallReturned<Self::GuardCallOwner>>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error>;

    fn adopt_completed_guard(&mut self, o: &mut l::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::GuardCallOwner, Self::GuardTailOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> g::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::GuardCallOwner; type Error = S::Error;
    fn write_index(&mut self, o: &mut g::Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.guard_write_index(&mut o.storage.owners, &mut o.call, pc, address, word)
    }
    fn read_index(&mut self, o: &mut g::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.guard_read_index(&mut o.storage.owners, &mut o.call, pc, address)
    }
    fn call(&mut self, o: &mut g::Owners<Self>, request: g::CallRequest) -> Result<(), Self::Error> {
        self.services.guard_call(&mut o.storage.owners, &mut o.call, request)
    }
}
impl<S: Services> g::native_return::Services for Scope<S> {
    type TailOwner = S::GuardTailOwner;
    fn mov_rax_rbx(&mut self, o: &mut g::Owners<Self>, pc: u32, word: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.guard_mov_rax_rbx(&mut o.storage.owners, &mut o.call, pc, word, returned)
    }
    fn add_rsp(&mut self, o: &mut g::Owners<Self>, pc: u32, amount: u64, returned: &mut Option<g::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.guard_add_rsp(&mut o.storage.owners, &mut o.call, pc, amount, returned)
    }
    fn pop_rbx(&mut self, o: &mut g::Owners<Self>, pc: u32, returned: &mut Option<g::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.guard_pop_rbx(&mut o.storage.owners, &mut o.call, pc, returned)
    }
    fn ret(&mut self, o: &mut g::Owners<Self>, pc: u32, returned: &mut Option<g::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.guard_ret(&mut o.storage.owners, &mut o.call, pc, returned)
    }
    fn tail_57a7fc(&mut self, o: &mut g::Owners<Self>, request: g::native_return::TailRequest, tail: &mut Option<g::native_return::TailCallReturned<Self::TailOwner>>) -> Result<(), Self::Error> {
        self.services.guard_tail(&mut o.storage.owners, &mut o.call, request, tail)
    }
}
impl<S: Services> g::native_entry::Services for Scope<S> {
    fn push_rbx(&mut self, o: &mut g::Owners<Self>, pc: u32, returned: &mut Option<g::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.guard_push_rbx(&mut o.storage.owners, &mut o.call, pc, returned)
    }
    fn sub_rsp(&mut self, o: &mut g::Owners<Self>, pc: u32, amount: u64, returned: &mut Option<g::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.guard_sub_rsp(&mut o.storage.owners, &mut o.call, pc, amount, returned)
    }
}

