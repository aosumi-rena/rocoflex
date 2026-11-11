use super::finite_power::{ExplicitExceptionState, signed_i64_to_f64_candidate_bits};
use super::integer_truncate::{masked_truncate_bits, MaskedTruncationFormat};
use super::scalar_multiply_f64::multiply_f64_candidate;
use super::scalar_divide_f64::divide_f64_candidate;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr, ScalarF32Rounding};
use super::vorbis_power::VorbisScalarError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmProbeArithmeticOperation { MultiplyF64, DivideF64 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmProbeMathError {
    ArchitectureNotClosed { architecture: ScalarF32Architecture, advanced: ExplicitExceptionState },
    UnsupportedEnvironment { advanced: ExplicitExceptionState },
    ArithmeticRequired {
        operation: PcmProbeArithmeticOperation, left: u64, right: u64, reason: VorbisScalarError,
    },
}
impl PcmProbeMathError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed { .. } => "PCM probe requires explicit IntelSdm arithmetic admission",
            Self::UnsupportedEnvironment { .. } => "PCM probe requires explicit RNE/all-masked/gradual controls",
            Self::ArithmeticRequired { operation: PcmProbeArithmeticOperation::MultiplyF64, .. } =>
                "PCM MULSD shared source candidate continuation required",
            Self::ArithmeticRequired { operation: PcmProbeArithmeticOperation::DivideF64, .. } =>
                "PCM DIVSD shared source candidate continuation required",
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::ArchitectureNotClosed { advanced, .. } | Self::UnsupportedEnvironment { advanced } => advanced,
            Self::ArithmeticRequired { reason, .. } => reason.known_advanced_state(entry),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmProbeWideningOutcome {
    pub bits: u64,
    pub raised_status: u32,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmProbeTruncationOutcome {
    pub value: i64,

    pub raised_status: u32,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmProbeArithmeticReceipt {
    pub operation: PcmProbeArithmeticOperation,
    pub left: u64,
    pub right: u64,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
    pub candidate_bits: Option<u64>,
    pub raised_status: Option<u8>,
    pub unmasked: Option<u8>,
    pub result_bits: Option<u64>,
    pub error: Option<PcmProbeMathError>,
}

pub struct SourcePcmProbeMath {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
    last_arithmetic: Option<PcmProbeArithmeticReceipt>,
    first_arithmetic_failure: Option<PcmProbeArithmeticReceipt>,
}
impl SourcePcmProbeMath {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, PcmProbeMathError> {
        if architecture != ScalarF32Architecture::IntelSdm {
            return Err(PcmProbeMathError::ArchitectureNotClosed { architecture, advanced: state });
        }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(PcmProbeMathError::UnsupportedEnvironment { advanced: state });
        }
        Ok(Self { architecture, state, last_arithmetic: None, first_arithmetic_failure: None })
    }
    pub fn name(&self) -> &'static str { "source-pcm-probe/sse2-i64-f64/rne-masked-gradual" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn last_arithmetic(&self) -> Option<&PcmProbeArithmeticReceipt> { self.last_arithmetic.as_ref() }

    pub fn first_arithmetic_failure(&self) -> Option<&PcmProbeArithmeticReceipt> {
        self.first_arithmetic_failure.as_ref()
    }

    pub fn signed_i64_to_f64_bits(&mut self, value: i64) -> Result<u64, PcmProbeMathError> {
        self.signed_i64_to_f64_bits_with_status(value).map(|outcome| outcome.bits)
    }

    pub fn signed_i64_to_f64_bits_with_status(&mut self, value: i64)
        -> Result<PcmProbeWideningOutcome, PcmProbeMathError> {
        let (bits, status) = signed_i64_to_f64_candidate_bits(value, ScalarF32Rounding::NearestEven);
        self.state = self.state.with_source_status(status);
        Ok(PcmProbeWideningOutcome { bits, raised_status: status, exception_state: self.state })
    }

    pub fn multiply_f64_bits(&mut self, left: u64, right: u64) -> Result<u64, PcmProbeMathError> {
        self.binary_f64_bits(PcmProbeArithmeticOperation::MultiplyF64, left, right)
    }

    pub fn divide_f64_bits(&mut self, left: u64, right: u64) -> Result<u64, PcmProbeMathError> {
        self.binary_f64_bits(PcmProbeArithmeticOperation::DivideF64, left, right)
    }

    fn binary_f64_bits(&mut self, operation: PcmProbeArithmeticOperation,
        left: u64, right: u64) -> Result<u64, PcmProbeMathError> {
        self.last_arithmetic = Some(PcmProbeArithmeticReceipt { operation, left, right,
            entry: self.state, advanced: self.state, candidate_bits: None,
            raised_status: None, unmasked: None, result_bits: None, error: None });
        let result = self.binary_f64_once(operation, left, right);
        if let Some(receipt) = self.last_arithmetic.as_mut() {
            receipt.advanced = self.state;
            match result {
                Ok(bits) => receipt.result_bits = Some(bits),
                Err(error) => {
                    receipt.error = Some(error);
                    if self.first_arithmetic_failure.is_none() {
                        self.first_arithmetic_failure = Some(*receipt);
                    }
                }
            }
        }
        result
    }

    fn binary_f64_once(&mut self, operation: PcmProbeArithmeticOperation,
        left: u64, right: u64) -> Result<u64, PcmProbeMathError> {
        let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
            .ok_or(PcmProbeMathError::UnsupportedEnvironment { advanced: self.state })?;
        let candidate = match operation {
            PcmProbeArithmeticOperation::MultiplyF64 => multiply_f64_candidate(left, right, before),
            PcmProbeArithmeticOperation::DivideF64 => divide_f64_candidate(left, right, before),
        };
        if let Some(receipt) = self.last_arithmetic.as_mut() {
            receipt.candidate_bits = candidate.bits;
            receipt.raised_status = Some(candidate.raised);
        }
        let continuation = |advanced| PcmProbeMathError::ArithmeticRequired {
            operation, left, right, reason: VorbisScalarError::SourceContinuationRequired {
                operation: match operation {
                    PcmProbeArithmeticOperation::MultiplyF64 => "PCM-probe/MULSD candidate/status continuation required",
                    PcmProbeArithmeticOperation::DivideF64 => "PCM-probe/DIVSD candidate/status continuation required",
                }, advanced,
            },
        };
        let raised = ScalarF32Exceptions::from_explicit_bits(candidate.raised)
            .ok_or_else(|| continuation(self.state))?;
        let (_, unmasked) = before.with_explicit_raised_status(raised);
        if let Some(receipt) = self.last_arithmetic.as_mut() {
            receipt.unmasked = Some(unmasked.bits());
        }

        self.state = self.state.with_source_status(u32::from(raised.bits()));
        if unmasked.bits() != 0 { return Err(continuation(self.state)); }
        candidate.bits.ok_or_else(|| continuation(self.state))
    }

    pub fn truncate_f64_bits_to_i64(&mut self, bits: u64) -> Result<i64, PcmProbeMathError> {
        Ok(self.truncate_f64_bits_to_i64_with_status(bits).value)
    }

    pub fn truncate_f64_bits_to_i64_with_status(&mut self, bits: u64) -> PcmProbeTruncationOutcome {
        let (value, raised_status) = masked_truncate_bits(bits, MaskedTruncationFormat::F64ToSigned64, false);
        self.state = self.state.with_source_status(raised_status);
        PcmProbeTruncationOutcome { value, raised_status, exception_state: self.state }
    }
}

