use super::integer_truncate::{masked_truncate_bits, MaskedTruncationFormat};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Mxcsr, ScalarF32Exceptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCvttMxcsrCapabilities { supported_bits: u32 }
impl SourceCvttMxcsrCapabilities {

    pub fn from_explicit_supported_bits(supported_bits: u32) -> Result<Self, SourceRawCvttError> {
        if supported_bits & !0xffff != 0 || supported_bits & 0x3f != 0x3f {
            return Err(SourceRawCvttError::CapabilityMaskRequired { supported_bits });
        }
        Ok(Self { supported_bits })
    }
    pub fn supported_bits(self) -> u32 { self.supported_bits }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvttOutcome<T> {
    Completed { bits: T, state: ScalarF32Mxcsr, raised: ScalarF32Exceptions },
    Trapped { state: ScalarF32Mxcsr, raised: ScalarF32Exceptions, unmasked: ScalarF32Exceptions },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRawCvttError {
    ArchitectureNotClosed { architecture: ScalarF32Architecture, mxcsr: u32 },
    CapabilityMaskRequired { supported_bits: u32 },
    ReservedMxcsrBits { mxcsr: u32 },
    UnsupportedMxcsrBits { mxcsr: u32, supported_bits: u32, unsupported_bits: u32 },
    CoreStatusRequired { mxcsr: u32, raised_status: u32 },
}
impl SourceRawCvttError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed { .. } => "raw CVTT requires explicit IntelSdm arithmetic admission",
            Self::CapabilityMaskRequired { .. } => "explicit effective MXCSR support bitset is outside admitted capability domain",
            Self::ReservedMxcsrBits { .. } => "raw MXCSR contains unsupported architectural reserved bits",
            Self::UnsupportedMxcsrBits { .. } => "raw MXCSR sets bits outside the explicitly supplied capability bitset",
            Self::CoreStatusRequired { .. } => "CVTT candidate status is outside the closed invalid-or-precision policy",
        }
    }

    pub fn known_advanced_mxcsr(self, entry: u32) -> u32 {
        match self {
            Self::ArchitectureNotClosed { mxcsr, .. } | Self::ReservedMxcsrBits { mxcsr }
                | Self::UnsupportedMxcsrBits { mxcsr, .. } | Self::CoreStatusRequired { mxcsr, .. } => mxcsr,
            Self::CapabilityMaskRequired { .. } => entry,
        }
    }
}

pub(super) fn admit(mxcsr: u32, capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<ScalarF32Mxcsr, SourceRawCvttError> {
    if architecture != ScalarF32Architecture::IntelSdm {
        return Err(SourceRawCvttError::ArchitectureNotClosed { architecture, mxcsr });
    }
    let state = ScalarF32Mxcsr::from_explicit_word(mxcsr)
        .ok_or(SourceRawCvttError::ReservedMxcsrBits { mxcsr })?;
    let unsupported_bits = mxcsr & !capabilities.supported_bits;
    if unsupported_bits != 0 {
        return Err(SourceRawCvttError::UnsupportedMxcsrBits { mxcsr,
            supported_bits: capabilities.supported_bits, unsupported_bits });
    }
    Ok(state)
}

fn finish<T>(bits: T, raised_status: u32, before: ScalarF32Mxcsr)
    -> Result<SourceRawCvttOutcome<T>, SourceRawCvttError> {

    let raised = match raised_status {
        0 => ScalarF32Exceptions::from_explicit_bits(0),
        1 => Some(ScalarF32Exceptions::INVALID),
        0x20 => Some(ScalarF32Exceptions::PRECISION),
        _ => None,
    }.ok_or(SourceRawCvttError::CoreStatusRequired { mxcsr: before.word(), raised_status })?;
    let (state, unmasked) = before.with_explicit_raised_status(raised);
    if unmasked.bits() != 0 { Ok(SourceRawCvttOutcome::Trapped { state, raised, unmasked }) }
    else { Ok(SourceRawCvttOutcome::Completed { bits, state, raised }) }
}

pub fn truncate_f32_to_i32_with_raw_mxcsr(bits: u32, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawCvttOutcome<u32>, SourceRawCvttError> {
    let before = admit(mxcsr, capabilities, architecture)?;
    let (value, raised_status) = masked_truncate_bits(u64::from(bits),
        MaskedTruncationFormat::F32ToSigned32, before.denormals_are_zero());
    finish(value as u32, raised_status, before)
}

pub fn truncate_f64_to_i64_with_raw_mxcsr(bits: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawCvttOutcome<u64>, SourceRawCvttError> {
    let before = admit(mxcsr, capabilities, architecture)?;
    let (value, raised_status) = masked_truncate_bits(bits,
        MaskedTruncationFormat::F64ToSigned64, before.denormals_are_zero());
    finish(value as u64, raised_status, before)
}

pub fn truncate_f32_to_i64_with_raw_mxcsr(bits: u32, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawCvttOutcome<u64>, SourceRawCvttError> {
    let before = admit(mxcsr, capabilities, architecture)?;
    let (value, raised_status) = masked_truncate_bits(u64::from(bits),
        MaskedTruncationFormat::F32ToSigned64, before.denormals_are_zero());
    finish(value as u64, raised_status, before)
}

pub fn truncate_f64_to_i32_with_raw_mxcsr(bits: u64, mxcsr: u32,
    capabilities: SourceCvttMxcsrCapabilities, architecture: ScalarF32Architecture)
    -> Result<SourceRawCvttOutcome<u32>, SourceRawCvttError> {
    let before = admit(mxcsr, capabilities, architecture)?;
    let (value, raised_status) = masked_truncate_bits(bits,
        MaskedTruncationFormat::F64ToSigned32, before.denormals_are_zero());
    finish(value as u32, raised_status, before)
}
