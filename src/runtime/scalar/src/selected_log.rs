use super::cpu0_log::ScalarPositiveLog;
use super::finite_power::{ExplicitExceptionState, MaskedMathError, MathErrorCallbackPolicy, MathErrorWithState};
use super::finite_power::{MathErrorCall, MathErrorCallbackFloat32, MathErrorOperation, MathErrorServiceError, SourceMathErrorCallback};
use super::finite_power::NormalArithmeticError;
use super::shared_scalar_f32::{ScalarF32Architecture, ScalarF32Error, ScalarF32InstructionSet, ScalarF32Mxcsr, ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome, SourceScalarF32};

mod tables {
    include!("../support/log_data.rs");
}

fn finite_components(bits: u32) -> Result<(bool, u32, i32, u32), NormalArithmeticError> {
    let field = (bits >> 23) & 255;
    let fraction = bits & 0x007f_ffff;
    if field == 255 { return Err(NormalArithmeticError::NonNormalOperand); }
    Ok((bits >> 31 != 0, if field == 0 { fraction } else { fraction | 0x0080_0000 },
        if field == 0 { -149 } else { field as i32 - 150 },
        if field == 0 && fraction != 0 { 2 } else { 0 }))
}

fn finite_divide_f32(left: u32, right: u32) -> Result<(u32, u32), NormalArithmeticError> {
    let (lsign, mut ls, mut lp, lf) = finite_components(left)?;
    let (rsign, mut rs, mut rp, rf) = finite_components(right)?;
    if rs == 0 { return Err(NormalArithmeticError::NonNormalOperand); }
    let sign = u32::from(lsign ^ rsign) << 31;
    let input_flags = lf | rf;
    if ls == 0 { return Ok((sign, input_flags)); }
    let left_shift = ls.leading_zeros() - 8;
    let right_shift = rs.leading_zeros() - 8;
    ls <<= left_shift;
    rs <<= right_shift;
    lp -= left_shift as i32;
    rp -= right_shift as i32;
    let below = ls < rs;
    let numerator = u64::from(ls) << if below { 24 } else { 23 };
    let divisor = u64::from(rs);
    let mut retained = numerator / divisor;
    let remainder = numerator % divisor;
    retained += u64::from(2 * remainder > divisor || (2 * remainder == divisor && retained & 1 != 0));
    let mut field = lp - rp + 127 - i32::from(below);
    if retained == (1 << 24) { retained >>= 1; field += 1; }
    if field >= 255 { return Ok((sign | 0x7f80_0000, input_flags | 0x28)); }
    if field >= 1 {
        return Ok((sign | ((field as u32) << 23) | (retained as u32 & 0x007f_ffff),
            input_flags | if remainder != 0 { 0x20 } else { 0 }));
    }

    let quantum_power = lp - rp + 149;
    let (whole, lost, denominator) = if quantum_power >= 0 {

        let numerator = u64::from(ls) << (quantum_power as u32);
        (numerator / divisor, numerator % divisor, divisor)
    } else if quantum_power == -1 {
        let denominator = divisor << 1;
        (u64::from(ls) / denominator, u64::from(ls) % denominator, denominator)
    } else { return Ok((sign, input_flags | 0x30)); };
    let rounded = whole + u64::from(2 * lost > denominator || (2 * lost == denominator && whole & 1 != 0));

    Ok((sign | rounded as u32, input_flags | if lost != 0 { 0x30 } else { 0 }))
}

fn normal_divide_f32(left: u32, right: u32) -> Result<(u32, u32), NormalArithmeticError> {
    let (_, _, _, lf) = finite_components(left)?;
    let (_, _, _, rf) = finite_components(right)?;
    if lf != 0 || rf != 0 { return Err(NormalArithmeticError::NonNormalOperand); }
    let result = finite_divide_f32(left, right)?;
    let magnitude = result.0 & 0x7fff_ffff;
    if magnitude >= 0x7f80_0000 || (left & 0x7fff_ffff != 0 && magnitude < 0x0080_0000) {
        return Err(NormalArithmeticError::NonNormalResult);
    }
    Ok(result)
}

