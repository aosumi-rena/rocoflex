use crate::{conditional_gate::GateError, matrix_f32::Operation, matrix_policy::{BoundMatrix, ExecutionError},
    matrix_value_traits::{GenericFirstTap, SourceAccumulatingMatrix}};
#[path = "streaming_matrix_backing.rs"]
mod backing;
pub use backing::{BackingError, BackingRecord, SuppliedBackings};

pub trait RequiredBacking {
    type Error;
    fn destination_words(&mut self, operation: Operation, input_width: usize, output_width: usize)
        -> Result<&mut [u32], Self::Error>;
}

pub trait Postprocess {
    type Error;
    fn overwrite_slot56(&mut self, output: &mut [f32]) -> Result<(), Self::Error>;
    fn accumulate_slot38(&mut self, output: &mut [f32]) -> Result<(), Self::Error>;
}
pub(crate) fn apply_postprocess<P: Postprocess>(postprocess: Option<&mut P>, operation: Operation,
    output: &mut [f32]) -> Result<(), P::Error> {
    match postprocess {
        Some(postprocess) => match operation {
            Operation::Overwrite => postprocess.overwrite_slot56(output),
            Operation::Accumulate => postprocess.accumulate_slot38(output),
        },
        None => Ok(()),
    }
}

pub enum NoPostprocess {}
impl Postprocess for NoPostprocess {
    type Error = std::convert::Infallible;
    fn overwrite_slot56(&mut self, _: &mut [f32]) -> Result<(), Self::Error> { match *self {} }
    fn accumulate_slot38(&mut self, _: &mut [f32]) -> Result<(), Self::Error> { match *self {} }
}
pub struct Parts<B, P> {
    pub matrix: BoundMatrix, pub backing: B, pub postprocess: Option<P>,
}
pub enum Failure<B, P> {
    InputWidth { expected: usize, actual: usize },
    OutputWidth { expected: usize, actual: usize },
    Backing(B), BackingExtent { required_prefix: usize, actual: usize },
    Matrix(ExecutionError), Postprocess(P),
}
pub struct StreamingMatrix<B: RequiredBacking, P: Postprocess> {
    parts: Parts<B, P>, operation: Option<Operation>, failure: Option<Failure<B::Error, P::Error>>,
}
impl<B: RequiredBacking, P: Postprocess> StreamingMatrix<B, P> {

    pub fn from_explicit_parts(parts: Parts<B, P>) -> Self {
        Self { parts, operation: None, failure: None }
    }
    pub fn parts(&self) -> &Parts<B, P> { &self.parts }
    pub fn operation(&self) -> Option<Operation> { self.operation }
    pub fn failure(&self) -> Option<&Failure<B::Error, P::Error>> { self.failure.as_ref() }
    pub fn into_ready_parts(self) -> Result<Parts<B, P>, Self> {
        if self.failure.is_some() || self.operation.is_some() { Err(self) } else { Ok(self.parts) }
    }
    fn execute(&mut self, input: &[f32], output: &mut [f32], operation: Operation) -> Result<(), GateError> {
        if self.failure.is_some() || self.operation.is_some() { return Err(GateError::ResetRequired); }
        self.operation = Some(operation);
        let (rows, columns) = self.parts.matrix.plan().dimensions();
        if input.len() != columns {
            self.failure = Some(Failure::InputWidth { expected: columns, actual: input.len() });
            return Err(GateError::RowWidth { expected: columns, actual: input.len() });
        }
        if output.len() != rows {
            self.failure = Some(Failure::OutputWidth { expected: rows, actual: output.len() });
            return Err(GateError::RowWidth { expected: rows, actual: output.len() });
        }
        let words = match self.parts.backing.destination_words(operation, columns, rows) {
            Ok(words) => words,
            Err(error) => {
                self.failure = Some(Failure::Backing(error));
                return Err(GateError::Dependency("streaming matrix destination backing required"));
            }
        };
        if words.len() < rows {
            self.failure = Some(Failure::BackingExtent { required_prefix: rows, actual: words.len() });
            return Err(GateError::Dependency("streaming matrix destination prefix missing"));
        }

        for (word, value) in words[..rows].iter_mut().zip(output.iter()) { *word = value.to_bits(); }
        if let Err(error) = self.parts.matrix.execute(input, words, 0, operation) {
            self.failure = Some(Failure::Matrix(error));
            return Err(GateError::Dependency("streaming matrix kernel stopped"));
        }
        for (value, &word) in output.iter_mut().zip(words.iter()) { *value = f32::from_bits(word); }
        if let Err(error) = apply_postprocess(self.parts.postprocess.as_mut(), operation, output) {
            self.failure = Some(Failure::Postprocess(error));
            return Err(GateError::Dependency("streaming matrix postprocessor stopped"));
        }
        self.operation = None;
        Ok(())
    }
}
impl<B: RequiredBacking, P: Postprocess> GenericFirstTap for StreamingMatrix<B, P> {
    fn overwrite_and_postprocess(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), GateError> {
        self.execute(input, output, Operation::Overwrite)
    }
}
impl<B: RequiredBacking, P: Postprocess> SourceAccumulatingMatrix for StreamingMatrix<B, P> {
    fn input_width(&self) -> usize { self.parts.matrix.plan().dimensions().1 }
    fn output_width(&self) -> usize { self.parts.matrix.plan().dimensions().0 }
    fn accumulate(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), GateError> {
        self.execute(input, output, Operation::Accumulate)
    }
}
