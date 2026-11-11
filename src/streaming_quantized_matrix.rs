use crate::{conditional_gate::GateError, matrix_f32::{self, Operation},
    matrix_value_traits::{GenericFirstTap, SourceAccumulatingMatrix},
    quantized_matrix::{OverwriteFold, QuantizedMatrix}, streaming_matrix::{apply_postprocess, Postprocess}};
pub struct Parts<P> {
    pub matrix: QuantizedMatrix, pub overwrite_fold: OverwriteFold, pub postprocess: Option<P>,
}
pub enum Failure<E> {
    InputWidth { expected: usize, actual: usize },
    OutputWidth { expected: usize, actual: usize },
    Environment(matrix_f32::Error), Matrix(String), Postprocess(E),
}

pub struct QuantizedValueTap<P: Postprocess> {
    parts: Parts<P>, operation: Option<Operation>, failure: Option<Failure<P::Error>>,
}
impl<P: Postprocess> QuantizedValueTap<P> {

    pub fn from_explicit_product_parts(parts: Parts<P>) -> Self {
        Self { parts, operation: None, failure: None }
    }
    pub fn parts(&self) -> &Parts<P> { &self.parts }
    pub fn operation(&self) -> Option<Operation> { self.operation }
    pub fn failure(&self) -> Option<&Failure<P::Error>> { self.failure.as_ref() }
    pub fn into_ready_parts(self) -> Result<Parts<P>, Self> {
        if self.failure.is_some() || self.operation.is_some() { Err(self) } else { Ok(self.parts) }
    }
    fn execute(&mut self, input: &[f32], output: &mut [f32], operation: Operation) -> Result<(), GateError> {
        if self.failure.is_some() || self.operation.is_some() { return Err(GateError::ResetRequired); }
        self.operation = Some(operation);
        let columns = self.parts.matrix.input_width();
        let rows = self.parts.matrix.output_width();
        if input.len() != columns {
            self.failure = Some(Failure::InputWidth { expected: columns, actual: input.len() });
            return Err(GateError::RowWidth { expected: columns, actual: input.len() });
        }
        if output.len() != rows {
            self.failure = Some(Failure::OutputWidth { expected: rows, actual: output.len() });
            return Err(GateError::RowWidth { expected: rows, actual: output.len() });
        }
        if let Err(error) = matrix_f32::check_environment() {
            self.failure = Some(Failure::Environment(error));
            return Err(GateError::Dependency("quantized value tap requires its floating environment"));
        }
        let result = match operation {
            Operation::Overwrite => self.parts.matrix.forward_unbiased_saturation(input,
                self.parts.overwrite_fold.saturate_each()).map(|values| output.copy_from_slice(&values)),
            Operation::Accumulate => self.parts.matrix.forward_add_wide(input, output),
        };
        if let Err(error) = result {
            self.failure = Some(Failure::Matrix(error));
            return Err(GateError::Dependency("quantized value tap kernel stopped"));
        }
        if let Err(error) = apply_postprocess(self.parts.postprocess.as_mut(), operation, output) {
            self.failure = Some(Failure::Postprocess(error));
            return Err(GateError::Dependency("quantized value tap postprocessor stopped"));
        }
        self.operation = None;
        Ok(())
    }
}
impl<P: Postprocess> GenericFirstTap for QuantizedValueTap<P> {
    fn overwrite_and_postprocess(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), GateError> {
        self.execute(input, output, Operation::Overwrite)
    }
}
impl<P: Postprocess> SourceAccumulatingMatrix for QuantizedValueTap<P> {
    fn input_width(&self) -> usize { self.parts.matrix.input_width() }
    fn output_width(&self) -> usize { self.parts.matrix.output_width() }
    fn accumulate(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), GateError> {
        self.execute(input, output, Operation::Accumulate)
    }
}
