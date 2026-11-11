use super::{Scope, DuplicateCopyServices};
use super::super as l;
use l::member_construction as m;
use m::duplicate as d;
use d::copy as w;
use w::{diagnostic as a, diagnostic_call as ac};
pub trait DuplicateCopyDiagnosticServices: DuplicateCopyServices {
    type MemberDuplicateCopyDiagnosticCallOwner;

    fn member_duplicate_copy_diagnostic_and(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_diagnostic_dispatch(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn member_duplicate_copy_diagnostic_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_diagnostic_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn member_duplicate_copy_diagnostic_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_member_duplicate_copy_diagnostic(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::MemberDuplicateCopyDiagnosticCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: DuplicateCopyDiagnosticServices> m::duplicate_call::CopyDiagnosticServices for Scope<S> {
    type DuplicateCopyDiagnosticCallOwner = S::MemberDuplicateCopyDiagnosticCallOwner;
    fn duplicate_copy_diagnostic_and(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_diagnostic_and(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, pc, address, returned)
    }
    fn duplicate_copy_diagnostic_dispatch(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_diagnostic_dispatch(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, request)
    }
    fn duplicate_copy_diagnostic_sub(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_diagnostic_sub(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, returned)
    }
    fn duplicate_copy_diagnostic_add(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_diagnostic_add(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, returned)
    }
    fn duplicate_copy_diagnostic_ret(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::DuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.member_duplicate_copy_diagnostic_ret(&mut o.storage.owners, &mut o.calls, duplicate, copy, call, returned)
    }
    fn adopt_completed_duplicate_copy_diagnostic(&mut self, o: &mut m::Owners<Self>, duplicate: &mut d::Calls<Self::DuplicateCallOwner>, copy: &mut w::Calls<Self::DuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::DuplicateCopyDiagnosticCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_member_duplicate_copy_diagnostic(&mut o.storage.owners, &mut o.calls, duplicate, copy, history, held)
    }
}
