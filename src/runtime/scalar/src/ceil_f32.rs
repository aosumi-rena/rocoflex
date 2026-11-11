use super::finite_power::{ExplicitExceptionState, PowerError};
use super::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};
use super::floor_f32::{source_integral_mask_bits, quiet_nan_596140_bits};
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Error, ScalarF32InstructionSet,
    ScalarF32Mxcsr, ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome, SourceScalarF32};

const SOURCE_LITERAL_5AA6DC: u32 = 0x3f80_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCeilF32Error {
    ArchitectureNotClosed,
    SharedCore { error: ScalarF32Error, advanced: ExplicitExceptionState },
    LogicalStateRequired { reported_mxcsr: u32, advanced: ExplicitExceptionState },
    TrapRequired { reported_mxcsr: u32, unmasked: u8, advanced: ExplicitExceptionState },
}

#[derive(Clone, Copy, Debug)]
pub struct SourceCeilArithmeticAttempt {
    pub instruction: ScalarF32Operation,
    pub before: ExplicitExceptionState,
    pub reported: Option<ScalarF32Outcome>,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceCeilCallReceipt {
    pub input_bits: u32,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
    pub arithmetic: Option<SourceCeilArithmeticAttempt>,
    pub state_admission_error: Option<PowerError>,
    pub result_bits: Option<u32>,
    pub error: Option<SourceCeilF32Error>,
}
impl SourceCeilF32Error {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "57dd28 requires explicit Intel instruction semantics",
            Self::SharedCore { .. } => "57dda6 shared legacy ADDSS service required",
            Self::LogicalStateRequired { .. } => "57dd28 returned scalar state is outside logical admission",
            Self::TrapRequired { .. } => "57dda6 actual scalar trap continuation required",
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

pub struct SourceCeil57dd28 {
    scalar: SourceScalarF32,
    state: ExplicitExceptionState,
    last_call: Option<SourceCeilCallReceipt>,
    first_failure: Option<SourceCeilCallReceipt>,
}
impl SourceCeil57dd28 {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceCeilF32Error> {
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceCeilF32Error::ArchitectureNotClosed); }
        let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Addss])
            .map_err(|error| SourceCeilF32Error::SharedCore { error, advanced: state })?;
        let scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions)
            .map_err(|error| SourceCeilF32Error::SharedCore { error, advanced: state })?;
        Ok(Self { scalar, state, last_call: None, first_failure: None })
    }
    pub fn name(&self) -> &'static str { "source57dd28/f32/bitmask+legacy-addss" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.scalar.architecture() }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn last_call(&self) -> Option<&SourceCeilCallReceipt> { self.last_call.as_ref() }

    pub fn first_failure(&self) -> Option<&SourceCeilCallReceipt> { self.first_failure.as_ref() }
    pub fn evaluate(&mut self, value: f32) -> Result<f32, SourceCeilF32Error> {
        self.evaluate_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn evaluate_bits(&mut self, bits: u32) -> Result<u32, SourceCeilF32Error> {
        self.last_call = Some(SourceCeilCallReceipt { input_bits: bits, entry: self.state,
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
    fn evaluate_source_bits(&mut self, bits: u32) -> Result<u32, SourceCeilF32Error> {
        let magnitude = bits & 0x7fff_ffff;
        let negative = bits != magnitude;
        if magnitude >= 0x4b80_0000 {
            return Ok(if magnitude > 0x7f80_0000 { quiet_nan_596140_bits(bits) } else { bits });
        }
        if magnitude < 0x3f80_0000 {
            return Ok(if magnitude == 0 { bits }
                else if negative { 0x8000_0000 } else { SOURCE_LITERAL_5AA6DC });
        }
        let truncated = source_integral_mask_bits(bits);
        if negative || truncated == bits { return Ok(truncated); }
        let entry = self.state;
        let instruction = ScalarF32Operation::Addss { left: truncated, right: SOURCE_LITERAL_5AA6DC };
        if let Some(receipt) = self.last_call.as_mut() {
            receipt.arithmetic = Some(SourceCeilArithmeticAttempt { instruction, before: entry, reported: None });
        }
        let input = ScalarF32Mxcsr::from_explicit_word(entry.mxcsr())
            .ok_or(SourceCeilF32Error::LogicalStateRequired { reported_mxcsr: entry.mxcsr(), advanced: entry })?;
        let outcome = self.scalar.execute(instruction, input)
            .map_err(|error| SourceCeilF32Error::SharedCore { error, advanced: entry })?;
        if let Some(attempt) = self.last_call.as_mut().and_then(|receipt| receipt.arithmetic.as_mut()) {
            attempt.reported = Some(outcome);
        }
        match project_scalar_f32_outcome(entry, outcome) {
            SourceScalarF32Projection::Completed { bits, advanced } => {
                self.state = advanced; Ok(bits)
            }
            SourceScalarF32Projection::Trapped { reported_mxcsr, unmasked, advanced } => {
                self.state = advanced;
                Err(SourceCeilF32Error::TrapRequired { reported_mxcsr, unmasked, advanced })
            }
            SourceScalarF32Projection::ReportedStateRequired { reported, reason, known_advanced } => {
                let reported_mxcsr = match reported {
                    ScalarF32Outcome::Completed { state, .. } | ScalarF32Outcome::Trapped { state, .. } => state.word(),
                };
                if let Some(receipt) = self.last_call.as_mut() { receipt.state_admission_error = Some(reason); }
                self.state = known_advanced;
                Err(SourceCeilF32Error::LogicalStateRequired { reported_mxcsr, advanced: known_advanced })
            }
        }
    }
}
