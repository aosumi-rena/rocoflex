use crate::source_scalar::{SourceF64ValueMath, SourceF64ValueError,
    SourceF64ValueCompletion, SourceF64ValueCandidateReceipt, ExplicitExceptionState};
use super::{TwiddleMath, TwiddleRequest, TwiddleOperation, TwiddleScale, TwiddleFftError};
pub use crate::ltv_block::LtvFftDirection as SourceFftDirection;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformAdmissionError { PriorFailure, Input, Environment(crate::matrix_f32::Error) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwiddleStage { Widen, Phase, Half, Narrow }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFftMathFailure {
    pub direction: SourceFftDirection, pub ordinal: usize, pub request: TwiddleRequest,
    pub source_word: u32, pub stage: TwiddleStage,
    pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState,
    pub cause: SourceF64ValueError,
    pub completed: [Option<SourceF64ValueCompletion>; 4],
    pub actual_candidate: Option<SourceF64ValueCandidateReceipt>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFftFailure {
    Admission { direction: SourceFftDirection, cause: TransformAdmissionError },
    Math(SourceFftMathFailure),
    Size { direction: SourceFftDirection, length: usize },
    Ordinal { direction: SourceFftDirection, ordinal: usize, request: TwiddleRequest },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceTwiddleCompletion {
    pub direction: SourceFftDirection, pub ordinal: usize, pub request: TwiddleRequest,
    pub source_word: u32, pub result_bits: u32,
    pub completed: [Option<SourceF64ValueCompletion>; 4],
}
struct SourceTwiddleMath {
    math: SourceF64ValueMath, source_word: u32, direction: SourceFftDirection,
    ordinal: usize, failure: Option<SourceFftFailure>, last: Option<SourceTwiddleCompletion>,
}
impl SourceTwiddleMath {
    fn operation(&mut self, request: TwiddleRequest, ordinal: usize, stage: TwiddleStage,
        input: u64, completed: &mut [Option<SourceF64ValueCompletion>; 4])
        -> Result<u64, SourceFftMathFailure>
    {
        let entry = self.math.exception_state();
        let result = match stage {
            TwiddleStage::Widen => self.math.widen_f32_bits(input as u32),
            TwiddleStage::Phase => match request.operation {
                TwiddleOperation::Cosine57ddb0 => self.math.cosine_57ddb0_bits(input, self.source_word),
                TwiddleOperation::Sine586960 => self.math.sine_586960_bits(input, self.source_word),
            },

            TwiddleStage::Half => self.math.multiply_bits(input, 0.5_f64.to_bits()),
            TwiddleStage::Narrow => self.math.narrow_f64_bits(input).map(u64::from),
        };
        match result {
            Ok(bits) => {

                let at = match stage { TwiddleStage::Widen => 0, TwiddleStage::Phase => 1,
                    TwiddleStage::Half => 2, TwiddleStage::Narrow => 3 };
                completed[at] = self.math.last_completion();
                Ok(bits)
            }
            Err(cause) => {
                let failed = SourceFftMathFailure { direction: self.direction, ordinal, request,
                    source_word: self.source_word, stage, entry, advanced: self.math.exception_state(),
                    cause, completed: *completed, actual_candidate: self.math.last_candidate() };
                self.failure = Some(SourceFftFailure::Math(failed));
                Err(failed)
            }
        }
    }
}
impl TwiddleMath for SourceTwiddleMath {
    type Error = SourceFftFailure;
    fn evaluate(&mut self, request: TwiddleRequest) -> Result<u32, Self::Error> {
        if let Some(failed) = self.failure { return Err(failed); }
        let ordinal = self.ordinal;
        let Some(next) = ordinal.checked_add(1) else {
            let failed = SourceFftFailure::Ordinal { direction: self.direction, ordinal, request };
            self.failure = Some(failed);
            return Err(failed);
        };
        self.ordinal = next;
        let mut completed = [None; 4];
        let angle = self.operation(request, ordinal, TwiddleStage::Widen,
            u64::from(request.angle_f32_bits), &mut completed).map_err(SourceFftFailure::Math)?;
        let value = self.operation(request, ordinal, TwiddleStage::Phase, angle, &mut completed).map_err(SourceFftFailure::Math)?;
        let value = if request.scale == TwiddleScale::Half {
            self.operation(request, ordinal, TwiddleStage::Half, value, &mut completed).map_err(SourceFftFailure::Math)?
        } else { value };
        let bits = self.operation(request, ordinal, TwiddleStage::Narrow, value, &mut completed).map_err(SourceFftFailure::Math)? as u32;
        self.last = Some(SourceTwiddleCompletion { direction: self.direction, ordinal, request,
            source_word: self.source_word, result_bits: bits, completed });
        Ok(bits)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFftMathBindingCause { AlreadyFailed }
pub struct SourceFftMathBindingFailure {
    math: SourceF64ValueMath, source_word: u32, cause: SourceFftMathBindingCause,
}
impl SourceFftMathBindingFailure {
    pub fn math(&self) -> &SourceF64ValueMath { &self.math }
    pub fn source_word(&self) -> u32 { self.source_word }
    pub fn cause(&self) -> SourceFftMathBindingCause { self.cause }
}

pub struct SourceFftMath {
    inner: SourceTwiddleMath,
    failure_message: Option<String>,
}
impl SourceFftMath {
    pub fn from_explicit_parts(math: SourceF64ValueMath, source_word: u32)
        -> Result<Self, SourceFftMathBindingFailure>
    {
        Self::bind_with_failure(math, source_word, |math, source_word, cause|
            SourceFftMathBindingFailure { math, source_word, cause })
    }

    pub(crate) fn bind_with_failure<R>(math: SourceF64ValueMath, source_word: u32,
        refusal: impl FnOnce(SourceF64ValueMath, u32, SourceFftMathBindingCause) -> R)
        -> Result<Self, R>
    {
        if math.failure().is_some() {
            return Err(refusal(math, source_word, SourceFftMathBindingCause::AlreadyFailed));
        }
        Ok(Self { inner: SourceTwiddleMath { math, source_word,
            direction: SourceFftDirection::ForwardPlus1, ordinal: 0, failure: None, last: None },
            failure_message: None })
    }
    pub fn math(&self) -> &SourceF64ValueMath { &self.inner.math }
    pub fn source_word(&self) -> u32 { self.inner.source_word }
    pub fn failure(&self) -> Option<&SourceFftFailure> { self.inner.failure.as_ref() }
    pub fn failure_message(&self) -> Option<&str> { self.failure_message.as_deref() }

    pub fn last_source_twiddle(&self) -> Option<&SourceTwiddleCompletion> { self.inner.last.as_ref() }

    pub fn transform_in_place(&mut self, direction: SourceFftDirection, values: &mut [f32])
        -> Result<(), SourceFftFailure>
    {
        if let Some(first) = self.inner.failure { return Err(first); }
        if ![512, 1024, 2048].contains(&values.len()) {
            let failed = SourceFftFailure::Size { direction, length: values.len() };
            self.inner.failure = Some(failed);
            self.failure_message = Some("unsupported source RDFT size".to_owned());
            return Err(failed);
        }
        self.transform_with_expected_width(values.len(), direction, values)
    }
    pub(crate) fn transform_with_expected_width(&mut self, transform: usize,
        direction: SourceFftDirection, values: &mut [f32]) -> Result<(), SourceFftFailure>
    {
        if let Some(first) = self.inner.failure { return Err(first); }
        if let Err(cause) = admit_transform(transform, &self.failure_message, values) {
            let failed = SourceFftFailure::Admission { direction, cause };
            if cause != TransformAdmissionError::PriorFailure {
                self.inner.failure = Some(failed);
                self.failure_message = Some(admission_message(transform, cause));
            }
            return Err(failed);
        }
        self.inner.direction = direction;

        self.inner.ordinal = 0;
        let result = match direction {
            SourceFftDirection::ForwardPlus1 => super::rdft_forward_with_twiddle(values, &mut self.inner),
            SourceFftDirection::InverseMinus1 => super::rdft_inverse_supported_with_twiddle(values, &mut self.inner),
        };
        match result {
            Ok(()) => Ok(()),
            Err(TwiddleFftError::Math(first)) => {

                self.failure_message = Some(match first {
                    SourceFftFailure::Math(failed) => failed.cause.reason().to_owned(),
                    SourceFftFailure::Ordinal { .. } => "source RDFT twiddle ordinal overflow".to_owned(),
                    SourceFftFailure::Admission { cause, .. } => admission_message(transform, cause),
                    SourceFftFailure::Size { .. } => "unsupported source RDFT size".to_owned(),
                });
                Err(first)
            }
            Err(TwiddleFftError::UnsupportedSize { length }) => {
                let failed = SourceFftFailure::Size { direction, length };
                self.inner.failure = Some(failed);
                self.failure_message = Some("unsupported source RDFT size".to_owned());
                Err(failed)
            }
        }
    }
}
pub(crate) fn admit_transform(transform: usize, failure: &Option<String>, values: &[f32])
    -> Result<(), TransformAdmissionError>
{
    if failure.is_some() { return Err(TransformAdmissionError::PriorFailure); }
    if values.len() != transform || values.iter().any(|v| !v.is_finite()) {
        return Err(TransformAdmissionError::Input);
    }
    crate::matrix_f32::check_environment().map_err(TransformAdmissionError::Environment)
}
pub(crate) fn admission_message(transform: usize, cause: TransformAdmissionError) -> String {
    match cause {
        TransformAdmissionError::Input => format!("filter RDFT requires {transform} finite input lanes"),
        TransformAdmissionError::Environment(error) => format!("filter RDFT environment: {error:?}"),
        TransformAdmissionError::PriorFailure => "filter RDFT prior failure".to_owned(),
    }
}
