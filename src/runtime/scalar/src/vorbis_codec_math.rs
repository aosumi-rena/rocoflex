use super::finite_power::{ExplicitExceptionState, MathErrorServiceError, SourceMathErrorCallback};
use super::selected_log::{Error as LogError, LogArithmeticError, LogEvaluationError,
    SelectedProfilePositiveLog, SourceLogPlan};
use super::selected_phase::{PhaseError, PhaseEvaluationError, SourcePhaseNonzero};
use super::selected_phase_word_zero::{SourcePhaseWordZeroMdct, SourceMdctPhaseWordZeroError};
use super::selected_coefficient_trig::{ScalarTrigError, ScalarTrigEvaluationError, SourceCoefficientTrigNonzero};
use super::selected_ring_trig::{SourceFloor0CosineWordZero, SourceRingTrigError};
use super::selected_exp::{ExpError, SourceExpEvaluationError, SourceExpPlan, SourceScalarExpNonzero};
use super::selected_exp_word_zero::{SourceScalarExpWordZero, SourceWordZeroExpCallReceipt, SourceWordZeroExpError};
use super::square_root::{SourceSquareRoot, SquareRootError, SquareRootEvaluationError};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::vorbis_arctangent::SourceVorbisArctangent;
use super::vorbis_power::VorbisScalarError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VorbisCodecMathError {
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    LogBinding(LogError),
    Log(LogArithmeticError),
    Phase(PhaseError),
    WordZeroPhase(SourceMdctPhaseWordZeroError),
    Cosine(ScalarTrigError),
    WordZeroCosine(SourceRingTrigError),
    Exponential(ExpError),
    WordZeroExponential(SourceWordZeroExpError),
    SquareRoot(SquareRootError),
    Arctangent(VorbisScalarError),
    MathError(MathErrorServiceError),
}

impl VorbisCodecMathError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "explicit scalar architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "explicit scalar state is not RNE/all-masked/gradual",
            Self::LogBinding(_) => "5834c0 explicit log binding required",
            Self::Log(_) => "5834c0 source log continuation required",
            Self::Phase(PhaseError::NonzeroSourceProfileRequired) => "57ddb0/586960 explicit nonzero source selector required",
            Self::Phase(_) => "57ddb0/586960 source F64 phase continuation required",
            Self::WordZeroPhase(error) => error.reason(),
            Self::Cosine(ScalarTrigError::NonzeroSourceProfileRequired) => "57e330 explicit nonzero source selector required",
            Self::Cosine(_) => "57e330 source F32 cosine continuation required",
            Self::WordZeroCosine(error) => error.reason(),
            Self::Exponential(ExpError::NonzeroSourceProfileRequired) => "57f8e0 explicit nonzero source selector required",
            Self::Exponential(_) => "57f8e0 source exponential continuation required",
            Self::WordZeroExponential(error) => error.reason(),
            Self::SquareRoot(_) => "SQRTSS/587694 source square-root continuation required",
            Self::Arctangent(error) => error.reason(),
            Self::MathError(_) => "explicit source math-error callback continuation required",
        }
    }

    pub fn known_advanced_state(&self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::Log(LogArithmeticError::ArithmeticRequired { advanced, .. }
                | LogArithmeticError::ScalarServiceRequired { advanced, .. }
                | LogArithmeticError::UnexpectedScalarState { advanced, .. }
                | LogArithmeticError::UnexpectedScalarTrap { advanced, .. }) => *advanced,
            Self::Log(LogArithmeticError::Dispatch(LogError::CallbackRequiredFromEntry { entry })) => entry.exception_state,
            Self::Phase(error) => phase_advanced(error, entry),
            Self::WordZeroPhase(error) => error.known_advanced_state(entry),
            Self::WordZeroCosine(error) => error.known_advanced_state(entry),
            Self::WordZeroExponential(error) => error.known_advanced_state(entry),
            Self::Cosine(ScalarTrigError::NormalArithmeticRequired { advanced, .. }) => *advanced,
            Self::Cosine(ScalarTrigError::Reduction(error)) => phase_advanced(error, entry),
            Self::Cosine(ScalarTrigError::Conversion(required)) => required.advanced,
            Self::Exponential(ExpError::NormalArithmeticRequired { advanced, .. }) => *advanced,
            Self::Exponential(ExpError::ConversionRequired(required)) => required.advanced,
            Self::Exponential(ExpError::OrdinaryPathRequired { entry }
                | ExpError::CallbackRequiredFromEntry { entry }) => entry.exception_state,
            Self::Arctangent(error) => error.known_advanced_state(entry),
            Self::MathError(MathErrorServiceError::PathRequired { advanced }) => *advanced,
            Self::MathError(MathErrorServiceError::Callback(failure)) => failure.advanced,
            Self::MathError(MathErrorServiceError::Conversion(required)) => required.advanced,
            _ => entry,
        }
    }
}

