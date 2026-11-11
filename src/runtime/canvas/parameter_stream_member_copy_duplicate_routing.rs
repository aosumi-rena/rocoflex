use super::{Scope, Services as DuplicateServices};
use super::super as m;
use m::duplicate as d;
use d::{copy as w, copy_call as wc};
pub trait CopyServices: DuplicateServices {
    type DuplicateCopyCallOwner;

    fn duplicate_copy_call(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, request: w::CallRequest) -> Result<(), Self::Error>;
    fn duplicate_copy_read_word16(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error>;
    fn duplicate_copy_write_word16(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error>;
    fn duplicate_copy_write_dword(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error>;
    fn duplicate_copy_push(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Push>) -> Result<(), Self::Error>;
    fn duplicate_copy_sub(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_copy_add(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_copy_pop(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_return::Pop>) -> Result<(), Self::Error>;
    fn duplicate_copy_ret(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, returned: &mut Option<w::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate_copy(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, history: wc::History, held: &mut Option<wc::Retained<Self::DuplicateCopyCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: CopyServices> wc::Services for Scope<S> {
    type CopyCallOwner = S::DuplicateCopyCallOwner;
    fn copy_call(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, request: w::CallRequest) -> Result<(), Self::Error> {
        self.services.duplicate_copy_call(&mut o.storage.owners, &mut o.calls, copy, request)
    }
    fn copy_read_word16(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, pc: u32, address: u64) -> Result<u16, Self::Error> {
        self.services.duplicate_copy_read_word16(&mut o.storage.owners, &mut o.calls, copy, pc, address)
    }
    fn copy_write_word16(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, pc: u32, address: u64, word: u16) -> Result<(), Self::Error> {
        self.services.duplicate_copy_write_word16(&mut o.storage.owners, &mut o.calls, copy, pc, address, word)
    }
    fn copy_write_dword(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, pc: u32, address: u64, word: u32) -> Result<(), Self::Error> {
        self.services.duplicate_copy_write_dword(&mut o.storage.owners, &mut o.calls, copy, pc, address, word)
    }
    fn copy_push(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, returned: &mut Option<w::native_entry::Push>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_push(&mut o.storage.owners, &mut o.calls, copy, returned)
    }
    fn copy_sub(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, returned: &mut Option<w::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_sub(&mut o.storage.owners, &mut o.calls, copy, returned)
    }
    fn copy_add(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, returned: &mut Option<w::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_add(&mut o.storage.owners, &mut o.calls, copy, returned)
    }
    fn copy_pop(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, returned: &mut Option<w::native_return::Pop>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_pop(&mut o.storage.owners, &mut o.calls, copy, returned)
    }
    fn copy_ret(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, returned: &mut Option<w::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_ret(&mut o.storage.owners, &mut o.calls, copy, returned)
    }
    fn adopt_completed_copy(&mut self, o: &mut d::Owners<Self>, history: wc::History, held: &mut Option<wc::Retained<Self::CopyCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_duplicate_copy(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
