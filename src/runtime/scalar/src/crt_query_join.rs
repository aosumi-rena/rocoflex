use super::crt_query_resolve::{SourceCrtQueryResolve5941fc, SourceCrtQueryResolveProgress,
    SourceCrtQueryResolveRequest, SourceCrtQueryResolveCallKind, SourceCrtQueryResolveReceipt,
    SourceCrtQueryResolveError};
use super::crt_query_owned::SourceCrtQueryCompletion;
use super::crt_query_return::{SourceCrtQueryCallerReturned, SourceCrtQueryCallEnvelope};
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetObservation,
    validate_source_crt_native_ret};
use super::crt_query_refresh::{SourceCrtQueryRefresh58e238, SourceCrtQueryRefreshProgress,
    SourceCrtQueryRefreshRequest, SourceCrtQueryRefreshReceipt, SourceCrtQueryRefreshError};

#[must_use = "retain actual resolver/fresh-context/frame/FP/native/resources through original-slot store"]
pub struct SourceCrtQueryResolverAccepted<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub source: SourceCrtQueryRefreshProgress,
}
#[must_use = "retain entire rejected actual resolver capsule with stopped original refresh"]
pub struct SourceCrtQueryResolverError<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub pending: SourceCrtQueryRefreshProgress,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub observed_call: SourceCrtQueryCallEnvelope,
    pub observed_ret: SourceCrtNativeRetObservation,
    pub actual_rbx: u64,
    pub source: Option<SourceCrtQueryRefreshError>,
}
impl SourceCrtQueryRefresh58e238 {

    pub fn accept_owned_resolver_return<P, S, H, T, N>(&mut self,
        actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
        project_call: fn(&T) -> SourceCrtQueryCallEnvelope,
        project_ret: fn(&N) -> SourceCrtNativeRetObservation,
        project_rbx: fn(&P, &S) -> u64)
        -> Result<SourceCrtQueryResolverAccepted<P, S, H, T, N>, SourceCrtQueryResolverError<P, S, H, T, N>> {
        self.stop();
        let pending = self.progress();
        let actual_frame = (actual.owner.owner.current_frame)(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let observed_call = project_call(&actual.owner.target_owner);
        let observed_ret = project_ret(&actual.owner.actual_return);
        let actual_rbx = project_rbx(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let entry = self.entry();
        let resolver = match actual.owner.owner.source() { SourceCrtQueryCompletion::Resolve(value) => Some(value), _ => None };
        let expected = match pending {
            SourceCrtQueryRefreshProgress::Request(request @ SourceCrtQueryRefreshRequest::Required5941fc { .. }) => Some(request), _ => None,
        };
        let matches = match resolver {
            Some(resolver) if expected.is_some() => {
                let child = resolver.entry();
                child.actual_module_base == entry.actual_module_base
                    && child.actual_entry_rsp == entry.actual_entry_rsp.wrapping_sub(0x30)
                    && child.actual_entry_rbx == entry.actual_slot_rdx && actual_rbx == entry.actual_slot_rdx
                    && child.actual_entry_rax == self.source_rax()
                    && child.actual_entry_rcx == entry.actual_thread_rcx && child.actual_entry_rdx == entry.actual_slot_rdx
                    && observed_call == actual.source_call && observed_call.actual_module_base == entry.actual_module_base
                    && observed_call.source_call_pc == 0x58e25b && observed_call.source_resume_pc == 0x58e260
                    && observed_call.target_rva == 0x5941fc
                    && observed_call.source_rcx == Some(entry.actual_thread_rcx)
                    && observed_call.source_rdx == Some(entry.actual_slot_rdx)
                    && observed_call.actual_work_rsp == entry.actual_entry_rsp.wrapping_sub(0x28)
                    && actual.actual_return.resume_pc == observed_call.source_resume_pc
                    && actual.actual_return.work_rsp == observed_call.actual_work_rsp
                    && actual.actual_return.rax == actual_frame.actual_rax
                    && actual_frame.actual_rip == entry.actual_module_base.wrapping_add(0x58e260)
                    && actual_frame.actual_rsp == observed_call.actual_work_rsp
                    && actual.owner.expected.before == resolver.actual_ret_frame()
                    && actual.owner.expected.actual_required_target == actual_frame.actual_rip
                    && validate_source_crt_native_ret(actual.owner.expected, actual.owner.actual_retired).is_ok()
                    && observed_ret == SourceCrtNativeRetObservation::Retired(actual.owner.actual_retired)
                    && actual_frame == actual.owner.actual_retired.after
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtQueryResolverError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx, source: None });
        }
        let Some(request) = expected else {
            return Err(SourceCrtQueryResolverError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx, source: None });
        };
        match self.accept_actual_completion(SourceCrtQueryRefreshReceipt::Returned5941fc { request, actual_frame, actual_rbx }) {
            Ok(source) => Ok(SourceCrtQueryResolverAccepted { actual, source }),
            Err(source) => Err(SourceCrtQueryResolverError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx, source: Some(source) }),
        }
    }
}