fn phase_advanced(error: &PhaseError, entry: ExplicitExceptionState) -> ExplicitExceptionState {
    match error {
        PhaseError::NormalArithmeticRequired { advanced, .. } => *advanced,
        _ => entry,
    }
}

pub struct SourceVorbisCodecMath {
    explicit_source_word: u32,
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
    last_word_zero_exp_call: Option<SourceWordZeroExpCallReceipt>,
    first_word_zero_exp_failure: Option<SourceWordZeroExpCallReceipt>,
}

impl SourceVorbisCodecMath {
    pub fn bind(explicit_source_word: u32, architecture: ScalarF32Architecture,
        state: ExplicitExceptionState) -> Result<Self, VorbisCodecMathError>
    {
        if architecture != ScalarF32Architecture::IntelSdm {
            return Err(VorbisCodecMathError::ArchitectureNotClosed);
        }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(VorbisCodecMathError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { explicit_source_word, architecture, state,
            last_word_zero_exp_call: None, first_word_zero_exp_failure: None })
    }
    pub fn explicit_source_word(&self) -> u32 { self.explicit_source_word }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }

    pub fn last_word_zero_exp_call(&self) -> Option<&SourceWordZeroExpCallReceipt> {
        self.last_word_zero_exp_call.as_ref()
    }

    pub fn first_word_zero_exp_failure(&self) -> Option<&SourceWordZeroExpCallReceipt> {
        self.first_word_zero_exp_failure.as_ref()
    }
    pub fn name(&self) -> &'static str { "source-vorbis/explicit-scalar-leaves" }

    pub fn with_explicit_callback<'a, C: SourceMathErrorCallback>(
        &'a mut self, callback: &'a mut C,
    ) -> SourceVorbisCodecWithCallback<'a, C> {
        SourceVorbisCodecWithCallback { backend: self, callback }
    }

    fn finish<T>(&mut self, result: Result<(T, ExplicitExceptionState), VorbisCodecMathError>)
        -> Result<T, VorbisCodecMathError>
    {
        match result {
            Ok((value, advanced)) => { self.state = advanced; Ok(value) }
            Err(error) => { self.state = error.known_advanced_state(self.state); Err(error) }
        }
    }
    fn retain_word_zero_exp_call(&mut self, leaf: &SourceScalarExpWordZero) {
        let receipt = leaf.last_call().cloned();
        if self.first_word_zero_exp_failure.is_none() {
            if let Some(failed) = receipt.as_ref().filter(|call| call.error.is_some()) {
                self.first_word_zero_exp_failure = Some(failed.clone());
            }
        }
        self.last_word_zero_exp_call = receipt;
    }
    fn log(&self) -> Result<SelectedProfilePositiveLog, VorbisCodecMathError> {
        SelectedProfilePositiveLog::bind(self.explicit_source_word, self.state.mxcsr())
            .map_err(VorbisCodecMathError::LogBinding)
    }
    fn phase(&self) -> Result<SourcePhaseNonzero, VorbisCodecMathError> {
        SourcePhaseNonzero::bind(self.explicit_source_word, self.state.mxcsr())
            .map_err(VorbisCodecMathError::Phase)
    }
    fn cosine(&self) -> Result<SourceCoefficientTrigNonzero, VorbisCodecMathError> {
        SourceCoefficientTrigNonzero::bind(self.explicit_source_word, self.state.mxcsr())
            .map_err(VorbisCodecMathError::Cosine)
    }
    fn exponential(&self) -> Result<SourceScalarExpNonzero, VorbisCodecMathError> {
        SourceScalarExpNonzero::bind(self.explicit_source_word, self.state.mxcsr())
            .map_err(VorbisCodecMathError::Exponential)
    }
    fn square_root(&self) -> Result<SourceSquareRoot, VorbisCodecMathError> {
        SourceSquareRoot::bind(self.state.mxcsr()).map_err(VorbisCodecMathError::SquareRoot)
    }

    pub fn log_f32(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        let leaf = self.log()?;
        let bits = value.to_bits();
        let entry = leaf.source_entry_with_explicit_state(bits, self.state);
        let result = match entry.plan {
            SourceLogPlan::PositiveFinite { .. } => leaf.positive_with_explicit_state(bits, self.state, self.architecture)
                .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(VorbisCodecMathError::Log),
            SourceLogPlan::CompleteBits { bits } => Ok((f32::from_bits(bits), entry.exception_state)),
            SourceLogPlan::MaskedError { .. } => Err(VorbisCodecMathError::Log(
                LogArithmeticError::Dispatch(LogError::CallbackRequiredFromEntry { entry }))),
        };
        self.finish(result)
    }
    pub fn log_f32_with_callback(&mut self, value: f32, callback: &mut impl SourceMathErrorCallback)
        -> Result<f32, VorbisCodecMathError>
    {
        let leaf = self.log()?;
        let result = leaf.with_callback(value.to_bits(), self.state, self.architecture, callback)
            .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(|error| match error {
                LogEvaluationError::Arithmetic(error) => VorbisCodecMathError::Log(error),
                LogEvaluationError::MathError(error) => VorbisCodecMathError::MathError(error),
            });
        self.finish(result)
    }
    pub fn cosine_f64(&mut self, value: f64) -> Result<f64, VorbisCodecMathError> {
        if self.explicit_source_word == 0 { return self.mdct_phase_word_zero(value, false); }
        let leaf = self.phase()?;
        let result = leaf.cos_with_explicit_state(value.to_bits(), self.state)
            .map(|out| (f64::from_bits(out.bits), out.exception_state)).map_err(VorbisCodecMathError::Phase);
        self.finish(result)
    }
    pub fn sine_f64(&mut self, value: f64) -> Result<f64, VorbisCodecMathError> {
        if self.explicit_source_word == 0 { return self.mdct_phase_word_zero(value, true); }
        let leaf = self.phase()?;
        let result = leaf.sin_with_explicit_state(value.to_bits(), self.state)
            .map(|out| (f64::from_bits(out.bits), out.exception_state)).map_err(VorbisCodecMathError::Phase);
        self.finish(result)
    }
    pub fn cosine_f64_with_callback(&mut self, value: f64, callback: &mut impl SourceMathErrorCallback)
        -> Result<f64, VorbisCodecMathError>
    {
        if self.explicit_source_word == 0 { return self.mdct_phase_word_zero(value, false); }
        let leaf = self.phase()?;
        let result = leaf.cos_with_callback(value.to_bits(), self.state, callback)
            .map(|out| (f64::from_bits(out.bits), out.exception_state)).map_err(phase_error);
        self.finish(result)
    }
    pub fn sine_f64_with_callback(&mut self, value: f64, callback: &mut impl SourceMathErrorCallback)
        -> Result<f64, VorbisCodecMathError>
    {
        if self.explicit_source_word == 0 { return self.mdct_phase_word_zero(value, true); }
        let leaf = self.phase()?;
        let result = leaf.sin_with_callback(value.to_bits(), self.state, callback)
            .map(|out| (f64::from_bits(out.bits), out.exception_state)).map_err(phase_error);
        self.finish(result)
    }
    fn mdct_phase_word_zero(&mut self, value: f64, sine: bool) -> Result<f64, VorbisCodecMathError> {
        let mut leaf = SourcePhaseWordZeroMdct::bind(self.explicit_source_word, self.architecture, self.state)
            .map_err(VorbisCodecMathError::WordZeroPhase)?;

        let value = if sine { leaf.sine_f64(value) } else { leaf.cosine_f64(value) };
        let result = value.map(|out| (out, leaf.exception_state())).map_err(VorbisCodecMathError::WordZeroPhase);
        self.finish(result)
    }
    pub fn cosine_f32(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        if self.explicit_source_word == 0 { return self.floor0_cosine_word_zero(value); }
        let leaf = self.cosine()?;
        let result = leaf.cosf_with_explicit_state(value.to_bits(), self.state)
            .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(VorbisCodecMathError::Cosine);
        self.finish(result)
    }
    pub fn cosine_f32_with_callback(&mut self, value: f32, callback: &mut impl SourceMathErrorCallback)
        -> Result<f32, VorbisCodecMathError>
    {
        if self.explicit_source_word == 0 { return self.floor0_cosine_word_zero(value); }
        let leaf = self.cosine()?;
        let result = leaf.cosf_with_callback(value.to_bits(), self.state, callback)
            .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(|error| match error {
                ScalarTrigEvaluationError::Arithmetic(error) => VorbisCodecMathError::Cosine(error),
                ScalarTrigEvaluationError::MathError(error) => VorbisCodecMathError::MathError(error),
            });
        self.finish(result)
    }
    fn floor0_cosine_word_zero(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        let mut leaf = SourceFloor0CosineWordZero::bind(self.explicit_source_word, self.architecture, self.state)
            .map_err(VorbisCodecMathError::WordZeroCosine)?;
        let result = leaf.cosine_f32(value).map(|out| (out, leaf.exception_state()))
            .map_err(VorbisCodecMathError::WordZeroCosine);
        self.finish(result)
    }

    pub fn sqrt_ss(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        let leaf = self.square_root()?;
        let result = leaf.raw_sqrtss_with_explicit_state(value.to_bits(), self.state)
            .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(VorbisCodecMathError::SquareRoot);
        self.finish(result)
    }

    pub fn sqrt_wrapper(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        let leaf = self.square_root()?;
        let result = leaf.direct_with_explicit_state(value.to_bits(), self.state)
            .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(VorbisCodecMathError::SquareRoot);
        self.finish(result)
    }
    pub fn sqrt_wrapper_with_callback(&mut self, value: f32, callback: &mut impl SourceMathErrorCallback)
        -> Result<f32, VorbisCodecMathError>
    {
        let leaf = self.square_root()?;
        let result = leaf.evaluate_with_callback(value.to_bits(), self.state, callback)
            .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(|error| match error {
                SquareRootEvaluationError::Arithmetic(error) => VorbisCodecMathError::SquareRoot(error),
                SquareRootEvaluationError::MathError(error) => VorbisCodecMathError::MathError(error),
            });
        self.finish(result)
    }

    pub fn exp_f32(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        if self.explicit_source_word == 0 {
            let mut leaf = SourceScalarExpWordZero::bind(self.explicit_source_word, self.architecture, self.state)
                .map_err(VorbisCodecMathError::WordZeroExponential)?;
            let result = leaf.evaluate(value, 0).map(|out| (out, leaf.exception_state()))
                .map_err(VorbisCodecMathError::WordZeroExponential);
            self.retain_word_zero_exp_call(&leaf);
            return self.finish(result);
        }
        let leaf = self.exponential()?;
        let bits = value.to_bits();
        let entry = leaf.source_entry_with_explicit_state(bits, self.state);
        let result = match entry.plan {
            SourceExpPlan::CompleteBits { bits } => Ok((f32::from_bits(bits), entry.exception_state)),
            SourceExpPlan::Ordinary { .. } => leaf.ordinary_with_explicit_upper_lane(bits, 0, self.state)
                .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(VorbisCodecMathError::Exponential),
            SourceExpPlan::MaskedRangeError { .. } => Err(VorbisCodecMathError::Exponential(
                ExpError::CallbackRequiredFromEntry { entry })),
        };
        self.finish(result)
    }
    pub fn exp_f32_with_callback(&mut self, value: f32, callback: &mut impl SourceMathErrorCallback)
        -> Result<f32, VorbisCodecMathError>
    {
        if self.explicit_source_word == 0 {
            let mut leaf = SourceScalarExpWordZero::bind(self.explicit_source_word, self.architecture, self.state)
                .map_err(VorbisCodecMathError::WordZeroExponential)?;
            let result = leaf.with_callback(value, 0, callback).map(|out| (out, leaf.exception_state()))
                .map_err(VorbisCodecMathError::WordZeroExponential);
            self.retain_word_zero_exp_call(&leaf);
            return self.finish(result);
        }
        let leaf = self.exponential()?;
        let result = leaf.with_explicit_upper_lane_and_callback(value.to_bits(), 0, self.state, callback)
            .map(|out| (f32::from_bits(out.bits), out.exception_state)).map_err(|error| match error {
                SourceExpEvaluationError::Arithmetic(error) => VorbisCodecMathError::Exponential(error),
                SourceExpEvaluationError::MathError(error) => VorbisCodecMathError::MathError(error),
            });
        self.finish(result)
    }

    pub fn arctangent_f32(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        let mut leaf = SourceVorbisArctangent::bind(self.architecture, self.state)
            .map_err(VorbisCodecMathError::Arctangent)?;
        let result = leaf.arctangent_f32(value)
            .map(|out| (out, leaf.exception_state())).map_err(VorbisCodecMathError::Arctangent);
        self.finish(result)
    }
}

