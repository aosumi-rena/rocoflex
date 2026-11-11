pub use crate::ltv_window_values::{LtvWindowValueError, SourceLtvWindowCosine};
use crate::ltv_render::SourceLtvBlockConfig;

impl<C: SourceLtvWindowCosine + ?Sized> SourceLtvWindowCosine for &mut C {
    type Error = C::Error;
    fn cosf_57e330(&mut self, angle: f32) -> Result<f32, Self::Error> {
        C::cosf_57e330(&mut **self, angle)
    }
}

impl SourceLtvWindowCosine for crate::source_scalar::SourceFiniteCosineWordZero {
    type Error = crate::source_scalar::SourceFiniteCosineWordZeroError;
    fn cosf_57e330(&mut self, angle: f32) -> Result<f32, Self::Error> {
        self.cosine_f32(angle)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Normalization, Samples }

#[derive(Clone, Copy, Debug)]
pub struct Call {
    pub phase: Phase,
    pub angle: f32,
    pub result: Option<f32>,
}

pub enum Error<E> {
    Environment(crate::matrix_f32::Error),
    Value(LtvWindowValueError<E>),
}

pub struct Failure<C: SourceLtvWindowCosine> {
    config: SourceLtvBlockConfig,
    cosine: C,
    normalizer: Option<f32>,
    samples: Vec<f32>,
    calls: Vec<Call>,
    phase: Phase,
    error: Error<C::Error>,
}

impl<C: SourceLtvWindowCosine> Failure<C> {
    pub fn config(&self) -> SourceLtvBlockConfig { self.config }
    pub fn cosine(&self) -> &C { &self.cosine }
    pub fn normalizer(&self) -> Option<f32> { self.normalizer }
    pub fn sample_prefix(&self) -> &[f32] { &self.samples }
    pub fn calls(&self) -> &[Call] { &self.calls }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn error(&self) -> &Error<C::Error> { &self.error }
}

pub struct Completed<C> {
    config: SourceLtvBlockConfig,
    cosine: C,
    normalizer: f32,
    samples: Vec<f32>,
    calls: Vec<Call>,
}

impl<C> Completed<C> {
    pub fn config(&self) -> SourceLtvBlockConfig { self.config }
    pub fn cosine(&self) -> &C { &self.cosine }
    pub fn normalizer(&self) -> f32 { self.normalizer }
    pub fn samples(&self) -> &[f32] { &self.samples }
    pub fn calls(&self) -> &[Call] { &self.calls }

    pub fn into_parts(self) -> (SourceLtvBlockConfig, Vec<f32>, C, Vec<Call>, f32) {
        (self.config, self.samples, self.cosine, self.calls, self.normalizer)
    }
}

struct Recorded<'a, C> {
    cosine: &'a mut C,
    calls: &'a mut Vec<Call>,
    phase: Phase,
}

impl<C: SourceLtvWindowCosine> SourceLtvWindowCosine for Recorded<'_, C> {
    type Error = C::Error;
    fn cosf_57e330(&mut self, angle: f32) -> Result<f32, Self::Error> {
        let index = self.calls.len();
        self.calls.push(Call { phase: self.phase, angle, result: None });
        let value = self.cosine.cosf_57e330(angle)?;
        self.calls[index].result = Some(value);
        Ok(value)
    }
}

pub fn build<C: SourceLtvWindowCosine>(config: SourceLtvBlockConfig, mut cosine: C)
    -> Result<Completed<C>, Failure<C>>
{
    let mut calls = Vec::new();
    let mut samples = Vec::new();
    if let Err(error) = crate::matrix_f32::check_environment() {
        return Err(Failure { config, cosine, normalizer: None, samples, calls,
            phase: Phase::Normalization, error: Error::Environment(error) });
    }
    let sum = match crate::ltv_window_values::normalization(config,
        &mut Recorded { cosine: &mut cosine, calls: &mut calls, phase: Phase::Normalization })
    {
        Ok(sum) => sum,
        Err(error) => return Err(Failure { config, cosine, normalizer: None, samples, calls,
            phase: Phase::Normalization, error: Error::Value(error) }),
    };
    if let Err(error) = crate::matrix_f32::check_environment() {
        return Err(Failure { config, cosine, normalizer: Some(sum), samples, calls,
            phase: Phase::Samples, error: Error::Environment(error) });
    }
    let result = crate::ltv_window_values::write_samples(config, sum,
        &mut Recorded { cosine: &mut cosine, calls: &mut calls, phase: Phase::Samples },
        &mut |_, value| samples.push(value));
    if let Err(error) = result {
        return Err(Failure { config, cosine, normalizer: Some(sum), samples, calls,
            phase: Phase::Samples, error: Error::Value(error) });
    }
    Ok(Completed { config, cosine, normalizer: sum, samples, calls })
}
