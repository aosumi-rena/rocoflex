use super::{Scope, CopyServices};
use super::super as m;
use m::duplicate as d;
use d::copy as w;
use w::{diagnostic as a, diagnostic_call as ac};
pub trait CopyDiagnosticServices: CopyServices {
    type DuplicateCopyDiagnosticCallOwner;

    fn duplicate_copy_diagnostic_and(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_dispatch(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_sub(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_add(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn duplicate_copy_diagnostic_ret(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_duplicate_copy_diagnostic(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::DuplicateCopyDiagnosticCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: CopyDiagnosticServices> d::copy_call::DiagnosticServices for Scope<S> {
    type CopyDiagnosticCallOwner = S::DuplicateCopyDiagnosticCallOwner;
    fn copy_diagnostic_and(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_and(&mut o.storage.owners, &mut o.calls, copy, call, pc, address, returned)
    }
    fn copy_diagnostic_dispatch(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_dispatch(&mut o.storage.owners, &mut o.calls, copy, call, request)
    }
    fn copy_diagnostic_sub(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_sub(&mut o.storage.owners, &mut o.calls, copy, call, returned)
    }
    fn copy_diagnostic_add(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_add(&mut o.storage.owners, &mut o.calls, copy, call, returned)
    }
    fn copy_diagnostic_ret(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.duplicate_copy_diagnostic_ret(&mut o.storage.owners, &mut o.calls, copy, call, returned)
    }
    fn adopt_completed_copy_diagnostic(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::CopyDiagnosticCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_duplicate_copy_diagnostic(&mut o.storage.owners, &mut o.calls, copy, history, held)
    }
}
