use super::raw_scalar_maximum_f32::{raw_minmax_with_mxcsr, SourceRawMaxssOutcome, SourceRawMaxssError};
use super::raw_scalar_truncate::{SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_select_f32::SourceF32MinMaxOpcode;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawMinssOutcome {
    Completed { bits: u32, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawMinssError {
    ControlRequired { reason: SourceRawCvttError },
    CoreStatusRequired { mxcsr: u32, raised_status: u8 },
    NaNForwardDazRequired { left: u32, right: u32, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
}
impl SourceRawMinssError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "MINSS requires explicit Intel/MXCSR/effective capabilities",
            Self::CoreStatusRequired { .. } => "MINSS candidate status is outside invalid-or-denormal policy",
            Self::NaNForwardDazRequired { .. } => "MINSS first-NaN/second-subnormal forwarding under DAZ requires separate authority",
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

pub fn minss_with_raw_mxcsr(left: u32, right: u32, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawMinssOutcome, SourceRawMinssError> {
    let result = raw_minmax_with_mxcsr(SourceF32MinMaxOpcode::Minss,
        left, right, mxcsr, capabilities, architecture).map_err(|error| match error {
            SourceRawMaxssError::ControlRequired { reason } => SourceRawMinssError::ControlRequired { reason },
            SourceRawMaxssError::CoreStatusRequired { mxcsr, raised_status } =>
                SourceRawMinssError::CoreStatusRequired { mxcsr, raised_status },
            SourceRawMaxssError::NaNForwardDazRequired { left, right, state, raised } =>
                SourceRawMinssError::NaNForwardDazRequired { left, right, state, raised },
        })?;
    Ok(match result {
        SourceRawMaxssOutcome::Completed { bits, state, raised } => SourceRawMinssOutcome::Completed { bits, state, raised },
        SourceRawMaxssOutcome::Trapped { state, raised, unmasked } => SourceRawMinssOutcome::Trapped { state, raised, unmasked },
    })
}
