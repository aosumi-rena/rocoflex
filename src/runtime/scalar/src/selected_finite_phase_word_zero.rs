use super::finite_power::ExplicitExceptionState;
use super::phase_status::PhaseStatusAccumulator;
use super::selected_phase::PhaseWithStatus;
use super::selected_phase_word_zero::{SourcePhaseWordZeroShort, SourcePhaseWordZeroModerate,
    SourcePhaseWordZeroLarge, SourceMdctPhaseWordZeroError, SourceModeratePhaseWordZeroError,
    SourceLargePhaseWordZeroError};
use super::shared_scalar_f32::ScalarF32Architecture;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFinitePhaseWordZeroError {
    NonFiniteInput { bits: u64 },
    Short(SourceMdctPhaseWordZeroError),
    Moderate(SourceModeratePhaseWordZeroError),
    Large(SourceLargePhaseWordZeroError),
}
impl SourceFinitePhaseWordZeroError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NonFiniteInput { .. } => "CPU0 finite F64 phase requires its separately owned nonfinite helper continuation",
            Self::Short(error) => error.reason(),
            Self::Moderate(error) => error.reason(),
            Self::Large(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::NonFiniteInput { .. } => entry,
            Self::Short(error) => error.known_advanced_state(entry),
            Self::Moderate(error) => error.known_advanced_state(entry),
            Self::Large(error) => error.known_advanced_state(entry),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFinitePhaseWordZeroStatusFailure {
    pub error: SourceFinitePhaseWordZeroError,
    pub exception_state: ExplicitExceptionState,
    pub raised_status: u32,
}

pub struct SourceFinitePhaseWordZero { short: SourcePhaseWordZeroShort }
impl SourceFinitePhaseWordZero {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceFinitePhaseWordZeroError> {
        Ok(Self { short: SourcePhaseWordZeroShort::bind(word, architecture, state)
            .map_err(SourceFinitePhaseWordZeroError::Short)? })
    }
    pub fn name(&self) -> &'static str { "source57ddb0+586960/word0/finite/checked-separate" }
    pub fn explicit_source_word(&self) -> u32 { self.short.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.short.architecture() }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.short.exception_state() }
    pub fn sine_f64(&mut self, value: f64) -> Result<f64, SourceFinitePhaseWordZeroError> {
        self.sine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn cosine_f64(&mut self, value: f64) -> Result<f64, SourceFinitePhaseWordZeroError> {
        self.cosine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn sine_bits(&mut self, bits: u64) -> Result<u64, SourceFinitePhaseWordZeroError> { self.evaluate(bits, true) }
    pub fn cosine_bits(&mut self, bits: u64) -> Result<u64, SourceFinitePhaseWordZeroError> { self.evaluate(bits, false) }
    fn evaluate(&mut self, bits: u64, sine: bool) -> Result<u64, SourceFinitePhaseWordZeroError> {
        self.evaluate_with_status(bits, sine).map(|result| result.bits).map_err(|failure| failure.error)
    }

    pub fn sine_bits_with_status(&mut self, bits: u64)
        -> Result<PhaseWithStatus, SourceFinitePhaseWordZeroStatusFailure> {
        self.evaluate_with_status(bits, true)
    }
    pub fn cosine_bits_with_status(&mut self, bits: u64)
        -> Result<PhaseWithStatus, SourceFinitePhaseWordZeroStatusFailure> {
        self.evaluate_with_status(bits, false)
    }
    fn evaluate_with_status(&mut self, bits: u64, sine: bool)
        -> Result<PhaseWithStatus, SourceFinitePhaseWordZeroStatusFailure> {
        let mut a = PhaseStatusAccumulator::new(self.exception_state());
        let result = self.evaluate_observed(bits, sine, &mut a);
        self.short.install_source_continuation_state(a.state);
        match result {
            Ok(bits) => Ok(PhaseWithStatus { bits, exception_state: a.state, raised_status: a.raised }),
            Err(error) => Err(SourceFinitePhaseWordZeroStatusFailure {
                error, exception_state: a.state, raised_status: a.raised,
            }),
        }
    }
    fn evaluate_observed(&self, bits: u64, sine: bool, a: &mut PhaseStatusAccumulator)
        -> Result<u64, SourceFinitePhaseWordZeroError> {
        let magnitude = bits & 0x7fff_ffff_ffff_ffff;
        if magnitude >= 0x7ff0_0000_0000_0000 {
            return Err(SourceFinitePhaseWordZeroError::NonFiniteInput { bits });
        }
        if magnitude < 0x4020_0000_0000_0000 {
            return self.short.evaluate_observed(bits, sine, a).map_err(SourceFinitePhaseWordZeroError::Short);
        }
        let entry = a.state;
        let word = self.explicit_source_word();
        let architecture = self.architecture();
        if magnitude < 0x411e_8480_0000_0000 {
            let branch = SourcePhaseWordZeroModerate::bind(word, architecture, entry)
                .map_err(SourceFinitePhaseWordZeroError::Moderate)?;
            branch.evaluate_observed(bits, sine, a).map_err(SourceFinitePhaseWordZeroError::Moderate)
        } else {
            let branch = SourcePhaseWordZeroLarge::bind(word, architecture, entry)
                .map_err(SourceFinitePhaseWordZeroError::Large)?;
            branch.evaluate_observed(bits, sine, a).map_err(SourceFinitePhaseWordZeroError::Large)
        }
    }
}
