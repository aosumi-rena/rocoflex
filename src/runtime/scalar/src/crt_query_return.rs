use super::crt_query_owned::SourceCrtQueryBeforeRet;
use super::crt_query_owned::SourceCrtQueryCompletion;
use super::crt_storage_native_return::{SourceCrtNativeRetExpectation, SourceCrtNativeRetFrame,
    SourceCrtNativeRetObservation, SourceCrtNativeRetired, SourceCrtNativeRetPhase,
    SourceCrtNativeRetError, SourceCrtNativeReturnFailure, accept_source_crt_native_ret_receipt,
    validate_source_crt_native_ret};

#[must_use = "retain SAME before-RET source/frame/FP/native/resource capsule and actual target owner"]
pub struct SourceCrtQueryRetBindFailure<P, S, H, T> {
    pub owner: SourceCrtQueryBeforeRet<P, S, H>,
    pub target_owner: T,
    pub expected: SourceCrtNativeRetExpectation,
    pub actual_frame: SourceCrtNativeRetFrame,
}
#[must_use = "retain query owner plus independent actual caller target and pending return prefix"]
pub struct SourceCrtQueryNativeRet<P, S, H, T> {
    owner: SourceCrtQueryBeforeRet<P, S, H>,
    target_owner: T,
    expected: SourceCrtNativeRetExpectation,
    phase: SourceCrtNativeRetPhase,
}
#[must_use = "retain actual returned original query/frame/FP/protected/native/physical/caller owners"]
pub struct SourceCrtQueryReturned<P, S, H, T, N> {
    pub owner: SourceCrtQueryBeforeRet<P, S, H>,
    pub target_owner: T,
    pub expected: SourceCrtNativeRetExpectation,
    pub actual_retired: SourceCrtNativeRetired,
    pub actual_return: N,
}
#[must_use = "retain full query/target capsule and exact raw pending return phase"]
pub struct SourceCrtQueryNativeRetParts<P, S, H, T> {
    owner: SourceCrtQueryBeforeRet<P, S, H>,
    target_owner: T,
    expected: SourceCrtNativeRetExpectation,
    phase: SourceCrtNativeRetPhase,
}
impl<P, S, H, T> SourceCrtQueryNativeRetParts<P, S, H, T> {
    pub fn source(&self) -> SourceCrtQueryCompletion { self.owner.source() }
    pub fn expected(&self) -> SourceCrtNativeRetExpectation { self.expected }
    pub fn phase(&self) -> SourceCrtNativeRetPhase { self.phase }
    pub fn history(&self) -> &H { &self.owner.history }
    pub fn target_owner(&self) -> &T { &self.target_owner }
    pub fn actual_prefix_services(&self) -> (&P, &S) { (&self.owner.prefix, &self.owner.services) }

    pub fn actual_prefix_services_mut(&mut self) -> (&mut P, &mut S) {
        (&mut self.owner.prefix, &mut self.owner.services)
    }

    pub fn resume_same_pending_ret(self) -> SourceCrtQueryNativeRet<P, S, H, T> {
        SourceCrtQueryNativeRet { owner: self.owner, target_owner: self.target_owner,
            expected: self.expected, phase: self.phase }
    }
}
impl<P, S, H, T> SourceCrtQueryNativeRet<P, S, H, T> {

    pub fn bind_same_before_ret(owner: SourceCrtQueryBeforeRet<P, S, H>, target_owner: T,
        expected_target: fn(&T) -> u64)
        -> Result<Self, SourceCrtQueryRetBindFailure<P, S, H, T>> {
        let expected = SourceCrtNativeRetExpectation { before: owner.source().actual_ret_frame(),
            actual_required_target: expected_target(&target_owner) };
        let actual_frame = (owner.current_frame)(&owner.prefix, &owner.services);
        if actual_frame != expected.before {
            return Err(SourceCrtQueryRetBindFailure { owner, target_owner, expected, actual_frame });
        }
        Ok(Self { owner, target_owner, expected, phase: SourceCrtNativeRetPhase::ActualReceiptRequired })
    }
    pub fn phase(&self) -> SourceCrtNativeRetPhase { self.phase }
    pub fn accept_actual_ret<N, E>(mut self, actual: N,
        project: fn(&N) -> SourceCrtNativeRetObservation,
        apply: fn(&mut P, &mut S, &mut N) -> Result<(), E>)
        -> Result<SourceCrtQueryReturned<P, S, H, T, N>,
            SourceCrtNativeReturnFailure<Self, SourceCrtNativeRetError<N, E>>> {
        self.phase = SourceCrtNativeRetPhase::ActualContinuationRequired;
        let current_frame = self.owner.current_frame;
        let (actual_return, actual_retired) = match accept_source_crt_native_ret_receipt(
            &mut self.owner, self.expected, actual, project,
            |owner, actual| apply(&mut owner.prefix, &mut owner.services, actual),
            |owner| current_frame(&owner.prefix, &owner.services)) {
            Ok(value) => value,
            Err(cause) => return Err(SourceCrtNativeReturnFailure { owner: self, cause }),
        };
        Ok(SourceCrtQueryReturned { owner: self.owner, target_owner: self.target_owner,
            expected: self.expected, actual_retired, actual_return })
    }

