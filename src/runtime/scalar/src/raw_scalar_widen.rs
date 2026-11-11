use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_widen_f32::widen_f32_candidate;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvtss2sdOutcome {
    Completed { bits: u64, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },

    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvtss2sdError {
    ControlRequired { reason: SourceRawCvttError },
    CandidateRequired { source: u32, mxcsr: u32, raised_status: u8 },
}
impl SourceRawCvtss2sdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "CVTSS2SD requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::CandidateRequired { .. } => "CVTSS2SD candidate status is outside invalid-or-denormal policy",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CandidateRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn cvtss2sd_with_raw_mxcsr(source: u32, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawCvtss2sdOutcome, SourceRawCvtss2sdError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawCvtss2sdError::ControlRequired { reason })?;
    let (bits, raised_status) = widen_f32_candidate(source, before.denormals_are_zero());
    let raised = match raised_status {
        0 | 1 | 2 => ScalarF32Exceptions::from_explicit_bits(raised_status),
        _ => None,
    }.ok_or(SourceRawCvtss2sdError::CandidateRequired { source, mxcsr, raised_status })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 { Ok(SourceRawCvtss2sdOutcome::Trapped { state, raised, unmasked }) }
    else { Ok(SourceRawCvtss2sdOutcome::Completed { bits, state, raised }) }
}
