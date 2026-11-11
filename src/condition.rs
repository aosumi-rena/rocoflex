pub use crate::condition_values::{PhaseOperation, PhaseServices, PhaseWork, RowError, Site, Stage, Work};
pub use crate::model2_operator_binding::ConditionModuleConfig as Config;
use crate::condition_values::{self, Mode};

pub trait Dct {
    type Error;
    fn input_width(&self) -> usize;
    fn output_width(&self) -> usize;

    fn overwrite_prefix(&mut self, input: &[f32], prefix: &mut [f32]) -> Result<(), Self::Error>;
}

pub enum ServiceError<D, P> { Dct(D), Phase(P) }
pub struct BindingFailure<D, P> {
    pub reason: &'static str, pub config: Config, pub frequencies: Vec<f32>, pub dct: D, pub phase: P,
}
pub struct Condition<D, P> {
    config: Config, frequencies: Vec<f32>, dct: D, phase: P, width: usize,
}
pub struct StartFailure<D, P> {
    pub reason: &'static str, pub condition: Condition<D, P>,
    pub input: Vec<Vec<f32>>, pub phase_words: Vec<u32>, pub status: u8,
}
pub struct Failure<D, P> { pub row: usize, pub cause: RowError<ServiceError<D, P>> }
pub enum Poll<'a, D, P> {
    Pending { completed: usize, total: usize }, Complete, Failed(&'a Failure<D, P>),
}
pub struct Run<D: Dct, P: PhaseServices> {
    condition: Condition<D, P>, input: Vec<Vec<f32>>, phase_words: Vec<u32>, status: u8,
    output: Vec<Vec<f32>>, current: Vec<f32>, work: Work, next: usize,
    failure: Option<Failure<D::Error, P::Error>>,
}
pub struct Completed<D, P> {
    pub(crate) condition: Condition<D, P>, pub(crate) input: Vec<Vec<f32>>, pub(crate) phase_words: Vec<u32>,
    pub(crate) status: u8, pub(crate) rows: Vec<Vec<f32>>, pub(crate) work: Work,
}
impl<D, P> Completed<D, P> {
    pub fn condition(&self) -> &Condition<D, P> { &self.condition }
    pub fn input(&self) -> &[Vec<f32>] { &self.input }
    pub fn phase_words(&self) -> &[u32] { &self.phase_words }
    pub fn status(&self) -> u8 { self.status }
    pub fn rows(&self) -> &[Vec<f32>] { &self.rows }
    pub fn work(&self) -> &Work { &self.work }
}

impl<D: Dct, P: PhaseServices> Condition<D, P> {
    pub fn from_explicit_services(config: Config, frequencies: Vec<f32>, dct: D, phase: P)
        -> Result<Self, BindingFailure<D, P>>
    {
        let width = frequencies.len().checked_mul(2).and_then(|n| n.checked_add(usize::from(config.first)));
        let trim = if config.mode == 0 { 8 } else { 1 };
        let reason = if config.bind_direct(crate::model2_operator_binding::CONDITION_TAG,
            crate::model2_operator_binding::CONDITION_CLASS).is_err() {
            Some("invalid conditioning parameter words")
        } else if width.is_none_or(|value| value > i32::MAX as usize)
            || config.output_width as usize <= trim
            || dct.input_width() != config.output_width as usize - trim
            || dct.output_width() < usize::from(config.first) {
            Some("conditioning DCT/feature dimensions differ")
        } else if frequencies.iter().any(|value| !value.is_finite()) {
            Some("conditioning frequencies exceed the finite value domain")
        } else { None };
        if let Some(reason) = reason { return Err(BindingFailure { reason, config, frequencies, dct, phase }); }

        Ok(Self { config, frequencies, dct, phase, width: width.expect("admitted width") })
    }
    pub fn config(&self) -> Config { self.config }
    pub fn output_width(&self) -> usize { self.width }
    pub fn frequencies(&self) -> &[f32] { &self.frequencies }
    pub fn dct(&self) -> &D { &self.dct }
    pub fn phase(&self) -> &P { &self.phase }

