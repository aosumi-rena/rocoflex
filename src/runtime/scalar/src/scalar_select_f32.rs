use super::finite_power::{compare_f32_with_explicit_state, ExplicitExceptionState,
    SourceF32CompareOpcode, SourceF32Comparison};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::shared_scalar_f32::comparison::select_minmax_f32_bits;
pub use super::shared_scalar_f32::comparison::F32MinMaxOpcode as SourceF32MinMaxOpcode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF32Selection {
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCmpssPredicate { LessThanSignaling, LessEqualSignaling }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF32MaskComparison {

    pub mask_bits: u32,

    pub unordered: bool,
    pub less: bool,
    pub equal: bool,
    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF32SelectionError { ArchitectureNotClosed }

impl SourceF32SelectionError {
    pub fn reason(self) -> &'static str {
        match self { Self::ArchitectureNotClosed => "explicit Intel MINSS/MAXSS/CMPSS architecture required" }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState { entry }
}

fn signaling_relation(left: u32, right: u32, state: ExplicitExceptionState,
    architecture: ScalarF32Architecture) -> Result<SourceF32Comparison, SourceF32SelectionError> {
    if architecture != ScalarF32Architecture::IntelSdm {
        return Err(SourceF32SelectionError::ArchitectureNotClosed);
    }

    Ok(compare_f32_with_explicit_state(SourceF32CompareOpcode::Comiss, left, right, state))
}

pub fn minmax_f32_with_explicit_state(opcode: SourceF32MinMaxOpcode,
    left: u32, right: u32, state: ExplicitExceptionState,
    architecture: ScalarF32Architecture) -> Result<SourceF32Selection, SourceF32SelectionError> {
    let relation = signaling_relation(left, right, state, architecture)?;
    Ok(SourceF32Selection { bits: select_minmax_f32_bits(opcode, left, right,
            relation.carry, relation.parity, relation.zero),
        exception_state: relation.exception_state })
}

pub fn cmpss_with_explicit_state(predicate: SourceCmpssPredicate,
    left: u32, right: u32, state: ExplicitExceptionState,
    architecture: ScalarF32Architecture) -> Result<SourceF32MaskComparison, SourceF32SelectionError> {
    let relation = signaling_relation(left, right, state, architecture)?;
    let unordered = relation.parity;
    let less = relation.carry && !unordered;
    let equal = relation.zero && !unordered;
    let selected = match predicate {
        SourceCmpssPredicate::LessThanSignaling => less,
        SourceCmpssPredicate::LessEqualSignaling => less || equal,
    };
    Ok(SourceF32MaskComparison { mask_bits: if selected { u32::MAX } else { 0 },
        unordered, less, equal, exception_state: relation.exception_state })
}
