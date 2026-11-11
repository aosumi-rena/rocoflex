use super::finite_power::NormalArithmeticError;
use super::raw_scalar_add::{raw_sum_with_mxcsr, SourceRawAddsdOutcome, SourceRawAddsdError};
use super::raw_scalar_truncate::SourceCvttMxcsrCapabilities;
use super::scalar_add_f64::ScalarF64SumOperation;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawSubsdOutcome {
    Completed { bits: u64, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawSubsdError {
    ControlRequired { reason: super::raw_scalar_truncate::SourceRawCvttError },
    AlignmentRequired { left: u64, right: u64, mxcsr: u32, reason: NormalArithmeticError },
    CandidateRequired { left: u64, right: u64, mxcsr: u32, raised_status: u8 },
}
impl SourceRawSubsdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "SUBSD requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::AlignmentRequired { .. } => "SUBSD exact aligned continuation is required",
            Self::CandidateRequired { .. } => "SUBSD candidate/exception continuation is not closed",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::AlignmentRequired { mxcsr, .. } | Self::CandidateRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn subsd_with_raw_mxcsr(left: u64, right: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawSubsdOutcome, SourceRawSubsdError> {
    let result = raw_sum_with_mxcsr(ScalarF64SumOperation::Subtract,
        left, right, mxcsr, capabilities, architecture).map_err(|error| match error {
            SourceRawAddsdError::ControlRequired { reason } => SourceRawSubsdError::ControlRequired { reason },
            SourceRawAddsdError::AlignmentRequired { left, right, mxcsr, reason } =>
                SourceRawSubsdError::AlignmentRequired { left, right, mxcsr, reason },
            SourceRawAddsdError::CandidateRequired { left, right, mxcsr, raised_status } =>
                SourceRawSubsdError::CandidateRequired { left, right, mxcsr, raised_status },
        })?;
    Ok(match result {
        SourceRawAddsdOutcome::Completed { bits, state, raised } => SourceRawSubsdOutcome::Completed { bits, state, raised },
        SourceRawAddsdOutcome::Trapped { state, raised, unmasked } => SourceRawSubsdOutcome::Trapped { state, raised, unmasked },
    })
}
