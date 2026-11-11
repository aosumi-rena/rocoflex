use super::finite_power::ExplicitExceptionState;
use super::selected_phase::PhaseError;
use super::selected_ring_trig::{SourceFiniteCosineWordZero, SourceFiniteCosineWordZeroError,
    SourceRingTrigError, finite_sine_word_zero_graph};
use super::shared_scalar_f32::ScalarF32Architecture;

pub struct SourceFiniteTrigWordZero {
    cosine: SourceFiniteCosineWordZero,
    last_call: Option<SourceFiniteTrigWordZeroCallReceipt>,
    first_failure: Option<SourceFiniteTrigWordZeroCallReceipt>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFiniteTrigWordZeroOperation { Sine586ed0, Cosine57e330 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFiniteTrigWordZeroCallReceipt {
    pub operation: SourceFiniteTrigWordZeroOperation,
    pub input_bits: u32,
    pub explicit_source_word: u32,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
    pub result_bits: Option<u32>,
    pub error: Option<SourceFiniteTrigWordZeroError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFiniteTrigWordZeroError {
    NonFiniteInput { bits: u32 },
    Short(SourceRingTrigError),
    LargeSine(PhaseError),
    LargeSineKernel(SourceRingTrigError),
    Cosine(SourceFiniteCosineWordZeroError),
}
impl SourceFiniteTrigWordZeroError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NonFiniteInput { .. } => "CPU0 finite trig requires non-CRT finite F32 input",
            Self::Short(error) => error.reason(),
            Self::LargeSineKernel(error) => error.reason(),
            Self::LargeSine(PhaseError::ConsumedUndefinedBitScan) => "CPU0 sine head consumed undefined zero BSR result",
            Self::LargeSine(_) => "CPU0 sine head arithmetic/source continuation required",
            Self::Cosine(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::NonFiniteInput { .. } => entry,
            Self::Short(error) => error.known_advanced_state(entry),
            Self::LargeSineKernel(error) => error.known_advanced_state(entry),
            Self::LargeSine(PhaseError::NormalArithmeticRequired { advanced, .. }) => advanced,
            Self::LargeSine(_) => entry,
            Self::Cosine(error) => error.known_advanced_state(entry),
        }
    }
}
impl SourceFiniteTrigWordZero {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceFiniteTrigWordZeroError> {
        Ok(Self { cosine: SourceFiniteCosineWordZero::bind(word, architecture, state)
            .map_err(SourceFiniteTrigWordZeroError::Cosine)?, last_call: None, first_failure: None })
    }
    pub fn name(&self) -> &'static str { "source57e330+586ed0/word0/finite/separate" }
    pub fn explicit_source_word(&self) -> u32 { self.cosine.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.cosine.architecture() }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.cosine.exception_state() }
    pub fn last_call(&self) -> Option<&SourceFiniteTrigWordZeroCallReceipt> { self.last_call.as_ref() }

    pub fn first_failure(&self) -> Option<&SourceFiniteTrigWordZeroCallReceipt> { self.first_failure.as_ref() }
    pub fn sinf(&mut self, value: f32) -> Result<f32, SourceFiniteTrigWordZeroError> {
        self.sinf_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn cosf(&mut self, value: f32) -> Result<f32, SourceFiniteTrigWordZeroError> {
        self.cosf_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn cosf_bits(&mut self, bits: u32) -> Result<u32, SourceFiniteTrigWordZeroError> {
        let entry = self.exception_state();
        let result = self.cosine.cosine_bits(bits).map_err(SourceFiniteTrigWordZeroError::Cosine);
        self.retain_call(SourceFiniteTrigWordZeroOperation::Cosine57e330, bits, entry, result);
        result
    }
    pub fn sinf_bits(&mut self, bits: u32) -> Result<u32, SourceFiniteTrigWordZeroError> {
        let entry = self.exception_state();
        let result = match finite_sine_word_zero_graph(&self.cosine, bits) {
            Ok((bits, advanced)) => { self.cosine.install_source_sine_state(advanced); Ok(bits) }
            Err(error) => {
                self.cosine.install_source_sine_state(error.known_advanced_state(entry)); Err(error)
            }
        };
        self.retain_call(SourceFiniteTrigWordZeroOperation::Sine586ed0, bits, entry, result);
        result
    }
    fn retain_call(&mut self, operation: SourceFiniteTrigWordZeroOperation, input_bits: u32,
        entry: ExplicitExceptionState, result: Result<u32, SourceFiniteTrigWordZeroError>) {
        let receipt = SourceFiniteTrigWordZeroCallReceipt { operation, input_bits,
            explicit_source_word: self.explicit_source_word(), entry, advanced: self.exception_state(),
            result_bits: result.as_ref().ok().copied(), error: result.as_ref().err().copied() };
        self.last_call = Some(receipt);
        if result.is_err() && self.first_failure.is_none() { self.first_failure = Some(receipt); }
    }
}
