use crate::{matrix_f32::Operation, matrix_policy::{BoundMatrix, ExecutionError, Request},
    vector_f32::Backing};

pub struct Dct { matrices: [BoundMatrix; 2] }
pub struct BindingFailure { pub matrices: [BoundMatrix; 2] }
pub struct StartFailure {
    pub reason: &'static str,
    pub dct: Dct,
    pub input: Vec<Vec<f32>>,
    pub output: Vec<Backing>,
}
pub struct Failure { pub row: usize, pub error: ExecutionError }
pub struct Run {
    dct: Dct,
    input: Vec<Vec<f32>>,
    output: Vec<Backing>,
    next: usize,
    failure: Option<Failure>,
}
pub enum Poll<'a> { Pending { completed: usize, total: usize }, Complete, Failed(&'a Failure) }
pub struct Completed {
    pub dct: Dct,
    pub input: Vec<Vec<f32>>,
    pub output: Vec<Backing>,
}

impl Dct {

    pub fn from_explicit_matrices(matrices: [BoundMatrix; 2]) -> Result<Self, BindingFailure> {
        if matrices.iter().any(|matrix| matrix.plan().request() != Request::DctCoefficients) {
            return Err(BindingFailure { matrices });
        }
        Ok(Self { matrices })
    }
    pub fn matrices(&self) -> &[BoundMatrix; 2] { &self.matrices }
    pub fn input_width(&self) -> usize { self.matrices[0].plan().dimensions().1 }
    pub fn output_width(&self) -> usize { self.matrices[0].plan().dimensions().0 }

    pub(crate) fn overwrite_separate(&self, input: &[f32], output: &mut Backing)
        -> Result<(), ExecutionError>
    {
        self.matrices[0].execute(input, output.full_bits_mut(), 0, Operation::Overwrite)
    }

    pub fn begin(self, input: Vec<Vec<f32>>, output: Vec<Backing>) -> Result<Run, StartFailure> {
        let reason = if input.len() != output.len() {
            Some("DCT row counts differ")
        } else if input.iter().any(|row| row.len() != self.input_width()
            || row.iter().any(|value| !value.is_finite())) {
            Some("DCT requires finite complete input rows")
        } else if output.iter().any(|row| row.logical_width() != self.output_width()) {
            Some("DCT destination logical width differs")
        } else { None };
        if let Some(reason) = reason { return Err(StartFailure { reason, dct: self, input, output }); }
        Ok(Run { dct: self, input, output, next: 0, failure: None })
    }
}

impl Run {
    pub fn dct(&self) -> &Dct { &self.dct }
    pub fn input(&self) -> &[Vec<f32>] { &self.input }

    pub fn output(&self) -> &[Backing] { &self.output }
    pub fn completed_rows(&self) -> usize { self.next }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }

    pub fn poll(&mut self, row_budget: usize) -> Poll<'_> {
        if self.failure.is_none() {
            for _ in 0..row_budget {
                if self.next == self.input.len() { break; }
                let result = self.dct.overwrite_separate(&self.input[self.next], &mut self.output[self.next]);
                if let Err(error) = result {
                    self.failure = Some(Failure { row: self.next, error }); break;
                }
                self.next += 1;
            }
        }
        if let Some(failure) = &self.failure { Poll::Failed(failure) }
        else if self.next == self.input.len() { Poll::Complete }
        else { Poll::Pending { completed: self.next, total: self.input.len() } }
    }

    pub fn into_completed(self) -> Result<Completed, Self> {
        if self.failure.is_some() || self.next != self.input.len() { return Err(self); }
        Ok(Completed { dct: self.dct, input: self.input, output: self.output })
    }
}
