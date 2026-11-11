use super::{canvas_primitives::F32Operation, monitor_expansion as graph};
use crate::source_scalar::{self as scalar, ExplicitExceptionState, ScalarF32Architecture,
    SourceF64ValueMath, SourceF64ValueError, SourceUiSerialExp5b0580, SourceUiSerialExpError,
    SourceScalarTruncationError, LogArithmeticError};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Error, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome, ScalarF32Mxcsr};

pub(super) trait HelperUpper {
    type Error;
    fn original_upper(&mut self, source_rva: u32, argument: u32) -> Result<u64, Self::Error>;
}
#[derive(Clone, Copy)]
pub(super) struct Configuration {
    pub architecture: ScalarF32Architecture,
    pub ui_word_8a5ed4: u32,
}
pub(super) enum BindingCause { Shared(ScalarF32Error), Helper(SourceUiSerialExpError) }
pub(super) struct BindingFailure<U> {
    pub configuration: Configuration, pub state: ExplicitExceptionState, pub upper: U, pub cause: BindingCause,
}
pub(super) enum Cause<E> {
    Allocation(std::collections::TryReserveError), Shared(ScalarF32Error),
    ReportedState { mxcsr: u32 }, Trap { mxcsr: u32, unmasked: u8 },
    ReportedScalarOutcome(ScalarF32Outcome),
    Divide(LogArithmeticError), Wide(SourceF64ValueError),
    Helper(SourceUiSerialExpError), Upper(E), Truncation(SourceScalarTruncationError),
    UnsupportedF32(F32Operation),
}
pub(super) struct Call {
    pub request: graph::Request, pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState,
    pub upper: Option<u64>, pub result: Option<graph::Word>,
}
pub(super) struct Failure<E> {
    pub request: graph::Request, pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState,
    pub cause: Cause<E>,
}
pub(super) enum FailedLeaf { Wide(SourceF64ValueMath), Helper(SourceUiSerialExp5b0580) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stopped { Failed, AlreadyFailed }

pub(super) struct Provider<U: HelperUpper> {
    configuration: Configuration, state: ExplicitExceptionState, shared: SourceScalarF32, upper: U,
    calls: Vec<Call>, failure: Option<Failure<U::Error>>, failed_leaf: Option<FailedLeaf>,
}
impl<U: HelperUpper> Provider<U> {
    pub fn from_explicit_parts(configuration: Configuration, state: ExplicitExceptionState, upper: U)
        -> Result<Self, BindingFailure<U>>
    {

        if let Err(error) = SourceUiSerialExp5b0580::bind(configuration.ui_word_8a5ed4, configuration.architecture, state) {
            return Err(BindingFailure { configuration, state, upper, cause: BindingCause::Helper(error) });
        }
        let backend = ScalarF32InstructionSet::from_explicit_opcodes(&[
            ScalarF32Opcode::Cvtsi2ssSigned32, ScalarF32Opcode::Addss,
            ScalarF32Opcode::Subss, ScalarF32Opcode::Mulss,
        ]).and_then(|instructions| SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions));
        let shared = match backend { Ok(backend) => backend, Err(error) =>
            return Err(BindingFailure { configuration, state, upper, cause: BindingCause::Shared(error) }) };
        Ok(Self { configuration, state, shared, upper, calls: Vec::new(), failure: None, failed_leaf: None })
    }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn calls(&self) -> &[Call] { &self.calls }
    pub fn failure(&self) -> Option<&Failure<U::Error>> { self.failure.as_ref() }
    pub fn failed_leaf(&self) -> Option<&FailedLeaf> { self.failed_leaf.as_ref() }
    pub fn upper_supplier(&self) -> &U { &self.upper }

