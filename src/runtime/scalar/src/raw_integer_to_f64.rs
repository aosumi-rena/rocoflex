use super::finite_power::signed_i64_to_f64_candidate_bits;
use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Mxcsr, ScalarF32Exceptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvtsi2sdOutcome {
    Completed { bits: u64, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvtsi2sdError {
    ControlRequired { reason: SourceRawCvttError },
    CoreStatusRequired { mxcsr: u32, raised_status: u32 },
}
impl SourceRawCvtsi2sdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "signed64 CVTSI2SD requires admitted explicit Intel/MXCSR/capabilities",
            Self::CoreStatusRequired { .. } => "signed64 CVTSI2SD rounder status is outside precision-only policy",
        }
    }

    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CoreStatusRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn signed_i64_to_f64_with_raw_mxcsr(value: i64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawCvtsi2sdOutcome, SourceRawCvtsi2sdError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawCvtsi2sdError::ControlRequired { reason })?;
    let (bits, raised_status) = signed_i64_to_f64_candidate_bits(value, before.rounding());
    let raised = match raised_status {
        0 => ScalarF32Exceptions::from_explicit_bits(0),
        0x20 => Some(ScalarF32Exceptions::PRECISION),
        _ => None,
    }.ok_or(SourceRawCvtsi2sdError::CoreStatusRequired { mxcsr, raised_status })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 { Ok(SourceRawCvtsi2sdOutcome::Trapped { state, raised, unmasked }) }
    else { Ok(SourceRawCvtsi2sdOutcome::Completed { bits, state, raised }) }
}
