use super::RequiredBacking;
use crate::{matrix_f32::Operation, vector_f32::Backing};

pub struct BackingRecord {
    pub operation: Operation, pub input_width: usize, pub destination: Backing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackingError {
    PriorFailure, Exhausted { issued: usize },
    Operation { expected: Operation, actual: Operation },
    InputWidth { expected: usize, actual: usize },
    OutputWidth { expected: usize, actual: usize },
}
pub struct SuppliedBackings {
    records: Vec<BackingRecord>, issued: usize, failure: Option<BackingError>,
}
impl SuppliedBackings {

    pub fn from_explicit_records(records: Vec<BackingRecord>) -> Self {
        Self { records, issued: 0, failure: None }
    }
    pub fn records(&self) -> &[BackingRecord] { &self.records }

    pub fn issued(&self) -> usize { self.issued }
    pub fn failure(&self) -> Option<BackingError> { self.failure }
    pub fn into_ready_records(self) -> Result<(Vec<BackingRecord>, usize), Self> {
        if self.failure.is_some() { Err(self) } else { Ok((self.records, self.issued)) }
    }
}
impl RequiredBacking for SuppliedBackings {
    type Error = BackingError;
    fn destination_words(&mut self, operation: Operation, input_width: usize, output_width: usize)
        -> Result<&mut [u32], Self::Error> {
        if self.failure.is_some() { return Err(BackingError::PriorFailure); }
        let reason = match self.records.get(self.issued) {
            None => Some(BackingError::Exhausted { issued: self.issued }),
            Some(record) if record.operation != operation =>
                Some(BackingError::Operation { expected: record.operation, actual: operation }),
            Some(record) if record.input_width != input_width =>
                Some(BackingError::InputWidth { expected: record.input_width, actual: input_width }),
            Some(record) if record.destination.logical_width() != output_width =>
                Some(BackingError::OutputWidth { expected: record.destination.logical_width(), actual: output_width }),
            Some(_) => None,
        };
        if let Some(error) = reason { self.failure = Some(error); return Err(error); }
        let index = self.issued;

        self.issued += 1;
        Ok(self.records[index].destination.full_bits_mut())
    }
}
