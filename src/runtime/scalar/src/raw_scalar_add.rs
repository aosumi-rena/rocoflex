use super::finite_power::NormalArithmeticError;
use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_add_f64::{sum_f64_candidate, ScalarF64SumOperation};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawAddsdOutcome {
    Completed { bits: u64, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawAddsdError {
    ControlRequired { reason: SourceRawCvttError },
    AlignmentRequired { left: u64, right: u64, mxcsr: u32, reason: NormalArithmeticError },
    CandidateRequired { left: u64, right: u64, mxcsr: u32, raised_status: u8 },
}
impl SourceRawAddsdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "ADDSD requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::AlignmentRequired { .. } => "ADDSD exact aligned continuation is required",
            Self::CandidateRequired { .. } => "ADDSD candidate/exception continuation is not closed",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::AlignmentRequired { mxcsr, .. } | Self::CandidateRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn addsd_with_raw_mxcsr(left: u64, right: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawAddsdOutcome, SourceRawAddsdError> {
    raw_sum_with_mxcsr(ScalarF64SumOperation::Add, left, right, mxcsr, capabilities, architecture)
}

pub(super) fn raw_sum_with_mxcsr(operation: ScalarF64SumOperation,
    left: u64, right: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawAddsdOutcome, SourceRawAddsdError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawAddsdError::ControlRequired { reason })?;
    let candidate = sum_f64_candidate(operation, left, right, before).map_err(|required| {
        let state = if required.raised == 2 {
            before.with_explicit_raised_status(ScalarF32Exceptions::DENORMAL).0
        } else { before };
        SourceRawAddsdError::AlignmentRequired { left, right, mxcsr: state.word(), reason: required.reason }
    })?;
    let raised = ScalarF32Exceptions::from_explicit_bits(candidate.raised)
        .ok_or(SourceRawAddsdError::CandidateRequired { left, right, mxcsr, raised_status: candidate.raised })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 { return Ok(SourceRawAddsdOutcome::Trapped { state, raised, unmasked }); }
    let bits = candidate.bits.ok_or(SourceRawAddsdError::CandidateRequired {
        left, right, mxcsr: state.word(), raised_status: candidate.raised })?;
    Ok(SourceRawAddsdOutcome::Completed { bits, state, raised })
}
