use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_narrow_f64::narrow_f64_candidate;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvtsd2ssOutcome {
    Completed { bits: u32, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },

    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvtsd2ssError {
    ControlRequired { reason: SourceRawCvttError },
    CandidateRequired { source: u64, mxcsr: u32, raised_status: u8 },
}
impl SourceRawCvtsd2ssError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "CVTSD2SS requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::CandidateRequired { .. } => "CVTSD2SS candidate/exception continuation is not closed",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CandidateRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn cvtsd2ss_with_raw_mxcsr(source: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawCvtsd2ssOutcome, SourceRawCvtsd2ssError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawCvtsd2ssError::ControlRequired { reason })?;
    let candidate = narrow_f64_candidate(source, before);
    let raised = ScalarF32Exceptions::from_explicit_bits(candidate.raised)
        .ok_or(SourceRawCvtsd2ssError::CandidateRequired { source, mxcsr, raised_status: candidate.raised })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 {
        return Ok(SourceRawCvtsd2ssOutcome::Trapped { state, raised, unmasked });
    }
    let bits = candidate.bits.ok_or(SourceRawCvtsd2ssError::CandidateRequired {
        source, mxcsr: state.word(), raised_status: candidate.raised })?;
    Ok(SourceRawCvtsd2ssOutcome::Completed { bits, state, raised })
}