fn phase_error(error: PhaseEvaluationError) -> VorbisCodecMathError {
    match error {
        PhaseEvaluationError::Arithmetic(error) => VorbisCodecMathError::Phase(error),
        PhaseEvaluationError::MathError(error) => VorbisCodecMathError::MathError(error),
    }
}

pub struct SourceVorbisCodecWithCallback<'a, C: SourceMathErrorCallback> {
    backend: &'a mut SourceVorbisCodecMath,
    callback: &'a mut C,
}

impl<C: SourceMathErrorCallback> SourceVorbisCodecWithCallback<'_, C> {
    pub fn backend(&self) -> &SourceVorbisCodecMath { self.backend }
    pub fn callback(&self) -> &C { self.callback }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.backend.exception_state() }
    pub fn name(&self) -> &'static str { self.backend.name() }
    pub fn log_f32(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        self.backend.log_f32_with_callback(value, self.callback)
    }
    pub fn cosine_f64(&mut self, value: f64) -> Result<f64, VorbisCodecMathError> {
        self.backend.cosine_f64_with_callback(value, self.callback)
    }
    pub fn sine_f64(&mut self, value: f64) -> Result<f64, VorbisCodecMathError> {
        self.backend.sine_f64_with_callback(value, self.callback)
    }
    pub fn cosine_f32(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        self.backend.cosine_f32_with_callback(value, self.callback)
    }
    pub fn sqrt_ss(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        self.backend.sqrt_ss(value)
    }
    pub fn sqrt_wrapper(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        self.backend.sqrt_wrapper_with_callback(value, self.callback)
    }
    pub fn exp_f32(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        self.backend.exp_f32_with_callback(value, self.callback)
    }
    pub fn arctangent_f32(&mut self, value: f32) -> Result<f32, VorbisCodecMathError> {
        self.backend.arctangent_f32(value)
    }
}
