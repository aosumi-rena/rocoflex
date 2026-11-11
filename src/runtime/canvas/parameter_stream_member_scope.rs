use super::{m, ParentState, History, Retained};
pub trait Services: super::super::Services {
    type MemberCallOwner;

    fn member_call(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, request: m::CallRequest) -> Result<(), Self::Error>;
    fn member_read_word(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn member_write_word(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;
    fn member_write_dword(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn member_read_vector(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error>;
    fn member_write_vector(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error>;
    fn member_and_zero(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error>;
    fn member_push(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error>;
    fn member_sub(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn member_move_rax(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error>;
    fn member_add(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn member_pop(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error>;
    fn member_ret(&mut self, o: &mut super::super::Owners<Self>, calls: &mut m::Calls<Self::MemberCallOwner>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_member(&mut self, o: &mut super::super::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::MemberCallOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> m::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::MemberCallOwner; type Error = S::Error;
    fn call(&mut self, o: &mut m::Owners<Self>, request: m::CallRequest) -> Result<(), Self::Error> {
        self.services.member_call(&mut o.storage.owners, &mut o.calls, request)
    }
    fn read_word(&mut self, o: &mut m::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.member_read_word(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn write_word(&mut self, o: &mut m::Owners<Self>, pc: u32, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.member_write_word(&mut o.storage.owners, &mut o.calls, pc, address, word)
    }
    fn write_dword(&mut self, o: &mut m::Owners<Self>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.member_write_dword(&mut o.storage.owners, &mut o.calls, pc, address, word)
    }
    fn read_vector(&mut self, o: &mut m::Owners<Self>, pc: u32, address: u64) -> Result<[u8; 16], Self::Error> {
        self.services.member_read_vector(&mut o.storage.owners, &mut o.calls, pc, address)
    }
    fn write_vector(&mut self, o: &mut m::Owners<Self>, pc: u32, address: u64, bits: [u8; 16]) -> Result<(), Self::Error> {
        self.services.member_write_vector(&mut o.storage.owners, &mut o.calls, pc, address, bits)
    }
    fn and_dword_zero(&mut self, o: &mut m::Owners<Self>, pc: u32, address: u64, returned: &mut Option<m::AndDwordReturned>) -> Result<(), Self::Error> {
        self.services.member_and_zero(&mut o.storage.owners, &mut o.calls, pc, address, returned)
    }
}
impl<S: Services> m::native_entry::Services for Scope<S> {
    fn push_rbx_5612f0(&mut self, o: &mut m::Owners<Self>, returned: &mut Option<m::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.member_push(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn sub_rsp_20_5612f2(&mut self, o: &mut m::Owners<Self>, returned: &mut Option<m::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.member_sub(&mut o.storage.owners, &mut o.calls, returned)
    }
}
impl<S: Services> m::native_return::Services for Scope<S> {
    fn mov_rax_rbx_5613a5(&mut self, o: &mut m::Owners<Self>, receiver: u64, returned: &mut Option<u64>) -> Result<(), Self::Error> {
        self.services.member_move_rax(&mut o.storage.owners, &mut o.calls, receiver, returned)
    }
    fn add_rsp_20_5613a8(&mut self, o: &mut m::Owners<Self>, returned: &mut Option<m::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.member_add(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn pop_rbx_5613ac(&mut self, o: &mut m::Owners<Self>, returned: &mut Option<m::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.member_pop(&mut o.storage.owners, &mut o.calls, returned)
    }
    fn ret_5613ad(&mut self, o: &mut m::Owners<Self>, returned: &mut Option<m::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.member_ret(&mut o.storage.owners, &mut o.calls, returned)
    }
}
