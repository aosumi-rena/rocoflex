#[path="packed_f32_state.rs"]
mod state;
pub use self::state::{Kind1Rounding, Kind1Exceptions, Kind1Mxcsr,
    Kind1PackedOperation, Kind1PackedOutcome};
#[path="packed_f32_integer_core.rs"]
mod integer_core;
#[path="scalar_f32_comparison.rs"]
pub mod comparison;
pub(crate) use self::integer_core::{evaluate_packed_instruction,
    evaluate_subtract_low, evaluate_convert_u32_low, evaluate_convert_i32_low, evaluate_convert_i32_xmm, evaluate_convert_i64_low};

pub use self::state::{Kind1Rounding as ScalarF32Rounding,
    Kind1Exceptions as ScalarF32Exceptions, Kind1Mxcsr as ScalarF32Mxcsr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarF32Architecture { IntelSdm, Unclosed }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarF32Opcode {
    VAddss, VSubss, VMulss, Vfmadd213ss, Vfmadd231ss, Vfmsub213ss,
    Addss, Subss, Mulss, Cvtsi2ssSigned32, Cvtsi2ssZeroExtendedU32, AddpsXmm,
    MulpsXmm, Cvtdq2psXmm,
    Cvtsi2ssSigned64,
    Comiss, Ucomiss,
    Maxss,
    Cmpeqss,
    CmpeqpsXmm,
    Cmpltss,
    Cmpless,
}
impl ScalarF32Opcode {
    fn mask(self) -> u32 { 1 << (self as u8) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarF32InstructionSet(u32);
impl ScalarF32InstructionSet {
    pub fn from_explicit_opcodes(opcodes: &[ScalarF32Opcode]) -> Result<Self, ScalarF32Error> {
        let bits = opcodes.iter().fold(0, |bits, opcode| bits | opcode.mask());
        if bits == 0 { return Err(ScalarF32Error::EmptyInstructionSet); }
        Ok(Self(bits))
    }
    pub fn admits(self, opcode: ScalarF32Opcode) -> bool { self.0 & opcode.mask() != 0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarF32Error { ArchitectureNotClosed, EmptyInstructionSet, InstructionNotAdmitted }

#[derive(Debug, Clone, Copy)]
pub enum ScalarF32Operation {
    Cvtsi2ssSigned32 { value: i32 },

    Cvtsi2ssZeroExtendedU32 { value: u32 },
    Addss { left: u32, right: u32 },
    Subss { left: u32, right: u32 },
    Mulss { left: u32, right: u32 },
    VAddss { left: u32, right: u32 },
    VSubss { left: u32, right: u32 },
    VMulss { left: u32, right: u32 },
    Vfmadd213ss { destination: u32, source2: u32, source3: u32 },
    Vfmadd231ss { destination: u32, source2: u32, source3: u32 },
    Vfmsub213ss { destination: u32, source2: u32, source3: u32 },
    Cvtsi2ssSigned64 { value: i64 },
}
impl ScalarF32Operation {
    pub fn opcode(self) -> ScalarF32Opcode {
        match self {
            Self::Cvtsi2ssSigned32 { .. } => ScalarF32Opcode::Cvtsi2ssSigned32,
            Self::Cvtsi2ssZeroExtendedU32 { .. } => ScalarF32Opcode::Cvtsi2ssZeroExtendedU32,
            Self::Addss { .. } => ScalarF32Opcode::Addss,
            Self::Subss { .. } => ScalarF32Opcode::Subss,
            Self::Mulss { .. } => ScalarF32Opcode::Mulss,
            Self::VAddss { .. } => ScalarF32Opcode::VAddss,
            Self::VSubss { .. } => ScalarF32Opcode::VSubss,
            Self::VMulss { .. } => ScalarF32Opcode::VMulss,
            Self::Vfmadd213ss { .. } => ScalarF32Opcode::Vfmadd213ss,
            Self::Vfmadd231ss { .. } => ScalarF32Opcode::Vfmadd231ss,
            Self::Vfmsub213ss { .. } => ScalarF32Opcode::Vfmsub213ss,
            Self::Cvtsi2ssSigned64 { .. } => ScalarF32Opcode::Cvtsi2ssSigned64,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScalarF32Outcome {
    Completed { bits: u32, state: ScalarF32Mxcsr },
    Trapped { state: ScalarF32Mxcsr, unmasked: ScalarF32Exceptions },
}
#[derive(Debug, Clone, Copy)]
pub enum XmmF32Outcome {
    Completed { bits: [u32; 4], state: ScalarF32Mxcsr },
    Trapped { state: ScalarF32Mxcsr, unmasked: ScalarF32Exceptions },
}

enum ScalarMaskPredicate { Equal, Less, LessEqual }
fn relation_mask_candidate(left: u32, right: u32, daz: bool,
    quiet: bool, predicate: ScalarMaskPredicate) -> (u32, u8) {
    let (relation, raised) = comparison::f32_comparison_candidate(left, right, quiet, daz);
    let selected = match predicate {
        ScalarMaskPredicate::Equal => relation == comparison::F32Relation::Equal,
        ScalarMaskPredicate::Less => relation == comparison::F32Relation::Less,
        ScalarMaskPredicate::LessEqual => matches!(relation,
            comparison::F32Relation::Less | comparison::F32Relation::Equal),
    };
    (if selected { u32::MAX } else { 0 }, raised)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarF32CompareOperation {
    Comiss { left: u32, right: u32 },
    Ucomiss { left: u32, right: u32 },
}
impl ScalarF32CompareOperation {
    pub fn opcode(self) -> ScalarF32Opcode {
        match self { Self::Comiss { .. } => ScalarF32Opcode::Comiss,
            Self::Ucomiss { .. } => ScalarF32Opcode::Ucomiss }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarF32ComparisonFlags {
    pub carry: bool,
    pub parity: bool,
    pub zero: bool,
}
impl ScalarF32ComparisonFlags {
    pub fn defined_zero_mask(self) -> u32 { 0x890 }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarF32ComparisonOutcome {
    Completed { flags: ScalarF32ComparisonFlags, state: ScalarF32Mxcsr },

    Trapped { state: ScalarF32Mxcsr, unmasked: ScalarF32Exceptions },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarF32SelectionRequired {

    NaNForwardDaz { left: u32, right: u32 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarF32SelectionOutcome {
    Completed { bits: u32, state: ScalarF32Mxcsr },
    Trapped { state: ScalarF32Mxcsr, unmasked: ScalarF32Exceptions },
    Required { state: ScalarF32Mxcsr, required: ScalarF32SelectionRequired },
}

pub struct SourceScalarF32 { architecture: ScalarF32Architecture, instructions: ScalarF32InstructionSet }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceScalarF32Trap { pub state: ScalarF32Mxcsr, pub unmasked: ScalarF32Exceptions }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceScalarF32StateError {
    Instruction { error: ScalarF32Error, state: ScalarF32Mxcsr },
    Trapped(SourceScalarF32Trap),
    AlreadyTrapped(SourceScalarF32Trap),
    SelectionRequired { required: ScalarF32SelectionRequired, state: ScalarF32Mxcsr },
    AlreadySelectionRequired { required: ScalarF32SelectionRequired, state: ScalarF32Mxcsr },
}
impl SourceScalarF32StateError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Instruction { error: ScalarF32Error::ArchitectureNotClosed, .. } => "scalar F32 architecture is not closed",
            Self::Instruction { error: ScalarF32Error::EmptyInstructionSet, .. } => "scalar F32 instruction set is empty",
            Self::Instruction { error: ScalarF32Error::InstructionNotAdmitted, .. } => "scalar F32 instruction is not admitted",
            Self::Trapped(_) => "scalar F32 instruction raised an unmasked exception",
            Self::AlreadyTrapped(_) => "scalar F32 instruction sequence is stopped after a trap",
            Self::SelectionRequired { .. } => "scalar F32 first-NaN/second-subnormal forwarding under DAZ requires separate authority",
            Self::AlreadySelectionRequired { .. } => "scalar F32 instruction sequence is stopped at an unresolved selection",
        }
    }
    pub fn advanced_state(&self) -> ScalarF32Mxcsr {
        match *self { Self::Instruction { state, .. } | Self::SelectionRequired { state, .. } | Self::AlreadySelectionRequired { state, .. } => state, Self::Trapped(trap) | Self::AlreadyTrapped(trap) => trap.state }
    }
}

pub struct SourceScalarF32WithState {
    backend: SourceScalarF32,
    state: ScalarF32Mxcsr,
    trap: Option<SourceScalarF32Trap>,
    selection_required: Option<ScalarF32SelectionRequired>,
}
impl SourceScalarF32WithState {
    pub fn from_explicit_backend(backend: SourceScalarF32, state: ScalarF32Mxcsr) -> Self { Self { backend, state, trap: None, selection_required: None } }
    pub fn state(&self) -> ScalarF32Mxcsr { self.state }
    pub fn trapped(&self) -> Option<SourceScalarF32Trap> { self.trap }
    pub fn selection_required(&self) -> Option<ScalarF32SelectionRequired> { self.selection_required }
    pub fn backend(&self) -> &SourceScalarF32 { &self.backend }
    pub fn into_owned_parts(self) -> (SourceScalarF32, ScalarF32Mxcsr, Option<SourceScalarF32Trap>) { (self.backend, self.state, self.trap) }

    pub fn into_owned_parts_with_selection_boundary(self) -> (SourceScalarF32, ScalarF32Mxcsr, Option<SourceScalarF32Trap>, Option<ScalarF32SelectionRequired>) {
        (self.backend, self.state, self.trap, self.selection_required)
    }
    pub fn name(&self) -> &'static str { "explicit integer F32 instruction service" }
    fn ready(&self) -> Result<(), SourceScalarF32StateError> {
        if let Some(trap) = self.trap { return Err(SourceScalarF32StateError::AlreadyTrapped(trap)); }
        if let Some(required) = self.selection_required { return Err(SourceScalarF32StateError::AlreadySelectionRequired { required, state: self.state }); }
        Ok(())
    }
    fn commit_xmm(&mut self, outcome: Result<XmmF32Outcome, ScalarF32Error>) -> Result<[u32; 4], SourceScalarF32StateError> {
        match outcome {
            Err(error) => Err(SourceScalarF32StateError::Instruction { error, state: self.state }),
            Ok(XmmF32Outcome::Completed { bits, state }) => { self.state = state; Ok(bits) },
            Ok(XmmF32Outcome::Trapped { state, unmasked }) => {
                self.state = state; let trap = SourceScalarF32Trap { state, unmasked }; self.trap = Some(trap);
                Err(SourceScalarF32StateError::Trapped(trap))
            }
        }
    }
    fn scalar(&mut self, operation: ScalarF32Operation) -> Result<u32, SourceScalarF32StateError> {
        self.ready()?;
        let outcome = self.backend.execute(operation, self.state);
        self.commit_scalar(outcome)
    }
    fn commit_scalar(&mut self, outcome: Result<ScalarF32Outcome, ScalarF32Error>) -> Result<u32, SourceScalarF32StateError> {
        match outcome {
            Err(error) => Err(SourceScalarF32StateError::Instruction { error, state: self.state }),
            Ok(ScalarF32Outcome::Completed { bits, state }) => { self.state = state; Ok(bits) },
            Ok(ScalarF32Outcome::Trapped { state, unmasked }) => {
                self.state = state; let trap = SourceScalarF32Trap { state, unmasked }; self.trap = Some(trap);
                Err(SourceScalarF32StateError::Trapped(trap))
            }
        }
    }
    fn compare(&mut self, operation: ScalarF32CompareOperation)
        -> Result<ScalarF32ComparisonFlags, SourceScalarF32StateError> {
        self.ready()?;
        match self.backend.execute_comparison(operation, self.state) {
            Err(error) => Err(SourceScalarF32StateError::Instruction { error, state: self.state }),
            Ok(ScalarF32ComparisonOutcome::Completed { flags, state }) => { self.state = state; Ok(flags) },
            Ok(ScalarF32ComparisonOutcome::Trapped { state, unmasked }) => {
                self.state = state; let trap = SourceScalarF32Trap { state, unmasked }; self.trap = Some(trap);
                Err(SourceScalarF32StateError::Trapped(trap))
            }
        }
    }

    pub fn comiss(&mut self, left: u32, right: u32)
        -> Result<ScalarF32ComparisonFlags, SourceScalarF32StateError> {
        self.compare(ScalarF32CompareOperation::Comiss { left, right })
    }

    pub fn ucomiss(&mut self, left: u32, right: u32)
        -> Result<ScalarF32ComparisonFlags, SourceScalarF32StateError> {
        self.compare(ScalarF32CompareOperation::Ucomiss { left, right })
    }
    pub fn cvtdq2ps_xmm(&mut self, source: [i32; 4]) -> Result<[u32; 4], SourceScalarF32StateError> {
        self.ready()?; let outcome = self.backend.execute_cvtdq2ps_xmm(source, self.state); self.commit_xmm(outcome)
    }
    pub fn mulps_xmm(&mut self, left: [u32; 4], right: [u32; 4]) -> Result<[u32; 4], SourceScalarF32StateError> {
        self.ready()?; let outcome = self.backend.execute_mulps_xmm(left, right, self.state); self.commit_xmm(outcome)
    }

    pub fn cvtsi2ss_signed32_after_xorps(&mut self, value: i32) -> Result<u32, SourceScalarF32StateError> {
        self.scalar(ScalarF32Operation::Cvtsi2ssSigned32 { value })
    }
    pub fn addss(&mut self, left: u32, right: u32) -> Result<u32, SourceScalarF32StateError> {
        self.scalar(ScalarF32Operation::Addss { left, right })
    }
    pub fn subss(&mut self, left: u32, right: u32) -> Result<u32, SourceScalarF32StateError> {
        self.scalar(ScalarF32Operation::Subss { left, right })
    }
    pub fn mulss(&mut self, left: u32, right: u32) -> Result<u32, SourceScalarF32StateError> {
        self.scalar(ScalarF32Operation::Mulss { left, right })
    }

    pub fn cmpeqss(&mut self, left: u32, right: u32) -> Result<u32, SourceScalarF32StateError> {
        self.ready()?;
        let outcome = self.backend.execute_cmpeqss(left, right, self.state);
        self.commit_scalar(outcome)
    }

    pub fn cmpltss(&mut self, left: u32, right: u32) -> Result<u32, SourceScalarF32StateError> {
        self.ready()?;
        let outcome = self.backend.execute_cmpltss(left, right, self.state);
        self.commit_scalar(outcome)
    }

    pub fn cmpless(&mut self, left: u32, right: u32) -> Result<u32, SourceScalarF32StateError> {
        self.ready()?;
        let outcome = self.backend.execute_cmpless(left, right, self.state);
        self.commit_scalar(outcome)
    }

    pub fn cmpeqps_xmm(&mut self, left: [u32; 4], right: [u32; 4])
        -> Result<[u32; 4], SourceScalarF32StateError> {
        self.ready()?;
        let outcome = self.backend.execute_cmpeqps_xmm(left, right, self.state);
        self.commit_xmm(outcome)
    }

    pub fn maxss(&mut self, left: u32, right: u32) -> Result<u32, SourceScalarF32StateError> {
        self.ready()?;
        match self.backend.execute_maxss(left, right, self.state) {
            Err(error) => Err(SourceScalarF32StateError::Instruction { error, state: self.state }),
            Ok(ScalarF32SelectionOutcome::Completed { bits, state }) => { self.state = state; Ok(bits) },
            Ok(ScalarF32SelectionOutcome::Trapped { state, unmasked }) => {
                self.state = state; let trap = SourceScalarF32Trap { state, unmasked }; self.trap = Some(trap);
                Err(SourceScalarF32StateError::Trapped(trap))
            },
            Ok(ScalarF32SelectionOutcome::Required { state, required }) => {
                self.state = state; self.selection_required = Some(required);
                Err(SourceScalarF32StateError::SelectionRequired { required, state })
            },
        }
    }

    pub fn cvtsi2ss_signed64(&mut self, value: i64) -> Result<u32, SourceScalarF32StateError> {
        self.scalar(ScalarF32Operation::Cvtsi2ssSigned64 { value })
    }
}
impl SourceScalarF32 {
    pub fn from_explicit_architecture(architecture: ScalarF32Architecture, instructions: ScalarF32InstructionSet)
        -> Result<Self, ScalarF32Error>
    {
        if architecture == ScalarF32Architecture::Unclosed { return Err(ScalarF32Error::ArchitectureNotClosed); }
        Ok(Self { architecture, instructions })
    }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn instruction_set(&self) -> ScalarF32InstructionSet { self.instructions }

    pub fn execute_cmpeqss(&mut self, left: u32, right: u32, before: ScalarF32Mxcsr)
        -> Result<ScalarF32Outcome, ScalarF32Error> {
        self.execute_scalar_mask_comparison(ScalarF32Opcode::Cmpeqss, left, right, before)
    }

    pub fn execute_cmpltss(&mut self, left: u32, right: u32, before: ScalarF32Mxcsr)
        -> Result<ScalarF32Outcome, ScalarF32Error> {
        self.execute_scalar_mask_comparison(ScalarF32Opcode::Cmpltss, left, right, before)
    }

    pub fn execute_cmpless(&mut self, left: u32, right: u32, before: ScalarF32Mxcsr)
        -> Result<ScalarF32Outcome, ScalarF32Error> {
        self.execute_scalar_mask_comparison(ScalarF32Opcode::Cmpless, left, right, before)
    }

    fn execute_scalar_mask_comparison(&mut self, opcode: ScalarF32Opcode,
        left: u32, right: u32, before: ScalarF32Mxcsr)
        -> Result<ScalarF32Outcome, ScalarF32Error> {
        if !self.instructions.admits(opcode) { return Err(ScalarF32Error::InstructionNotAdmitted); }
        let (quiet, selected) = match opcode {
            ScalarF32Opcode::Cmpeqss => (true, ScalarMaskPredicate::Equal),
            ScalarF32Opcode::Cmpltss => (false, ScalarMaskPredicate::Less),
            ScalarF32Opcode::Cmpless => (false, ScalarMaskPredicate::LessEqual),
            _ => return Err(ScalarF32Error::InstructionNotAdmitted),
        };
        let (bits, raised) = relation_mask_candidate(left, right,
            before.denormals_are_zero(), quiet, selected);
        let (state, unmasked) = before.with_explicit_raised_status(ScalarF32Exceptions(raised));
        if unmasked.bits() != 0 { return Ok(ScalarF32Outcome::Trapped { state, unmasked }); }
        Ok(ScalarF32Outcome::Completed { bits, state })
    }

    pub fn execute_cmpeqps_xmm(&mut self, left: [u32; 4], right: [u32; 4], before: ScalarF32Mxcsr)
        -> Result<XmmF32Outcome, ScalarF32Error> {
        if !self.instructions.admits(ScalarF32Opcode::CmpeqpsXmm) { return Err(ScalarF32Error::InstructionNotAdmitted); }
        let mut bits = [0; 4];
        let mut raised = 0;
        for lane in 0..4 {
            let (mask, lane_raised) = relation_mask_candidate(left[lane], right[lane],
                before.denormals_are_zero(), true, ScalarMaskPredicate::Equal);
            bits[lane] = mask;
            raised |= lane_raised;
        }
        let (state, unmasked) = before.with_explicit_raised_status(ScalarF32Exceptions(raised));
        if unmasked.bits() != 0 { return Ok(XmmF32Outcome::Trapped { state, unmasked }); }
        Ok(XmmF32Outcome::Completed { bits, state })
    }

    pub fn execute_maxss(&mut self, left: u32, right: u32, before: ScalarF32Mxcsr)
        -> Result<ScalarF32SelectionOutcome, ScalarF32Error> {
        if !self.instructions.admits(ScalarF32Opcode::Maxss) { return Err(ScalarF32Error::InstructionNotAdmitted); }
        let (candidate, raised) = comparison::minmax_f32_candidate(comparison::F32MinMaxOpcode::Maxss,
            left, right, before.denormals_are_zero());
        let (state, unmasked) = before.with_explicit_raised_status(ScalarF32Exceptions(raised));
        if unmasked.bits() != 0 { return Ok(ScalarF32SelectionOutcome::Trapped { state, unmasked }); }
        Ok(match candidate {
            Some(bits) => ScalarF32SelectionOutcome::Completed { bits, state },
            None => ScalarF32SelectionOutcome::Required { state,
                required: ScalarF32SelectionRequired::NaNForwardDaz { left, right } },
        })
    }

    pub fn execute_comparison(&mut self, operation: ScalarF32CompareOperation, before: ScalarF32Mxcsr)
        -> Result<ScalarF32ComparisonOutcome, ScalarF32Error> {
        if !self.instructions.admits(operation.opcode()) { return Err(ScalarF32Error::InstructionNotAdmitted); }
        let (left, right, quiet) = match operation {
            ScalarF32CompareOperation::Comiss { left, right } => (left, right, false),
            ScalarF32CompareOperation::Ucomiss { left, right } => (left, right, true),
        };
        let (relation, raised) = comparison::f32_comparison_candidate(left, right, quiet, before.denormals_are_zero());

        let (state, unmasked) = before.with_explicit_raised_status(ScalarF32Exceptions(raised));
        if unmasked.bits() != 0 { return Ok(ScalarF32ComparisonOutcome::Trapped { state, unmasked }); }
        let (carry, parity, zero) = relation.flag_bits();
        Ok(ScalarF32ComparisonOutcome::Completed { flags: ScalarF32ComparisonFlags { carry, parity, zero }, state })
    }

    pub fn execute(&mut self, operation: ScalarF32Operation, state: ScalarF32Mxcsr)
        -> Result<ScalarF32Outcome, ScalarF32Error>
    {
        if !self.instructions.admits(operation.opcode()) { return Err(ScalarF32Error::InstructionNotAdmitted); }
        let packed = match operation {
            ScalarF32Operation::Cvtsi2ssSigned32 { value } => evaluate_convert_i32_low(value, state),
            ScalarF32Operation::Cvtsi2ssSigned64 { value } => evaluate_convert_i64_low(value, state),
            ScalarF32Operation::Cvtsi2ssZeroExtendedU32 { value } => evaluate_convert_u32_low(value, state),
            ScalarF32Operation::Addss { left, right } | ScalarF32Operation::VAddss { left, right } => evaluate_packed_instruction(
                Kind1PackedOperation::Vaddps { left: [left; 8], right: [right; 8] }, state),
            ScalarF32Operation::Subss { left, right } | ScalarF32Operation::VSubss { left, right } => evaluate_subtract_low(left, right, state),
            ScalarF32Operation::Mulss { left, right } | ScalarF32Operation::VMulss { left, right } => evaluate_packed_instruction(
                Kind1PackedOperation::Vmulps { left: [left; 8], right: [right; 8] }, state),
            ScalarF32Operation::Vfmadd213ss { destination, source2, source3 } => evaluate_packed_instruction(
                Kind1PackedOperation::Vfmadd231ps { accumulator: [source3; 8], left: [source2; 8], right: [destination; 8] }, state),
            ScalarF32Operation::Vfmadd231ss { destination, source2, source3 } => evaluate_packed_instruction(
                Kind1PackedOperation::Vfmadd231ps { accumulator: [destination; 8], left: [source2; 8], right: [source3; 8] }, state),
            ScalarF32Operation::Vfmsub213ss { destination, source2, source3 } => {

                let nan = |bits: u32| bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
                let addend = if nan(destination) || nan(source2) || nan(source3) { source3 }
                    else { source3 ^ 0x8000_0000 };
                evaluate_packed_instruction(Kind1PackedOperation::Vfmadd231ps {
                    accumulator: [addend; 8], left: [source2; 8], right: [destination; 8] }, state)
            }
        };
        Ok(match packed {
            Kind1PackedOutcome::Completed { result, state } => ScalarF32Outcome::Completed { bits: result[0], state },
            Kind1PackedOutcome::Trapped { state, unmasked } => ScalarF32Outcome::Trapped { state, unmasked },
        })
    }

    pub fn execute_addps_xmm(&mut self, left: [u32; 4], right: [u32; 4], state: ScalarF32Mxcsr)
        -> Result<XmmF32Outcome, ScalarF32Error>
    {
        if !self.instructions.admits(ScalarF32Opcode::AddpsXmm) {
            return Err(ScalarF32Error::InstructionNotAdmitted);
        }
        let left = [left[0], left[1], left[2], left[3], left[0], left[1], left[2], left[3]];
        let right = [right[0], right[1], right[2], right[3], right[0], right[1], right[2], right[3]];
        Ok(match evaluate_packed_instruction(Kind1PackedOperation::Vaddps { left, right }, state) {
            Kind1PackedOutcome::Completed { result, state } => XmmF32Outcome::Completed {
                bits: [result[0], result[1], result[2], result[3]], state },
            Kind1PackedOutcome::Trapped { state, unmasked } => XmmF32Outcome::Trapped { state, unmasked },
        })
    }

    pub fn execute_mulps_xmm(&mut self, left: [u32; 4], right: [u32; 4], state: ScalarF32Mxcsr)
        -> Result<XmmF32Outcome, ScalarF32Error>
    {
        if !self.instructions.admits(ScalarF32Opcode::MulpsXmm) {
            return Err(ScalarF32Error::InstructionNotAdmitted);
        }
        let left = [left[0], left[1], left[2], left[3], left[0], left[1], left[2], left[3]];
        let right = [right[0], right[1], right[2], right[3], right[0], right[1], right[2], right[3]];
        Ok(match evaluate_packed_instruction(Kind1PackedOperation::Vmulps { left, right }, state) {
            Kind1PackedOutcome::Completed { result, state } => XmmF32Outcome::Completed {
                bits: [result[0], result[1], result[2], result[3]], state },
            Kind1PackedOutcome::Trapped { state, unmasked } => XmmF32Outcome::Trapped { state, unmasked },
        })
    }

    pub fn execute_cvtdq2ps_xmm(&mut self, source: [i32; 4], state: ScalarF32Mxcsr)
        -> Result<XmmF32Outcome, ScalarF32Error>
    {
        if !self.instructions.admits(ScalarF32Opcode::Cvtdq2psXmm) {
            return Err(ScalarF32Error::InstructionNotAdmitted);
        }
        Ok(match evaluate_convert_i32_xmm(source, state) {
            Kind1PackedOutcome::Completed { result, state } => XmmF32Outcome::Completed {
                bits: [result[0], result[1], result[2], result[3]], state },
            Kind1PackedOutcome::Trapped { state, unmasked } => XmmF32Outcome::Trapped { state, unmasked },
        })
    }
}
