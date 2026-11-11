use super::raw_scalar_select_f64::{selection_f64_bits, SourceF64SelectionOpcode};
use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_compare_f64::f64_comparison_candidate;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCmpltpdOutcome {
    Completed {
        masks: [u64; 2],
        state: ScalarF32Mxcsr,
        raised: ScalarF32Exceptions,
    },
    Trapped {
        state: ScalarF32Mxcsr,
        raised: ScalarF32Exceptions,
        unmasked: ScalarF32Exceptions,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCmpltpdError {
    ControlRequired { reason: SourceRawCvttError },
    CoreStatusRequired {
        left: [u64; 2],
        right: [u64; 2],
        mxcsr: u32,
        lane_raised: [u8; 2],
    },

    RequiredCrossLanePriority {
        left: [u64; 2],
        right: [u64; 2],
        mxcsr: u32,
        lane_raised: [u8; 2],
    },
    CoreMaskRequired {
        left: [u64; 2],
        right: [u64; 2],
        mxcsr: u32,
        lane: u8,
    },
}

impl SourceRawCmpltpdError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "CMPLTPD requires explicit Intel/MXCSR/effective capabilities",
            Self::CoreStatusRequired { .. } => "CMPLTPD lane status is outside invalid-or-denormal policy",
            Self::RequiredCrossLanePriority { .. } => "CMPLTPD fresh cross-lane invalid-plus-denormal priority is unimplemented",
            Self::CoreMaskRequired { .. } => "CMPLTPD existing signaling-less mask projection requires continuation",
        }
    }

    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::CoreStatusRequired { mxcsr, .. }
                | Self::RequiredCrossLanePriority { mxcsr, .. }
                | Self::CoreMaskRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn cmpltpd_with_raw_mxcsr(
    left: [u64; 2],
    right: [u64; 2],
    mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities,
    architecture: ScalarF32Architecture,
) -> Result<SourceRawCmpltpdOutcome, SourceRawCmpltpdError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawCmpltpdError::ControlRequired { reason })?;
    let daz = before.denormals_are_zero();
    let (relation0, raised0) = f64_comparison_candidate(left[0], right[0], false, daz);
    let (relation1, raised1) = f64_comparison_candidate(left[1], right[1], false, daz);
    let lane_raised = [raised0, raised1];
    if lane_raised.iter().any(|&bits| !matches!(bits, 0 | 1 | 2)) {
        return Err(SourceRawCmpltpdError::CoreStatusRequired {
            left, right, mxcsr, lane_raised,
        });
    }
    let raised_bits = raised0 | raised1;
    if raised_bits == 3 {
        return Err(SourceRawCmpltpdError::RequiredCrossLanePriority {
            left, right, mxcsr, lane_raised,
        });
    }
    let raised = ScalarF32Exceptions::from_explicit_bits(raised_bits)
        .ok_or(SourceRawCmpltpdError::CoreStatusRequired {
            left, right, mxcsr, lane_raised,
        })?;
    let masks = [
        selection_f64_bits(SourceF64SelectionOpcode::Cmpltsd,
            left[0], right[0], relation0, daz)
            .ok_or(SourceRawCmpltpdError::CoreMaskRequired {
                left, right, mxcsr, lane: 0,
            })?,
        selection_f64_bits(SourceF64SelectionOpcode::Cmpltsd,
            left[1], right[1], relation1, daz)
            .ok_or(SourceRawCmpltpdError::CoreMaskRequired {
                left, right, mxcsr, lane: 1,
            })?,
    ];
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 {
        return Ok(SourceRawCmpltpdOutcome::Trapped { state, raised, unmasked });
    }
    Ok(SourceRawCmpltpdOutcome::Completed { masks, state, raised })
}