    pub fn begin(self, input: Vec<Vec<f32>>, phase_words: Vec<u32>, status: u8)
        -> Result<Run<D, P>, StartFailure<D, P>>
    {
        let reason = if input.len() != phase_words.len() {
            Some("conditioning phase-word count differs")
        } else if input.iter().any(|row| row.len() != self.config.output_width as usize
            || row.iter().any(|value| !value.is_finite())) {
            Some("conditioning requires finite complete feature rows")
        } else if input.len().checked_mul(self.width).and_then(|n| n.checked_mul(4))
            .is_none_or(|bytes| bytes > isize::MAX as usize) {
            Some("conditioning output exceeds owned storage")
        } else { None };
        if let Some(reason) = reason { return Err(StartFailure { reason, condition: self, input, phase_words, status }); }

        let current = vec![0.0; self.width];
        let work = Work { dct_prefix: vec![0.0; usize::from(self.config.first)],
            phase: PhaseWork::default(), stage: Stage::Ready };
        Ok(Run { condition: self, input, phase_words, status, output: Vec::new(), current,
            work, next: 0, failure: None })
    }
}

struct PhaseAdapter<'a, P, E> { phase: &'a mut P, marker: std::marker::PhantomData<E> }
impl<D, P: PhaseServices> PhaseServices for PhaseAdapter<'_, P, D> {
    type Error = ServiceError<D, P::Error>;
    fn phase_u32_to_f64(&mut self, value: u32) -> Result<f64, Self::Error> {
        self.phase.phase_u32_to_f64(value).map_err(ServiceError::Phase)
    }
    fn period_u16_to_f64(&mut self, value: u16) -> Result<f64, Self::Error> {
        self.phase.period_u16_to_f64(value).map_err(ServiceError::Phase)
    }
    fn frequency_f32_to_f64(&mut self, value: f32) -> Result<f64, Self::Error> {
        self.phase.frequency_f32_to_f64(value).map_err(ServiceError::Phase)
    }
    fn multiply_f64(&mut self, left: f64, right: f64) -> Result<f64, Self::Error> {
        self.phase.multiply_f64(left, right).map_err(ServiceError::Phase)
    }
    fn divide_f64(&mut self, left: f64, right: f64) -> Result<f64, Self::Error> {
        self.phase.divide_f64(left, right).map_err(ServiceError::Phase)
    }
    fn sin_586960(&mut self, angle: f64) -> Result<f64, Self::Error> {
        self.phase.sin_586960(angle).map_err(ServiceError::Phase)
    }
    fn cos_57ddb0(&mut self, angle: f64) -> Result<f64, Self::Error> {
        self.phase.cos_57ddb0(angle).map_err(ServiceError::Phase)
    }
    fn narrow_f64_to_f32(&mut self, value: f64) -> Result<f32, Self::Error> {
        self.phase.narrow_f64_to_f32(value).map_err(ServiceError::Phase)
    }
}

impl<D: Dct, P: PhaseServices> Run<D, P> {
    pub fn condition(&self) -> &Condition<D, P> { &self.condition }
    pub fn input(&self) -> &[Vec<f32>] { &self.input }
    pub fn phase_words(&self) -> &[u32] { &self.phase_words }
    pub fn output_prefix(&self) -> &[Vec<f32>] { &self.output }
    pub fn current_row(&self) -> &[f32] { &self.current }
    pub fn work(&self) -> &Work { &self.work }
    pub fn failure(&self) -> Option<&Failure<D::Error, P::Error>> { self.failure.as_ref() }
    pub fn poll(&mut self, row_budget: usize) -> Poll<'_, D::Error, P::Error> {
        if self.failure.is_none() {
            for _ in 0..row_budget {
                if self.next == self.input.len() { break; }
                let condition = &mut self.condition;
                let config = condition.config;
                let dct = &mut condition.dct;
                let current = &mut self.current;
                let result = condition_values::forward_row(&self.input[self.next],
                    if config.mode == 0 { Mode::Zero } else { Mode::One },
                    usize::from(config.first), config.second, self.phase_words[self.next],
                    &condition.frequencies, condition.width, &mut self.work,
                    &mut |input, prefix| dct.overwrite_prefix(input, prefix).map_err(ServiceError::Dct),
                    &mut |index, value| { current[index] = value; Ok(()) },
                    &mut PhaseAdapter { phase: &mut condition.phase, marker: std::marker::PhantomData });
                if let Err(cause) = result { self.failure = Some(Failure { row: self.next, cause }); break; }
                self.output.push(self.current.clone()); self.next += 1;
            }
        }
        if let Some(failure) = &self.failure { Poll::Failed(failure) }
        else if self.next == self.input.len() { Poll::Complete }
        else { Poll::Pending { completed: self.next, total: self.input.len() } }
    }
    pub fn into_completed(self) -> Result<Completed<D, P>, Self> {
        if self.failure.is_some() || self.next != self.input.len() { return Err(self); }
        Ok(Completed { condition: self.condition, input: self.input, phase_words: self.phase_words,
            status: self.status, rows: self.output, work: self.work })
    }
}
