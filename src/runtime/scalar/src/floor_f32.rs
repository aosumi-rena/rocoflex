use super::finite_power::{ExplicitExceptionState, PowerError};
use super::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Error, ScalarF32InstructionSet,
    ScalarF32Mxcsr, ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome, SourceScalarF32};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFloorF32Error {
    ArchitectureNotClosed,
    SharedCore { error: ScalarF32Error, advanced: ExplicitExceptionState },
    LogicalStateRequired { reported_mxcsr: u32, advanced: ExplicitExceptionState },
    TrapRequired { reported_mxcsr: u32, unmasked: u8, advanced: ExplicitExceptionState },
}

#[derive(Clone, Copy, Debug)]
pub struct SourceFloorCallReceipt {
    pub input_bits: u32,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
    pub arithmetic: Option<SourceFloorArithmeticAttempt>,
    pub state_admission_error: Option<PowerError>,
    pub result_bits: Option<u32>,
    pub error: Option<SourceFloorF32Error>,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceFloorArithmeticAttempt {
    pub instruction: ScalarF32Operation,
    pub before: ExplicitExceptionState,
    pub reported: Option<ScalarF32Outcome>,
}
impl SourceFloorF32Error {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "580388 requires explicit Intel instruction semantics",
            Self::SharedCore { .. } => "5803fb shared legacy SUBSS service required",
            Self::LogicalStateRequired { .. } => "580388 returned scalar state is outside logical admission",
            Self::TrapRequired { .. } => "5803fb actual scalar trap continuation required",
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::ArchitectureNotClosed => entry,
            Self::SharedCore { advanced, .. } | Self::LogicalStateRequired { advanced, .. }
                | Self::TrapRequired { advanced, .. } => advanced,
        }
    }
}

pub struct SourceFloor580388 {
    scalar: SourceScalarF32,
    state: ExplicitExceptionState,
    last_call: Option<SourceFloorCallReceipt>,
    first_failure: Option<SourceFloorCallReceipt>,
}
impl SourceFloor580388 {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceFloorF32Error> {
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceFloorF32Error::ArchitectureNotClosed); }
        let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Subss])
            .map_err(|error| SourceFloorF32Error::SharedCore { error, advanced: state })?;
        let scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions)
            .map_err(|error| SourceFloorF32Error::SharedCore { error, advanced: state })?;
        Ok(Self { scalar, state, last_call: None, first_failure: None })
    }
    pub fn name(&self) -> &'static str { "source580388/f32/bitmask+legacy-subss" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.scalar.architecture() }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn last_call(&self) -> Option<&SourceFloorCallReceipt> { self.last_call.as_ref() }

    pub fn first_failure(&self) -> Option<&SourceFloorCallReceipt> { self.first_failure.as_ref() }
    pub fn evaluate(&mut self, value: f32) -> Result<f32, SourceFloorF32Error> {
        self.evaluate_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn evaluate_bits(&mut self, bits: u32) -> Result<u32, SourceFloorF32Error> {
        self.last_call = Some(SourceFloorCallReceipt { input_bits: bits, entry: self.state,
            advanced: self.state, arithmetic: None, state_admission_error: None,
            result_bits: None, error: None });
        let result = self.evaluate_source_bits(bits);
        if let Some(receipt) = self.last_call.as_mut() {
            receipt.advanced = self.state;
            match result {
                Ok(bits) => receipt.result_bits = Some(bits),
                Err(error) => {
                    receipt.error = Some(error);
                    if self.first_failure.is_none() { self.first_failure = Some(*receipt); }
                }
            }
        }
        result
    }
    fn evaluate_source_bits(&mut self, bits: u32) -> Result<u32, SourceFloorF32Error> {
        let magnitude = bits & 0x7fff_ffff;
        let negative = bits != magnitude;
        if magnitude >= 0x4b80_0000 {
            return Ok(if magnitude > 0x7f80_0000 { quiet_nan_596140_bits(bits) } else { bits });
        }
        if magnitude < 0x3f80_0000 {
            return Ok(if magnitude == 0 { bits } else if negative { 0xbf80_0000 } else { 0 });
        }
        let truncated = source_integral_mask_bits(bits);
        if !negative || truncated == bits { return Ok(truncated); }
        let entry = self.state;
        let instruction = ScalarF32Operation::Subss { left: truncated, right: 0x3f80_0000 };
        if let Some(receipt) = self.last_call.as_mut() {
            receipt.arithmetic = Some(SourceFloorArithmeticAttempt { instruction, before: entry, reported: None });
        }
        let input = ScalarF32Mxcsr::from_explicit_word(entry.mxcsr())
            .ok_or(SourceFloorF32Error::LogicalStateRequired { reported_mxcsr: entry.mxcsr(), advanced: entry })?;
        let outcome = self.scalar.execute(instruction, input)
            .map_err(|error| SourceFloorF32Error::SharedCore { error, advanced: entry })?;
        if let Some(attempt) = self.last_call.as_mut().and_then(|receipt| receipt.arithmetic.as_mut()) {
            attempt.reported = Some(outcome);
        }
        match project_scalar_f32_outcome(entry, outcome) {
            SourceScalarF32Projection::Completed { bits, advanced } => {
                self.state = advanced; Ok(bits)
            }
            SourceScalarF32Projection::Trapped { reported_mxcsr, unmasked, advanced } => {
                self.state = advanced;
                Err(SourceFloorF32Error::TrapRequired { reported_mxcsr, unmasked, advanced })
            }
            SourceScalarF32Projection::ReportedStateRequired { reported, reason, known_advanced } => {
                let reported_mxcsr = match reported {
                    ScalarF32Outcome::Completed { state, .. } | ScalarF32Outcome::Trapped { state, .. } => state.word(),
                };
                if let Some(receipt) = self.last_call.as_mut() { receipt.state_admission_error = Some(reason); }
                self.state = known_advanced;
                Err(SourceFloorF32Error::LogicalStateRequired { reported_mxcsr, advanced: known_advanced })
            }
        }
    }
}

pub(super) fn source_integral_mask_bits(bits: u32) -> u32 {
    let shift = 0x96u32.wrapping_sub(bits >> 23) & 31;
    bits & u32::MAX.wrapping_shl(shift)
}

pub(super) fn quiet_nan_596140_bits(bits: u32) -> u32 { bits | 0x0040_0000 }
