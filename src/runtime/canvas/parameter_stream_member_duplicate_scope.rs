use super::{d, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type DuplicateCallOwner;

    fn duplicate_call(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, request: d::CallRequest) -> Result<(), Self::Error>;
    fn duplicate_read_word16(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn duplicate_write_word(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn duplicate_save(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, pc: u32, register: d::native_entry::Register, returned: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error>;
    fn duplicate_push(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error>;
    fn duplicate_sub(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_restore(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, pc: u32, register: d::native_return::Register, returned: &mut Option<d::native_return::Load>) -> Result<(), Self::Error>;
    fn duplicate_add(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_pop(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error>;
    fn duplicate_ret(&mut self, o: &mut super::super::Owners<Self>, calls: &mut d::Calls<Self::DuplicateCallOwner>, returned: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate(&mut self, o: &mut super::super::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::DuplicateCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> d::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::DuplicateCallOwner; type Error = S::Error;
    fn call(&mut self, o: &mut d::Owners<Self>, request: d::CallRequest) -> Result<(), Self::Error> {
        self.services.duplicate_call(&mut o.storage.owners, &mut o.calls, request)
    }
    fn read_word16(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64) -> Result<u16, Self::Error> {
        self.services.duplicate_read_word16(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn write_word(&mut self, o: &mut d::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.duplicate_write_word(&mut o.storage.owners, &mut o.calls, pc, address, word)
    }
}
impl<S: Services> d::native_entry::Services for Scope<S> {
    fn save_register(&mut self, o: &mut d::Owners<Self>, pc: u32, register: d::native_entry::Register, returned: &mut Option<d::native_entry::Store>) -> Result<(), Self::Error> {
        self.services.duplicate_save(&mut o.storage.owners, &mut o.calls, pc, register, returned)
    }
    fn push_rdi_57c45f(&mut self, o: &mut d::Owners<Self>, returned: &mut Option<d::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.duplicate_push(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn sub_rsp_30_57c460(&mut self, o: &mut d::Owners<Self>, returned: &mut Option<d::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_sub(&mut o.storage.owners, &mut o.calls, returned)
    }
}
impl<S: Services> d::native_return::Services for Scope<S> {
    fn restore_register(&mut self, o: &mut d::Owners<Self>, pc: u32, register: d::native_return::Register, returned: &mut Option<d::native_return::Load>) -> Result<(), Self::Error> {
        self.services.duplicate_restore(&mut o.storage.owners, &mut o.calls, pc, register, returned)
    }
    fn add_rsp_30_57c4b7(&mut self, o: &mut d::Owners<Self>, returned: &mut Option<d::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_add(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn pop_rdi_57c4bb(&mut self, o: &mut d::Owners<Self>, returned: &mut Option<d::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.duplicate_pop(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn ret_57c4bc(&mut self, o: &mut d::Owners<Self>, returned: &mut Option<d::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_ret(&mut o.storage.owners, &mut o.calls, returned)
    }
}
