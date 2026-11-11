use super::exp_range_continuation::{SourceF32MatherrOuterArguments, SourceF32MatherrOuterReturnReceipt};
use super::finite_power::ExplicitExceptionState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTrigSpecialFunctionF32 { Sine586ed0, Cosine57e330 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTrigSpecialActualReceipt {
    pub outer: SourceF32MatherrOuterReturnReceipt,

    pub actual_return_slot_f32_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTrigSpecialCompletion {
    pub function: SourceTrigSpecialFunctionF32,
    pub input_bits: u32,
    pub bits: u32,
    pub advanced: ExplicitExceptionState,
    pub actual_outer_and_slot: Option<SourceTrigSpecialActualReceipt>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTrigSpecialProgress {
    RequiredOuter596150(SourceF32MatherrOuterArguments),
    Complete(SourceTrigSpecialCompletion),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTrigSpecialError {
    NonfiniteHelperEntryRequired { bits: u32, entry: ExplicitExceptionState },
    OuterBindingRequired { pending: SourceTrigSpecialProgress, actual: SourceTrigSpecialActualReceipt },
    CookieFailureContinuationRequired { actual: SourceTrigSpecialActualReceipt },
}
impl SourceTrigSpecialError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NonfiniteHelperEntryRequired { .. } => "597ac8/597af0 special entry requires actual nonfinite F32 input",
            Self::OuterBindingRequired { .. } => "trig special outer/slot receipt does not match pending596150 call",
            Self::CookieFailureContinuationRequired { .. } => "trig special actual596150 cookie-failure continuation required",
        }
    }
    pub fn known_advanced_state(self) -> ExplicitExceptionState {
        match self {
            Self::NonfiniteHelperEntryRequired { entry, .. } => entry,
            Self::OuterBindingRequired { actual, .. } | Self::CookieFailureContinuationRequired { actual } => actual.outer.advanced,
        }
    }
}

#[must_use = "retain actual outer callback/frame/slot resources and advanced-state suffix"]
pub struct SourceTrigSpecialServiceFailure<E> {
    pub cause: E,
    pub pending: SourceF32MatherrOuterArguments,
    pub known_advanced: Option<ExplicitExceptionState>,
}
pub struct SourceTrigSpecialRejectedFailure<E> {
    pub cause: E,
    pub supplied_known_advanced: Option<ExplicitExceptionState>,
    pub completed: SourceTrigSpecialCompletion,
}

pub struct SourceTrigSpecialF32 {
    function: SourceTrigSpecialFunctionF32,
    input_bits: u32,
    progress: SourceTrigSpecialProgress,
    known_state: ExplicitExceptionState,
    cookie_fast: fn(u64, u64) -> bool,
    failed_service: bool,
}
impl SourceTrigSpecialF32 {
    pub fn begin(function: SourceTrigSpecialFunctionF32, bits: u32, entry: ExplicitExceptionState,
        actual_name_address: u64, verify_source_cookie_fast_return: fn(u64, u64) -> bool)
        -> Result<Self, SourceTrigSpecialError> {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude < 0x7f80_0000 {
            return Err(SourceTrigSpecialError::NonfiniteHelperEntryRequired { bits, entry });
        }
        let progress = if magnitude > 0x7f80_0000 {

            SourceTrigSpecialProgress::Complete(SourceTrigSpecialCompletion {
                function, input_bits: bits, bits: bits | 0x0040_0000,
                advanced: entry, actual_outer_and_slot: None,
            })
        } else {
            let operation_edx = match function {
                SourceTrigSpecialFunctionF32::Sine586ed0 => 0x1e,
                SourceTrigSpecialFunctionF32::Cosine57e330 => 0x12,
            };
            SourceTrigSpecialProgress::RequiredOuter596150(SourceF32MatherrOuterArguments {
                actual_name_rcx: actual_name_address, operation_edx,

                proposed_r8: 0xffc0_0000, original_class_r9d: 1,
                status_request_stack5: 8, errno_value_stack6: 0x21,
                argument1_stack7_f32: bits, argument2_stack8_f32: 0,
                kind_stack9: 1, saved_mxcsr: entry.mxcsr(),
            })
        };
        Ok(Self { function, input_bits: bits, progress, known_state: entry,
            cookie_fast: verify_source_cookie_fast_return, failed_service: false })
    }
    pub fn progress(&self) -> SourceTrigSpecialProgress { self.progress }
    pub fn known_prefix_state(&self) -> ExplicitExceptionState { self.known_state }
    pub fn request(&self) -> Option<SourceF32MatherrOuterArguments> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceF32MatherrOuterArguments> {
        match self.progress { SourceTrigSpecialProgress::RequiredOuter596150(call) => Some(call), _ => None }
    }
    pub fn accept_actual_outer_and_slot(&mut self, actual: SourceTrigSpecialActualReceipt)
        -> Result<SourceTrigSpecialProgress, SourceTrigSpecialError> {
        let SourceTrigSpecialProgress::RequiredOuter596150(expected) = self.progress else {
            return Err(SourceTrigSpecialError::OuterBindingRequired { pending: self.progress, actual });
        };
        if actual.outer.call != expected {
            return Err(SourceTrigSpecialError::OuterBindingRequired { pending: self.progress, actual });
        }
        if !(self.cookie_fast)(actual.outer.actual_cookie_passed_rcx, actual.outer.actual_cookie_global_word) {
            return Err(SourceTrigSpecialError::CookieFailureContinuationRequired { actual });
        }
        self.known_state = actual.outer.advanced;
        self.progress = SourceTrigSpecialProgress::Complete(SourceTrigSpecialCompletion {
            function: self.function, input_bits: self.input_bits,
            bits: actual.actual_return_slot_f32_bits, advanced: actual.outer.advanced,
            actual_outer_and_slot: Some(actual),
        });
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E, known_advanced: Option<ExplicitExceptionState>)
        -> Result<SourceTrigSpecialServiceFailure<E>, SourceTrigSpecialRejectedFailure<E>> {
        match self.progress {
            SourceTrigSpecialProgress::RequiredOuter596150(pending) => {
                if let Some(state) = known_advanced { self.known_state = state; }
                self.failed_service = true;
                Ok(SourceTrigSpecialServiceFailure { cause, pending, known_advanced })
            }
            SourceTrigSpecialProgress::Complete(completed) => Err(SourceTrigSpecialRejectedFailure {
                cause, supplied_known_advanced: known_advanced, completed }),
        }
    }
}
