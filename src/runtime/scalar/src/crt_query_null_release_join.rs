use super::crt_query_context::{SourceCrtQueryContext58db7c, SourceCrtQueryContextProgress,
    SourceCrtQueryContextRequest, SourceCrtQueryContextCallKind, SourceCrtQueryContextReceipt,
    SourceCrtQueryContextError};
use super::crt_query_owned::SourceCrtQueryCompletion;
use super::crt_query_return::{SourceCrtQueryCallerReturned, SourceCrtQueryCallEnvelope};
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetObservation,
    validate_source_crt_native_ret};

#[must_use = "retain all earlier allocation/initializer/native/frame/FP resources across NULL call"]
pub struct SourceCrtQueryNullReleaseAccepted<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>, pub source: SourceCrtQueryContextProgress,
}
#[must_use = "retain rejected full NULL-call capsule and stopped original context"]
pub struct SourceCrtQueryNullReleaseJoinError<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>, pub pending: SourceCrtQueryContextProgress,
    pub actual_frame: SourceCrtNativeRetFrame, pub observed_call: SourceCrtQueryCallEnvelope,
    pub observed_ret: SourceCrtNativeRetObservation, pub actual_rbx_rsi_rdi: (u64, u64, u64),
    pub source: Option<SourceCrtQueryContextError>,
}
impl SourceCrtQueryContext58db7c {

    pub fn accept_owned_null_release_return<P, S, H, T, N>(&mut self,
        actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
        project_call: fn(&T) -> SourceCrtQueryCallEnvelope,
        project_ret: fn(&N) -> SourceCrtNativeRetObservation,
        project_registers: fn(&P, &S) -> (u64, u64, u64))
        -> Result<SourceCrtQueryNullReleaseAccepted<P, S, H, T, N>, SourceCrtQueryNullReleaseJoinError<P, S, H, T, N>> {
        self.stop();
        let pending = self.progress(); let entry = self.entry();
        let actual_frame = (actual.owner.owner.current_frame)(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let observed_call = project_call(&actual.owner.target_owner);
        let observed_ret = project_ret(&actual.owner.actual_return);
        let actual_rbx_rsi_rdi = project_registers(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let child = match actual.owner.owner.source() { SourceCrtQueryCompletion::NullRelease(value) => Some(value), _ => None };
        let expected = match pending {
            SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::RequiredCall(call))
                if call.source_rcx == Some(0) => match call.kind {
                    SourceCrtQueryContextCallKind::ReleaseNullAfterInitialize => Some((call, 0x58dc24, 0x58dc29)),
                    SourceCrtQueryContextCallKind::ReleaseFailure => Some((call, 0x58dbf5, 0x58dbfa)), _ => None },
            _ => None,
        };
        let matches = match (child, expected) {
            (Some(child), Some((expected, call_pc, resume_pc))) => {
                let child_entry = child.entry();
                child_entry.actual_module_base == entry.actual_module_base
                    && child_entry.actual_entry_rsp == expected.actual_callee_entry_rsp && child_entry.actual_entry_rcx == 0
                    && child_entry.actual_entry_rbx_rsi_rdi == (expected.required_rbx, expected.required_rsi, expected.required_rdi)
                    && actual_rbx_rsi_rdi == child_entry.actual_entry_rbx_rsi_rdi
                    && observed_call == actual.source_call && observed_call.actual_module_base == entry.actual_module_base
                    && observed_call.source_call_pc == call_pc && observed_call.source_resume_pc == resume_pc
                    && observed_call.target_rva == 0x58d910
                    && observed_call.source_rcx == Some(0) && observed_call.source_rdx.is_none()
                    && observed_call.actual_work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.resume_pc == observed_call.source_resume_pc
                    && actual.actual_return.work_rsp == expected.required_actual_resume_rsp
                    && actual.actual_return.rax == actual_frame.actual_rax
                    && actual_frame.actual_rax == child_entry.actual_entry_rax
                    && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.required_actual_resume_rsp
                    && actual.owner.expected.before == child.actual_ret_frame()
                    && actual.owner.expected.actual_required_target == expected.required_actual_resume
                    && validate_source_crt_native_ret(actual.owner.expected, actual.owner.actual_retired).is_ok()
                    && observed_ret == SourceCrtNativeRetObservation::Retired(actual.owner.actual_retired)
                    && actual_frame == actual.owner.actual_retired.after
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtQueryNullReleaseJoinError { actual, pending, actual_frame, observed_call, observed_ret, actual_rbx_rsi_rdi, source: None });
        }
        let Some((request, _, _)) = expected else {
            return Err(SourceCrtQueryNullReleaseJoinError { actual, pending, actual_frame, observed_call, observed_ret, actual_rbx_rsi_rdi, source: None });
        };
        match self.accept_actual_completion(SourceCrtQueryContextReceipt::CallContinued { request, actual_frame,
            actual_rbx: actual_rbx_rsi_rdi.0, actual_rsi: actual_rbx_rsi_rdi.1, actual_rdi: actual_rbx_rsi_rdi.2, actual_extra_push: None }) {
            Ok(source) => Ok(SourceCrtQueryNullReleaseAccepted { actual, source }),
            Err(source) => Err(SourceCrtQueryNullReleaseJoinError { actual, pending, actual_frame, observed_call, observed_ret, actual_rbx_rsi_rdi, source: Some(source) }),
        }
    }
}
