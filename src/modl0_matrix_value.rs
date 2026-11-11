use crate::{
    logical_read_rows::ReadRows,
    matrix_f32::Operation,
    matrix_policy::{self, BoundMatrix},
    quantized_matrix::{self, OverwriteFold, QuantizedMatrix},
    vector_f32::Backing,
};

pub enum Matrix {
    Float(BoundMatrix),
    Quantized { matrix: QuantizedMatrix, overwrite_fold: OverwriteFold },
}
pub enum BatchError {
    Float(matrix_policy::BatchError),
    Quantized(quantized_matrix::BatchError),
}
impl Matrix {
    pub fn input_width(&self) -> usize {
        match self {
            Self::Float(matrix) => matrix.plan().dimensions().1,
            Self::Quantized { matrix, .. } => matrix.input_width(),
        }
    }
    pub fn output_width(&self) -> usize {
        match self {
            Self::Float(matrix) => matrix.plan().dimensions().0,
            Self::Quantized { matrix, .. } => matrix.output_width(),
        }
    }
    pub fn execute_read_backings(&self, input: &dyn ReadRows, destinations: &mut [Backing])
        -> Result<(), BatchError>
    {
        match self {
            Self::Float(matrix) => matrix.execute_read_backings(input, destinations, Operation::Overwrite)
                .map_err(BatchError::Float),
            Self::Quantized { matrix, overwrite_fold } => matrix.execute_read_backings(input,
                destinations, *overwrite_fold)
                .map_err(BatchError::Quantized),
        }
    }
}
