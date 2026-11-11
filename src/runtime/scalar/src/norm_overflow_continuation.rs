use super::exp_range_continuation::{SourceF32MatherrOuterArguments, SourceF32MatherrOuterReturnReceipt};
use super::finite_power::ExplicitExceptionState;
use super::norm_f32::SourceNormOverflowRequest;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceNormOverflowCompletion {
    pub request: SourceNormOverflowRequest,
    pub bits: u32,
    pub advanced: ExplicitExceptionState,

    pub actual_outer_return: SourceF32MatherrOuterReturnReceipt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceNormOverflowProgress {
    RequiredOuter596150(SourceF32MatherrOuterArguments),
    Complete(SourceNormOverflowCompletion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceNormOverflowError {
    SourceOverflowRequestRequired { supplied: SourceNormOverflowRequest },
    OuterReturnBindingRequired { pending: SourceNormOverflowProgress, actual: SourceF32MatherrOuterReturnReceipt },
    CookieFailureContinuationRequired { actual: SourceF32MatherrOuterReturnReceipt },
}
impl SourceNormOverflowError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::SourceOverflowRequestRequired { .. } => "579900 requires its actual finite-pair overflow prefix/596150 argument constants",
            Self::OuterReturnBindingRequired { .. } => "579900 actual596150 return does not bind the pending source arguments",
            Self::CookieFailureContinuationRequired { .. } => "579900 actual596150 cookie-failure continuation required",
        }
    }
    pub fn known_advanced_state(self) -> ExplicitExceptionState {
        match self {
            Self::SourceOverflowRequestRequired { supplied } => supplied.advanced,
            Self::OuterReturnBindingRequired { actual, .. } | Self::CookieFailureContinuationRequired { actual } => actual.advanced,
        }
    }
}
#[must_use = "retain actual outer callback/frame/errno resources and their advanced-state suffix"]
pub struct SourceNormOverflowServiceFailure<E> {
    pub cause: E,
    pub pending: SourceF32MatherrOuterArguments,

    pub known_advanced: Option<ExplicitExceptionState>,
}
pub struct SourceNormOverflowRejectedFailure<E> {
    pub cause: E,
    pub supplied_known_advanced: Option<ExplicitExceptionState>,
    pub completed: SourceNormOverflowCompletion,
}

pub struct SourceNormOverflow579900 {
    original: SourceNormOverflowRequest,
    progress: SourceNormOverflowProgress,
    known_state: ExplicitExceptionState,
    cookie_fast: fn(u64, u64) -> bool,
    failed_service: bool,
}
impl SourceNormOverflow579900 {
    pub fn begin(request: SourceNormOverflowRequest, actual_name_address: u64,
        verify_source_cookie_fast_return: fn(u64, u64) -> bool) -> Result<Self, SourceNormOverflowError>
    {

        if request.argument1_f32_bits & 0x7fff_ffff >= 0x7f80_0000
            || request.argument2_f32_bits & 0x7fff_ffff >= 0x7f80_0000
            || request.proposed_f32_bits != 0x7f80_0000 || request.edx != 0x18 || request.r9d != 3
            || request.stack_arg5 != 0x11 || request.stack_arg6 != 0x22 || request.stack_arg9 != 2
            || request.root_f64_bits <= 0x47ef_ffff_e000_0000 || request.root_f64_bits >= 0x4800_0000_0000_0000
        { return Err(SourceNormOverflowError::SourceOverflowRequestRequired { supplied: request }); }
        let call = SourceF32MatherrOuterArguments {
            actual_name_rcx: actual_name_address, operation_edx: request.edx,
            proposed_r8: u64::from(request.proposed_f32_bits), original_class_r9d: request.r9d as i32,
            status_request_stack5: request.stack_arg5, errno_value_stack6: request.stack_arg6,
            argument1_stack7_f32: request.argument1_f32_bits, argument2_stack8_f32: request.argument2_f32_bits,
            kind_stack9: request.stack_arg9, saved_mxcsr: request.advanced.mxcsr(),
        };
        Ok(Self { original: request, progress: SourceNormOverflowProgress::RequiredOuter596150(call),
            known_state: request.advanced, cookie_fast: verify_source_cookie_fast_return, failed_service: false })
    }
    pub fn original_request(&self) -> SourceNormOverflowRequest { self.original }
    pub fn progress(&self) -> SourceNormOverflowProgress { self.progress }
    pub fn known_prefix_state(&self) -> ExplicitExceptionState { self.known_state }
    pub fn request(&self) -> Option<SourceF32MatherrOuterArguments> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceF32MatherrOuterArguments> {
        match self.progress { SourceNormOverflowProgress::RequiredOuter596150(call) => Some(call), _ => None }
    }
    pub fn accept_actual_outer_return(&mut self, actual: SourceF32MatherrOuterReturnReceipt)
        -> Result<SourceNormOverflowProgress, SourceNormOverflowError>
    {
        let SourceNormOverflowProgress::RequiredOuter596150(expected) = self.progress else {
            return Err(SourceNormOverflowError::OuterReturnBindingRequired { pending: self.progress, actual });
        };
        if actual.call != expected {
            return Err(SourceNormOverflowError::OuterReturnBindingRequired { pending: self.progress, actual });
        }
        if !(self.cookie_fast)(actual.actual_cookie_passed_rcx, actual.actual_cookie_global_word) {
            return Err(SourceNormOverflowError::CookieFailureContinuationRequired { actual });
        }
        self.known_state = actual.advanced;
        self.progress = SourceNormOverflowProgress::Complete(SourceNormOverflowCompletion {
            request: self.original, bits: actual.outer_return_f32_bits,
            advanced: actual.advanced, actual_outer_return: actual,
        });
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E, known_advanced: Option<ExplicitExceptionState>)
        -> Result<SourceNormOverflowServiceFailure<E>, SourceNormOverflowRejectedFailure<E>>
    {
        match self.progress {
            SourceNormOverflowProgress::RequiredOuter596150(pending) => {
                if let Some(state) = known_advanced { self.known_state = state; }
                self.failed_service = true;
                Ok(SourceNormOverflowServiceFailure { cause, pending, known_advanced })
            }
            SourceNormOverflowProgress::Complete(completed) => Err(SourceNormOverflowRejectedFailure {
                cause, supplied_known_advanced: known_advanced, completed }),
        }
    }
}
