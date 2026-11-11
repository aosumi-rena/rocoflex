use crate::source_scalar as literal;
use literal::{ExplicitExceptionState, ScalarF32Architecture};
use literal::shared_scalar_f32::{SourceScalarF32, ScalarF32Opcode, ScalarF32Operation,
    ScalarF32InstructionSet, ScalarF32Mxcsr, ScalarF32Outcome, ScalarF32Error};

#[allow(dead_code)]
#[path = "runtime/canvas/parameter_range.rs"]
pub(super) mod range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CallInput {
    pub pc: u32, pub kind: range::Call, pub operand: u32,
    pub entry: ExplicitExceptionState, pub architecture: ScalarF32Architecture,
}
pub(super) struct CallReturned { pub input: CallInput, pub bits: u32, pub advanced: ExplicitExceptionState }
pub(super) struct CallStopped<E> {
    pub input: CallInput, pub error: E, pub known_advanced: Option<ExplicitExceptionState>,
}
pub(super) trait Calls {
    type Error;

    fn call(&mut self, input: CallInput) -> Result<CallReturned, CallStopped<Self::Error>>;
}

pub(super) enum BindFailure {
    Scalar { reason: ScalarF32Error, architecture: ScalarF32Architecture, entry: ExplicitExceptionState },
}
pub(super) enum Failure {
    State { pc: u32, reported_mxcsr: u32, errno: i32 },
    Scalar { pc: u32, reason: ScalarF32Error },
    Trap { pc: u32, reported_mxcsr: u32, unmasked: u8, state_admitted: bool },
    Division { pc: u32, error: literal::LogArithmeticError },
    Selection { pc: u32, error: literal::SourceF32SelectionError },
    Floor { pc: u32, error: literal::SourceFloorF32Error },
    Call { expected: CallInput, matched: bool, known_advanced: Option<ExplicitExceptionState> },
    ContextState { expected: ExplicitExceptionState, received_entry: ExplicitExceptionState,
        received_after: ExplicitExceptionState },
}
pub(super) enum CallFailure<E> {
    AlreadyStopped,
    Service(CallStopped<E>),
    Mismatched(CallReturned),
}
pub(super) enum Operation {
    Binary { opcode: range::Binary, left: u32, right: u32 },
    Compare { opcode: range::CompareOpcode, left: u32, right: u32 },
    FloorValue { operand: u32 },
    Call(CallInput),
}
pub(super) enum Value {
    Bits(u32), Comparison { value: range::Comparison, mask_bits: Option<u32> },
}
pub(super) struct Last {
    pub pc: u32, pub operation: Operation, pub entry: ExplicitExceptionState,
    pub result: Option<Value>,
}

#[must_use = "retain arithmetic/call failure prefixes and the caller's actual context"]
pub(super) struct Math {
    scalar: SourceScalarF32,
    state: ExplicitExceptionState,
    failure: Option<Failure>,
    last: Option<Last>,
}
pub(super) struct Retained {
    pub scalar: SourceScalarF32, pub known_prefix_state: ExplicitExceptionState,
    pub failure: Option<Failure>, pub last: Option<Last>,
}

