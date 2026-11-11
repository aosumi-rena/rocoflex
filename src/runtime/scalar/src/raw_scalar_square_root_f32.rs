use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};
use super::square_root::sqrt_f32_candidate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawSqrtssOutcome {
    Completed { bits: u32, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawSqrtssError {
    ControlRequired { reason: SourceRawCvttError },
    CandidateRequired { source: u32, mxcsr: u32, raised_status: u8 },
}
impl SourceRawSqrtssError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "SQRTSS requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::CandidateRequired { .. } => "SQRTSS candidate/exception continuation is not closed",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CandidateRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn sqrtss_with_raw_mxcsr(source: u32, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawSqrtssOutcome, SourceRawSqrtssError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawSqrtssError::ControlRequired { reason })?;
    let candidate = sqrt_f32_candidate(source, before.denormals_are_zero(),
        before.masks().bits() & 2 != 0, before.rounding());
    let raised = ScalarF32Exceptions::from_explicit_bits(candidate.raised)
        .ok_or(SourceRawSqrtssError::CandidateRequired { source, mxcsr, raised_status: candidate.raised })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 { return Ok(SourceRawSqrtssOutcome::Trapped { state, raised, unmasked }); }
    let bits = candidate.bits.ok_or(SourceRawSqrtssError::CandidateRequired {
        source, mxcsr: state.word(), raised_status: candidate.raised })?;
    Ok(SourceRawSqrtssOutcome::Completed { bits, state, raised })
}
