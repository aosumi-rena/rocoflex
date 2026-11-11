use super::finite_power::ExplicitExceptionState;

pub const SOURCE_MATHERR_XFG_TYPE_HASH: u64 = 0xd888_07e6_5854_7170;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceMatherrExceptionRecord {
    pub class: i32,
    pub name_address: u64,
    pub argument1_f64_bits: u64,
    pub argument2_f64_bits: u64,
    pub result_f64_bits: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceMatherr59601cArguments {
    pub exception: SourceMatherrExceptionRecord,

    pub restore_control_word: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMatherr59601cStep { SecurityWord, EncodedMatherrWord, GuardedCall, ErrnoWrite }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMatherr59601cRequest {

    SecurityWord,
    EncodedMatherrWord { security_word: u64 },
    GuardedCall { target: u64, type_hash: u64, exception: SourceMatherrExceptionRecord },

    ErrnoWrite { value: i32 },
}

impl SourceMatherr59601cRequest {
    pub fn step(self) -> SourceMatherr59601cStep {
        match self {
            Self::SecurityWord => SourceMatherr59601cStep::SecurityWord,
            Self::EncodedMatherrWord { .. } => SourceMatherr59601cStep::EncodedMatherrWord,
            Self::GuardedCall { .. } => SourceMatherr59601cStep::GuardedCall,
            Self::ErrnoWrite { .. } => SourceMatherr59601cStep::ErrnoWrite,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceMatherrEncodedSnapshot {
    pub security_word: u64,
    pub encoded_word: u64,
    pub decoded_target: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceMatherrCallbackCompletion {
    pub target: u64,
    pub type_hash: u64,
    pub return_eax: u32,
    pub exception: SourceMatherrExceptionRecord,
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceMatherrErrnoCompletion {
    pub written_value: i32,
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceMatherr59601cCompletion {
    pub result_f64_bits: u64,
    pub callback_return_eax: u32,
    pub exception: SourceMatherrExceptionRecord,
    pub advanced: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMatherr59601cProgress {
    Request(SourceMatherr59601cRequest),
    Complete(SourceMatherr59601cCompletion),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMatherr59601cError {
    RestoreControlsRequired { arguments: SourceMatherr59601cArguments, entry: ExplicitExceptionState },
    UnexpectedStep { pending: SourceMatherr59601cProgress, supplied: SourceMatherr59601cStep },
    UnexpectedCallbackReceipt { pending: SourceMatherr59601cProgress, receipt: SourceMatherrCallbackCompletion },
    UnexpectedErrnoReceipt { pending: SourceMatherr59601cProgress, receipt: SourceMatherrErrnoCompletion },
    UnexpectedErrnoRecordReceipt { pending: SourceMatherr59601cProgress, receipt: SourceMatherrErrnoCompletion, actual_exception: SourceMatherrExceptionRecord },
    GuardedReceiptMismatch { expected_target: u64, receipt: SourceMatherrCallbackCompletion },
    ErrnoReceiptMismatch { expected_value: i32, receipt: SourceMatherrErrnoCompletion },
    ErrnoRecordReceiptMismatch { expected_value: i32, receipt: SourceMatherrErrnoCompletion, actual_exception: SourceMatherrExceptionRecord },
}

impl SourceMatherr59601cError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::RestoreControlsRequired { .. } => "59601c restored controls are outside bounded RNE/masked/gradual domain",
            Self::UnexpectedStep { .. } | Self::UnexpectedCallbackReceipt { .. } | Self::UnexpectedErrnoReceipt { .. }
                | Self::UnexpectedErrnoRecordReceipt { .. } =>
                "59601c completion does not match its pending source service",
            Self::GuardedReceiptMismatch { .. } => "590ba0 guarded callback receipt has a different target/type binding",
            Self::ErrnoReceiptMismatch { .. } | Self::ErrnoRecordReceiptMismatch { .. } =>
                "599f64 actual errno-write receipt does not match requested value/state",
        }
    }
}

pub struct SourceMatherr59601cServiceFailure<E> {
    pub cause: E,
    pub exception: SourceMatherrExceptionRecord,
    pub advanced: ExplicitExceptionState,
    pub pending: SourceMatherr59601cProgress,
}

pub struct SourceMatherr59601cRejectedFailure<E> {
    pub cause: E,
    pub exception: SourceMatherrExceptionRecord,
    pub supplied_advanced: ExplicitExceptionState,
    pub completed: SourceMatherr59601cCompletion,
}

pub struct SourceMatherr59601c {
    original_class: i32,
    restore_control_word: u32,
    exception: SourceMatherrExceptionRecord,
    state: ExplicitExceptionState,
    decode_source_pointer_word: fn(u64, u64) -> u64,
    encoded_snapshot: Option<SourceMatherrEncodedSnapshot>,
    progress: SourceMatherr59601cProgress,
}

impl SourceMatherr59601c {
    pub fn begin(arguments: SourceMatherr59601cArguments, entry: ExplicitExceptionState,
        decode_source_pointer_word: fn(u64, u64) -> u64) -> Result<Self, SourceMatherr59601cError>
    {
        if arguments.restore_control_word & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceMatherr59601cError::RestoreControlsRequired { arguments, entry });
        }
        Ok(Self {
            original_class: arguments.exception.class,
            restore_control_word: arguments.restore_control_word,
            exception: arguments.exception,
            state: entry,
            decode_source_pointer_word,
            encoded_snapshot: None,
            progress: SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::SecurityWord),
        })
    }
    pub fn progress(&self) -> SourceMatherr59601cProgress { self.progress }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn exception_record(&self) -> SourceMatherrExceptionRecord { self.exception }
    pub fn original_class(&self) -> i32 { self.original_class }
    pub fn restore_control_word(&self) -> u32 { self.restore_control_word }
    pub fn encoded_snapshot(&self) -> Option<SourceMatherrEncodedSnapshot> { self.encoded_snapshot }

    fn unexpected(&self, supplied: SourceMatherr59601cStep) -> SourceMatherr59601cError {
        SourceMatherr59601cError::UnexpectedStep { pending: self.progress, supplied }
    }
    pub fn supply_security_word(&mut self, value: u64)
        -> Result<SourceMatherr59601cProgress, SourceMatherr59601cError>
    {
        if !matches!(self.progress, SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::SecurityWord)) {
            return Err(self.unexpected(SourceMatherr59601cStep::SecurityWord));
        }
        self.progress = SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::EncodedMatherrWord { security_word: value });
        Ok(self.progress)
    }
    pub fn supply_encoded_matherr_word(&mut self, encoded_word: u64)
        -> Result<SourceMatherr59601cProgress, SourceMatherr59601cError>
    {
        let security_word = match self.progress {
            SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::EncodedMatherrWord { security_word }) => security_word,
            _ => return Err(self.unexpected(SourceMatherr59601cStep::EncodedMatherrWord)),
        };
        let target = (self.decode_source_pointer_word)(encoded_word, security_word);
        self.encoded_snapshot = Some(SourceMatherrEncodedSnapshot { security_word, encoded_word, decoded_target: target });
        if target == 0 {

            self.unhandled();
        } else {
            self.progress = SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::GuardedCall {
                target, type_hash: SOURCE_MATHERR_XFG_TYPE_HASH, exception: self.exception,
            });
        }
        Ok(self.progress)
    }
    pub fn supply_callback_completion(&mut self, receipt: SourceMatherrCallbackCompletion)
        -> Result<SourceMatherr59601cProgress, SourceMatherr59601cError>
    {
        let target = match self.progress {
            SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::GuardedCall { target, .. }) => target,
            _ => return Err(SourceMatherr59601cError::UnexpectedCallbackReceipt { pending: self.progress, receipt }),
        };
        if receipt.target != target || receipt.type_hash != SOURCE_MATHERR_XFG_TYPE_HASH {
            return Err(SourceMatherr59601cError::GuardedReceiptMismatch { expected_target: target, receipt });
        }
        self.exception = receipt.exception;
        self.state = receipt.advanced;
        if receipt.return_eax != 0 { self.complete(receipt.return_eax); }
        else { self.unhandled(); }
        Ok(self.progress)
    }
    fn unhandled(&mut self) {

        let value = source_errno_value_for_class(self.original_class);
        if let Some(value) = value {
            self.progress = SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::ErrnoWrite { value });
        } else { self.complete(0); }
    }

    pub fn supply_errno_completion(&mut self, receipt: SourceMatherrErrnoCompletion)
        -> Result<SourceMatherr59601cProgress, SourceMatherr59601cError>
    {
        let unchanged = self.exception;
        self.supply_errno_completion_with_record(receipt, unchanged).map_err(|error| match error {
            SourceMatherr59601cError::UnexpectedErrnoRecordReceipt { pending, receipt, .. } =>
                SourceMatherr59601cError::UnexpectedErrnoReceipt { pending, receipt },
            SourceMatherr59601cError::ErrnoRecordReceiptMismatch { expected_value, receipt, .. } =>
                SourceMatherr59601cError::ErrnoReceiptMismatch { expected_value, receipt },
            other => other,
        })
    }

    pub fn supply_errno_completion_with_record(&mut self, receipt: SourceMatherrErrnoCompletion,
        actual_exception: SourceMatherrExceptionRecord)
        -> Result<SourceMatherr59601cProgress, SourceMatherr59601cError>
    {
        let value = match self.progress {
            SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::ErrnoWrite { value }) => value,
            _ => return Err(SourceMatherr59601cError::UnexpectedErrnoRecordReceipt { pending: self.progress, receipt, actual_exception }),
        };
        if receipt.written_value != value || receipt.advanced.errno() != value {
            return Err(SourceMatherr59601cError::ErrnoRecordReceiptMismatch { expected_value: value, receipt, actual_exception });
        }
        self.exception = actual_exception;
        self.state = receipt.advanced;
        self.complete(0);
        Ok(self.progress)
    }
    fn complete(&mut self, callback_return_eax: u32) {
        self.progress = SourceMatherr59601cProgress::Complete(SourceMatherr59601cCompletion {
            result_f64_bits: self.exception.result_f64_bits,
            callback_return_eax, exception: self.exception, advanced: self.state,
        });
    }

    pub fn retain_service_failure<E>(&mut self, exception: SourceMatherrExceptionRecord,
        advanced: ExplicitExceptionState, cause: E)
        -> Result<SourceMatherr59601cServiceFailure<E>, SourceMatherr59601cRejectedFailure<E>>
    {
        if let SourceMatherr59601cProgress::Complete(completed) = self.progress {
            return Err(SourceMatherr59601cRejectedFailure { cause, exception, supplied_advanced: advanced, completed });
        }
        self.exception = exception;
        self.state = advanced;
        if let SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::GuardedCall { target, type_hash, .. }) = self.progress {
            self.progress = SourceMatherr59601cProgress::Request(SourceMatherr59601cRequest::GuardedCall { target, type_hash, exception });
        }
        Ok(SourceMatherr59601cServiceFailure { cause, exception, advanced, pending: self.progress })
    }
}

pub(super) fn source_errno_value_for_class(class: i32) -> Option<i32> {
    match class { 1 => Some(33), 2 | 3 => Some(34), _ => None }
}
