use super::finite_power::{ExplicitExceptionState, MaskedMathError};
use super::matherr_continuation::{SourceMatherrEncodedSnapshot, source_errno_value_for_class};
use super::selected_exp::{SourceExpEntry, SourceExpPlan, source_entry_plan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExp596150Arguments {
    pub actual_name_rcx: u64,
    pub operation_edx: u32,
    pub proposed_r8: u64,
    pub original_class_r9d: i32,
    pub status_request_stack5: u32,
    pub errno_value_stack6: u32,
    pub argument1_stack7_f32: u32,
    pub argument2_stack8_f32: u32,
    pub kind_stack9: u32,
    pub saved_mxcsr: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExp596084Request {
    pub request_ecx: u32,
    pub saved_mxcsr_rdx: u64,
    pub current_mxcsr: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExp596084Receipt {
    pub request: SourceExp596084Request,
    pub return_eax: u32,
    pub remaining_request_ebx: u32,
    pub source_or_status_calls: [Option<u8>; 2],
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExpRangeRequest {
    Status596084(SourceExp596084Request),

    PresenceSecurityWord,
    PresenceEncodedMatherrWord { security_word: u64 },

    SelectedOuterReturn { call: SourceExp596150Arguments, xmm6_after_argument2_load: [u32; 4] },

    ErrnoWrite { value: i32 },

    AbsentOuterReturn { call: SourceExp596150Arguments },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExpOuterReturnReceipt {
    pub call: SourceExp596150Arguments,
    pub outer_return_f32_bits: u32,
    pub actual_helper_xmm6_after_outer: [u32; 4],
    pub actual_cookie_passed_rcx: u64,
    pub actual_cookie_global_word: u64,
    pub advanced: ExplicitExceptionState,
}

pub type SourceF32MatherrOuterArguments = SourceExp596150Arguments;
pub type SourceF32MatherrOuterReturnReceipt = SourceExpOuterReturnReceipt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExpRangeActualReceipt {
    Status(SourceExp596084Receipt),
    PresenceSecurityWord { value: u64 },
    PresenceEncodedMatherrWord { security_word: u64, encoded_word: u64 },
    SelectedOuterReturn(SourceExpOuterReturnReceipt),
    ErrnoWrite { written_value: i32, advanced: ExplicitExceptionState },
    AbsentOuterReturn(SourceExpOuterReturnReceipt),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceExp597c90Completion {
    pub bits: u32,

    pub result_xmm0_at_597d2a: [u32; 4],
    pub originally_proposed_bits: u32,
    pub discarded_outer_return_bits: u32,
    pub presence_snapshot: SourceMatherrEncodedSnapshot,
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExp597c90Progress {
    Request(SourceExpRangeRequest),
    Complete(SourceExp597c90Completion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExpRangeError {
    RangeEntryRequired { entry: SourceExpEntry },
    InvalidRangeEntry { entry: SourceExpEntry },
    UnexpectedReceipt { pending: SourceExp597c90Progress, receipt: SourceExpRangeActualReceipt },
    StatusReceiptRequired { request: SourceExp596084Request, receipt: SourceExp596084Receipt },
    UnmaskedStatusHelperRequired { receipt: SourceExp596084Receipt },
    PresenceReadBindingRequired { expected_security: u64, receipt: SourceExpRangeActualReceipt },
    ErrnoReceiptRequired { expected_value: i32, receipt: SourceExpRangeActualReceipt },
    OuterReturnBindingRequired { expected: SourceExp596150Arguments, receipt: SourceExpOuterReturnReceipt },
    CookieFailureContinuationRequired { receipt: SourceExpOuterReturnReceipt },
    AbsentReturnStateRequired { expected: ExplicitExceptionState, receipt: SourceExpOuterReturnReceipt },
}
impl SourceExpRangeError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::RangeEntryRequired { .. } => "597c90 requires an actual exp range-helper entry",
            Self::InvalidRangeEntry { .. } => "597c90 range plan does not match retained exp entry dispatch",
            Self::UnexpectedReceipt { .. } => "597c90 actual completion does not match its pending service",
            Self::StatusReceiptRequired { .. } => "596084 receipt needs same request, source state and errno preservation",
            Self::UnmaskedStatusHelperRequired { .. } => "596150 non-EAX1 status route requires actual59a288 continuation",
            Self::PresenceReadBindingRequired { .. } => "590b80 encoded-word receipt does not match fresh security read",
            Self::ErrnoReceiptRequired { .. } => "599f64 actual errno-write receipt does not match request/state",
            Self::OuterReturnBindingRequired { .. } => "596150 outer return receipt has different source call arguments",
            Self::CookieFailureContinuationRequired { .. } => "561c40 actual cookie failure continuation required",
            Self::AbsentReturnStateRequired { .. } => "596150 absent fast tail must preserve actual post-accessor logical state",
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::RangeEntryRequired { entry } | Self::InvalidRangeEntry { entry } => entry.exception_state,
            Self::UnexpectedReceipt { receipt, .. } => match receipt {
                SourceExpRangeActualReceipt::Status(receipt) => receipt.advanced,
                SourceExpRangeActualReceipt::SelectedOuterReturn(receipt)
                    | SourceExpRangeActualReceipt::AbsentOuterReturn(receipt) => receipt.advanced,
                SourceExpRangeActualReceipt::ErrnoWrite { advanced, .. } => advanced,
                _ => entry,
            },
            Self::StatusReceiptRequired { receipt, .. } | Self::UnmaskedStatusHelperRequired { receipt } => receipt.advanced,
            Self::OuterReturnBindingRequired { receipt, .. } | Self::CookieFailureContinuationRequired { receipt }
                | Self::AbsentReturnStateRequired { receipt, .. } => receipt.advanced,
            Self::ErrnoReceiptRequired { receipt: SourceExpRangeActualReceipt::ErrnoWrite { advanced, .. }, .. } => advanced,
            _ => entry,
        }
    }
}

pub struct SourceExp597c90ServiceFailure<E> {
    pub cause: E,
    pub known_advanced: ExplicitExceptionState,
    pub pending: SourceExpRangeRequest,
}
pub struct SourceExp597c90RejectedFailure<E> {
    pub cause: E,
    pub supplied_known_advanced: ExplicitExceptionState,
    pub completed: SourceExp597c90Completion,
}

pub struct SourceExp597c90 {
    call: SourceExp596150Arguments,
    state: ExplicitExceptionState,
    progress: SourceExp597c90Progress,
    presence: Option<SourceMatherrEncodedSnapshot>,
    status_receipt: Option<SourceExp596084Receipt>,
    decode: fn(u64, u64) -> u64,
    cookie_fast: fn(u64, u64) -> bool,
    failed_service: bool,
}
impl SourceExp597c90 {
    pub fn begin(entry: SourceExpEntry, actual_name_address: u64,
        decode_source_pointer_word: fn(u64, u64) -> u64,
        verify_source_cookie_fast_return: fn(u64, u64) -> bool) -> Result<Self, SourceExpRangeError>
    {
        let SourceExpPlan::MaskedRangeError { input_bits, helper_input_bits, requested_result_bits, error, .. } = entry.plan
            else { return Err(SourceExpRangeError::RangeEntryRequired { entry }); };
        let (actual_plan, flags) = source_entry_plan(input_bits);
        if actual_plan != entry.plan || entry.exception_state.mxcsr() & flags != flags {
            return Err(SourceExpRangeError::InvalidRangeEntry { entry });
        }
        let (status, class) = match error {
            MaskedMathError::Underflow => (0x12, 4),
            MaskedMathError::Overflow => (0x11, 3),
            _ => return Err(SourceExpRangeError::InvalidRangeEntry { entry }),
        };
        let call = SourceExp596150Arguments {
            actual_name_rcx: actual_name_address, operation_edx: 0x14,
            proposed_r8: u64::from(requested_result_bits), original_class_r9d: class,
            status_request_stack5: status, errno_value_stack6: 0x22,
            argument1_stack7_f32: helper_input_bits, argument2_stack8_f32: 0,
            kind_stack9: 1, saved_mxcsr: entry.exception_state.mxcsr(),
        };
        let request = SourceExp596084Request { request_ecx: status,
            saved_mxcsr_rdx: u64::from(call.saved_mxcsr), current_mxcsr: entry.exception_state.mxcsr() };

        Ok(Self { call, state: entry.exception_state,
            progress: SourceExp597c90Progress::Request(SourceExpRangeRequest::Status596084(request)),
            presence: None, status_receipt: None, decode: decode_source_pointer_word,
            cookie_fast: verify_source_cookie_fast_return, failed_service: false })
    }
    pub fn progress(&self) -> SourceExp597c90Progress { self.progress }
    pub fn request(&self) -> Option<SourceExpRangeRequest> {
        if self.failed_service { return None; }
        match self.progress { SourceExp597c90Progress::Request(request) => Some(request), _ => None }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceExpRangeRequest> {
        match self.progress { SourceExp597c90Progress::Request(request) => Some(request), _ => None }
    }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn arguments(&self) -> SourceExp596150Arguments { self.call }
    pub fn presence_snapshot(&self) -> Option<SourceMatherrEncodedSnapshot> { self.presence }
    pub fn status_receipt(&self) -> Option<SourceExp596084Receipt> { self.status_receipt }
    fn request_next(&mut self, next: SourceExpRangeRequest) { self.progress = SourceExp597c90Progress::Request(next); }
    pub fn accept_actual_completion(&mut self, receipt: SourceExpRangeActualReceipt)
        -> Result<SourceExp597c90Progress, SourceExpRangeError>
    {
        match (self.progress, receipt) {
            (SourceExp597c90Progress::Request(SourceExpRangeRequest::Status596084(request)), SourceExpRangeActualReceipt::Status(actual)) => {

                if actual.request != request || actual.advanced.errno() != self.state.errno()
                    || actual.advanced.mxcsr() & !0x3f != self.state.mxcsr() & !0x3f
                    || actual.advanced.mxcsr() & self.state.mxcsr() & 0x3f != self.state.mxcsr() & 0x3f
                { return Err(SourceExpRangeError::StatusReceiptRequired { request, receipt: actual }); }
                if actual.return_eax != 1 || actual.remaining_request_ebx != 0 {
                    return Err(SourceExpRangeError::UnmaskedStatusHelperRequired { receipt: actual });
                }
                self.state = actual.advanced;
                self.status_receipt = Some(actual);
                self.request_next(SourceExpRangeRequest::PresenceSecurityWord);
            }
            (SourceExp597c90Progress::Request(SourceExpRangeRequest::PresenceSecurityWord), SourceExpRangeActualReceipt::PresenceSecurityWord { value }) => {
                self.request_next(SourceExpRangeRequest::PresenceEncodedMatherrWord { security_word: value });
            }
            (SourceExp597c90Progress::Request(SourceExpRangeRequest::PresenceEncodedMatherrWord { security_word }),
                SourceExpRangeActualReceipt::PresenceEncodedMatherrWord { security_word: actual_security, encoded_word }) => {
                if actual_security != security_word { return Err(SourceExpRangeError::PresenceReadBindingRequired { expected_security: security_word, receipt }); }
                let target = (self.decode)(encoded_word, security_word);
                self.presence = Some(SourceMatherrEncodedSnapshot { security_word, encoded_word, decoded_target: target });
                if target != 0 {
                    self.request_next(SourceExpRangeRequest::SelectedOuterReturn { call: self.call, xmm6_after_argument2_load: [0; 4] });
                } else if let Some(value) = source_errno_value_for_class(self.call.original_class_r9d) {
                    self.request_next(SourceExpRangeRequest::ErrnoWrite { value });
                } else { self.request_next(SourceExpRangeRequest::AbsentOuterReturn { call: self.call }); }
            }
            (SourceExp597c90Progress::Request(SourceExpRangeRequest::ErrnoWrite { value }), SourceExpRangeActualReceipt::ErrnoWrite { written_value, advanced }) => {
                if written_value != value || advanced.errno() != value {
                    return Err(SourceExpRangeError::ErrnoReceiptRequired { expected_value: value, receipt });
                }
                self.state = advanced;
                self.request_next(SourceExpRangeRequest::AbsentOuterReturn { call: self.call });
            }
            (SourceExp597c90Progress::Request(SourceExpRangeRequest::SelectedOuterReturn { .. }), SourceExpRangeActualReceipt::SelectedOuterReturn(actual)) => {
                self.complete_outer(actual, false)?;
            }
            (SourceExp597c90Progress::Request(SourceExpRangeRequest::AbsentOuterReturn { .. }), SourceExpRangeActualReceipt::AbsentOuterReturn(actual)) => {
                self.complete_outer(actual, true)?;
            }
            _ => return Err(SourceExpRangeError::UnexpectedReceipt { pending: self.progress, receipt }),
        }
        self.failed_service = false;
        Ok(self.progress)
    }
    fn complete_outer(&mut self, actual: SourceExpOuterReturnReceipt, absent: bool) -> Result<(), SourceExpRangeError> {
        if actual.call != self.call {
            return Err(SourceExpRangeError::OuterReturnBindingRequired { expected: self.call, receipt: actual });
        }
        if !(self.cookie_fast)(actual.actual_cookie_passed_rcx, actual.actual_cookie_global_word) {
            return Err(SourceExpRangeError::CookieFailureContinuationRequired { receipt: actual });
        }
        if absent && actual.advanced != self.state {
            return Err(SourceExpRangeError::AbsentReturnStateRequired { expected: self.state, receipt: actual });
        }

        let Some(presence_snapshot) = self.presence else {
            return Err(SourceExpRangeError::UnexpectedReceipt { pending: self.progress,
                receipt: if absent { SourceExpRangeActualReceipt::AbsentOuterReturn(actual) }
                    else { SourceExpRangeActualReceipt::SelectedOuterReturn(actual) } });
        };
        self.state = actual.advanced;
        self.progress = SourceExp597c90Progress::Complete(SourceExp597c90Completion {
            bits: actual.actual_helper_xmm6_after_outer[0],
            result_xmm0_at_597d2a: actual.actual_helper_xmm6_after_outer,
            originally_proposed_bits: self.call.proposed_r8 as u32,
            discarded_outer_return_bits: actual.outer_return_f32_bits,
            presence_snapshot, advanced: actual.advanced,
        });
        Ok(())
    }
    pub fn retain_service_failure<E>(&mut self, cause: E, known_advanced: ExplicitExceptionState)
        -> Result<SourceExp597c90ServiceFailure<E>, SourceExp597c90RejectedFailure<E>>
    {
        match self.progress {
            SourceExp597c90Progress::Request(pending) => {
                self.state = known_advanced;
                self.failed_service = true;
                Ok(SourceExp597c90ServiceFailure { cause, known_advanced, pending })
            }
            SourceExp597c90Progress::Complete(completed) => Err(SourceExp597c90RejectedFailure {
                cause, supplied_known_advanced: known_advanced, completed,
            }),
        }
    }
}
