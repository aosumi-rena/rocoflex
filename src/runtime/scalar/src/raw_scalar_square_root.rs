use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};
use super::square_root_f64::sqrt_f64_candidate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawSqrtsdOutcome {
    Completed { bits: u64, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawSqrtsdError {
    ControlRequired { reason: SourceRawCvttError },
    CandidateRequired { source: u64, mxcsr: u32, raised_status: u8 },
}
impl SourceRawSqrtsdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "SQRTSD requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::CandidateRequired { .. } => "SQRTSD candidate/exception continuation is not closed",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CandidateRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn sqrtsd_with_raw_mxcsr(source: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawSqrtsdOutcome, SourceRawSqrtsdError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawSqrtsdError::ControlRequired { reason })?;
    let candidate = sqrt_f64_candidate(source, before);
    let raised = ScalarF32Exceptions::from_explicit_bits(candidate.raised)
        .ok_or(SourceRawSqrtsdError::CandidateRequired { source, mxcsr, raised_status: candidate.raised })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 { return Ok(SourceRawSqrtsdOutcome::Trapped { state, raised, unmasked }); }
    let bits = candidate.bits.ok_or(SourceRawSqrtsdError::CandidateRequired {
        source, mxcsr: state.word(), raised_status: candidate.raised })?;
    Ok(SourceRawSqrtsdOutcome::Completed { bits, state, raised })
}
