use super::finite_power::{comparison_flag_bits, ExplicitExceptionState, PowerError};
use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_compare_f64::f64_comparison_candidate;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceUcomisdFlags {
    pub zero: bool,
    pub parity: bool,
    pub carry: bool,
    pub defined_zero_mask: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawUcomisdOutcome {
    Completed { flags: SourceUcomisdFlags, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },

    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawUcomisdError {
    ControlRequired { reason: SourceRawCvttError },
    CoreStatusRequired { mxcsr: u32, raised_status: u8 },
}
impl SourceRawUcomisdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "UCOMISD requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::CoreStatusRequired { .. } => "UCOMISD candidate status is outside invalid-or-denormal policy",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CoreStatusRequired { mxcsr, .. } => mxcsr,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceStrictUcomisdOutcome {
    pub reported: SourceRawUcomisdOutcome,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceStrictUcomisdError {
    ArchitectureNotClosed { architecture: ScalarF32Architecture, advanced: ExplicitExceptionState },
    UnsupportedEnvironment { advanced: ExplicitExceptionState },
    EvaluationRequired { reason: SourceRawUcomisdError, advanced: ExplicitExceptionState },

    ReportedStateRequired { reported: SourceRawUcomisdOutcome, reason: PowerError,
        known_advanced: ExplicitExceptionState },
}
impl SourceStrictUcomisdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed { .. } => "strict UCOMISD requires explicit IntelSdm arithmetic admission",
            Self::UnsupportedEnvironment { .. } => "strict UCOMISD requires explicit RNE/allmasked/gradual controls",
            Self::EvaluationRequired { .. } => "strict UCOMISD shared evaluation continuation required",
            Self::ReportedStateRequired { .. } => "strict UCOMISD reported outcome state requires admission",
        }
    }
    pub fn known_advanced_state(self, _entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::ArchitectureNotClosed { advanced, .. } | Self::UnsupportedEnvironment { advanced }
                | Self::EvaluationRequired { advanced, .. } => advanced,
            Self::ReportedStateRequired { known_advanced, .. } => known_advanced,
        }
    }
}

pub fn ucomisd_f64_with_raw_mxcsr(left: u64, right: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawUcomisdOutcome, SourceRawUcomisdError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawUcomisdError::ControlRequired { reason })?;
    evaluate_ucomisd(left, right, before)
}

pub fn ucomisd_f64_with_explicit_state(left: u64, right: u64, entry: ExplicitExceptionState,
    architecture: ScalarF32Architecture) -> Result<SourceStrictUcomisdOutcome, SourceStrictUcomisdError> {
    if architecture != ScalarF32Architecture::IntelSdm {
        return Err(SourceStrictUcomisdError::ArchitectureNotClosed { architecture, advanced: entry });
    }
    if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 {
        return Err(SourceStrictUcomisdError::UnsupportedEnvironment { advanced: entry });
    }
    let before = ScalarF32Mxcsr::from_explicit_word(entry.mxcsr())
        .ok_or(SourceStrictUcomisdError::UnsupportedEnvironment { advanced: entry })?;
    let reported = evaluate_ucomisd(left, right, before)
        .map_err(|reason| SourceStrictUcomisdError::EvaluationRequired { reason, advanced: entry })?;
    let state = match reported {
        SourceRawUcomisdOutcome::Completed { state, .. } | SourceRawUcomisdOutcome::Trapped { state, .. } => state,
    };
    let exception_state = ExplicitExceptionState::from_explicit_parts(state.word(), entry.errno())
        .map_err(|reason| SourceStrictUcomisdError::ReportedStateRequired {
            reported, reason, known_advanced: entry })?;
    Ok(SourceStrictUcomisdOutcome { reported, exception_state })
}

fn evaluate_ucomisd(left: u64, right: u64, before: ScalarF32Mxcsr)
    -> Result<SourceRawUcomisdOutcome, SourceRawUcomisdError> {
    let (relation, raised_status) = f64_comparison_candidate(left, right, true, before.denormals_are_zero());
    let raised = match raised_status {
        0 | 1 | 2 => ScalarF32Exceptions::from_explicit_bits(raised_status),
        _ => None,
    }.ok_or(SourceRawUcomisdError::CoreStatusRequired { mxcsr: before.word(), raised_status })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 {
        return Ok(SourceRawUcomisdOutcome::Trapped { state, raised, unmasked });
    }
    let (carry, parity, zero) = comparison_flag_bits(relation);
    Ok(SourceRawUcomisdOutcome::Completed {
        flags: SourceUcomisdFlags { zero, parity, carry, defined_zero_mask: 0x890 }, state, raised,
    })
}
