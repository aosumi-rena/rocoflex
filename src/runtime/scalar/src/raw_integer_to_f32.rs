use super::raw_scalar_truncate::{admit, SourceCvttMxcsrCapabilities, SourceRawCvttError};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Error,
    ScalarF32InstructionSet, ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome,
    SourceScalarF32};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvtsi2ssError {
    ControlRequired { reason: SourceRawCvttError },
    SharedCoreRequired { error: ScalarF32Error, mxcsr: u32 },
}
impl SourceRawCvtsi2ssError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ControlRequired { .. } => "signed64 CVTSI2SS requires admitted explicit Intel/MXCSR/capabilities",
            Self::SharedCoreRequired { .. } => "signed64 CVTSI2SS shared opcode/architecture admission required",
        }
    }

    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ControlRequired { reason } => reason.known_advanced_mxcsr(entry),
            Self::SharedCoreRequired { mxcsr, .. } => mxcsr,
        }
    }
}

pub fn signed_i64_to_f32_with_raw_mxcsr(value: i64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<ScalarF32Outcome, SourceRawCvtsi2ssError> {
    let before = admit(mxcsr, capabilities, architecture)
        .map_err(|reason| SourceRawCvtsi2ssError::ControlRequired { reason })?;
    let required = |error| SourceRawCvtsi2ssError::SharedCoreRequired { error, mxcsr };
    let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Cvtsi2ssSigned64])
        .map_err(required)?;
    let mut scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions)
        .map_err(required)?;
    scalar.execute(ScalarF32Operation::Cvtsi2ssSigned64 { value }, before).map_err(required)
}
