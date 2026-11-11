pub use super::number_construction as number;
#[path = "parameter_number_call_provider.rs"]
mod provider;
pub use provider::{Scope, Services, TextCopyServices, TextCopyEntryServices};
#[path = "parameter_number_builder_provider.rs"]
mod builder_provider;
pub use builder_provider::{BuilderServices, BuilderEntryServices};
#[path = "parameter_number_format_provider.rs"]
mod format_provider;
pub use format_provider::FormatLookupMemberQueryServices;
pub use format_provider::FormatLookupMemberDuplicateServices;
pub use format_provider::FormatLookupMemberDuplicateCopyServices;
pub use format_provider::FormatLookupMemberDuplicateCopyAccessorServices;
pub use format_provider::FormatLookupMemberDuplicateCopyDiagnosticServices;
pub use format_provider::FormatLookupMemberDuplicateCopyDiagnosticDispatchServices;
pub use format_provider::{FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerServices, FormatLookupMemberDuplicateCopyDiagnosticDispatchMetadataServices};
pub use format_provider::{FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerContextServices, FormatLookupMemberDuplicateCopyDiagnosticDispatchHandlerEncodedSlotServices};
pub use format_provider::{FormatServices, FormatConstructorServices, FormatLookupServices, FormatLookupFallbackServices, FormatLookupRegistrationServices, FormatLookupMemberServices, FormatLookupGuardServices, FormatLookupWrapperServices, FormatLookupInitializationServices, FormatLookupResolverServices, FormatLookupResolverEntryServices, FormatLookupResolverProtectedServices, FormatLookupResolverResolutionServices, FormatLookupResolverLoadServices, FormatLookupResolverPriorCleanupServices, Resources as FormatResources};
#[path = "parameter_number_call_entry.rs"]
pub mod native_entry;
pub type Calls<R> = [Option<number::CallReturned<R>>; 7];
#[derive(Clone, Copy, Debug)]
pub struct ActualCall {
    pub call_pc: u32, pub caller_rsp: u64, pub callee_entry_rsp: u64, pub callee_rdx: u64,
    pub original_rdi: u64, pub original_xmm6: [u32; 4], pub callee: number::ActualBody,
}
#[derive(Clone, Copy, Debug)]
pub struct History {
    pub actual: ActualCall, pub body: number::Captures, pub native: number::native_return::History,
}

pub struct ParentState<S: super::Services> {
    owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    phase: super::Phase, attempted: Option<super::Phase>, cursor: u64, remaining: u32, stopped: bool,
    binding: ActualCall,
}
impl<S: super::Services> ParentState<S> {
    fn into_parent(self, services: S) -> super::Prepared<S> { super::Prepared { services, owners: self.owners,
        actual: self.actual, captures: self.captures, phase: self.phase, attempted: self.attempted,
        cursor: self.cursor, remaining: self.remaining, stopped: self.stopped } }
}
pub enum Admission { NotPendingFormat, MissingState, CallBinding, BodyEntry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission }
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress {
    pub history: number::native_return::History, pub phase: number::native_return::Phase,
    pub pop: usize, pub attempted: Option<(number::native_return::Phase, usize)>, pub stopped: bool,
}
pub struct Retained<R> {
    pub actual: ActualCall, pub calls: Calls<R>, pub body_actual: number::ActualBody, pub body: number::Captures,
    pub saved: Option<number::Nonvolatile>, pub phase: number::Phase, pub attempted: Option<number::Phase>, pub stopped: bool,
    pub native: Option<NativeProgress>,
}
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub retained: Retained<S::NumberCallOwner> }
pub enum ReturnFault<E> { ReturnBinding, Adoption(E), MissingTemporary, ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> {
    pub parent: super::Parts<S>, pub retained: Option<Retained<S::NumberCallOwner>>, pub history: History, pub fault: ReturnFault<S::Error>,
}
impl<S: Services> super::Prepared<S> {

