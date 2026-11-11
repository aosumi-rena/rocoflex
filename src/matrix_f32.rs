#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArithmeticProfile { SseSeparate, AvxSeparate, AvxFused }

impl ArithmeticProfile {
    fn lanes(self) -> usize { match self { Self::SseSeparate => 4, _ => 8 } }
    fn row_capacity(self) -> usize { match self { Self::SseSeparate => 56, _ => 112 } }
    fn rounded_rows(self, rows: usize) -> Option<usize> {
        rows.checked_add(self.lanes() - 1).map(|n| n / self.lanes() * self.lanes())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation { Overwrite, Accumulate }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelCall {
    pub rows: usize,
    pub columns: usize,
    pub input_column_start: usize,
    pub output_row_start: usize,
    pub operation: Operation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompilerPath { Direct, Blocked }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachePolicy { profile: ArithmeticProfile, column_capacity: usize }

impl CachePolicy {

    pub fn from_explicit_cache(profile: ArithmeticProfile, column_capacity: usize)
        -> Result<Self, Error>
    {
        if column_capacity == 0 || column_capacity > i32::MAX as usize {
            return Err(Error::Dimensions);
        }
        Ok(Self { profile, column_capacity })
    }
    pub fn profile(self) -> ArithmeticProfile { self.profile }
    pub fn column_capacity(self) -> usize { self.column_capacity }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout { rows: usize, columns: usize, column_stride: usize }

impl Layout {
    pub fn from_dimensions(rows: usize, columns: usize) -> Result<Self, Error> {
        if rows == 0 || columns == 0 || rows > i32::MAX as usize || columns > i32::MAX as usize {
            return Err(Error::Dimensions);
        }
        let column_stride = rows.checked_mul(4).and_then(|n| n.checked_add(63))
            .filter(|&n| n <= i32::MAX as usize)
            .map(|n| 16 * (n >> 6)).ok_or(Error::Dimensions)?;
        column_stride.checked_mul(columns).and_then(|n| n.checked_mul(4))
            .filter(|&n| n <= i32::MAX as usize).ok_or(Error::Dimensions)?;
        Ok(Self { rows, columns, column_stride })
    }
    pub fn rows(self) -> usize { self.rows }
    pub fn columns(self) -> usize { self.columns }
    pub fn column_stride(self) -> usize { self.column_stride }
    pub fn storage_words(self) -> usize { self.column_stride * self.columns }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error { Dimensions, Extent, NonfiniteRead, Environment, UnsupportedPlatform }

pub struct Matrix {
    layout: Layout,
    coefficient_words: Vec<u32>,
    policy: CachePolicy,
}

pub struct BindingFailure { pub error: Error, pub all_coefficient_words: Vec<u32> }

pub struct BatchError { pub row: Option<usize>, pub error: Error }

pub(crate) fn check_environment() -> Result<(), Error> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        let mut control = 0u32;
        unsafe { core::arch::asm!("stmxcsr [{address}]", address = in(reg) &mut control,
            options(nostack, preserves_flags)); }
        if control & 0xffc0 != 0x1f80 { return Err(Error::Environment); }
        Ok(())
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
    { Err(Error::UnsupportedPlatform) }
}

impl Matrix {

    pub fn from_row_major_bits(rows: usize, columns: usize, row_major: &[u32],
        policy: CachePolicy, all_initial_words: Vec<u32>) -> Result<Self, BindingFailure>
    {
        if rows.checked_mul(columns) != Some(row_major.len()) {
            return Err(BindingFailure { error: Error::Extent, all_coefficient_words: all_initial_words });
        }
        let mut matrix = Self::from_complete_backing(rows, columns, policy, all_initial_words)?;
        for row in 0..rows {
            for column in 0..columns {
                matrix.coefficient_words[column * matrix.layout.column_stride + row] =
                    row_major[row * columns + column];
            }
        }
        Ok(matrix)
    }

    pub fn from_complete_backing(rows: usize, columns: usize, policy: CachePolicy,
        all_coefficient_words: Vec<u32>) -> Result<Self, BindingFailure>
    {
        let layout = match Layout::from_dimensions(rows, columns) {
            Ok(layout) => layout,
            Err(error) => return Err(BindingFailure { error, all_coefficient_words }),
        };
        if all_coefficient_words.len() != layout.storage_words()
            || rows.checked_add(policy.profile.row_capacity() - 1).is_none_or(|n| n > i32::MAX as usize)
            || columns.checked_add(policy.column_capacity - 1).is_none_or(|n| n > i32::MAX as usize)
            || (columns - 1).checked_mul(4).is_none_or(|n| n > i32::MAX as usize)
        {
            return Err(BindingFailure { error: Error::Extent, all_coefficient_words });
        }
        Ok(Self { layout, coefficient_words: all_coefficient_words, policy })
    }

    pub fn layout(&self) -> Layout { self.layout }
    pub fn cache_policy(&self) -> CachePolicy { self.policy }
    pub fn coefficient_words(&self) -> &[u32] { &self.coefficient_words }
    pub fn into_coefficient_words(self) -> Vec<u32> { self.coefficient_words }

    pub fn compiler_path(&self) -> CompilerPath {
        if self.layout.rows <= self.policy.profile.row_capacity()
            && self.layout.columns <= self.policy.column_capacity
        { CompilerPath::Direct } else { CompilerPath::Blocked }
    }

    pub fn calls(&self, operation: Operation) -> impl Iterator<Item = KernelCall> {
        let layout = self.layout;
        let row_capacity = self.policy.profile.row_capacity();
        let column_capacity = self.policy.column_capacity;
        (0..layout.columns).step_by(column_capacity).flat_map(move |column| {
            (0..layout.rows).step_by(row_capacity).map(move |row| KernelCall {
                rows: row_capacity.min(layout.rows - row),
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
        check_environment()?;
        if input.len() < self.layout.columns || row_offset > i32::MAX as usize {
            return Err(Error::Extent);
        }
        let row_capacity = self.policy.profile.row_capacity();

        for start in (0..self.layout.rows).step_by(row_capacity) {
            let rows = row_capacity.min(self.layout.rows - start);
            let end = row_offset.checked_add(start)
                .and_then(|n| n.checked_add(self.policy.profile.rounded_rows(rows)?))
                .ok_or(Error::Extent)?;
            if end > self.layout.column_stride || end > destination_words.len() {
                return Err(Error::Extent);
            }
        }
        let mut scratch = destination_words.to_vec();

        for call in self.calls(operation) {
            self.execute_chunk(input, &mut scratch, row_offset + call.output_row_start,
                call.rows, call.input_column_start, call.columns,
                call.operation == Operation::Accumulate)?;
        }
        destination_words.copy_from_slice(&scratch);
        Ok(())
    }

    fn execute_chunk(&self, input: &[f32], destination: &mut [u32], row: usize,
        rows: usize, column: usize, columns: usize, accumulate: bool) -> Result<(), Error>
    {
        execute_packed_block(self.policy.profile, self.layout.column_stride,
            &self.coefficient_words, input, destination,
            PackedBlock { row, rows, column, columns, accumulate })
    }
}

pub(crate) struct PackedBlock {
    pub row: usize, pub rows: usize, pub column: usize, pub columns: usize, pub accumulate: bool,
}

pub(crate) fn execute_packed_block(profile: ArithmeticProfile, stride: usize,
    coefficients: &[u32], input: &[f32], destination: &mut [u32], block: PackedBlock)
    -> Result<(), Error>
{
    execute_packed_block_from_bits(profile, stride, coefficients.len(),
        |index| coefficients[index], input, destination, block)
}

pub(crate) fn execute_packed_block_from_bits(profile: ArithmeticProfile, stride: usize,
    coefficient_words: usize, coefficient_bits: impl Fn(usize) -> u32,
    input: &[f32], destination: &mut [u32], block: PackedBlock) -> Result<(), Error>
{
        let PackedBlock { row, rows, column, columns, accumulate } = block;
        if rows == 0 || columns == 0 || stride == 0 { return Err(Error::Dimensions); }
        let end_row = row.checked_add(profile.rounded_rows(rows).ok_or(Error::Extent)?)
            .ok_or(Error::Extent)?;
        let end_column = column.checked_add(columns).ok_or(Error::Extent)?;
        if end_row > stride || end_row > destination.len() || end_column > input.len()
            || coefficient_words % stride != 0 || end_column > coefficient_words / stride
        { return Err(Error::Extent); }
        if input[column..end_column].iter().any(|v| !v.is_finite())
            || (column..end_column).any(|c| (row..end_row)
                .any(|r| !f32::from_bits(coefficient_bits(c * stride + r)).is_finite()))
            || (accumulate && destination[row..end_row].iter()
                .any(|&v| !f32::from_bits(v).is_finite()))
        { return Err(Error::NonfiniteRead); }
        for r in row..end_row {
            let (mut value, first) = if accumulate { (f32::from_bits(destination[r]), column) }
                else { (input[column] * f32::from_bits(coefficient_bits(column * stride + r)), column + 1) };
            for c in first..end_column {
                let weight = f32::from_bits(coefficient_bits(c * stride + r));
                value = if profile == ArithmeticProfile::AvxFused {
                    input[c].mul_add(weight, value)
                } else { (input[c] * weight) + value };
            }
            destination[r] = value.to_bits();
        }
        Ok(())
}
