pub use crate::ltv_coefficient_values::{CoefficientError, SourceCoefficientMath};
use crate::ltv_coefficient_values::transform_values_into;
use crate::ltv_render::{self, LtvArithmeticPolicy, SourceLtvBlockConfig, SourceLtvFft};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Transform,
    Extent,
    NonfiniteInput,
    Environment(crate::matrix_f32::Error),
    Coefficient(CoefficientError),
    NonfiniteOutput,
}

pub struct StartFailure<M, F> {
    pub error: Error,
    pub transform: usize,
    pub rows: Vec<Vec<f32>>,
    pub math: M,
    pub fft: F,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure { pub row: usize, pub error: Error }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poll { Pending { completed: usize, total: usize }, Complete, Failed(Failure) }

#[derive(Clone, Copy)]
enum State { Pending, Complete, Failed(Failure) }

pub struct Prepared<M, F> {
    transform: usize,
    rows: Vec<Vec<f32>>,
    math: M,
    fft: F,
    packed: Vec<Vec<f32>>,
    scratch: Vec<f32>,
    state: State,
}

pub struct Completed<M, F> {
    transform: usize,
    rows: Vec<Vec<f32>>,
    math: M,
    fft: F,
    packed: Vec<Vec<f32>>,
}

pub struct CompletedParts<M, F> {
    pub transform: usize, pub original_rows: Vec<Vec<f32>>,
    pub packed_rows: Vec<Vec<f32>>, pub math: M, pub fft: F,
}
impl<M, F> Completed<M, F> {

    pub fn into_parts(self) -> CompletedParts<M, F> {
        CompletedParts { transform: self.transform, original_rows: self.rows,
            packed_rows: self.packed, math: self.math, fft: self.fft }
    }
}

impl<M: SourceCoefficientMath, F: SourceLtvFft> Prepared<M, F> {

    pub fn from_explicit_rows(transform: usize, rows: Vec<Vec<f32>>, math: M, fft: F)
        -> Result<Self, StartFailure<M, F>>
    {
        let error = if !transform.is_power_of_two() || transform < 4
            || transform > i32::MAX as usize { Some(Error::Transform) }
        else if rows.len().checked_mul(transform)
            .and_then(|words| words.checked_mul(std::mem::size_of::<f32>()))
            .is_none_or(|bytes| bytes > isize::MAX as usize) { Some(Error::Extent) }
        else if rows.iter().flatten().any(|value| !value.is_finite()) {
            Some(Error::NonfiniteInput)
        } else { None };
        if let Some(error) = error { return Err(StartFailure { error, transform, rows, math, fft }); }
        Ok(Self { transform, rows, math, fft, packed: Vec::new(),
            scratch: vec![0.0; transform], state: State::Pending })
    }

    pub fn original_rows(&self) -> &[Vec<f32>] { &self.rows }
    pub fn packed_prefix(&self) -> &[Vec<f32>] { &self.packed }
    pub fn current_scratch(&self) -> &[f32] { &self.scratch }
    pub fn math(&self) -> &M { &self.math }
    pub fn fft(&self) -> &F { &self.fft }

    fn fail(&mut self, error: Error) -> Poll {
        let failure = Failure { row: self.packed.len(), error };
        self.state = State::Failed(failure);
        Poll::Failed(failure)
    }

    pub fn poll(&mut self, row_budget: usize) -> Poll {
        match self.state {
            State::Complete => return Poll::Complete,
            State::Failed(failure) => return Poll::Failed(failure),
            State::Pending => {},
        }
        for _ in 0..row_budget {
            if self.packed.len() == self.rows.len() { break; }
            if let Err(error) = crate::matrix_f32::check_environment() {
                return self.fail(Error::Environment(error));
            }
            if let Err(error) = transform_values_into(self.transform,
                &self.rows[self.packed.len()], &mut self.scratch, &mut self.fft, &mut self.math)
            { return self.fail(Error::Coefficient(error)); }
            if self.scratch.iter().any(|value| !value.is_finite()) {
                return self.fail(Error::NonfiniteOutput);
            }
            self.packed.push(std::mem::replace(&mut self.scratch, vec![0.0; self.transform]));
        }
        if self.packed.len() == self.rows.len() {
            self.state = State::Complete;
            Poll::Complete
        } else { Poll::Pending { completed: self.packed.len(), total: self.rows.len() } }
    }

    pub fn into_completed(self) -> Result<Completed<M, F>, Self> {
        if !matches!(self.state, State::Complete) { return Err(self); }
        Ok(Completed { transform: self.transform, rows: self.rows, math: self.math,
            fft: self.fft, packed: self.packed })
    }
}

pub struct Rendering<M, F> {
    renderer: ltv_render::Prepared<F>,
    math: M,
    coefficient_rows: Vec<Vec<f32>>,
}

pub struct JoinFailure<M, F> {
    pub filter: ltv_render::StartFailure<F>,
    pub math: M,
    pub coefficient_rows: Vec<Vec<f32>>,
}

pub struct Samples<M, F> {
    pub filter: ltv_render::Completed<F>,
    pub math: M,
    pub coefficient_rows: Vec<Vec<f32>>,
}

impl<M, F: SourceLtvFft> Completed<M, F> {
    pub fn transform_length(&self) -> usize { self.transform }
    pub fn original_rows(&self) -> &[Vec<f32>] { &self.rows }
    pub fn packed_rows(&self) -> &[Vec<f32>] { &self.packed }
    pub fn math(&self) -> &M { &self.math }
    pub fn fft(&self) -> &F { &self.fft }

    pub fn into_render(self, config: SourceLtvBlockConfig, arithmetic: LtvArithmeticPolicy,
        window: Vec<f32>, signal: Vec<Vec<f32>>)
        -> Result<Rendering<M, F>, JoinFailure<M, F>>
    {
        let input = ltv_render::Input { config, arithmetic, window, signal,
            coefficients: self.packed };

        if config.transform_length() != self.transform {
            return Err(JoinFailure {
                filter: ltv_render::StartFailure { error: ltv_render::Error::Dimensions,
                    input, fft: self.fft },
                math: self.math, coefficient_rows: self.rows,
            });
        }
        match ltv_render::Prepared::from_reset(input, self.fft) {
            Ok(renderer) => Ok(Rendering { renderer, math: self.math, coefficient_rows: self.rows }),
            Err(filter) => Err(JoinFailure { filter, math: self.math, coefficient_rows: self.rows }),
        }
    }
}

impl<M, F: SourceLtvFft> Rendering<M, F> {
    pub fn poll(&mut self, block_budget: usize) -> ltv_render::Poll { self.renderer.poll(block_budget) }
    pub fn renderer(&self) -> &ltv_render::Prepared<F> { &self.renderer }
    pub fn math(&self) -> &M { &self.math }
    pub fn coefficient_rows(&self) -> &[Vec<f32>] { &self.coefficient_rows }
    pub fn into_completed(self) -> Result<Samples<M, F>, Self> {
        match self.renderer.into_completed() {
            Ok(filter) => Ok(Samples { filter, math: self.math, coefficient_rows: self.coefficient_rows }),
            Err(renderer) => Err(Self { renderer, math: self.math, coefficient_rows: self.coefficient_rows }),
        }
    }
}
