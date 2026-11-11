use super::{Scope, MemberDuplicateCopyServices};
use super::super as c;
use c::object_lookup as l;
use l::member_construction as m;
use m::duplicate as d;
use d::copy as w;
use w::{diagnostic as a, diagnostic_call as ac};
pub trait MemberDuplicateCopyDiagnosticServices: MemberDuplicateCopyServices {
    type LookupMemberDuplicateCopyDiagnosticCallOwner;

    fn lookup_member_duplicate_copy_diagnostic_and(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyDiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_copy_diagnostic_dispatch(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyDiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_copy_diagnostic_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_copy_diagnostic_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_member_duplicate_copy_diagnostic_ret(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::LookupMemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_completed_lookup_member_duplicate_copy_diagnostic(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, member: &mut m::Calls<Self::LookupMemberCallOwner>, duplicate: &mut d::Calls<Self::LookupMemberDuplicateCallOwner>, copy: &mut w::Calls<Self::LookupMemberDuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::LookupMemberDuplicateCopyDiagnosticCallOwner>>) -> Result<(), Self::Error>;
}
impl<S: MemberDuplicateCopyDiagnosticServices> l::member_call::DuplicateCopyDiagnosticServices for Scope<S> {
    type MemberDuplicateCopyDiagnosticCallOwner = S::LookupMemberDuplicateCopyDiagnosticCallOwner;
    fn member_duplicate_copy_diagnostic_and(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, pc: u32, address: u64, returned: &mut Option<a::AndReturned>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_diagnostic_and(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, pc, address, returned)
    }
    fn member_duplicate_copy_diagnostic_dispatch(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, request: a::CallRequest) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_diagnostic_dispatch(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, request)
    }
    fn member_duplicate_copy_diagnostic_sub(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_entry::Subtract>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_diagnostic_sub(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, returned)
    }
    fn member_duplicate_copy_diagnostic_add(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_diagnostic_add(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, returned)
    }
    fn member_duplicate_copy_diagnostic_ret(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, call: &mut Option<a::CallReturned<Self::MemberDuplicateCopyDiagnosticCallOwner>>, returned: &mut Option<a::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.lookup_member_duplicate_copy_diagnostic_ret(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, call, returned)
    }
    fn adopt_completed_member_duplicate_copy_diagnostic(&mut self, o: &mut l::Owners<Self>, member: &mut m::Calls<Self::MemberCallOwner>, duplicate: &mut d::Calls<Self::MemberDuplicateCallOwner>, copy: &mut w::Calls<Self::MemberDuplicateCopyCallOwner>, history: ac::History, held: &mut Option<ac::Retained<Self::MemberDuplicateCopyDiagnosticCallOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_lookup_member_duplicate_copy_diagnostic(&mut o.storage.owners, &mut o.calls, member, duplicate, copy, history, held)
    }
}