    pub fn into_parts(self) -> Result<(Configuration, ExplicitExceptionState, U, Vec<Call>), Self> {
        if self.failure.is_some() { Err(self) }
        else { Ok((self.configuration, self.state, self.upper, self.calls)) }
    }
    fn call(&mut self, request: graph::Request) -> Result<graph::Word, Stopped> {
        if self.failure.is_some() { return Err(Stopped::AlreadyFailed); }
        let entry = self.state;
        if let Err(error) = self.calls.try_reserve(1) {
            self.failure = Some(Failure { request, entry, advanced: entry, cause: Cause::Allocation(error) });
            return Err(Stopped::Failed);
        }
        let index = self.calls.len();
        self.calls.push(Call { request, entry, advanced: entry, upper: None, result: None });
        let result = self.execute(index, request);
        self.calls[index].advanced = self.state;
        match result {
            Ok(word) => { self.calls[index].result = Some(word); Ok(word) }
            Err(cause) => {
                self.failure = Some(Failure { request, entry, advanced: self.state, cause });
                Err(Stopped::Failed)
            }
        }
    }
    fn execute(&mut self, index: usize, request: graph::Request) -> Result<graph::Word, Cause<U::Error>> {
        use graph::{Operation, Word};
        match request.operation {
            Operation::F32(operation) => {
                let operation = match operation {
                    F32Operation::Cvtsi2ssSigned32(value) => ScalarF32Operation::Cvtsi2ssSigned32 { value },
                    F32Operation::Add { destination: left, source: right } => ScalarF32Operation::Addss { left, right },
                    F32Operation::Sub { destination: left, source: right } => ScalarF32Operation::Subss { left, right },
                    F32Operation::Mul { destination: left, source: right } => ScalarF32Operation::Mulss { left, right },
                    F32Operation::Div { destination, source } => {
                        return match scalar::raw_divss_with_explicit_state(destination, source, self.state, self.configuration.architecture) {
                            Ok(result) => { self.state = result.exception_state; Ok(Word::F32(result.bits)) }
                            Err(error) => {
                                self.state = match &error {
                                    LogArithmeticError::ArithmeticRequired { advanced, .. }
                                    | LogArithmeticError::ScalarServiceRequired { advanced, .. }
                                    | LogArithmeticError::UnexpectedScalarState { advanced, .. }
                                    | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => *advanced,
                                    LogArithmeticError::Dispatch(_) => self.state,
                                };
                                Err(Cause::Divide(error))
                            }
                        };
                    }
                    other => return Err(Cause::UnsupportedF32(other)),
                };
                let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                    .ok_or(Cause::ReportedState { mxcsr: self.state.mxcsr() })?;
                let outcome = self.shared.execute(operation, before).map_err(Cause::Shared)?;
                use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};
                match project_scalar_f32_outcome(self.state, outcome) {
                    SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(Word::F32(bits)) }
                    SourceScalarF32Projection::Trapped { reported_mxcsr: mxcsr, unmasked, advanced } => {
                        self.state = advanced; Err(Cause::Trap { mxcsr, unmasked })
                    }
                    SourceScalarF32Projection::ReportedStateRequired { reported, known_advanced, .. } => {
                        self.state = known_advanced; Err(Cause::ReportedScalarOutcome(reported))
                    }
                }
            }
            Operation::Cvtss2sd { .. } | Operation::Mulsd { .. } | Operation::Addsd { .. } => {
                let mut leaf = SourceF64ValueMath::bind(self.configuration.architecture, self.state).map_err(Cause::Wide)?;
                let result = match request.operation {
                    Operation::Cvtss2sd { source } => leaf.widen_f32_bits(source),
                    Operation::Mulsd { destination, source } => leaf.multiply_bits(destination, source),
                    Operation::Addsd { destination, source } => leaf.add_bits(destination, source),
                    _ => unreachable!("private wide dispatch"),
                };
                self.state = leaf.exception_state();
                match result { Ok(bits) => Ok(Word::F64(bits)), Err(error) => {
                    self.failed_leaf = Some(FailedLeaf::Wide(leaf)); Err(Cause::Wide(error))
                } }
            }
            Operation::Helper5b0580 { argument } => {
                let upper = self.upper.original_upper(request.source_rva, argument).map_err(Cause::Upper)?;
                self.calls[index].upper = Some(upper);
                let mut leaf = SourceUiSerialExp5b0580::bind(self.configuration.ui_word_8a5ed4,
                    self.configuration.architecture, self.state).map_err(Cause::Helper)?;
                let result = leaf.evaluate_bits(argument, upper);
                self.state = leaf.exception_state();
                match result { Ok(bits) => Ok(Word::F32(bits)), Err(error) => {
                    self.failed_leaf = Some(FailedLeaf::Helper(leaf)); Err(Cause::Helper(error))
                } }
            }
            Operation::Cvttss2siSigned32 { source } => {
                match scalar::truncate_f32_to_i32_with_explicit_state(source, self.state, self.configuration.architecture) {
                    Ok(result) => { self.state = result.exception_state; Ok(Word::Signed32(result.signed_value())) }
                    Err(error) => { self.state = error.known_advanced_state(self.state); Err(Cause::Truncation(error)) }
                }
            }
        }
    }
}
impl<U: HelperUpper> graph::Services for Provider<U> {
    type Error = Stopped;
    fn f32(&mut self, source_rva: u32, operation: F32Operation) -> Result<u32, Stopped> {
        match self.call(graph::Request { source_rva, operation: graph::Operation::F32(operation) })? {
            graph::Word::F32(bits) => Ok(bits), _ => unreachable!("private F32 dispatch"),
        }
    }
    fn cvtss2sd(&mut self, source_rva: u32, source: u32) -> Result<u64, Stopped> {
        match self.call(graph::Request { source_rva, operation: graph::Operation::Cvtss2sd { source } })? {
            graph::Word::F64(bits) => Ok(bits), _ => unreachable!("private F64 dispatch"),
        }
    }
    fn mulsd(&mut self, source_rva: u32, destination: u64, source: u64) -> Result<u64, Stopped> {
        match self.call(graph::Request { source_rva, operation: graph::Operation::Mulsd { destination, source } })? {
            graph::Word::F64(bits) => Ok(bits), _ => unreachable!("private F64 dispatch"),
        }
    }
    fn addsd(&mut self, source_rva: u32, destination: u64, source: u64) -> Result<u64, Stopped> {
        match self.call(graph::Request { source_rva, operation: graph::Operation::Addsd { destination, source } })? {
            graph::Word::F64(bits) => Ok(bits), _ => unreachable!("private F64 dispatch"),
        }
    }
    fn helper5b0580(&mut self, source_rva: u32, argument: u32) -> Result<u32, Stopped> {
        match self.call(graph::Request { source_rva, operation: graph::Operation::Helper5b0580 { argument } })? {
            graph::Word::F32(bits) => Ok(bits), _ => unreachable!("private helper dispatch"),
        }
    }
    fn cvttss2si_signed32(&mut self, source_rva: u32, source: u32) -> Result<i32, Stopped> {
        match self.call(graph::Request { source_rva, operation: graph::Operation::Cvttss2siSigned32 { source } })? {
            graph::Word::Signed32(value) => Ok(value), _ => unreachable!("private CVTT dispatch"),
        }
    }
}
