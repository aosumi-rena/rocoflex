use crate::gru_parameters::QuantizedGruParameters;
pub use crate::modl3_values::{Activation, Buffer, CallbackSite, RowError, Stage, Work};

pub trait Activations {
    type Error;
    fn activate(&mut self, kind: Activation, row: &mut [f32]) -> Result<(), Self::Error>;
}

pub struct ExistingFrontendActivations;
pub(crate) fn activate_frontend(kind: Activation, row: &mut [f32]) {
    for value in row {
        *value = match kind {
            Activation::Sigmoid => crate::timbre::runtime_sigmoid(*value),
            Activation::Tanh => crate::reduction::runtime_tanh(*value),
        };
    }
}
impl Activations for ExistingFrontendActivations {
    type Error = std::convert::Infallible;
    fn activate(&mut self, kind: Activation, row: &mut [f32]) -> Result<(), Self::Error> {
        activate_frontend(kind, row); Ok(())
    }
}

pub struct GenericVectorActivations { arithmetic: crate::activation::ArithmeticMode }
impl GenericVectorActivations {
    pub fn from_explicit_arithmetic(arithmetic: crate::activation::ArithmeticMode) -> Self {
        Self { arithmetic }
    }
    pub fn arithmetic(&self) -> crate::activation::ArithmeticMode { self.arithmetic }
}
impl Activations for GenericVectorActivations {
    type Error = crate::matrix_f32::Error;
    fn activate(&mut self, kind: Activation, row: &mut [f32]) -> Result<(), Self::Error> {
        crate::matrix_f32::check_environment()?;
        for value in row {
            *value = match kind {
                Activation::Sigmoid => self.arithmetic.sigmoid(*value),
                Activation::Tanh => self.arithmetic.tanh(*value),
            };
        }
        Ok(())
    }
}

pub enum CallbackCause<E> { Matrix(String), Activation(E) }
pub enum Cause<E> {
    Environment(crate::matrix_f32::Error),
    Row(RowError<CallbackCause<E>>),
    NonfiniteHidden,
}
pub struct Failure<E> { pub row: usize, pub cause: Cause<E> }

pub struct QuantizedGru<A> {
    parameters: QuantizedGruParameters,
    hidden: Vec<f32>,
    activations: A,
    work: Work,
}
pub struct BindingFailure<A> {
    pub parameters: QuantizedGruParameters,
    pub hidden: Vec<f32>,
    pub activations: A,
}
pub struct StartFailure<A> {
    pub reason: &'static str,
    pub model: QuantizedGru<A>,
    pub input: Vec<Vec<f32>>,
    pub status: u8,
}

pub struct Run<A: Activations> {
    model: QuantizedGru<A>,
    input: Vec<Vec<f32>>,
    status: u8,
    next: usize,
    output: Vec<Vec<f32>>,
    failure: Option<Failure<A::Error>>,
}
pub enum Poll<'a, E> {
    Pending { completed: usize, total: usize }, Complete, Failed(&'a Failure<E>),
}
pub struct Completed<A> {
    pub model: QuantizedGru<A>,
    pub input: Vec<Vec<f32>>,
    pub rows: Vec<Vec<f32>>,
    pub status: u8,
}

pub(crate) fn new_work(width: usize) -> Work {

    Work { reset: vec![0.0; width], update: vec![0.0; width],
        candidate: vec![0.0; width], stage: Stage::Ready }
}

impl<A: Activations> QuantizedGru<A> {
    pub fn from_explicit_state(parameters: QuantizedGruParameters, hidden: Vec<f32>, activations: A)
        -> Result<Self, BindingFailure<A>>
    {
        if hidden.len() != parameters.hidden_width() || hidden.iter().any(|v| !v.is_finite()) {
            return Err(BindingFailure { parameters, hidden, activations });
        }
        let work = new_work(hidden.len());
        Ok(Self { parameters, hidden, activations, work })
    }
    pub fn parameters(&self) -> &QuantizedGruParameters { &self.parameters }
    pub fn hidden(&self) -> &[f32] { &self.hidden }
    pub fn activations(&self) -> &A { &self.activations }
    pub fn work(&self) -> &Work { &self.work }

    pub fn begin(self, input: Vec<Vec<f32>>, status: u8) -> Result<Run<A>, StartFailure<A>> {
        let reason = if input.iter().any(|row| row.len() != self.parameters.input_width()
            || row.iter().any(|v| !v.is_finite())) {
            Some("GRU requires finite complete input rows")
        } else if input.len().checked_mul(self.parameters.hidden_width())
            .and_then(|n| n.checked_mul(std::mem::size_of::<f32>()))
            .is_none_or(|bytes| bytes > isize::MAX as usize) {
            Some("GRU output extent exceeds owned storage")
        } else { None };
        if let Some(reason) = reason { return Err(StartFailure { reason, model: self, input, status }); }
        Ok(Run { model: self, input, status, next: 0, output: Vec::new(), failure: None })
    }
}

impl<A: Activations> Run<A> {
    pub fn model(&self) -> &QuantizedGru<A> { &self.model }
    pub fn input(&self) -> &[Vec<f32>] { &self.input }
    pub fn output_prefix(&self) -> &[Vec<f32>] { &self.output }
    pub fn failure(&self) -> Option<&Failure<A::Error>> { self.failure.as_ref() }

    pub fn poll(&mut self, row_budget: usize) -> Poll<'_, A::Error> {
        if self.failure.is_none() {
            for _ in 0..row_budget {
                if self.next == self.input.len() { break; }
                if let Err(error) = crate::matrix_f32::check_environment() {
                    self.failure = Some(Failure { row: self.next, cause: Cause::Environment(error) });
                    break;
                }
                let model = &mut self.model;
                let parameters = &model.parameters;
                let biases = std::array::from_fn(|index| parameters.layers[index].bias.as_slice());
                let activations = &mut model.activations;
                let result = crate::modl3_values::forward_row(&self.input[self.next],
                    &mut model.hidden, biases, &parameters.reset_bias, &parameters.update_bias,
                    &mut model.work,
                    &mut |index, input, destination| parameters.layers[index]
                        .forward_add_wide(input, destination).map_err(CallbackCause::Matrix),
                    &mut |kind, row| activations.activate(kind, row).map_err(CallbackCause::Activation));
                if let Err(error) = result {
                    self.failure = Some(Failure { row: self.next, cause: Cause::Row(error) }); break;
                }
                if model.hidden.iter().any(|v| !v.is_finite()) {
                    self.failure = Some(Failure { row: self.next, cause: Cause::NonfiniteHidden }); break;
                }
                self.output.push(model.hidden.clone());
                self.next += 1;
            }
        }
        if let Some(failure) = &self.failure { Poll::Failed(failure) }
        else if self.next == self.input.len() { Poll::Complete }
        else { Poll::Pending { completed: self.next, total: self.input.len() } }
    }

    pub fn into_completed(self) -> Result<Completed<A>, Self> {
        if self.failure.is_some() || self.next != self.input.len() { return Err(self); }
        Ok(Completed { model: self.model, input: self.input, rows: self.output, status: self.status })
    }
}
