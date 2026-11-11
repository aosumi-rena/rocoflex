use super::finite_power::EntryComparison;
use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::scalar_compare_f64::f64_comparison_candidate;
use super::scalar_f64_operand::F64Operand;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Exceptions, ScalarF32Mxcsr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF64SelectionOpcode { Minsd, Cmpltsd }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawF64SelectionOutcome {
    Completed { bits: u64, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawF64SelectionError {
    ControlRequired { reason: SourceRawCvttError },
    CoreStatusRequired { opcode: SourceF64SelectionOpcode, mxcsr: u32, raised_status: u8 },

    NaNForwardDazRequired { left: u64, right: u64, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
}
impl SourceRawF64SelectionError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "MINSD/CMPLTSD requires explicit Intel/MXCSR/effective capabilities",
            Self::CoreStatusRequired { .. } => "MINSD/CMPLTSD status is outside invalid-or-denormal policy",
            Self::NaNForwardDazRequired { .. } => "MINSD first-NaN/second-subnormal forwarding under DAZ requires separate authority",
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

fn select_with_raw_mxcsr(opcode: SourceF64SelectionOpcode, left: u64, right: u64,
    mxcsr: u32, capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawF64SelectionOutcome, SourceRawF64SelectionError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawF64SelectionError::ControlRequired { reason })?;

    let (relation, raised_status) = f64_comparison_candidate(left, right, false, before.denormals_are_zero());
    let raised = match raised_status {
        0 | 1 | 2 => ScalarF32Exceptions::from_explicit_bits(raised_status),
        _ => None,
    }.ok_or(SourceRawF64SelectionError::CoreStatusRequired { opcode, mxcsr, raised_status })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 {
        return Ok(SourceRawF64SelectionOutcome::Trapped { state, raised, unmasked });
    }
    let bits = selection_f64_bits(opcode, left, right, relation, before.denormals_are_zero())
        .ok_or(SourceRawF64SelectionError::NaNForwardDazRequired { left, right, state, raised })?;
    Ok(SourceRawF64SelectionOutcome::Completed { bits, state, raised })
}

pub(super) fn selection_f64_bits(opcode: SourceF64SelectionOpcode,
    left: u64, right: u64, relation: EntryComparison, daz: bool) -> Option<u64> {
    Some(match opcode {
        SourceF64SelectionOpcode::Cmpltsd => if relation == EntryComparison::Less { u64::MAX } else { 0 },
        SourceF64SelectionOpcode::Minsd => {
            let a = F64Operand::from_bits(left);
            let b = F64Operand::from_bits(right);
            if daz && a.nan() && b.subnormal() {

                return None;
            }

            let selected = if relation == EntryComparison::Less { a } else { b };
            selected.with_daz(daz).bits()
        }
    })
}

pub fn minsd_with_raw_mxcsr(left: u64, right: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawF64SelectionOutcome, SourceRawF64SelectionError> {
    select_with_raw_mxcsr(SourceF64SelectionOpcode::Minsd, left, right, mxcsr, capabilities, architecture)
}

pub fn cmpltsd_with_raw_mxcsr(left: u64, right: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawF64SelectionOutcome, SourceRawF64SelectionError> {
    select_with_raw_mxcsr(SourceF64SelectionOpcode::Cmpltsd, left, right, mxcsr, capabilities, architecture)
}
