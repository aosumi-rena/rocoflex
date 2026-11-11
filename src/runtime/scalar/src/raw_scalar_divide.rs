use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_divide_f64::divide_f64_candidate;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawDivsdOutcome {
    Completed { bits: u64, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawDivsdError {
    ControlRequired { reason: SourceRawCvttError },
    CandidateRequired { left: u64, right: u64, mxcsr: u32, raised_status: u8 },
}
impl SourceRawDivsdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "DIVSD requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::CandidateRequired { .. } => "DIVSD candidate/exception continuation is not closed",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CandidateRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn divsd_with_raw_mxcsr(left: u64, right: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawDivsdOutcome, SourceRawDivsdError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawDivsdError::ControlRequired { reason })?;
    let candidate = divide_f64_candidate(left, right, before);
    let raised = ScalarF32Exceptions::from_explicit_bits(candidate.raised)
        .ok_or(SourceRawDivsdError::CandidateRequired { left, right, mxcsr, raised_status: candidate.raised })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 { return Ok(SourceRawDivsdOutcome::Trapped { state, raised, unmasked }); }
    let bits = candidate.bits.ok_or(SourceRawDivsdError::CandidateRequired {
        left, right, mxcsr: state.word(), raised_status: candidate.raised })?;
    Ok(SourceRawDivsdOutcome::Completed { bits, state, raised })
}
