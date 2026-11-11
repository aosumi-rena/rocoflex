use super::{Scope, Services as CopyServices};
use super::super as d;
use d::copy as w;
use w::{diagnostic as a, diagnostic_call as ac};
pub trait DiagnosticServices: CopyServices {
    type CopyDiagnosticCallOwner;

    fn copy_diagnostic_and(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error>;
    fn copy_diagnostic_dispatch(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn copy_diagnostic_sub(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn copy_diagnostic_add(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn copy_diagnostic_ret(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, call: &mut Option<a::CallReturned<Self::CopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_copy_diagnostic(&mut self, o: &mut d::Owners<Self>, copy: &mut w::Calls<Self::CopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::CopyDiagnosticCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: DiagnosticServices> ac::Services for Scope<S> {
    type DiagnosticCallOwner = S::CopyDiagnosticCallOwner;
    fn diagnostic_and(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_and(&mut o.storage.owners, &mut o.calls, call, pc, address, returned)
    }
    fn diagnostic_dispatch(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_dispatch(&mut o.storage.owners, &mut o.calls, call, request)
    }
    fn diagnostic_sub(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_sub(&mut o.storage.owners, &mut o.calls, call, returned)
    }
    fn diagnostic_add(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_add(&mut o.storage.owners, &mut o.calls, call, returned)
    }
    fn diagnostic_ret(&mut self, o: &mut w::Owners<Self>, call: &mut Option<a::CallReturned<Self::DiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.copy_diagnostic_ret(&mut o.storage.owners, &mut o.calls, call, returned)
    }
    fn adopt_completed_diagnostic(&mut self, o: &mut w::Owners<Self>, history: ac::History, held: &mut Option<ac::Retained<Self::DiagnosticCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_copy_diagnostic(&mut o.storage.owners, &mut o.calls, history, held)
    }
}
