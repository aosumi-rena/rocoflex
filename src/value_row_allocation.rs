use std::collections::TryReserveError;
use crate::{matrix_policy::FactoryState, vector_f32::{self, Backing}};

#[derive(Clone, Copy)]
pub struct BackingPolicy { pub factory: FactoryState, pub allocation_word: u32 }
pub enum Error {
    Dimensions(vector_f32::Error), Capacity(TryReserveError), Binding(vector_f32::BindingFailure),
}
pub enum LogicalError { Width(usize), Capacity(TryReserveError) }
impl From<LogicalError> for Error {
    fn from(error: LogicalError) -> Self {
        match error { LogicalError::Width(_) => Self::Dimensions(vector_f32::Error::Dimensions),
            LogicalError::Capacity(cause) => Self::Capacity(cause) }
    }
}
pub fn allocate_backing(width: usize, policy: BackingPolicy) -> Result<Backing, Error> {
    let capacity = Backing::minimum_capacity_bytes(width).map_err(Error::Dimensions)?;
    let mut words = Vec::new();
    words.try_reserve_exact(capacity / 4).map_err(Error::Capacity)?;
    words.resize(capacity / 4, policy.allocation_word);
    Backing::from_explicit_state(policy.factory, width, capacity, words).map_err(Error::Binding)
}
pub fn allocate_logical_row(width: usize, allocation_word: u32) -> Result<Vec<f32>, LogicalError> {
    if width == 0 || width > i32::MAX as usize { return Err(LogicalError::Width(width)); }
    let mut row = Vec::new();
    row.try_reserve_exact(width).map_err(LogicalError::Capacity)?;
    row.resize(width, f32::from_bits(allocation_word));
    Ok(row)
}
