use super::finite_power::ExplicitExceptionState;
use super::integer_truncate::{masked_truncate_bits, MaskedTruncationFormat};
use super::shared_scalar_f32::ScalarF32Architecture;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF32I32Truncation {

    pub low_dword: u32,
    pub raised_status: u32,
    pub exception_state: ExplicitExceptionState,
}
impl SourceF32I32Truncation {
    pub fn signed_value(self) -> i32 { self.low_dword as i32 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF32I64Truncation {

    pub low_qword: u64,
    pub raised_status: u32,
    pub exception_state: ExplicitExceptionState,
}
impl SourceF32I64Truncation {
    pub fn signed_value(self) -> i64 { self.low_qword as i64 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF64I32Truncation {

    pub low_dword: u32,
    pub raised_status: u32,
    pub exception_state: ExplicitExceptionState,
}
impl SourceF64I32Truncation {
    pub fn signed_value(self) -> i32 { self.low_dword as i32 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceScalarTruncationError {
    ArchitectureNotClosed { architecture: ScalarF32Architecture, advanced: ExplicitExceptionState },
    UnsupportedEnvironment { advanced: ExplicitExceptionState },
}
impl SourceScalarTruncationError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed { .. } => "scalar CVTT requires explicit IntelSdm arithmetic admission",
            Self::UnsupportedEnvironment { .. } => "scalar CVTT requires explicit RNE/allmasked/gradual controls",
        }
    }
    pub fn known_advanced_state(self, _entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self { Self::ArchitectureNotClosed { advanced, .. } | Self::UnsupportedEnvironment { advanced } => advanced }
    }
}

pub fn truncate_f32_to_i32_with_explicit_state(bits: u32, state: ExplicitExceptionState,
    architecture: ScalarF32Architecture) -> Result<SourceF32I32Truncation, SourceScalarTruncationError> {
    admit_scalar_state(state, architecture)?;
    let (value, raised_status) = masked_truncate_bits(u64::from(bits), MaskedTruncationFormat::F32ToSigned32, false);
    Ok(SourceF32I32Truncation { low_dword: value as u32, raised_status,
        exception_state: state.with_source_status(raised_status) })
}

fn admit_scalar_state(state: ExplicitExceptionState, architecture: ScalarF32Architecture)
    -> Result<(), SourceScalarTruncationError> {
    if architecture != ScalarF32Architecture::IntelSdm {
        return Err(SourceScalarTruncationError::ArchitectureNotClosed { architecture, advanced: state });
    }
    if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
        return Err(SourceScalarTruncationError::UnsupportedEnvironment { advanced: state });
    }
    Ok(())
}

pub fn truncate_f32_to_i64_with_explicit_state(bits: u32, state: ExplicitExceptionState,
    architecture: ScalarF32Architecture) -> Result<SourceF32I64Truncation, SourceScalarTruncationError> {
    admit_scalar_state(state, architecture)?;
    let (value, raised_status) = masked_truncate_bits(u64::from(bits), MaskedTruncationFormat::F32ToSigned64, false);
    Ok(SourceF32I64Truncation { low_qword: value as u64, raised_status,
        exception_state: state.with_source_status(raised_status) })
}

pub fn truncate_f64_to_i32_with_explicit_state(bits: u64, state: ExplicitExceptionState,
    architecture: ScalarF32Architecture) -> Result<SourceF64I32Truncation, SourceScalarTruncationError> {
    admit_scalar_state(state, architecture)?;
    let (value, raised_status) = masked_truncate_bits(bits, MaskedTruncationFormat::F64ToSigned32, false);
    Ok(SourceF64I32Truncation { low_dword: value as u32, raised_status,
        exception_state: state.with_source_status(raised_status) })
}
