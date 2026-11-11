use crate::{dct::Dct, matrix_policy::ExecutionError, vector_f32::Backing};

pub struct Provider {
    dct: Dct, output: Backing, last_input: Vec<f32>, failure: Option<Failure>,
}
pub struct BindingFailure { pub dct: Dct, pub output: Backing }
pub enum Failure { Width, Matrix(ExecutionError) }
#[derive(Clone, Copy, Debug)]
pub struct Stopped;

impl Provider {
    pub fn from_explicit_backing(dct: Dct, output: Backing) -> Result<Self, BindingFailure> {
        if output.logical_width() != dct.output_width() { return Err(BindingFailure { dct, output }); }
        Ok(Self { dct, output, last_input: Vec::new(), failure: None })
    }
    pub fn dct(&self) -> &Dct { &self.dct }
    pub fn output(&self) -> &Backing { &self.output }
    pub fn last_input(&self) -> &[f32] { &self.last_input }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }
}
impl crate::condition::Dct for Provider {
    type Error = Stopped;
    fn input_width(&self) -> usize { self.dct.input_width() }
    fn output_width(&self) -> usize { self.dct.output_width() }
    fn overwrite_prefix(&mut self, input: &[f32], prefix: &mut [f32]) -> Result<(), Stopped> {
        if self.failure.is_some() { return Err(Stopped); }
        self.last_input.clear(); self.last_input.extend_from_slice(input);
        if input.len() != self.dct.input_width() || prefix.len() > self.output.logical_width() {
            self.failure = Some(Failure::Width); return Err(Stopped);
        }
        if let Err(error) = self.dct.overwrite_separate(input, &mut self.output) {
            self.failure = Some(Failure::Matrix(error)); return Err(Stopped);
        }
        for (destination, &word) in prefix.iter_mut().zip(self.output.logical_bits()) {
            *destination = f32::from_bits(word);
        }
        Ok(())
    }
}
