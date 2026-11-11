use super::crt_query_calloc::SourceCrtCallocArguments;
use super::crt_query_calloc::{SourceCrtQueryCalloc58f074, SourceCrtQueryCallocProgress,
    SourceCrtQueryCallocRequest, SourceCrtCallocCallKind, SourceCrtQueryCallocReceipt,
    SourceCrtQueryCallocError};
use super::crt_query_context::{SourceCrtQueryContext58db7c, SourceCrtQueryContextProgress,
    SourceCrtQueryContextRequest, SourceCrtQueryContextCallKind, SourceCrtQueryContextReceipt,
    SourceCrtQueryContextError};
use super::crt_query_owned::SourceCrtQueryCompletion;
use super::crt_query_return::{SourceCrtQueryCallerReturned, SourceCrtQueryCallEnvelope};
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetObservation,
    validate_source_crt_native_ret};

#[must_use = "retain SAME actual allocation disposition, native/frame/FP and caller resources"]
pub struct SourceCrtQueryCallocAccepted<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub source: SourceCrtQueryContextProgress,
}

#[must_use = "retain actual fresh mode read/native return/frame/FP/caller owners"]
pub struct SourceCrtQueryModeAccepted<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub source: SourceCrtQueryCallocProgress,
}
#[must_use = "retain rejected mode capsule with stopped original calloc"]
pub struct SourceCrtQueryModeJoinError<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub pending: SourceCrtQueryCallocProgress,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub observed_call: SourceCrtQueryCallEnvelope,
    pub observed_ret: SourceCrtNativeRetObservation,
    pub actual_rbx: u64,
    pub source: Option<SourceCrtQueryCallocError>,
}
impl SourceCrtQueryCalloc58f074 {

    pub fn accept_owned_mode_return<P, S, H, T, N>(&mut self,
        actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
        project_call: fn(&T) -> SourceCrtQueryCallEnvelope,
        project_ret: fn(&N) -> SourceCrtNativeRetObservation,
        project_rbx: fn(&P, &S) -> u64)
        -> Result<SourceCrtQueryModeAccepted<P, S, H, T, N>, SourceCrtQueryModeJoinError<P, S, H, T, N>> {
        self.stop();
        let pending = self.progress();
        let entry = self.entry();
        let actual_frame = (actual.owner.owner.current_frame)(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let observed_call = project_call(&actual.owner.target_owner);
        let observed_ret = project_ret(&actual.owner.actual_return);
        let actual_rbx = project_rbx(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let mode = match actual.owner.owner.source() { SourceCrtQueryCompletion::Mode(value) => Some(value), _ => None };
        let expected = match pending {
            SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::RequiredCall(call))
                if call.kind == SourceCrtCallocCallKind::Mode59bfc8 => Some(call), _ => None,
        };
        let matches = match (mode, expected) {
            (Some(mode), Some(expected)) => {
                let child = mode.entry();
                child.actual_module_base == entry.actual_module_base
                    && child.actual_entry_rsp == expected.actual_caller_rsp.wrapping_sub(8)
                    && child.actual_entry_rbx == expected.required_bytes_rbx && actual_rbx == expected.required_bytes_rbx
                    && observed_call == actual.source_call && observed_call.actual_module_base == entry.actual_module_base
                    && observed_call.source_call_pc == 0x58f0a5 && observed_call.source_resume_pc == 0x58f0aa
                    && observed_call.target_rva == 0x59bfc8
                    && observed_call.source_rcx.is_none() && observed_call.source_rdx.is_none()
                    && observed_call.actual_work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.resume_pc == observed_call.source_resume_pc
                    && actual.actual_return.work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.rax == actual_frame.actual_rax
                    && actual_frame.actual_rax == u64::from(mode.actual_read_value())
                    && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.actual_caller_rsp
                    && actual.owner.expected.before == mode.actual_ret_frame()
                    && actual.owner.expected.actual_required_target == expected.required_actual_resume
                    && validate_source_crt_native_ret(actual.owner.expected, actual.owner.actual_retired).is_ok()
                    && observed_ret == SourceCrtNativeRetObservation::Retired(actual.owner.actual_retired)
                    && actual_frame == actual.owner.actual_retired.after
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtQueryModeJoinError { actual, pending, actual_frame, observed_call, observed_ret, actual_rbx, source: None });
        }
        let Some(request) = expected else {
            return Err(SourceCrtQueryModeJoinError { actual, pending, actual_frame, observed_call, observed_ret, actual_rbx, source: None });
        };
        match self.accept_actual_completion(SourceCrtQueryCallocReceipt::CallReturned { request, actual_frame, actual_rbx }) {
            Ok(source) => Ok(SourceCrtQueryModeAccepted { actual, source }),
            Err(source) => Err(SourceCrtQueryModeJoinError { actual, pending, actual_frame, observed_call, observed_ret, actual_rbx, source: Some(source) }),
        }
    }
}
#[must_use = "retain entire rejected calloc capsule and stopped original context cursor"]
pub struct SourceCrtQueryCallocJoinError<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
    pub pending: SourceCrtQueryContextProgress,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub observed_call: SourceCrtQueryCallEnvelope,
    pub observed_ret: SourceCrtNativeRetObservation,
    pub actual_rbx_rsi_rdi: (u64, u64, u64),
    pub source: Option<SourceCrtQueryContextError>,
}
impl SourceCrtQueryContext58db7c {