impl Math {
    pub fn bind(architecture: ScalarF32Architecture, entry: ExplicitExceptionState)
        -> Result<Self, BindFailure> {
        let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
            ScalarF32Opcode::Addss, ScalarF32Opcode::Subss, ScalarF32Opcode::Mulss,
        ]).map_err(|reason| BindFailure::Scalar { reason, architecture, entry })?;
        let scalar = SourceScalarF32::from_explicit_architecture(architecture, instructions)
            .map_err(|reason| BindFailure::Scalar { reason, architecture, entry })?;
        Ok(Self { scalar, state: entry, failure: None, last: None })
    }
    pub fn state(&self) -> ExplicitExceptionState { self.state }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }
    pub fn last(&self) -> Option<&Last> { self.last.as_ref() }
    pub fn into_owned_prefix(self) -> Retained {
        Retained { scalar: self.scalar, known_prefix_state: self.state,
            failure: self.failure, last: self.last }
    }

    pub fn accept_actual_context_return(&mut self, entry: ExplicitExceptionState,
        advanced: ExplicitExceptionState) -> Result<(), ()> {
        if self.failure.is_some() { return Err(()); }
        if entry != self.state {
            self.failure = Some(Failure::ContextState { expected: self.state,
                received_entry: entry, received_after: advanced });
            return Err(());
        }
        self.state = advanced; Ok(())
    }

    fn record(&mut self, pc: u32, operation: Operation) -> Result<(), ()> {
        if self.failure.is_some() { return Err(()); }
        self.last = Some(Last { pc, operation, entry: self.state, result: None }); Ok(())
    }
    fn scalar_state(&mut self, pc: u32, reported: u32) -> Result<(), ()> {
        let before = self.state;

        if before.mxcsr() & !0x3f != reported & !0x3f || before.mxcsr() & 0x3f & !reported != 0 {
            self.failure = Some(Failure::State { pc, reported_mxcsr: reported, errno: before.errno() });
            return Err(());
        }
        match ExplicitExceptionState::from_explicit_parts(reported, before.errno()) {
            Ok(state) => { self.state = state; Ok(()) }
            Err(_) => {
                self.failure = Some(Failure::State { pc, reported_mxcsr: reported, errno: before.errno() });
                Err(())
            }
        }
    }
    pub fn binary(&mut self, pc: u32, opcode: range::Binary, left: u32, right: u32) -> Result<u32, ()> {
        self.record(pc, Operation::Binary { opcode, left, right })?;
        let bits = match opcode {
            range::Binary::Add | range::Binary::Subtract | range::Binary::Multiply => {
                let operation = match opcode {
                    range::Binary::Add => ScalarF32Operation::Addss { left, right },
                    range::Binary::Subtract => ScalarF32Operation::Subss { left, right },
                    _ => ScalarF32Operation::Mulss { left, right },
                };
                let Some(state) = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr()) else {
                    self.failure = Some(Failure::State { pc, reported_mxcsr: self.state.mxcsr(), errno: self.state.errno() });
                    return Err(());
                };
                match self.scalar.execute(operation, state) {
                    Ok(ScalarF32Outcome::Completed { bits, state }) => {
                        if let Some(last) = &mut self.last { last.result = Some(Value::Bits(bits)); }
                        self.scalar_state(pc, state.word())?; bits
                    }
                    Ok(ScalarF32Outcome::Trapped { state, unmasked }) => {
                        let state_admitted = self.scalar_state(pc, state.word()).is_ok();
                        self.failure = Some(Failure::Trap { pc, reported_mxcsr: state.word(),
                            unmasked: unmasked.bits(), state_admitted });
                        return Err(());
                    }
                    Err(reason) => { self.failure = Some(Failure::Scalar { pc, reason }); return Err(()); }
                }
            }
            range::Binary::Divide => {
                match literal::raw_divss_with_explicit_state(left, right, self.state, self.scalar.architecture()) {
                    Ok(value) => { self.state = value.exception_state; value.bits }
                    Err(error) => {
                        self.state = match error {
                            literal::LogArithmeticError::ArithmeticRequired { advanced, .. }
                            | literal::LogArithmeticError::ScalarServiceRequired { advanced, .. }
                            | literal::LogArithmeticError::UnexpectedScalarState { advanced, .. }
                            | literal::LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => advanced,
                            literal::LogArithmeticError::Dispatch(_) => self.state,
                        };
                        self.failure = Some(Failure::Division { pc, error }); return Err(());
                    }
                }
            }
            range::Binary::Minimum | range::Binary::Maximum => {
                let operation = if opcode == range::Binary::Minimum {
                    literal::SourceF32MinMaxOpcode::Minss
                } else { literal::SourceF32MinMaxOpcode::Maxss };
                match literal::minmax_f32_with_explicit_state(operation, left, right, self.state, self.scalar.architecture()) {
                    Ok(value) => { self.state = value.exception_state; value.bits }
                    Err(error) => {
                        self.state = error.known_advanced_state(self.state);
                        self.failure = Some(Failure::Selection { pc, error }); return Err(());
                    }
                }
            }
        };
        if let Some(last) = &mut self.last { last.result = Some(Value::Bits(bits)); }
        Ok(bits)
    }

    pub fn compare(&mut self, pc: u32, opcode: range::CompareOpcode, left: u32, right: u32)
        -> Result<range::Comparison, ()> {
        self.record(pc, Operation::Compare { opcode, left, right })?;
        let (value, mask_bits) = match opcode {
            range::CompareOpcode::Ucomiss => {
                let value = literal::compare_f32_with_explicit_state(
                    literal::SourceF32CompareOpcode::Ucomiss, left, right, self.state);
                self.state = value.exception_state;
                (range::Comparison { unordered: value.parity, less: value.carry && !value.parity,
                    equal: value.zero && !value.parity }, None)
            }
            range::CompareOpcode::CmpLessScalar | range::CompareOpcode::CmpLessEqualScalar => {
                let predicate = if opcode == range::CompareOpcode::CmpLessScalar {
                    literal::SourceCmpssPredicate::LessThanSignaling
                } else { literal::SourceCmpssPredicate::LessEqualSignaling };
                match literal::cmpss_with_explicit_state(predicate, left, right, self.state, self.scalar.architecture()) {
                    Ok(value) => {
                        self.state = value.exception_state;
                        (range::Comparison { unordered: value.unordered, less: value.less,
                            equal: value.equal }, Some(value.mask_bits))
                    }
                    Err(error) => {
                        self.state = error.known_advanced_state(self.state);
                        self.failure = Some(Failure::Selection { pc, error }); return Err(());
                    }
                }
            }
        };
        if let Some(last) = &mut self.last { last.result = Some(Value::Comparison { value, mask_bits }); }
        Ok(value)
    }

    pub fn floor_value(&mut self, pc: u32, operand: u32) -> Result<u32, ()> {
        self.record(pc, Operation::FloorValue { operand })?;
        let entry = self.state;
        let mut floor = match literal::SourceFloor580388::bind(self.scalar.architecture(), entry) {
            Ok(floor) => floor,
            Err(error) => {
                self.state = error.known_advanced_state(entry);
                self.failure = Some(Failure::Floor { pc, error });
                return Err(());
            }
        };
        match floor.evaluate_bits(operand) {
            Ok(bits) => {

                if let Some(last) = &mut self.last { last.result = Some(Value::Bits(bits)); }
                self.scalar_state(pc, floor.exception_state().mxcsr())?;
                Ok(bits)
            }
            Err(error) => {
                self.state = error.known_advanced_state(entry);
                self.failure = Some(Failure::Floor { pc, error });
                Err(())
            }
        }
    }

    pub fn required_call<C: Calls>(&mut self, pc: u32, kind: range::Call, operand: u32, context: &mut C)
        -> Result<u32, CallFailure<C::Error>> {
        let expected = CallInput { pc, kind, operand, entry: self.state, architecture: self.scalar.architecture() };
        if self.record(pc, Operation::Call(expected)).is_err() { return Err(CallFailure::AlreadyStopped); }
        match context.call(expected) {
            Ok(returned) if returned.input == expected => {
                self.state = returned.advanced;
                if let Some(last) = &mut self.last { last.result = Some(Value::Bits(returned.bits)); }
                Ok(returned.bits)
            }
            Ok(returned) => {
                self.failure = Some(Failure::Call { expected, matched: false, known_advanced: None });
                Err(CallFailure::Mismatched(returned))
            }
            Err(stopped) => {
                let matched = stopped.input == expected;
                if matched && let Some(advanced) = stopped.known_advanced { self.state = advanced; }
                self.failure = Some(Failure::Call { expected, matched, known_advanced: stopped.known_advanced });
                Err(CallFailure::Service(stopped))
            }
        }
    }
}
