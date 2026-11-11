use super::shared_scalar_f32::comparison::minmax_f32_candidate;
use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_select_f32::SourceF32MinMaxOpcode;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawMaxssOutcome {
    Completed { bits: u32, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawMaxssError {
    ControlRequired { reason: SourceRawCvttError },
    CoreStatusRequired { mxcsr: u32, raised_status: u8 },

    NaNForwardDazRequired { left: u32, right: u32, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
}
impl SourceRawMaxssError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "MAXSS requires explicit Intel/MXCSR/effective capabilities",
            Self::CoreStatusRequired { .. } => "MAXSS candidate status is outside invalid-or-denormal policy",
            Self::NaNForwardDazRequired { .. } => "MAXSS first-NaN/second-subnormal forwarding under DAZ requires separate authority",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CoreStatusRequired { mxcsr, .. } => mxcsr,
            Self::NaNForwardDazRequired { state, .. } => state.word(),
        }
    }
}

pub fn maxss_with_raw_mxcsr(left: u32, right: u32, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawMaxssOutcome, SourceRawMaxssError> {
    raw_minmax_with_mxcsr(SourceF32MinMaxOpcode::Maxss, left, right, mxcsr, capabilities, architecture)
}

pub(super) fn raw_minmax_with_mxcsr(opcode: SourceF32MinMaxOpcode,
    left: u32, right: u32, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawMaxssOutcome, SourceRawMaxssError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawMaxssError::ControlRequired { reason })?;

    let (bits, raised_status) = minmax_f32_candidate(opcode, left, right, before.denormals_are_zero());
    let raised = match raised_status {
        0 | 1 | 2 => ScalarF32Exceptions::from_explicit_bits(raised_status),
        _ => None,
    }.ok_or(SourceRawMaxssError::CoreStatusRequired { mxcsr, raised_status })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 { return Ok(SourceRawMaxssOutcome::Trapped { state, raised, unmasked }); }
    let bits = bits.ok_or(SourceRawMaxssError::NaNForwardDazRequired { left, right, state, raised })?;
    Ok(SourceRawMaxssOutcome::Completed { bits, state, raised })
}
