use super::crt_query_context::{SourceCrtQueryContext58db7c, SourceCrtQueryContextProgress,
    SourceCrtQueryContextRequest, SourceCrtQueryContextCallKind, SourceCrtQueryContextReceipt,
    SourceCrtQueryContextError};
use super::crt_query_owned::SourceCrtQueryCompletion;
use super::crt_query_return::{SourceCrtQueryCallerReturned, SourceCrtQueryCallEnvelope};
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetObservation,
    validate_source_crt_native_ret};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryInitializationReturnedRegisters { pub rbx: u64, pub rsi: u64, pub rdi: u64, pub rbp: u64 }
#[must_use = "retain SAME actual initialized-body/storage/two child caps/native/frame/FP/resource capsule"]
pub struct SourceCrtQueryInitializationAccepted<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>, pub source: SourceCrtQueryContextProgress,
}
#[must_use = "retain rejected full initializer capsule and stopped original context"]
pub struct SourceCrtQueryInitializationJoinError<P, S, H, T, N> {
    pub actual: SourceCrtQueryCallerReturned<P, S, H, T, N>, pub pending: SourceCrtQueryContextProgress,
    pub actual_frame: SourceCrtNativeRetFrame, pub observed_call: SourceCrtQueryCallEnvelope,
    pub observed_ret: SourceCrtNativeRetObservation, pub registers: SourceCrtQueryInitializationReturnedRegisters,
    pub source: Option<SourceCrtQueryContextError>,
}
impl SourceCrtQueryContext58db7c {

    pub fn accept_owned_initialization_return<P, S, H, T, N>(&mut self,
        actual: SourceCrtQueryCallerReturned<P, S, H, T, N>,
        project_call: fn(&T) -> SourceCrtQueryCallEnvelope,
        project_ret: fn(&N) -> SourceCrtNativeRetObservation,
        project_registers: fn(&P, &S) -> SourceCrtQueryInitializationReturnedRegisters)
        -> Result<SourceCrtQueryInitializationAccepted<P, S, H, T, N>, SourceCrtQueryInitializationJoinError<P, S, H, T, N>> {
        self.stop();
        let pending = self.progress(); let entry = self.entry();
        let actual_frame = (actual.owner.owner.current_frame)(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let observed_call = project_call(&actual.owner.target_owner);
        let observed_ret = project_ret(&actual.owner.actual_return);
        let registers = project_registers(&actual.owner.owner.prefix, &actual.owner.owner.services);
        let initialization = match actual.owner.owner.source() { SourceCrtQueryCompletion::Initialization(value) => Some(value), _ => None };
        let expected = match pending {
            SourceCrtQueryContextProgress::Request(SourceCrtQueryContextRequest::RequiredCall(call))
                if call.kind == SourceCrtQueryContextCallKind::Initialize => Some(call), _ => None,
        };
        let matches = match (initialization, expected) {
            (Some(initialization), Some(expected)) => {
                let child = initialization.entry();
                child.actual_module_base == entry.actual_module_base
                    && child.actual_entry_rsp == expected.actual_callee_entry_rsp
                    && Some(child.actual_receiver_rcx) == expected.source_rcx
                    && registers.rbp == child.actual_entry_rbp
                    && (registers.rbx, registers.rsi, registers.rdi) == (expected.required_rbx, expected.required_rsi, expected.required_rdi)
                    && observed_call == actual.source_call && observed_call.actual_module_base == entry.actual_module_base
                    && observed_call.source_call_pc == 0x58dc1d && observed_call.source_resume_pc == 0x58dc22
                    && observed_call.target_rva == 0x58df48
                    && observed_call.source_rcx == expected.source_rcx && observed_call.source_rdx.is_none()
                    && observed_call.actual_work_rsp == expected.actual_caller_rsp
                    && actual.actual_return.resume_pc == observed_call.source_resume_pc
                    && actual.actual_return.work_rsp == expected.required_actual_resume_rsp
                    && actual.actual_return.rax == actual_frame.actual_rax
                    && actual_frame.actual_rip == expected.required_actual_resume
                    && actual_frame.actual_rsp == expected.required_actual_resume_rsp
                    && actual.owner.expected.before == initialization.actual_ret_frame()
                    && actual.owner.expected.actual_required_target == expected.required_actual_resume
                    && validate_source_crt_native_ret(actual.owner.expected, actual.owner.actual_retired).is_ok()
                    && observed_ret == SourceCrtNativeRetObservation::Retired(actual.owner.actual_retired)
                    && actual_frame == actual.owner.actual_retired.after
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtQueryInitializationJoinError { actual, pending, actual_frame, observed_call, observed_ret, registers, source: None });
        }
        let Some(request) = expected else {
            return Err(SourceCrtQueryInitializationJoinError { actual, pending, actual_frame, observed_call, observed_ret, registers, source: None });
        };
        match self.accept_actual_completion(SourceCrtQueryContextReceipt::CallContinued { request, actual_frame,
            actual_rbx: registers.rbx, actual_rsi: registers.rsi, actual_rdi: registers.rdi, actual_extra_push: None }) {
            Ok(source) => Ok(SourceCrtQueryInitializationAccepted { actual, source }),
            Err(source) => Err(SourceCrtQueryInitializationJoinError { actual, pending, actual_frame, observed_call, observed_ret, registers, source: Some(source) }),
        }
    }
}