    pub fn into_owned_parts(self) -> SourceCrtQueryNativeRetParts<P, S, H, T> {
        SourceCrtQueryNativeRetParts { owner: self.owner, target_owner: self.target_owner,
            expected: self.expected, phase: self.phase }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryCallEnvelope {
    pub actual_module_base: u64,
    pub source_call_pc: u32,
    pub source_resume_pc: u32,
    pub target_rva: u32,
    pub actual_work_rsp: u64,
    pub source_rcx: Option<u64>,
    pub source_rdx: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtQueryCallerReceipt { pub resume_pc: u32, pub work_rsp: u64, pub rax: u64 }
#[must_use = "retain SAME actual helper/module/cache/native/frame/FP/caller/resources"]
pub struct SourceCrtQueryCallerReturned<P, S, H, T, N> {
    pub owner: SourceCrtQueryReturned<P, S, H, T, N>,
    pub source_call: SourceCrtQueryCallEnvelope,
    pub actual_return: SourceCrtQueryCallerReceipt,
}
#[must_use = "retain entire returned query capsule on rejected original caller binding"]
pub struct SourceCrtQueryCallerBindFailure<P, S, H, T, N> {
    pub owner: SourceCrtQueryReturned<P, S, H, T, N>,
    pub source_call: SourceCrtQueryCallEnvelope,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub observed_ret: SourceCrtNativeRetObservation,
}
impl<P, S, H, T, N> SourceCrtQueryReturned<P, S, H, T, N> {

    pub fn admit_same_query_caller(self, project_call: fn(&T) -> SourceCrtQueryCallEnvelope,
        project_actual_ret: fn(&N) -> SourceCrtNativeRetObservation)
        -> Result<SourceCrtQueryCallerReturned<P, S, H, T, N>, SourceCrtQueryCallerBindFailure<P, S, H, T, N>> {
        let source_call = project_call(&self.target_owner);
        let actual_frame = (self.owner.current_frame)(&self.owner.prefix, &self.owner.services);
        let observed_ret = project_actual_ret(&self.actual_return);
        let source = self.owner.source();
        let (base, entry_rsp, helper_rva, arguments_match) = match source {
            SourceCrtQueryCompletion::Context(value) => {
                let entry = value.entry();
                (entry.actual_module_base, entry.actual_entry_rsp, 0x58db7c,
                    source_call.source_rcx.is_none() && source_call.source_rdx.is_none())
            }
            SourceCrtQueryCompletion::Refresh(value) => {
                let entry = value.entry();
                (entry.actual_module_base, entry.actual_entry_rsp, 0x58e238,
                    source_call.source_rcx == Some(entry.actual_thread_rcx)
                        && source_call.source_rdx == Some(entry.actual_slot_rdx))
            }
            SourceCrtQueryCompletion::Resolve(value) => {
                let entry = value.entry();
                (entry.actual_module_base, entry.actual_entry_rsp, 0x5941fc,
                    source_call.source_rcx == Some(entry.actual_entry_rcx)
                        && source_call.source_rdx == Some(entry.actual_entry_rdx))
            }
            SourceCrtQueryCompletion::Replace(value) => {
                let entry = value.entry();
                (entry.actual_module_base, entry.actual_entry_rsp, 0x59426c,
                    source_call.source_rcx == Some(entry.actual_slot_rcx)
                        && source_call.source_rdx == Some(entry.actual_incoming_rdx))
            }
            SourceCrtQueryCompletion::Calloc(value) => {
                let entry = value.entry();

                (entry.actual_module_base, entry.actual_entry_rsp, 0x58f074,
                    source_call.source_rcx == Some(entry.actual_count_rcx)
                        && source_call.source_rdx == Some(entry.actual_element_rdx))
            }
            SourceCrtQueryCompletion::Mode(value) => {
                let entry = value.entry();
                (entry.actual_module_base, entry.actual_entry_rsp, 0x59bfc8,
                    source_call.source_rcx.is_none() && source_call.source_rdx.is_none())
            }
            SourceCrtQueryCompletion::Initialization(value) => {
                let entry = value.entry();
                (entry.actual_module_base, entry.actual_entry_rsp, 0x58df48,
                    source_call.source_rcx == Some(entry.actual_receiver_rcx)
                        && source_call.source_rdx.is_none())
            }
            SourceCrtQueryCompletion::NullRelease(value) => {
                let entry = value.entry();
                (entry.actual_module_base, entry.actual_entry_rsp, 0x58d910,
                    source_call.source_rcx == Some(0) && entry.actual_entry_rcx == 0
                        && source_call.source_rdx.is_none())
            }
        };
        if source_call.actual_module_base != base || source_call.target_rva != helper_rva || !arguments_match
            || source_call.source_call_pc.checked_add(5) != Some(source_call.source_resume_pc)
            || entry_rsp != source_call.actual_work_rsp.wrapping_sub(8)
            || self.expected.before != source.actual_ret_frame()
            || self.expected.actual_required_target != base.wrapping_add(u64::from(source_call.source_resume_pc))
            || validate_source_crt_native_ret(self.expected, self.actual_retired).is_err()
            || observed_ret != SourceCrtNativeRetObservation::Retired(self.actual_retired)
            || actual_frame != self.actual_retired.after
            || actual_frame.actual_rsp != source_call.actual_work_rsp {
            return Err(SourceCrtQueryCallerBindFailure { owner: self, source_call, actual_frame, observed_ret });
        }
        let actual_return = SourceCrtQueryCallerReceipt { resume_pc: source_call.source_resume_pc,
            work_rsp: actual_frame.actual_rsp, rax: actual_frame.actual_rax };
        Ok(SourceCrtQueryCallerReturned { owner: self, source_call, actual_return })
    }
}
