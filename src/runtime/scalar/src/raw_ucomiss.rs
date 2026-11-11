use super::shared_scalar_f32::comparison::f32_comparison_candidate;
use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceUcomissFlags {
    pub zero: bool,
    pub parity: bool,
    pub carry: bool,
    pub defined_zero_mask: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawUcomissOutcome {
    Completed { flags: SourceUcomissFlags, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },

    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawUcomissError {
    ControlRequired { reason: SourceRawCvttError },
    CoreStatusRequired { mxcsr: u32, raised_status: u8 },
}
impl SourceRawUcomissError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "UCOMISS requires admitted explicit Intel/MXCSR/effective capabilities",
            Self::CoreStatusRequired { .. } => "UCOMISS candidate status is outside invalid-or-denormal policy",
        }
    }
    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CoreStatusRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn ucomiss_f32_with_raw_mxcsr(left: u32, right: u32, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawUcomissOutcome, SourceRawUcomissError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawUcomissError::ControlRequired { reason })?;
    let (relation, raised_status) = f32_comparison_candidate(left, right, true, before.denormals_are_zero());
    let raised = match raised_status {
        0 | 1 | 2 => ScalarF32Exceptions::from_explicit_bits(raised_status),
        _ => None,
    }.ok_or(SourceRawUcomissError::CoreStatusRequired { mxcsr, raised_status })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 {
        return Ok(SourceRawUcomissOutcome::Trapped { state, raised, unmasked });
    }
    let (carry, parity, zero) = relation.flag_bits();
    Ok(SourceRawUcomissOutcome::Completed {
        flags: SourceUcomissFlags { zero, parity, carry, defined_zero_mask: 0x890 },
        state, raised,
    })
}
