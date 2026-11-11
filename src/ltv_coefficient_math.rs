use crate::{ltv_coefficient_values::{complex, CoefficientError, SourceCoefficientMath},
    source_scalar::{self as scalar, ExplicitExceptionState, SourceExpDispatchError,
        SourceExpDispatchOutcome, SourceExpEntry, SourceFiniteTrigWordZero,
        SourceFiniteTrigWordZeroError, SourceCoefficientTrigNonzero, ScalarTrigError,
        SourceCoefficientScaledExp562834, SourceCoefficientScaledExpError,
        shared_scalar_f32::ScalarF32Architecture}};
#[path = "ltv_coefficient_arithmetic.rs"]
mod arithmetic;
pub use crate::ltv_coefficient_values::complex::Progress as ComplexProgress;

pub trait ScalarUpper {
    type Error;
    fn original_upper_f64(&mut self, input_bits: u32) -> Result<u64, Self::Error>;
}
impl<T: ScalarUpper + ?Sized> ScalarUpper for &mut T {
    type Error = T::Error;
    fn original_upper_f64(&mut self, input: u32) -> Result<u64, Self::Error> {
        T::original_upper_f64(&mut **self, input)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configuration {
    pub architecture: ScalarF32Architecture,
    pub exp_source_word: u32, pub trig_source_word: u32,
    pub overflow_word_8360d0: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingError { Architecture, Controls }
pub struct BindingFailure<U> {
    pub error: BindingError, pub config: Configuration, pub state: ExplicitExceptionState, pub upper: U,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Scalar { input: u32 }, Cosine { input: u32 }, Sine { input: u32 },
    Scaled { real: u32, multiplier: u32, exponent2: i16 },
    Addss { left: u32, right: u32 }, Subss { left: u32, right: u32 },
    Divss { left: u32, right: u32 }, Mulss { left: u32, right: u32 },
    ConvertI32 { value: i32 },
}
pub struct Call {
    pub operation: Operation, pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState,
    pub upper: Option<u64>, pub result: Option<u32>, pub source_status: Option<u32>,
}
pub enum Cause<E> {
    TraceAllocation(std::collections::TryReserveError), Upper(E),
    Scalar(SourceExpDispatchError), RangeRequired(SourceExpEntry),
    TrigZero(SourceFiniteTrigWordZeroError), TrigNonzero(ScalarTrigError),
    Scaled(SourceCoefficientScaledExpError),
    Shared(scalar::shared_scalar_f32::ScalarF32Error),
    ReportedState { mxcsr: u32 }, Trap { mxcsr: u32, unmasked: u8 },
    ReportedScalarOutcome(scalar::shared_scalar_f32::ScalarF32Outcome),
    Divide(scalar::LogArithmeticError),
}
pub struct Failure<E> {
    pub operation: Operation, pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState,
    pub cause: Cause<E>,
}

pub enum FailedLeaf {
    ScalarDispatch(scalar::SourceExpDispatchReceipt),
    TrigZero(SourceFiniteTrigWordZero), TrigNonzero(SourceCoefficientTrigNonzero),
    Scaled(SourceCoefficientScaledExp562834),
}
pub struct Provider<U: ScalarUpper> {
    config: Configuration, state: ExplicitExceptionState, upper: U,
    calls: Vec<Call>, failure: Option<Failure<U::Error>>, failed_leaf: Option<FailedLeaf>,
    complex_progress: Option<complex::Progress>,
}
impl<U: ScalarUpper> Provider<U> {
    pub fn from_explicit_parts(config: Configuration, state: ExplicitExceptionState, upper: U)
        -> Result<Self, BindingFailure<U>>
    {
        let error = if config.architecture != ScalarF32Architecture::IntelSdm { Some(BindingError::Architecture) }
            else if state.mxcsr() & 0xffff_ffc0 != 0x1f80 { Some(BindingError::Controls) }
            else { None };
        if let Some(error) = error { return Err(BindingFailure { error, config, state, upper }); }
        Ok(Self { config, state, upper, calls: Vec::new(), failure: None,
            failed_leaf: None, complex_progress: None })
    }
    pub fn configuration(&self) -> Configuration { self.config }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn upper_supplier(&self) -> &U { &self.upper }
    pub fn calls(&self) -> &[Call] { &self.calls }
    pub fn failure(&self) -> Option<&Failure<U::Error>> { self.failure.as_ref() }
    pub fn failed_leaf(&self) -> Option<&FailedLeaf> { self.failed_leaf.as_ref() }
    pub fn complex_progress(&self) -> Option<ComplexProgress> { self.complex_progress }

    pub fn scalar_57f8e0_bits(&mut self, input: u32) -> Result<u32, CoefficientError> { self.scalar_value(input) }

    pub fn into_parts(self) -> Result<(Configuration, ExplicitExceptionState, U, Vec<Call>, Option<ComplexProgress>), Self> {
        if self.failure.is_some() { return Err(self); }
        Ok((self.config, self.state, self.upper, self.calls, self.complex_progress))
    }

    fn begin(&mut self, operation: Operation) -> Result<usize, CoefficientError> {
        if self.failure.is_some() { return Err(CoefficientError::Math); }
        if let Err(error) = self.calls.try_reserve(1) {
            self.failure = Some(Failure { operation, entry: self.state, advanced: self.state,
                cause: Cause::TraceAllocation(error) });
            return Err(CoefficientError::Math);
        }
        let index = self.calls.len();
        self.calls.push(Call { operation, entry: self.state, advanced: self.state,
            upper: None, result: None, source_status: None });
        Ok(index)
    }
    fn finish(&mut self, index: usize, result: Result<u32, Cause<U::Error>>) -> Result<u32, CoefficientError> {
        self.calls[index].advanced = self.state;
        match result {
            Ok(bits) => { self.calls[index].result = Some(bits); Ok(bits) }
            Err(cause) => {
                self.failure = Some(Failure { operation: self.calls[index].operation,
                    entry: self.calls[index].entry, advanced: self.state, cause });
                Err(CoefficientError::Math)
            }
        }
    }
    fn scalar_value(&mut self, input: u32) -> Result<u32, CoefficientError> {
        let index = self.begin(Operation::Scalar { input })?;
        let result = self.scalar_once(index, input);
        self.finish(index, result)
    }
    fn scalar_once(&mut self, index: usize, input: u32) -> Result<u32, Cause<U::Error>> {
        let upper = self.upper.original_upper_f64(input).map_err(Cause::Upper)?;
        self.calls[index].upper = Some(upper);
        let entry = self.state;
        let receipt = scalar::selected_exp_with_explicit_upper_lane_receipt(self.config.exp_source_word,
            self.config.architecture, input, upper, entry);
        match receipt.outcome.clone() {
            Ok(SourceExpDispatchOutcome::Complete(result)) => {
                self.state = result.exception_state; Ok(result.bits)
            }
            Ok(SourceExpDispatchOutcome::RangeRequired(required)) => {
                self.state = required.exception_state;
                self.failed_leaf = Some(FailedLeaf::ScalarDispatch(receipt));
                Err(Cause::RangeRequired(required))
            }
            Err(error) => {
                self.state = error.known_advanced_state(entry);
                self.failed_leaf = Some(FailedLeaf::ScalarDispatch(receipt));
                Err(Cause::Scalar(error))
            }
        }
    }
    fn trig_value(&mut self, input: u32, sine: bool) -> Result<u32, CoefficientError> {
        let index = self.begin(if sine { Operation::Sine { input } } else { Operation::Cosine { input } })?;
        let result = self.trig_once(input, sine);
        self.finish(index, result)
    }
    fn trig_once(&mut self, input: u32, sine: bool) -> Result<u32, Cause<U::Error>> {
        let entry = self.state;
        if self.config.trig_source_word == 0 {
            let mut leaf = SourceFiniteTrigWordZero::bind(self.config.trig_source_word,
                self.config.architecture, entry).map_err(Cause::TrigZero)?;
            let result = if sine { leaf.sinf_bits(input) } else { leaf.cosf_bits(input) };
            self.state = leaf.exception_state();
            match result {
                Ok(bits) => Ok(bits),
                Err(error) => { self.failed_leaf = Some(FailedLeaf::TrigZero(leaf)); Err(Cause::TrigZero(error)) }
            }
        } else {
            let leaf = SourceCoefficientTrigNonzero::bind(self.config.trig_source_word,
                entry.mxcsr()).map_err(Cause::TrigNonzero)?;
            let result = if sine { leaf.sinf_with_explicit_state(input, entry) }
                else { leaf.cosf_with_explicit_state(input, entry) };
            match result {
                Ok(result) => { self.state = result.exception_state; Ok(result.bits) }
                Err(error) => {
                    self.state = error.known_advanced_state(entry);
                    self.failed_leaf = Some(FailedLeaf::TrigNonzero(leaf)); Err(Cause::TrigNonzero(error))
                }
            }
        }
    }
}
impl<U: ScalarUpper> complex::Services for Provider<U> {
    type Error = CoefficientError;
    fn cosf_57e330(&mut self, phase: u32) -> Result<u32, CoefficientError> { self.trig_value(phase, false) }
    fn sinf_586ed0(&mut self, phase: u32) -> Result<u32, CoefficientError> { self.trig_value(phase, true) }
    fn scaled_exp_562834(&mut self, real: u32, multiplier: u32, exponent2: i16)
        -> Result<complex::Scale, CoefficientError>
    {
        let index = self.begin(Operation::Scaled { real, multiplier, exponent2 })?;
        let result = match SourceCoefficientScaledExp562834::bind(self.config.architecture,
            self.state, self.config.overflow_word_8360d0)
        {
            Err(error) => { self.state = error.advanced; Err(Cause::Scaled(error)) }
            Ok(mut leaf) => {
                let result = leaf.evaluate_bits(real, multiplier, exponent2);
                self.state = leaf.exception_state();
                match result {
                    Ok(result) => { self.calls[index].source_status = Some(result.source_status); Ok(result.bits) }
                    Err(error) => { self.failed_leaf = Some(FailedLeaf::Scaled(leaf)); Err(Cause::Scaled(error)) }
                }
            }
        };
        let bits = self.finish(index, result)?;
        Ok(complex::Scale { bits, source_status: self.calls[index].source_status.expect("completed source scale status") })
    }
}
impl<U: ScalarUpper> SourceCoefficientMath for Provider<U> {
    fn scalar_57f8e0(&mut self, value: f32) -> Result<f32, CoefficientError> {
        self.scalar_57f8e0_bits(value.to_bits()).map(f32::from_bits)
    }
    fn complex_516c0(&mut self, real: f32, imaginary: f32) -> Result<[f32; 2], CoefficientError> {
        if self.failure.is_some() { return Err(CoefficientError::Math); }
        match complex::evaluate(self, real.to_bits(), imaginary.to_bits()) {
            Ok(result) => { self.complex_progress = Some(result.progress); Ok(result.bits.map(f32::from_bits)) }
            Err(failure) => { self.complex_progress = Some(failure.progress); Err(failure.cause) }
        }
    }
}
