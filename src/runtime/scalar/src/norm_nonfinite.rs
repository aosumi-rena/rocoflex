use super::finite_power::ExplicitExceptionState;
use super::norm_f32::source_norm_widening_prefix;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Error, ScalarF32InstructionSet,
    ScalarF32Mxcsr, ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome, SourceScalarF32};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceNormNonfiniteError {
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    FinitePairRequiresOrdinary { first_bits: u32, second_bits: u32, entry: ExplicitExceptionState },
    SharedCore { error: ScalarF32Error, advanced: ExplicitExceptionState },
    LogicalStateRequired { actual_word: u32, advanced: ExplicitExceptionState },
    SourceInstructionTrap { actual_word: u32, unmasked_bits: u8, advanced: ExplicitExceptionState },
}
impl SourceNormNonfiniteError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "57990d explicit arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "57990d state is not RNE/all-masked/gradual",
            Self::FinitePairRequiresOrdinary { .. } => "57990d requires at least one nonfinite F32; use actual ordinary norm for finite pair",
            Self::SharedCore { .. } => "57990d actual legacy MULSS/ADDSS instruction admission required",
            Self::LogicalStateRequired { .. } => "57990d actual scalar result state is outside explicit logical-state admission",
            Self::SourceInstructionTrap { .. } => "57990d actual scalar instruction trap continuation required",
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::FinitePairRequiresOrdinary { entry, .. } => entry,
            Self::SharedCore { advanced, .. } | Self::LogicalStateRequired { advanced, .. }
                | Self::SourceInstructionTrap { advanced, .. } => advanced,
            _ => entry,
        }
    }
}

pub struct SourceNormNonfinite57990d {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
}
impl SourceNormNonfinite57990d {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState) -> Result<Self, SourceNormNonfiniteError> {
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceNormNonfiniteError::ArchitectureNotClosed); }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceNormNonfiniteError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { architecture, state })
    }
    pub fn name(&self) -> &'static str { "source579824/nonfinite57990d/legacy-shared-core" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn evaluate(&mut self, first: f32, second: f32) -> Result<f32, SourceNormNonfiniteError> {
        self.evaluate_bits(first.to_bits(), second.to_bits()).map(f32::from_bits)
    }
    pub fn evaluate_bits(&mut self, first: u32, second: u32) -> Result<u32, SourceNormNonfiniteError> {
        if first & 0x7fff_ffff < 0x7f80_0000 && second & 0x7fff_ffff < 0x7f80_0000 {
            return Err(SourceNormNonfiniteError::FinitePairRequiresOrdinary {
                first_bits: first, second_bits: second, entry: self.state });
        }
        let entry = self.state;
        match self.graph(first, second) {
            Ok((bits, advanced)) => { self.state = advanced; Ok(bits) }
            Err(error) => { self.state = error.known_advanced_state(entry); Err(error) }
        }
    }
    fn graph(&self, first: u32, second: u32) -> Result<(u32, ExplicitExceptionState), SourceNormNonfiniteError> {
        let prefix = source_norm_widening_prefix(first, second, self.state);
        let mut state = prefix.state;
        let first_magnitude = prefix.classification_first & 0x7fff_ffff_ffff_ffff;
        let second_magnitude = prefix.classification_second & 0x7fff_ffff_ffff_ffff;
        if first_magnitude == 0x7ff0_0000_0000_0000 || second_magnitude == 0x7ff0_0000_0000_0000 {
            return Ok((0x7f80_0000, state));
        }
        let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Mulss, ScalarF32Opcode::Addss])
            .map_err(|error| SourceNormNonfiniteError::SharedCore { error, advanced: state })?;
        let mut scalar = SourceScalarF32::from_explicit_architecture(self.architecture, instructions)
            .map_err(|error| SourceNormNonfiniteError::SharedCore { error, advanced: state })?;
        let first_square = scalar_step(&mut scalar, ScalarF32Operation::Mulss { left: first, right: first }, &mut state)?;
        let second_square = scalar_step(&mut scalar, ScalarF32Operation::Mulss { left: second, right: second }, &mut state)?;
        let result = scalar_step(&mut scalar, ScalarF32Operation::Addss { left: first_square, right: second_square }, &mut state)?;
        Ok((result, state))
    }
}

fn scalar_step(scalar: &mut SourceScalarF32, operation: ScalarF32Operation, state: &mut ExplicitExceptionState)
    -> Result<u32, SourceNormNonfiniteError>
{
    let entry = *state;
    let input = ScalarF32Mxcsr::from_explicit_word(entry.mxcsr())
        .ok_or(SourceNormNonfiniteError::LogicalStateRequired { actual_word: entry.mxcsr(), advanced: entry })?;
    let outcome = scalar.execute(operation, input)
        .map_err(|error| SourceNormNonfiniteError::SharedCore { error, advanced: entry })?;
    match outcome {
        ScalarF32Outcome::Completed { bits, state: actual } => {
            let advanced = ExplicitExceptionState::from_explicit_parts(actual.word(), entry.errno())
                .map_err(|_| SourceNormNonfiniteError::LogicalStateRequired { actual_word: actual.word(), advanced: entry })?;
            *state = advanced;
            Ok(bits)
        }
        ScalarF32Outcome::Trapped { state: actual, unmasked } => {
            let advanced = ExplicitExceptionState::from_explicit_parts(actual.word(), entry.errno())
                .map_err(|_| SourceNormNonfiniteError::LogicalStateRequired { actual_word: actual.word(), advanced: entry })?;
            *state = advanced;
            Err(SourceNormNonfiniteError::SourceInstructionTrap { actual_word: actual.word(), unmasked_bits: unmasked.bits(), advanced })
        }
    }
}