#[must_use = "retain entire fresh helper/caller/protected/native/frame/FP/resource capsule"]
pub struct SourceCrtQueryFreshContextAccepted<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub source: SourceCrtQueryResolveProgress,
}
#[must_use = "retain exact stopped source and entire rejected fresh-context capsule"]
pub struct SourceCrtQueryFreshContextError<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub pending: SourceCrtQueryResolveProgress,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub observed_call: SourceCrtQueryCallEnvelope,
    pub observed_ret: SourceCrtNativeRetObservation,
    pub actual_rbx_rdi: (u64, u64),
    pub source: Option<SourceCrtQueryResolveError>,
}
impl SourceCrtQueryResolve5941fc {

    pub fn accept_owned_fresh_context<P, S, H, T, N>(&mut self,
        actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
        project_call: fn(&T) -> SourceCrtQueryCallEnvelope,
        project_ret: fn(&N) -> SourceCrtNativeRetObservation,
        project_rbx_rdi: fn(&P, &S) -> (u64, u64))
        -> Result<SourceCrtQueryFreshContextAccepted<P, S, H, T, N>,
            SourceCrtQueryFreshContextError<P, S, H, T, N>> {
        self.stop();
        let pending = self.progress();
        let actual_frame = (actual.owner.owner.current_frame)(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let observed_call = project_call(&actual.owner.target_owner);
        let observed_ret = project_ret(&actual.owner.actual_return);
        let actual_rbx_rdi = project_rbx_rdi(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let context = match actual.owner.owner.source() {
            SourceCrtQueryCompletion::Context(value) => Some(value), _ => None,
        };
        let expected = match pending {
            SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::RequiredCall(call))
                if call.kind == SourceCrtQueryResolveCallKind::FreshContext58db7c => Some(call), _ => None,
        };
        let entry = self.entry();
        let matches = match (context, expected) {
            (Some(context), Some(expected)) => {
                let child = context.entry();
                child.actual_module_base == entry.actual_module_base
                    && child.actual_entry_rsp == expected.actual_caller_rsp.wrapping_sub(8)
                    && child.actual_entry_rbx == entry.actual_entry_rbx && child.actual_entry_rdi == entry.actual_entry_rdi
                    && actual_rbx_rdi == (child.actual_entry_rbx, child.actual_entry_rdi)
                    && observed_call == actual.source_call
                    && observed_call.actual_module_base == entry.actual_module_base
                    && observed_call.source_call_pc == 0x594206 && observed_call.source_resume_pc == 0x59420b
                    && observed_call.target_rva == 0x58db7c
                    && observed_call.source_rcx.is_none() && observed_call.source_rdx.is_none()
                    && observed_call.actual_work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.resume_pc == observed_call.source_resume_pc
                    && actual.actual_return.work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.rax == actual_frame.actual_rax
                    && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.actual_caller_rsp
                    && actual.owner.expected.before == context.actual_ret_frame()
                    && actual.owner.expected.actual_required_target == expected.required_actual_resume
                    && validate_source_crt_native_ret(actual.owner.expected, actual.owner.actual_retired).is_ok()
                    && observed_ret == SourceCrtNativeRetObservation::Retired(actual.owner.actual_retired)
                    && actual_frame == actual.owner.actual_retired.after
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtQueryFreshContextError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rdi, source: None });
        }
        let Some(expected) = expected else {
            return Err(SourceCrtQueryFreshContextError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rdi, source: None });
        };
        match self.accept_actual_completion(SourceCrtQueryResolveReceipt::CallReturned {
            request: expected, actual_frame, actual_rbx: actual_rbx_rdi.0, actual_rdi: actual_rbx_rdi.1 }) {
            Ok(source) => Ok(SourceCrtQueryFreshContextAccepted { actual, source }),
            Err(source) => Err(SourceCrtQueryFreshContextError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rdi, source: Some(source) }),
        }
    }
}