    pub fn accept_owned_calloc_return<P, S, H, T, N>(&mut self,
        actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
        project_call: fn(&T) -> SourceCrtQueryCallEnvelope,
        project_ret: fn(&N) -> SourceCrtNativeRetObservation,
        project_nonvolatile: fn(&P, &S) -> (u64, u64, u64))
        -> Result<SourceCrtQueryCallocAccepted<P, S, H, T, N>, SourceCrtQueryCallocJoinError<P, S, H, T, N>> {
        self.stop();
        let pending = self.progress();
        let actual_frame = (actual.owner.owner.current_frame)(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let observed_call = project_call(&actual.owner.target_owner);
        let observed_ret = project_ret(&actual.owner.actual_return);
        let actual_rbx_rsi_rdi = project_nonvolatile(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let entry = self.entry();
        let child = match actual.owner.owner.source() { SourceCrtQueryCompletion::Calloc(value) => Some(value), _ => None };
        let expected = match pending {
            SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::RequiredCall(call))
                if call.kind == SourceCrtQueryContextCallKind::Allocate => Some(call), _ => None,
        };
        let matches = match (child, expected) {
            (Some(child), Some(expected)) => {
                let child_entry = child.entry();
                child_entry.arguments == SourceCrtCallocArguments::ContextOneBy3c8
                    && child_entry.actual_module_base == entry.actual_module_base
                    && child_entry.actual_entry_rsp == expected.actual_callee_entry_rsp
                    && child_entry.actual_entry_rbx == expected.required_rbx
                    && child_entry.actual_count_rcx == 1 && child_entry.actual_element_rdx == 0x3c8
                    && observed_call == actual.source_call
                    && observed_call.actual_module_base == entry.actual_module_base
                    && observed_call.source_call_pc == 0x58dbd9 && observed_call.source_resume_pc == 0x58dbde
                    && observed_call.target_rva == 0x58f074
                    && observed_call.source_rcx == Some(1) && observed_call.source_rdx == Some(0x3c8)
                    && observed_call.actual_work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.resume_pc == observed_call.source_resume_pc
                    && actual.actual_return.work_rsp == expected.required_actual_resume_rsp
                    && actual.actual_return.rax == actual_frame.actual_rax
                    && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.required_actual_resume_rsp
                    && actual_rbx_rsi_rdi == (expected.required_rbx, expected.required_rsi, expected.required_rdi)
                    && actual.owner.expected.before == child.actual_ret_frame()
                    && actual.owner.expected.actual_required_target == expected.required_actual_resume
                    && validate_source_crt_native_ret(actual.owner.expected, actual.owner.actual_retired).is_ok()
                    && observed_ret == SourceCrtNativeRetObservation::Retired(actual.owner.actual_retired)
                    && actual_frame == actual.owner.actual_retired.after
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtQueryCallocJoinError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rsi_rdi, source: None });
        }
        let Some(request) = expected else {
            return Err(SourceCrtQueryCallocJoinError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rsi_rdi, source: None });
        };
        match self.accept_actual_completion(SourceCrtQueryContextReceipt::CallContinued {
            request, actual_frame, actual_rbx: actual_rbx_rsi_rdi.0, actual_rsi: actual_rbx_rsi_rdi.1,
            actual_rdi: actual_rbx_rsi_rdi.2, actual_extra_push: None }) {
            Ok(source) => Ok(SourceCrtQueryCallocAccepted { actual, source }),
            Err(source) => Err(SourceCrtQueryCallocJoinError { actual, pending, actual_frame, observed_call,
                observed_ret, actual_rbx_rsi_rdi, source: Some(source) }),
        }
    }
}
