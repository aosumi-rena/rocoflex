use crate::matrix_f32::{self, ArithmeticProfile, PackedBlock};
pub use crate::matrix_f32::{Operation, KernelCall, CompilerPath};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachePolicy { fused: bool, column_capacity: usize }

impl CachePolicy {
    pub fn from_explicit_cache(fused: bool, column_capacity: usize) -> Result<Self, Error> {
        if column_capacity == 0 || column_capacity > i32::MAX as usize {
            return Err(Error::Dimensions);
        }
        Ok(Self { fused, column_capacity })
    }
    pub fn fused(self) -> bool { self.fused }
    pub fn column_capacity(self) -> usize { self.column_capacity }
    fn arithmetic(self) -> ArithmeticProfile {
        if self.fused { ArithmeticProfile::AvxFused } else { ArithmeticProfile::AvxSeparate }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout { rows: usize, columns: usize, column_stride: usize }

impl Layout {
    pub fn from_dimensions(rows: usize, columns: usize) -> Result<Self, Error> {
        if rows == 0 || columns == 0 || rows > i32::MAX as usize || columns > i32::MAX as usize {
            return Err(Error::Dimensions);
        }
        let column_stride = rows.checked_mul(2).and_then(|n| n.checked_add(63))
            .filter(|&n| n <= i32::MAX as usize)
            .map(|n| 32 * (n >> 6)).ok_or(Error::Dimensions)?;
        column_stride.checked_mul(columns).and_then(|n| n.checked_mul(2))
            .filter(|&n| n <= i32::MAX as usize).ok_or(Error::Dimensions)?;
        Ok(Self { rows, columns, column_stride })
    }
    pub fn rows(self) -> usize { self.rows }
    pub fn columns(self) -> usize { self.columns }

    pub fn column_stride(self) -> usize { self.column_stride }
    pub fn storage_words(self) -> usize { self.column_stride * self.columns }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Dimensions, Extent, NonfiniteRead, NonfiniteResult, Environment, UnsupportedPlatform,
}

impl From<matrix_f32::Error> for Error {
    fn from(error: matrix_f32::Error) -> Self {
        match error {
            matrix_f32::Error::Dimensions => Self::Dimensions,
            matrix_f32::Error::Extent => Self::Extent,
            matrix_f32::Error::NonfiniteRead => Self::NonfiniteRead,
            matrix_f32::Error::Environment => Self::Environment,
            matrix_f32::Error::UnsupportedPlatform => Self::UnsupportedPlatform,
        }
    }
}

pub struct BindingFailure { pub error: Error, pub all_coefficient_words: Vec<u16> }

pub struct BatchError { pub row: Option<usize>, pub error: Error }

pub struct Matrix {
    layout: Layout,
    coefficient_words: Vec<u16>,
    policy: CachePolicy,
}

impl Matrix {

    pub fn from_complete_backing(rows: usize, columns: usize, policy: CachePolicy,
        all_coefficient_words: Vec<u16>) -> Result<Self, BindingFailure>
    {
        let layout = match Layout::from_dimensions(rows, columns) {
            Ok(layout) => layout,
            Err(error) => return Err(BindingFailure { error, all_coefficient_words }),
        };
        if all_coefficient_words.len() != layout.storage_words()
            || layout.column_stride.checked_mul(2).is_none_or(|n| n > i32::MAX as usize)
            || rows.checked_add(111).is_none_or(|n| n > i32::MAX as usize)
            || columns.checked_add(policy.column_capacity - 1).is_none_or(|n| n > i32::MAX as usize)
            || (columns - 1).checked_mul(4).is_none_or(|n| n > i32::MAX as usize)
        {
            return Err(BindingFailure { error: Error::Extent, all_coefficient_words });
        }
        Ok(Self { layout, coefficient_words: all_coefficient_words, policy })
    }

    pub fn from_row_major_bits(rows: usize, columns: usize, row_major: &[u32],
        policy: CachePolicy, all_initial_words: Vec<u16>) -> Result<Self, BindingFailure>
    {
        if rows.checked_mul(columns) != Some(row_major.len()) {
            return Err(BindingFailure { error: Error::Extent, all_coefficient_words: all_initial_words });
        }
        let mut matrix = Self::from_complete_backing(rows, columns, policy, all_initial_words)?;
        for row in 0..rows {
            for column in 0..columns {
                matrix.coefficient_words[column * matrix.layout.column_stride + row] =
                    crate::half_storage::encode_f32_bits(row_major[row * columns + column]);
            }
        }
        Ok(matrix)
    }

    pub fn layout(&self) -> Layout { self.layout }
    pub fn cache_policy(&self) -> CachePolicy { self.policy }
    pub fn coefficient_words(&self) -> &[u16] { &self.coefficient_words }
    pub fn into_coefficient_words(self) -> Vec<u16> { self.coefficient_words }

    pub fn compiler_path(&self) -> CompilerPath {
        if self.layout.rows <= 112 && self.layout.columns <= self.policy.column_capacity {
            CompilerPath::Direct
        } else { CompilerPath::Blocked }
    }

    pub fn calls(&self, operation: Operation) -> impl Iterator<Item = KernelCall> {
        let layout = self.layout;
        let column_capacity = self.policy.column_capacity;
        (0..layout.columns).step_by(column_capacity).flat_map(move |column| {
            (0..layout.rows).step_by(112).map(move |row| KernelCall {
                rows: 112.min(layout.rows - row),
                columns: column_capacity.min(layout.columns - column),
                input_column_start: column,
                output_row_start: row,
                operation: if column != 0 || operation == Operation::Accumulate {
                    Operation::Accumulate
                } else { Operation::Overwrite },
            })
        })
    }

    pub fn execute_rows(&self, inputs: &[Vec<f32>], destinations: &mut [Vec<u32>],
        operation: Operation) -> Result<(), BatchError>
    {
        crate::matrix_batch::execute(inputs.len(), self.layout.columns, self.layout.rows,
            |row| inputs.get(row).map(Vec::as_slice), destinations, Error::Extent,
            |input, output| self.execute(input, output, 0, operation))
            .map_err(|(row, error)| BatchError { row, error })
    }

    pub fn execute_read_rows(&self, inputs: &dyn crate::logical_read_rows::ReadRows,
        destinations: &mut [Vec<u32>], operation: Operation) -> Result<(), BatchError>
    {
        crate::matrix_batch::execute(inputs.row_count(), self.layout.columns, self.layout.rows,
            |row| inputs.read_row(row), destinations, Error::Extent,
            |input, output| self.execute(input, output, 0, operation))
            .map_err(|(row, error)| BatchError { row, error })
    }

    pub fn execute_read_backings(&self, inputs: &dyn crate::logical_read_rows::ReadRows,
        destinations: &mut [crate::vector_f32::Backing], operation: Operation) -> Result<(), BatchError>
    {
        crate::matrix_batch::execute(inputs.row_count(), self.layout.columns, self.layout.rows,
            |row| inputs.read_row(row), destinations, Error::Extent,
            |input, output| self.execute(input, output, 0, operation))
            .map_err(|(row, error)| BatchError { row, error })
    }

    pub fn execute(&self, input: &[f32], destination_words: &mut [u32],
        row_offset: usize, operation: Operation) -> Result<(), Error>
    {
        matrix_f32::check_environment()?;
        if input.len() < self.layout.columns
            || row_offset.checked_mul(2).is_none_or(|n| n > i32::MAX as usize)
        {
            return Err(Error::Extent);
        }
        for row in (0..self.layout.rows).step_by(112) {
            let rows = 112.min(self.layout.rows - row);
            let rounded = rows.checked_add(7).ok_or(Error::Extent)? / 8 * 8;
            let end = row_offset.checked_add(row).and_then(|n| n.checked_add(rounded))
                .ok_or(Error::Extent)?;
            if end > self.layout.column_stride || end > destination_words.len() {
                return Err(Error::Extent);
            }
        }
        let mut scratch = destination_words.to_vec();
        for call in self.calls(operation) {
            let start = row_offset + call.output_row_start;

            matrix_f32::execute_packed_block_from_bits(self.policy.arithmetic(),
                self.layout.column_stride, self.coefficient_words.len(),
                |index| crate::half_storage::decode_to_f32_bits(self.coefficient_words[index]),
                input, &mut scratch, PackedBlock {
                    row: start, rows: call.rows, column: call.input_column_start,
                    columns: call.columns, accumulate: call.operation == Operation::Accumulate,
                })?;
            let end = start + (call.rows + 7) / 8 * 8;
            if scratch[start..end].iter().any(|&word| !f32::from_bits(word).is_finite()) {
                return Err(Error::NonfiniteResult);
            }
        }
        destination_words.copy_from_slice(&scratch);
        Ok(())
    }
}