#[must_use = "retain original slot update/counter/retirement/native/frame/FP owners through addressed release"]
pub struct SourceCrtQueryUpdateAccepted<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub source: SourceCrtQueryResolveProgress,
}
#[must_use = "retain entire rejected actual update capsule with stopped original resolver"]
pub struct SourceCrtQueryUpdateError<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub pending: SourceCrtQueryResolveProgress,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub observed_call: SourceCrtQueryCallEnvelope,
    pub observed_ret: SourceCrtNativeRetObservation,
    pub actual_rbx_rdi: (u64, u64),
    pub source: Option<SourceCrtQueryResolveError>,
}
impl SourceCrtQueryResolve5941fc {

    pub fn accept_owned_update_return<P, S, H, T, N>(&mut self,
        actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
        project_call: fn(&T) -> SourceCrtQueryCallEnvelope,
        project_ret: fn(&N) -> SourceCrtNativeRetObservation,
        project_rbx_rdi: fn(&P, &S) -> (u64, u64))
        -> Result<SourceCrtQueryUpdateAccepted<P, S, H, T, N>, SourceCrtQueryUpdateError<P, S, H, T, N>> {
        self.stop();
        let pending = self.progress();
        let actual_frame = (actual.owner.owner.current_frame)(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let observed_call = project_call(&actual.owner.target_owner);
        let observed_ret = project_ret(&actual.owner.actual_return);
        let actual_rbx_rdi = project_rbx_rdi(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let entry = self.entry();
        let update = match actual.owner.owner.source() { SourceCrtQueryCompletion::Replace(value) => Some(value), _ => None };
        let expected = match pending {
            SourceCrtQueryResolveProgress::Request(SourceCrtQueryResolveRequest::RequiredCall(call))
                if call.kind == SourceCrtQueryResolveCallKind::Update59426c => Some(call), _ => None,
        };
        let matches = match (update, expected) {
            (Some(update), Some(expected)) => {
                let child = update.entry();
                child.actual_module_base == entry.actual_module_base
                    && child.actual_entry_rsp == expected.actual_caller_rsp.wrapping_sub(8)
                    && expected.source_rcx == Some(child.actual_slot_rcx)
                    && expected.source_rdx == Some(child.actual_incoming_rdx)
                    && expected.required_saved_rdi == Some(child.actual_entry_rdi)
                    && actual_rbx_rdi == (child.actual_entry_rbx, child.actual_entry_rdi)
                    && observed_call == actual.source_call && observed_call.actual_module_base == entry.actual_module_base
                    && observed_call.source_call_pc == 0x59423f && observed_call.source_resume_pc == 0x594244
                    && observed_call.target_rva == 0x59426c
                    && observed_call.source_rcx == expected.source_rcx && observed_call.source_rdx == expected.source_rdx
                    && observed_call.actual_work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.resume_pc == observed_call.source_resume_pc
                    && actual.actual_return.work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.rax == actual_frame.actual_rax
                    && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.actual_caller_rsp
                    && actual.owner.expected.before == update.actual_ret_frame()
                    && actual.owner.expected.actual_required_target == expected.required_actual_resume
                    && validate_source_crt_native_ret(actual.owner.expected, actual.owner.actual_retired).is_ok()
                    && observed_ret == SourceCrtNativeRetObservation::Retired(actual.owner.actual_retired)
                    && actual_frame == actual.owner.actual_retired.after
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtQueryUpdateError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rdi, source: None });
        }
        let Some(request) = expected else {
            return Err(SourceCrtQueryUpdateError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rdi, source: None });
        };
        match self.accept_actual_completion(SourceCrtQueryResolveReceipt::CallReturned {
            request, actual_frame, actual_rbx: actual_rbx_rdi.0, actual_rdi: actual_rbx_rdi.1 }) {
            Ok(source) => Ok(SourceCrtQueryUpdateAccepted { actual, source }),
            Err(source) => Err(SourceCrtQueryUpdateError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rdi, source: Some(source) }),
        }
    }
}
