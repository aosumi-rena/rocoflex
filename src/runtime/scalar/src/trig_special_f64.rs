use super::finite_power::ExplicitExceptionState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTrigSpecialFunctionF64 { Sine586960, Cosine57ddb0 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTrigF64OuterArguments {
    pub actual_name_rcx: u64,
    pub operation_edx: u32,
    pub proposed_r8: u64,
    pub original_class_r9d: i32,
    pub status_request_stack5: u32,
    pub errno_value_stack6: u32,
    pub argument1_stack7_f64: u64,
    pub argument2_stack8_f64: u64,
    pub kind_stack9: u32,
    pub saved_mxcsr: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTrigSpecialF64ActualReceipt {
    pub call: SourceTrigF64OuterArguments,
    pub actual_outer_return_f64_bits: u64,
    pub actual_helper_xmm6_after_outer: [u32; 4],
    pub actual_cookie_passed_rcx: u64,
    pub actual_cookie_global_word: u64,

    pub actual_return_slot_f64_bits: u64,
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTrigSpecialF64Completion {
    pub function: SourceTrigSpecialFunctionF64,
    pub input_bits: u64,
    pub bits: u64,
    pub advanced: ExplicitExceptionState,
    pub actual_outer_and_slot: Option<SourceTrigSpecialF64ActualReceipt>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTrigSpecialF64Progress {
    RequiredOuter5962a0(SourceTrigF64OuterArguments),
    Complete(SourceTrigSpecialF64Completion),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTrigSpecialF64Error {
    NonfiniteHelperEntryRequired { bits: u64, entry: ExplicitExceptionState },
    OuterBindingRequired { pending: SourceTrigSpecialF64Progress, actual: SourceTrigSpecialF64ActualReceipt },
    CookieFailureContinuationRequired { actual: SourceTrigSpecialF64ActualReceipt },
}
impl SourceTrigSpecialF64Error {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NonfiniteHelperEntryRequired { .. } => "597adc/597b04 special entry requires actual nonfinite F64 input",
            Self::OuterBindingRequired { .. } => "trig F64 actual outer/slot receipt differs from pending WHOLE5962a0 call",
            Self::CookieFailureContinuationRequired { .. } => "trig F64 actual5962a0 cookie-failure continuation required",
        }
    }
    pub fn known_advanced_state(self) -> ExplicitExceptionState {
        match self {
            Self::NonfiniteHelperEntryRequired { entry, .. } => entry,
            Self::OuterBindingRequired { actual, .. } | Self::CookieFailureContinuationRequired { actual } => actual.advanced,
        }
    }
}
#[must_use = "retain actual WHOLE5962a0 stack/callback/errno owner and advanced suffix"]
pub struct SourceTrigSpecialF64ServiceFailure<E> {
    pub cause: E, pub pending: SourceTrigF64OuterArguments,
    pub known_advanced: Option<ExplicitExceptionState>,
}
pub struct SourceTrigSpecialF64RejectedFailure<E> {
    pub cause: E, pub supplied_known_advanced: Option<ExplicitExceptionState>,
    pub completed: SourceTrigSpecialF64Completion,
}

pub struct SourceTrigSpecialF64 {
    function: SourceTrigSpecialFunctionF64,
    input_bits: u64,
    progress: SourceTrigSpecialF64Progress,
    known_state: ExplicitExceptionState,
    cookie_fast: fn(u64, u64) -> bool,
    failed_service: bool,
}
impl SourceTrigSpecialF64 {
    pub fn begin(function: SourceTrigSpecialFunctionF64, bits: u64, entry: ExplicitExceptionState,
        actual_name_address: u64, verify_source_cookie_fast_return: fn(u64, u64) -> bool)
        -> Result<Self, SourceTrigSpecialF64Error> {
        let magnitude = bits & 0x7fff_ffff_ffff_ffff;
        if magnitude < 0x7ff0_0000_0000_0000 {
            return Err(SourceTrigSpecialF64Error::NonfiniteHelperEntryRequired { bits, entry });
        }
        let progress = if magnitude > 0x7ff0_0000_0000_0000 {
            SourceTrigSpecialF64Progress::Complete(SourceTrigSpecialF64Completion {
                function, input_bits: bits, bits: bits | 0x0008_0000_0000_0000,
                advanced: entry, actual_outer_and_slot: None,
            })
        } else {
            let operation_edx = match function {
                SourceTrigSpecialFunctionF64::Sine586960 => 0x1e,
                SourceTrigSpecialFunctionF64::Cosine57ddb0 => 0x12,
            };
            SourceTrigSpecialF64Progress::RequiredOuter5962a0(SourceTrigF64OuterArguments {
                actual_name_rcx: actual_name_address, operation_edx,
                proposed_r8: 0xfff8_0000_0000_0000, original_class_r9d: 1,
                status_request_stack5: 8, errno_value_stack6: 0x21,
                argument1_stack7_f64: bits, argument2_stack8_f64: 0,
                kind_stack9: 1, saved_mxcsr: entry.mxcsr(),
            })
        };
        Ok(Self { function, input_bits: bits, progress, known_state: entry,
            cookie_fast: verify_source_cookie_fast_return, failed_service: false })
    }
    pub fn progress(&self) -> SourceTrigSpecialF64Progress { self.progress }
    pub fn known_prefix_state(&self) -> ExplicitExceptionState { self.known_state }
    pub fn request(&self) -> Option<SourceTrigF64OuterArguments> {
        if self.failed_service { None } else { self.awaiting_actual_completion() }
    }
    pub fn awaiting_actual_completion(&self) -> Option<SourceTrigF64OuterArguments> {
        match self.progress { SourceTrigSpecialF64Progress::RequiredOuter5962a0(call) => Some(call), _ => None }
    }
    pub fn accept_actual_outer_and_slot(&mut self, actual: SourceTrigSpecialF64ActualReceipt)
        -> Result<SourceTrigSpecialF64Progress, SourceTrigSpecialF64Error> {
        let SourceTrigSpecialF64Progress::RequiredOuter5962a0(expected) = self.progress else {
            return Err(SourceTrigSpecialF64Error::OuterBindingRequired { pending: self.progress, actual });
        };
        if actual.call != expected {
            return Err(SourceTrigSpecialF64Error::OuterBindingRequired { pending: self.progress, actual });
        }
        if !(self.cookie_fast)(actual.actual_cookie_passed_rcx, actual.actual_cookie_global_word) {
            return Err(SourceTrigSpecialF64Error::CookieFailureContinuationRequired { actual });
        }
        self.known_state = actual.advanced;
        self.progress = SourceTrigSpecialF64Progress::Complete(SourceTrigSpecialF64Completion {
            function: self.function, input_bits: self.input_bits,
            bits: actual.actual_return_slot_f64_bits, advanced: actual.advanced,
            actual_outer_and_slot: Some(actual),
        });
        self.failed_service = false;
        Ok(self.progress)
    }
    pub fn retain_service_failure<E>(&mut self, cause: E, known_advanced: Option<ExplicitExceptionState>)
        -> Result<SourceTrigSpecialF64ServiceFailure<E>, SourceTrigSpecialF64RejectedFailure<E>> {
        match self.progress {
            SourceTrigSpecialF64Progress::RequiredOuter5962a0(pending) => {
                if let Some(state) = known_advanced { self.known_state = state; }
                self.failed_service = true;
                Ok(SourceTrigSpecialF64ServiceFailure { cause, pending, known_advanced })
            }
            SourceTrigSpecialF64Progress::Complete(completed) => Err(SourceTrigSpecialF64RejectedFailure {
                cause, supplied_known_advanced: known_advanced, completed }),
        }
    }
}