fn exact_small_integer_f32(value: i32) -> u32 {
    if value == 0 { return 0; }

    let magnitude = value.unsigned_abs();
    let highest = 31 - magnitude.leading_zeros();
    (u32::from(value < 0) << 31) | ((highest + 127) << 23)
        | ((magnitude << (23 - highest)) & 0x007f_ffff)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLogWithState {
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
}

pub fn normal_f32_divide_with_explicit_state(
    left: u32, right: u32, state: ExplicitExceptionState,
) -> Result<SourceLogWithState, LogArithmeticError> {
    let (bits, flags) = normal_divide_f32(left, right)
        .map_err(|reason| LogArithmeticError::ArithmeticRequired { operation: "normal_f32_division", reason, advanced: state })?;
    Ok(SourceLogWithState { bits, exception_state: state.with_source_status(flags) })
}

pub fn finite_f32_divide_with_explicit_state(
    left: u32, right: u32, state: ExplicitExceptionState,
) -> Result<SourceLogWithState, LogArithmeticError> {
    let (bits, flags) = finite_divide_f32(left, right)
        .map_err(|reason| LogArithmeticError::ArithmeticRequired { operation: "finite_f32_division", reason, advanced: state })?;
    Ok(SourceLogWithState { bits, exception_state: state.with_source_status(flags) })
}

pub fn raw_divss_with_explicit_state(
    left: u32, right: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture,
) -> Result<SourceLogWithState, LogArithmeticError> {
    if architecture != ScalarF32Architecture::IntelSdm {
        return Err(LogArithmeticError::ScalarServiceRequired {
            operation: "raw_divss_architecture_admission", reason: ScalarF32Error::ArchitectureNotClosed, advanced: state,
        });
    }
    let lm = left & 0x7fff_ffff;
    let rm = right & 0x7fff_ffff;
    let left_nan = lm > 0x7f80_0000;
    let right_nan = rm > 0x7f80_0000;
    if left_nan || right_nan {
        let bits = (if left_nan { left } else { right }) | 0x0040_0000;
        let invalid = (left_nan && left & 0x0040_0000 == 0) || (right_nan && right & 0x0040_0000 == 0);
        return Ok(SourceLogWithState { bits, exception_state: state.with_source_status(u32::from(invalid)) });
    }
    let left_infinite = lm == 0x7f80_0000;
    let right_infinite = rm == 0x7f80_0000;
    if (lm == 0 && rm == 0) || (left_infinite && right_infinite) {
        return Ok(SourceLogWithState { bits: 0xffc0_0000, exception_state: state.with_source_status(1) });
    }
    let sign = (left ^ right) & 0x8000_0000;
    if rm == 0 {
        return Ok(SourceLogWithState {
            bits: sign | 0x7f80_0000,
            exception_state: state.with_source_status(if left_infinite { 0 } else { 4 }),
        });
    }
    let denormal = (lm != 0 && lm < 0x0080_0000) || (rm != 0 && rm < 0x0080_0000);
    if left_infinite || right_infinite {
        return Ok(SourceLogWithState {
            bits: sign | if left_infinite { 0x7f80_0000 } else { 0 },
            exception_state: state.with_source_status(if denormal { 2 } else { 0 }),
        });
    }
    finite_f32_divide_with_explicit_state(left, right, state)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogArithmeticError {
    Dispatch(Error),
    ArithmeticRequired { operation: &'static str, reason: NormalArithmeticError, advanced: ExplicitExceptionState },
    ScalarServiceRequired { operation: &'static str, reason: ScalarF32Error, advanced: ExplicitExceptionState },
    UnexpectedScalarState { operation: &'static str, reported_mxcsr: u32, advanced: ExplicitExceptionState },
    UnexpectedScalarTrap { operation: &'static str, reported_mxcsr: u32, unmasked: u8, advanced: ExplicitExceptionState },
}

#[derive(Clone, Copy, Debug)]
pub enum SourceLogObservedInstruction {
    Scalar(ScalarF32Operation),
    Divide { left: u32, right: u32 },
}

#[derive(Clone, Copy, Debug)]
pub struct SourceLogArithmeticStep {
    pub operation: &'static str,
    pub instruction: SourceLogObservedInstruction,
    pub result_bits: u32,
    pub before: ExplicitExceptionState,
    pub after: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceLogArithmeticFailure {
    pub operation: &'static str,
    pub instruction: SourceLogObservedInstruction,
    pub before: ExplicitExceptionState,
    pub known_advanced: ExplicitExceptionState,
    pub reported: Option<ScalarF32Outcome>,
    pub error: LogArithmeticError,
}

pub trait SourceLogOperationObserver {
    fn observe_step(&mut self, step: SourceLogArithmeticStep);

    fn observe_failure(&mut self, _failure: SourceLogArithmeticFailure) {}
}

pub struct SourceLogFailureCapture<'a> {
    downstream: Option<&'a mut dyn SourceLogOperationObserver>,
    last_completed: Option<SourceLogArithmeticStep>,
    first_failure: Option<SourceLogArithmeticFailure>,
}
impl<'a> SourceLogFailureCapture<'a> {
    pub fn from_explicit_observer(downstream: Option<&'a mut dyn SourceLogOperationObserver>) -> Self {
        Self { downstream, last_completed: None, first_failure: None }
    }
    pub fn last_completed(&self) -> Option<&SourceLogArithmeticStep> { self.last_completed.as_ref() }
    pub fn first_failure(&self) -> Option<&SourceLogArithmeticFailure> { self.first_failure.as_ref() }
}
impl SourceLogOperationObserver for SourceLogFailureCapture<'_> {
    fn observe_step(&mut self, step: SourceLogArithmeticStep) {
        self.last_completed = Some(step);
        if let Some(downstream) = self.downstream.as_deref_mut() { downstream.observe_step(step); }
    }
    fn observe_failure(&mut self, failure: SourceLogArithmeticFailure) {
        if self.first_failure.is_none() { self.first_failure = Some(failure); }
        if let Some(downstream) = self.downstream.as_deref_mut() { downstream.observe_failure(failure); }
    }
}

pub fn exact_log_exponent_conversion(value: i32) -> Option<u32> {
    if !(-149..=384).contains(&value) { return None; }
    Some(exact_small_integer_f32(value))
}

struct LogArithmetic<'a> {
    state: ExplicitExceptionState,
    scalar: SourceScalarF32,
    observer: Option<&'a mut dyn SourceLogOperationObserver>,
    fused: bool,
}

impl<'a> LogArithmetic<'a> {
    fn new(state: ExplicitExceptionState, architecture: ScalarF32Architecture, fused: bool, observer: Option<&'a mut dyn SourceLogOperationObserver>) -> Result<Self, LogArithmeticError> {
        let opcodes: &[ScalarF32Opcode] = if fused {
            &[ScalarF32Opcode::VAddss, ScalarF32Opcode::VSubss, ScalarF32Opcode::VMulss,
                ScalarF32Opcode::Vfmadd213ss, ScalarF32Opcode::Vfmadd231ss, ScalarF32Opcode::Vfmsub213ss]
        } else { &[ScalarF32Opcode::Addss, ScalarF32Opcode::Subss, ScalarF32Opcode::Mulss] };
        let instructions = ScalarF32InstructionSet::from_explicit_opcodes(opcodes)
            .map_err(|reason| LogArithmeticError::ScalarServiceRequired { operation: "selected_log_instruction_admission", reason, advanced: state })?;
        let scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions)
            .map_err(|reason| LogArithmeticError::ScalarServiceRequired { operation: "selected_log_architecture_admission", reason, advanced: state })?;
        Ok(Self { state, scalar, observer, fused })
    }

    fn retain_failure(&mut self, operation: &'static str, instruction: SourceLogObservedInstruction,
        before: ExplicitExceptionState, reported: Option<ScalarF32Outcome>, error: LogArithmeticError,
    ) -> LogArithmeticError {
        if let Some(observer) = self.observer.as_deref_mut() {
            observer.observe_failure(SourceLogArithmeticFailure { operation, instruction, before,
                known_advanced: self.state, reported, error });
        }
        error
    }

    fn divide(&mut self, operation: &'static str, left: u32, right: u32) -> Result<u32, LogArithmeticError> {
        let before = self.state;
        let (bits, flags) = match normal_divide_f32(left, right) {
            Ok(result) => result,
            Err(reason) => {
                let error = LogArithmeticError::ArithmeticRequired { operation, reason, advanced: before };
                return Err(self.retain_failure(operation, SourceLogObservedInstruction::Divide { left, right },
                    before, None, error));
            }
        };
        self.state = self.state.with_source_status(flags);
        if let Some(observer) = self.observer.as_deref_mut() {
            observer.observe_step(SourceLogArithmeticStep { operation, instruction: SourceLogObservedInstruction::Divide { left, right }, result_bits: bits, before, after: self.state });
        }
        Ok(bits)
    }

    fn instruction(&mut self, operation: &'static str, source: ScalarF32Operation) -> Result<u32, LogArithmeticError> {
        let before_state = self.state;
        let before = self.state.mxcsr();
        let state = match ScalarF32Mxcsr::from_explicit_word(before) {
            Some(state) => state,
            None => {
                let error = LogArithmeticError::Dispatch(Error::UnsupportedEnvironment);
                return Err(self.retain_failure(operation, SourceLogObservedInstruction::Scalar(source),
                    before_state, None, error));
            }
        };
        let outcome = match self.scalar.execute(source, state) {
            Ok(outcome) => outcome,
            Err(reason) => {
                let error = LogArithmeticError::ScalarServiceRequired { operation, reason, advanced: self.state };
                return Err(self.retain_failure(operation, SourceLogObservedInstruction::Scalar(source),
                    before_state, None, error));
            }
        };
        match outcome {
            ScalarF32Outcome::Completed { bits, state } => {
                let reported_mxcsr = state.word();
                if before & !0x3f != reported_mxcsr & !0x3f || before & 0x3f & !reported_mxcsr != 0 {
                    let error = LogArithmeticError::UnexpectedScalarState { operation, reported_mxcsr, advanced: self.state };
                    return Err(self.retain_failure(operation, SourceLogObservedInstruction::Scalar(source),
                        before_state, Some(outcome), error));
                }
                self.state = self.state.with_source_status(reported_mxcsr & 0x3f);
                if let Some(observer) = self.observer.as_deref_mut() {
                    observer.observe_step(SourceLogArithmeticStep { operation, instruction: SourceLogObservedInstruction::Scalar(source), result_bits: bits, before: before_state, after: self.state });
                }
                Ok(bits)
            }
            ScalarF32Outcome::Trapped { state, unmasked } => {
                let reported_mxcsr = state.word();
                self.state = self.state.with_source_status(reported_mxcsr & 0x3f);
                let error = LogArithmeticError::UnexpectedScalarTrap { operation, reported_mxcsr,
                    unmasked: unmasked.bits(), advanced: self.state };
                Err(self.retain_failure(operation, SourceLogObservedInstruction::Scalar(source),
                    before_state, Some(outcome), error))
            }
        }
    }

    fn add(&mut self, operation: &'static str, left: u32, right: u32) -> Result<u32, LogArithmeticError> {
        let source = if self.fused { ScalarF32Operation::VAddss { left, right } } else { ScalarF32Operation::Addss { left, right } };
        self.instruction(operation, source)
    }
    fn subtract(&mut self, operation: &'static str, left: u32, right: u32) -> Result<u32, LogArithmeticError> {
        let source = if self.fused { ScalarF32Operation::VSubss { left, right } } else { ScalarF32Operation::Subss { left, right } };
        self.instruction(operation, source)
    }
    fn multiply(&mut self, operation: &'static str, left: u32, right: u32) -> Result<u32, LogArithmeticError> {
        let source = if self.fused { ScalarF32Operation::VMulss { left, right } } else { ScalarF32Operation::Mulss { left, right } };
        self.instruction(operation, source)
    }
    fn fmadd213(&mut self, operation: &'static str, destination: u32, source2: u32, source3: u32) -> Result<u32, LogArithmeticError> {
        self.instruction(operation, ScalarF32Operation::Vfmadd213ss { destination, source2, source3 })
    }
    fn fmadd231(&mut self, operation: &'static str, destination: u32, source2: u32, source3: u32) -> Result<u32, LogArithmeticError> {
        self.instruction(operation, ScalarF32Operation::Vfmadd231ss { destination, source2, source3 })
    }
    fn fmsub213(&mut self, operation: &'static str, destination: u32, source2: u32, source3: u32) -> Result<u32, LogArithmeticError> {
        self.instruction(operation, ScalarF32Operation::Vfmsub213ss { destination, source2, source3 })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedEnvironment,
    NonzeroSourceProfileRequired,
    PositiveFiniteRequired,
    ExceptionPathRequired,
    ExternalMathErrorRequired {
        input_bits: u32, requested_result_bits: u32, error: MaskedMathError,
    },
    CallbackRequiredFromEntry { entry: SourceLogEntry },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLogPlan {
    PositiveFinite { bits: u32 },
    CompleteBits { bits: u32 },
    MaskedError {
        input_bits: u32, requested_result_bits: u32, error: MaskedMathError,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLogEntry {
    pub plan: SourceLogPlan,

    pub exception_state: ExplicitExceptionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLogExceptionalOutcome {
    Complete { bits: u32, exception_state: ExplicitExceptionState },

    PositiveArithmeticRequired { bits: u32, exception_state: ExplicitExceptionState },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogExceptionEvaluationError {
    Dispatch(Error),
    MathError(MathErrorServiceError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogEvaluationError {
    Arithmetic(LogArithmeticError),
    MathError(MathErrorServiceError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Environment {
    explicit_current_mxcsr: u32,
}

impl Environment {
    pub fn from_explicit_current_mxcsr(mxcsr: u32) -> Result<Self, Error> {

        if mxcsr & 0xffc0 != 0x1f80 {
            return Err(Error::UnsupportedEnvironment);
        }
        Ok(Self { explicit_current_mxcsr: mxcsr })
    }

    pub fn explicit_current_mxcsr(self) -> u32 {
        self.explicit_current_mxcsr
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogBranch {
    SeparateOperations,
    FusedOperations,
}

enum Implementation {
    Separate(ScalarPositiveLog),
    Fused,
}

pub struct SelectedProfilePositiveLog {
    explicit_source_cpu_word: u32,
    environment: Environment,
    implementation: Implementation,
}

impl SelectedProfilePositiveLog {
    pub fn bind(explicit_source_cpu_word: u32, mxcsr: u32) -> Result<Self, Error> {
        let environment = Environment::from_explicit_current_mxcsr(mxcsr)?;
        Self::with_environment(explicit_source_cpu_word, environment)
    }

    pub fn with_environment(
        explicit_source_cpu_word: u32,
        environment: Environment,
    ) -> Result<Self, Error> {
        let implementation = if explicit_source_cpu_word == 0 {
            let log = ScalarPositiveLog::bind(0, environment.explicit_current_mxcsr())
                .map_err(|_| Error::UnsupportedEnvironment)?;
            Implementation::Separate(log)
        } else {
            Implementation::Fused
        };
        Ok(Self { explicit_source_cpu_word, environment, implementation })
    }

    pub fn explicit_source_cpu_word(&self) -> u32 {
        self.explicit_source_cpu_word
    }

    pub fn environment(&self) -> Environment {
        self.environment
    }

    pub fn nonzero_source_entry_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> Result<SourceLogEntry, Error> {
        if self.explicit_source_cpu_word == 0 { return Err(Error::NonzeroSourceProfileRequired); }
        Ok(self.source_entry_with_explicit_state(bits, state))
    }

    pub fn source_entry_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState,
    ) -> SourceLogEntry {
        let magnitude = bits & 0x7fff_ffff;
        let status = if magnitude != 0 && magnitude < 0x0080_0000 { 2 } else { 0 };
        SourceLogEntry {
            plan: self.plan_source_bits(bits), exception_state: state.with_source_status(status),
        }
    }

    pub fn nonzero_positive_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture,
    ) -> Result<SourceLogWithState, LogArithmeticError> {
        if self.explicit_source_cpu_word == 0 { return Err(LogArithmeticError::Dispatch(Error::NonzeroSourceProfileRequired)); }
        self.positive_graph(bits, state, architecture, None)
    }

    pub fn nonzero_positive_with_observer(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture,
        observer: &mut impl SourceLogOperationObserver,
    ) -> Result<SourceLogWithState, LogArithmeticError> {
        if self.explicit_source_cpu_word == 0 { return Err(LogArithmeticError::Dispatch(Error::NonzeroSourceProfileRequired)); }
        self.positive_graph(bits, state, architecture, Some(observer))
    }

    pub fn positive_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture,
    ) -> Result<SourceLogWithState, LogArithmeticError> {
        self.positive_graph(bits, state, architecture, None)
    }

    pub fn positive_with_observer(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture,
        observer: &mut impl SourceLogOperationObserver,
    ) -> Result<SourceLogWithState, LogArithmeticError> {
        self.positive_graph(bits, state, architecture, Some(observer))
    }

    fn positive_graph(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture,
        observer: Option<&mut dyn SourceLogOperationObserver>,
    ) -> Result<SourceLogWithState, LogArithmeticError> {
        let entry = self.source_entry_with_explicit_state(bits, state);
        if !matches!(entry.plan, SourceLogPlan::PositiveFinite { .. }) {
            return Err(LogArithmeticError::Dispatch(Error::PositiveFiniteRequired));
        }
        let fused = self.explicit_source_cpu_word != 0;
        let mut arithmetic = LogArithmetic::new(entry.exception_state, architecture, fused, observer)?;
        let mut index_bits = bits;
        let mut mantissa = bits & 0x007f_ffff;
        let mut exponent = (bits >> 23) as i32 - 127;

        let difference = arithmetic.subtract("initial_input_minus_one", bits, 0x3f80_0000)?;
        if exponent == -127 {
            let normalized = arithmetic.subtract("subnormal_normalization", mantissa | 0x3f80_0000, 0x3f80_0000)?;
            exponent = (normalized >> 23) as i32 - 253;
            mantissa = normalized & 0x007f_ffff;
            index_bits = mantissa;
        }

        let exponent_bits = exact_small_integer_f32(exponent);
        let rounded_index_bits = (index_bits & 0x007f_0000)
            + ((index_bits & 0x0000_8000) << 1);
        let output;
        if difference & 0x7fff_ffff < 0x3d80_0000 {

            let delta = arithmetic.subtract("near_one_input_minus_one", bits, 0x3f80_0000)?;
            let denominator = arithmetic.add("near_one_denominator", 0x4000_0000, delta)?;
            let ratio = arithmetic.divide("near_one_division", delta, denominator)?;
            let correction = arithmetic.multiply("near_one_correction", delta, ratio)?;
            let doubled = arithmetic.add("near_one_double_ratio", ratio, ratio)?;
            let square = arithmetic.multiply("near_one_square", doubled, doubled)?;
            let cube = arithmetic.multiply("near_one_cube", doubled, square)?;
            let tail = if fused {
                let polynomial = arithmetic.fmadd213("near_one_polynomial_fma", square, 0x3c4c_cccd, 0x3daa_aaab)?;
                arithmetic.fmsub213("near_one_tail_fmsub", polynomial, cube, correction)?
            } else {
                let scaled = arithmetic.multiply("near_one_polynomial_multiply", square, 0x3c4c_cccd)?;
                let polynomial = arithmetic.add("near_one_polynomial_add", scaled, 0x3daa_aaab)?;
                let product = arithmetic.multiply("near_one_tail_multiply", polynomial, cube)?;
                arithmetic.subtract("near_one_tail_subtract", product, correction)?
            };
            output = arithmetic.add("near_one_final_add", delta, tail)?;
        } else {
            let index = (rounded_index_bits >> 16) as usize;
            let normalized = mantissa | 0x3f00_0000;
            let rounded = rounded_index_bits | 0x3f00_0000;
            let delta = arithmetic.subtract("table_difference", rounded, normalized)?;
            let residual = arithmetic.multiply("table_reciprocal_product", delta, tables::RECIPROCAL[index])?;
            if fused {
                let polynomial = arithmetic.fmadd213("table_polynomial_fma", 0x3eaa_aaab, residual, 0x3f00_0000)?;
                let square = arithmetic.multiply("table_residual_square", residual, residual)?;
                let corrected = arithmetic.fmadd231("table_corrected_residual_fma", residual, polynomial, square)?;
                let low = arithmetic.fmsub213("table_low_fmsub", 0x3805_fdf4, exponent_bits, corrected)?;
                let high = arithmetic.fmadd213("table_high_fma", 0x3f31_7000, exponent_bits, tables::LOG_HIGH[index])?;
                let low = arithmetic.add("table_low_add", low, tables::LOG_LOW[index])?;
                output = arithmetic.add("table_final_add", high, low)?;
            } else {

                let scaled = arithmetic.multiply("table_polynomial_multiply", residual, 0x3eaa_aaab)?;
                let square = arithmetic.multiply("table_residual_square", residual, residual)?;
                let polynomial = arithmetic.add("table_polynomial_add", scaled, 0x3f00_0000)?;
                let product = arithmetic.multiply("table_polynomial_square_multiply", polynomial, square)?;
                let low_product = arithmetic.multiply("table_low_exponent_multiply", 0x3805_fdf4, exponent_bits)?;
                let corrected = arithmetic.add("table_corrected_residual_add", residual, product)?;
                let low = arithmetic.subtract("table_low_subtract", low_product, corrected)?;
                let high_product = arithmetic.multiply("table_high_exponent_multiply", 0x3f31_7000, exponent_bits)?;
                let low = arithmetic.add("table_low_add", low, tables::LOG_LOW[index])?;
                let high = arithmetic.add("table_high_add", high_product, tables::LOG_HIGH[index])?;
                output = arithmetic.add("table_final_add", high, low)?;
            }
        }
        Ok(SourceLogWithState { bits: output, exception_state: arithmetic.state })
    }

    pub fn nonzero_with_explicit_policy(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture, callback: MathErrorCallbackPolicy,
    ) -> Result<SourceLogWithState, LogArithmeticError> {
        if self.explicit_source_cpu_word == 0 { return Err(LogArithmeticError::Dispatch(Error::NonzeroSourceProfileRequired)); }
        self.with_explicit_policy(bits, state, architecture, callback)
    }

    pub fn nonzero_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<SourceLogWithState, LogEvaluationError> {
        if self.explicit_source_cpu_word == 0 {
            return Err(LogEvaluationError::Arithmetic(LogArithmeticError::Dispatch(Error::NonzeroSourceProfileRequired)));
        }
        self.with_callback(bits, state, architecture, callback)
    }

    pub fn with_explicit_policy(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture, callback: MathErrorCallbackPolicy,
    ) -> Result<SourceLogWithState, LogArithmeticError> {
        let entry = self.source_entry_with_explicit_state(bits, state);
        match entry.plan {
            SourceLogPlan::PositiveFinite { .. } => self.positive_with_explicit_state(bits, state, architecture),
            SourceLogPlan::CompleteBits { bits } => Ok(SourceLogWithState { bits, exception_state: entry.exception_state }),
            SourceLogPlan::MaskedError { requested_result_bits, error, .. } => {
                if callback == MathErrorCallbackPolicy::Present {
                    return Err(LogArithmeticError::Dispatch(Error::CallbackRequiredFromEntry { entry }));
                }
                Ok(SourceLogWithState { bits: requested_result_bits, exception_state: entry.exception_state.apply_masked_math_error(error) })
            }
        }
    }

    pub fn with_callback(
        &self, bits: u32, state: ExplicitExceptionState, architecture: ScalarF32Architecture, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<SourceLogWithState, LogEvaluationError> {
        let entry = self.source_entry_with_explicit_state(bits, state);
        match entry.plan {
            SourceLogPlan::PositiveFinite { .. } => self.positive_with_explicit_state(bits, state, architecture).map_err(LogEvaluationError::Arithmetic),
            SourceLogPlan::CompleteBits { bits } => Ok(SourceLogWithState { bits, exception_state: entry.exception_state }),
            SourceLogPlan::MaskedError { .. } => {
                let result = self.resolve_exception_with_callback(bits, entry.exception_state, callback).map_err(LogEvaluationError::MathError)?;
                Ok(SourceLogWithState { bits: result.bits, exception_state: result.advanced })
            }
        }
    }

    pub fn nonzero_exceptional_with_explicit_policy(
        &self, bits: u32, state: ExplicitExceptionState, callback: MathErrorCallbackPolicy,
    ) -> Result<SourceLogExceptionalOutcome, Error> {
        let entry = self.nonzero_source_entry_with_explicit_state(bits, state)?;
        match entry.plan {
            SourceLogPlan::CompleteBits { bits } =>
                Ok(SourceLogExceptionalOutcome::Complete { bits, exception_state: entry.exception_state }),
            SourceLogPlan::PositiveFinite { bits } =>
                Ok(SourceLogExceptionalOutcome::PositiveArithmeticRequired { bits, exception_state: entry.exception_state }),
            SourceLogPlan::MaskedError { requested_result_bits, error, .. } => {
                if callback == MathErrorCallbackPolicy::Present { return Err(Error::CallbackRequiredFromEntry { entry }); }
                Ok(SourceLogExceptionalOutcome::Complete {
                    bits: requested_result_bits, exception_state: entry.exception_state.apply_masked_math_error(error),
                })
            }
        }
    }

    pub fn nonzero_exceptional_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<SourceLogExceptionalOutcome, LogExceptionEvaluationError> {
        let entry = self.nonzero_source_entry_with_explicit_state(bits, state).map_err(LogExceptionEvaluationError::Dispatch)?;
        match entry.plan {
            SourceLogPlan::CompleteBits { bits } =>
                Ok(SourceLogExceptionalOutcome::Complete { bits, exception_state: entry.exception_state }),
            SourceLogPlan::PositiveFinite { bits } =>
                Ok(SourceLogExceptionalOutcome::PositiveArithmeticRequired { bits, exception_state: entry.exception_state }),
            SourceLogPlan::MaskedError { .. } => {
                let result = self.resolve_exception_with_callback(bits, entry.exception_state, callback)
                    .map_err(LogExceptionEvaluationError::MathError)?;
                Ok(SourceLogExceptionalOutcome::Complete { bits: result.bits, exception_state: result.advanced })
            }
        }
    }

    pub fn branch(&self) -> LogBranch {
        match &self.implementation {
            Implementation::Separate(_) => LogBranch::SeparateOperations,
            Implementation::Fused => LogBranch::FusedOperations,
        }
    }

    pub fn evaluate_bits(&self, positive_finite_bits: u32) -> Result<u32, Error> {
        self.evaluate(f32::from_bits(positive_finite_bits)).map(f32::to_bits)
    }

    pub fn plan_source_bits(&self, bits: u32) -> SourceLogPlan {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude > 0x7f80_0000 {
            return SourceLogPlan::CompleteBits { bits: bits | 0x0040_0000 };
        }
        if bits == 0x7f80_0000 { return SourceLogPlan::CompleteBits { bits }; }
        if magnitude == 0 {
            return SourceLogPlan::MaskedError {
                input_bits: bits, requested_result_bits: 0xff80_0000,
                error: MaskedMathError::Singularity,
            };
        }
        if bits & 0x8000_0000 != 0 {
            return SourceLogPlan::MaskedError {
                input_bits: bits, requested_result_bits: 0xffc0_0000,
                error: MaskedMathError::Domain,
            };
        }
        SourceLogPlan::PositiveFinite { bits }
    }

    pub fn evaluate_source_bits(&self, bits: u32) -> Result<u32, Error> {
        match self.plan_source_bits(bits) {
            SourceLogPlan::PositiveFinite { bits } => self.evaluate_bits(bits),
            SourceLogPlan::CompleteBits { bits } => Ok(bits),
            SourceLogPlan::MaskedError { input_bits, requested_result_bits, error } =>
                Err(Error::ExternalMathErrorRequired { input_bits, requested_result_bits, error }),
        }
    }

    pub fn resolve_exception_with_explicit_state(
        &self, bits: u32, state: ExplicitExceptionState, callback: MathErrorCallbackPolicy,
    ) -> Result<MathErrorWithState, Error> {
        let SourceLogPlan::MaskedError { input_bits, requested_result_bits, error } = self.plan_source_bits(bits)
            else { return Err(Error::ExceptionPathRequired); };
        if callback == MathErrorCallbackPolicy::Present {
            return Err(Error::ExternalMathErrorRequired { input_bits, requested_result_bits, error });
        }
        Ok(MathErrorWithState {
            bits: requested_result_bits,
            exception_state: state.apply_masked_math_error(error),
            error,
        })
    }

    pub fn resolve_exception_with_callback(
        &self, bits: u32, state: ExplicitExceptionState, callback: &mut impl SourceMathErrorCallback,
    ) -> Result<MathErrorCallbackFloat32, MathErrorServiceError> {
        let SourceLogPlan::MaskedError { requested_result_bits, error, .. } = self.plan_source_bits(bits)
            else { return Err(MathErrorServiceError::PathRequired { advanced: state }); };
        let (argument1_f64_bits, state) = state.widen_f32_for_math_error(bits);
        let request = MathErrorCall {
            operation: MathErrorOperation::Logarithm,
            error,
            argument1_f64_bits,
            argument2_f64_bits: 0,
            proposed_result_f64_bits: if requested_result_bits == 0xff80_0000 {
                0xfff0_0000_0000_0000
            } else { 0xfff8_0000_0000_0000 },
        };
        let mut result = state.call_math_error(request, callback).map_err(MathErrorServiceError::Callback)?
            .into_rne_f32().map_err(MathErrorServiceError::Conversion)?;
        result.bits = requested_result_bits;
        Ok(result)
    }

    pub fn evaluate(&self, value: f32) -> Result<f32, Error> {
        let bits = value.to_bits();

        if bits == 0 || bits >= 0x7f80_0000 {
            return Err(Error::PositiveFiniteRequired);
        }
        match &self.implementation {
            Implementation::Separate(log) => log.evaluate(value)
                .map_err(|_| Error::PositiveFiniteRequired),
            Implementation::Fused => Ok(super::fused_log::positive_finite(
                value, bits, &tables::RECIPROCAL, &tables::LOG_HIGH, &tables::LOG_LOW,
            )),
        }
    }
}