    pub fn bind_actual_number_body(mut self, actual: ActualCall) -> Result<number::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::Format { return Err(Rejected { parent: self, actual, fault: Admission::NotPendingFormat }); }
        let Some(request) = super::format_request(self.actual, &self.captures) else {
            return Err(Rejected { parent: self, actual, fault: Admission::MissingState });
        };
        self.captures.format_request = Some(request);
        let Some(limit) = self.captures.limit else { return Err(Rejected { parent: self, actual, fault: Admission::MissingState }); };
        let value_low = u64::from(actual.callee.value_xmm1[0]) | (u64::from(actual.callee.value_xmm1[1]) << 32);
        if actual.call_pc != request.pc || actual.caller_rsp != self.actual.rbp.wrapping_sub(0x40)
            || actual.callee_entry_rsp != actual.caller_rsp.wrapping_sub(8)
            || actual.callee.rbp != actual.callee_entry_rsp.wrapping_sub(0x78)
            || actual.callee.actual_image_base != self.actual.actual_image_base
            || actual.callee.output_rcx != request.destination_rcx || actual.callee_rdx != request.unchanged_rdx
            || actual.callee.precision_r8d != request.digits_r8d || actual.callee.flag_r9d != request.fourth_r9d
            || actual.callee.entry_rbx != u64::from(limit) || value_low != request.value_xmm1_f64_bits {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        if actual.callee.pc != 0x1cae9f { return Err(Rejected { parent: self, actual, fault: Admission::BodyEntry }); }
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures,
            phase: self.phase, attempted: self.attempted, cursor: self.cursor, remaining: self.remaining,
            stopped: true, binding: actual };
        let input = number::Input { services: Scope { services: self.services }, storage, actual: actual.callee };
        match number::Prepared::from_actual_body(input) {
            Ok(body) => Ok(body),
            Err(rejected) => Err(Rejected { parent: rejected.input.storage.into_parent(rejected.input.services.services), actual, fault: Admission::BodyEntry }),
        }
    }
}
impl<S: Services> number::Parts<Scope<S>> {
    pub fn into_preserved_formatter(self) -> Preserved<S> { preserve(self, None) }
}
impl<S: Services> number::native_return::Parts<Scope<S>> {
    pub fn into_preserved_formatter(self) -> Preserved<S> {
        let progress = NativeProgress { history: number::native_return::History { input: self.input, captures: self.captures },
            phase: self.phase, pop: self.pop, attempted: self.attempted, stopped: self.stopped };
        preserve(self.body, Some(progress))
    }
}
fn preserve<S: Services>(body: number::Parts<Scope<S>>, native: Option<NativeProgress>) -> Preserved<S> {
    let state = body.owners.storage;
    let retained = Retained { actual: state.binding, calls: body.owners.calls, body_actual: body.actual,
        body: body.captures, saved: body.saved, phase: body.phase, attempted: body.attempted, stopped: body.stopped, native };
    Preserved { parent: state.into_parent(body.services.services).into_parts(), retained }
}
impl<S: Services> number::native_return::Returned<Scope<S>> {

    pub fn into_formatter_parent(self) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let returned = *self.actual_return();
        let native_history = self.history();
        let parts = self.into_parts();
        let state = parts.body.owners.storage;
        let history = History { actual: state.binding, body: parts.body.captures, native: native_history };
        let mut retained = Some(Retained { actual: state.binding, calls: parts.body.owners.calls,
            body_actual: parts.body.actual, body: parts.body.captures, saved: parts.body.saved,
            phase: parts.body.phase, attempted: parts.body.attempted, stopped: parts.body.stopped,
            native: Some(NativeProgress { history: native_history, phase: parts.phase, pop: parts.pop,
                attempted: parts.attempted, stopped: parts.stopped }) });
        let binding = state.binding;
        let mut parent = state.into_parent(parts.body.services.services);
        if returned.target != parent.actual.actual_image_base.wrapping_add(0x2b6d66)
            || returned.after_rsp != binding.caller_rsp || returned.rax != parent.actual.rbp.wrapping_sub(0x10)
            || parent.captures.limit.map(u64::from) != Some(returned.restored[0])
            || returned.restored[1] != binding.original_rdi || returned.restored[2] != parent.actual.output_rdx
            || returned.restored[3] != parent.actual.rbp || returned.xmm6 != binding.original_xmm6 {
            return Err(ReturnRejected { parent: parent.into_parts(), retained, history, fault: ReturnFault::ReturnBinding });
        }
        if let Err(error) = parent.services.adopt_completed_number(&mut parent.owners, history, &mut retained) {
            return Err(ReturnRejected { parent: parent.into_parts(), retained, history, fault: ReturnFault::Adoption(error) });
        }
        if parent.owners.temporary.is_none() {
            return Err(ReturnRejected { parent: parent.into_parts(), retained, history, fault: ReturnFault::MissingTemporary });
        }
        if retained.is_some() {
            return Err(ReturnRejected { parent: parent.into_parts(), retained, history, fault: ReturnFault::ResourcesNotAdopted });
        }
        parent.captures.number_call = Some(history); parent.record_format_return();
        parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
