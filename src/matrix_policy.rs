use crate::{matrix_f32, matrix_half};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactoryState { flags: [u8; 16] }

impl FactoryState {

    pub fn from_explicit_flags(flags: [u8; 16]) -> Result<Self, Error> {
        admit_flags(flags)?;
        if flags != factory_dependency_mask(flags) { return Err(Error::FactoryDependencies); }
        Ok(Self { flags })
    }

    pub fn from_initializer_flags(flags: [u8; 16]) -> Result<Self, Error> {
        admit_flags(flags)?;
        Ok(Self { flags: factory_dependency_mask(flags) })
    }
    pub fn flags(self) -> [u8; 16] { self.flags }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompilerCacheState { flags: [u8; 16], column_capacity: usize }

impl CompilerCacheState {

    pub fn from_explicit_state(flags: [u8; 16], column_capacity: usize)
        -> Result<Self, Error>
    {
        admit_flags(flags)?;
        if column_capacity == 0 || column_capacity > i32::MAX as usize {
            return Err(Error::Dimensions);
        }
        Ok(Self { flags, column_capacity })
    }
    pub fn flags(self) -> [u8; 16] { self.flags }
    pub fn column_capacity(self) -> usize { self.column_capacity }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    FloatParameters,
    DctCoefficients,

    DifferentFactoryClone { mode: u8 },
}

impl Request {
    pub fn extra_flags(self) -> [u8; 2] {
        match self {
            Self::FloatParameters => [0, 1],
            Self::DctCoefficients => [0, 0],
            Self::DifferentFactoryClone { mode } => [0, u8::from(mode != 2)],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Dimensions, CacheFlagsDiffer, FactoryDependencies, ArithmeticOutsideFiniteBackend,
    NonBooleanFlag { offset: usize, value: u8 },
}

fn admit_flags(flags: [u8; 16]) -> Result<(), Error> {
    for (offset, &value) in flags[..8].iter().enumerate() {
        if value > 1 { return Err(Error::NonBooleanFlag { offset, value }); }
    }
    Ok(())
}

fn factory_dependency_mask(mut flags: [u8; 16]) -> [u8; 16] {
    if flags[2] == 0 {
        flags[0] = 0; flags[3] = 0; flags[4] = 0; flags[6] = 0;
    } else if flags[4] == 0 { flags[6] = 0; }
    flags
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage {
    F32 { layout: matrix_f32::Layout, cache: matrix_f32::CachePolicy },
    FiniteHalf { layout: matrix_half::Layout, cache: matrix_half::CachePolicy },

    RequiredSpecialized { rows: usize, columns: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan { request: Request, factory: FactoryState, storage: Storage }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadError { HeaderMissing, Dimensions, Truncated }

pub struct ParameterPayload<'a> {
    bytes: &'a [u8], rows: usize, columns: usize, required_bytes: usize,
}

impl<'a> ParameterPayload<'a> {

    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, PayloadError> {
        if bytes.len() < 8 { return Err(PayloadError::HeaderMissing); }
        let rows = i32::from_le_bytes(bytes[..4].try_into().expect("four-byte row prefix"));
        let columns = i32::from_le_bytes(bytes[4..8].try_into().expect("four-byte column prefix"));
        if rows <= 0 || columns <= 0 || bytes.len() > i32::MAX as usize {
            return Err(PayloadError::Dimensions);
        }
        let required_bytes = (rows as usize).checked_mul(columns as usize)
            .and_then(|n| n.checked_mul(4)).and_then(|n| n.checked_add(8))
            .filter(|&n| n <= i32::MAX as usize).ok_or(PayloadError::Dimensions)?;
        if bytes.len() < required_bytes { return Err(PayloadError::Truncated); }
        Ok(Self { bytes, rows: rows as usize, columns: columns as usize, required_bytes })
    }
    pub fn dimensions(&self) -> (usize, usize) { (self.rows, self.columns) }
    pub fn original_bytes(&self) -> &'a [u8] { self.bytes }
    pub fn trailing_bytes(&self) -> &'a [u8] { &self.bytes[self.required_bytes..] }
    pub fn row_major_bits(&self) -> impl Iterator<Item = u32> + '_ {
        self.bytes[8..self.required_bytes].chunks_exact(4)
            .map(|word| u32::from_le_bytes(word.try_into().expect("four-byte coefficient")))
    }

    pub fn bind(&self, plan: Plan, backing: CoefficientBacking)
        -> Result<BoundMatrix, BindingFailure>
    {
        if plan.dimensions() != self.dimensions()
        {
            return Err(BindingFailure {
                error: BindingError::LogicalDimensions, plan, backing,
            });
        }
        if matches!(plan.request(), Request::DifferentFactoryClone { .. }) {
            return Err(BindingFailure {
                error: BindingError::RequestKind, plan, backing,
            });
        }
        let logical = self.row_major_bits().collect::<Vec<_>>();
        plan.bind_row_major(&logical, backing)
    }
}

pub enum CoefficientBacking { F32(Vec<u32>), Half(Vec<u16>) }

#[derive(Clone, Copy)]
pub enum CoefficientFill { F32(u32), Half(u16) }
pub enum AllocationError {
    BackingFormat, RequiredSpecialized, Capacity(std::collections::TryReserveError),
}

pub enum BindingError {
    BackingFormat, LogicalDimensions, RequestKind, RequiredSpecialized,
    F32(matrix_f32::Error), Half(matrix_half::Error),
}

pub struct BindingFailure {
    pub error: BindingError,
    pub plan: Plan,
    pub backing: CoefficientBacking,
}

enum Kernel { F32(matrix_f32::Matrix), Half(matrix_half::Matrix) }

pub struct BoundMatrix { plan: Plan, kernel: Kernel }

pub enum ExecutionError {
    F32(matrix_f32::Error), Half(matrix_half::Error), Vector(crate::vector_f32::Error),
}

pub enum BatchError { F32(matrix_f32::BatchError), Half(matrix_half::BatchError) }

impl Plan {

    pub fn from_explicit_state(factory: FactoryState, context: CompilerCacheState,
        request: Request, rows: usize, columns: usize) -> Result<Self, Error>
    {
        if rows == 0 || columns == 0 || rows > i32::MAX as usize || columns > i32::MAX as usize {
            return Err(Error::Dimensions);
        }
        if !matches!(request, Request::DifferentFactoryClone { .. })
            && rows.checked_mul(columns).and_then(|n| n.checked_mul(4))
                .and_then(|n| n.checked_add(8)).is_none_or(|n| n > i32::MAX as usize)
        { return Err(Error::Dimensions); }
        if factory.flags != context.flags { return Err(Error::CacheFlagsDiffer); }
        let flags = factory.flags;
        if flags[6] == 0 && columns.checked_add(context.column_capacity - 1)
            .is_none_or(|n| n > i32::MAX as usize)
        { return Err(Error::Dimensions); }
        let storage = if flags[6] != 0 {
            Storage::RequiredSpecialized { rows, columns }
        } else if flags[0] != 0 && request.extra_flags()[1] != 0 {

            if flags[2] == 0 { return Err(Error::ArithmeticOutsideFiniteBackend); }
            let layout = matrix_half::Layout::from_dimensions(rows, columns)
                .map_err(|_| Error::Dimensions)?;
            let cache = matrix_half::CachePolicy::from_explicit_cache(flags[4] != 0,
                context.column_capacity).map_err(|_| Error::Dimensions)?;
            Storage::FiniteHalf { layout, cache }
        } else {
            let profile = match (flags[2] != 0, flags[4] != 0) {
                (false, false) => matrix_f32::ArithmeticProfile::SseSeparate,
                (true, false) => matrix_f32::ArithmeticProfile::AvxSeparate,
                (true, true) => matrix_f32::ArithmeticProfile::AvxFused,
                (false, true) => return Err(Error::ArithmeticOutsideFiniteBackend),
            };
            let layout = matrix_f32::Layout::from_dimensions(rows, columns)
                .map_err(|_| Error::Dimensions)?;
            let cache = matrix_f32::CachePolicy::from_explicit_cache(profile,
                context.column_capacity).map_err(|_| Error::Dimensions)?;
            Storage::F32 { layout, cache }
        };
        Ok(Self { request, factory, storage })
    }
    pub fn request(self) -> Request { self.request }
    pub fn factory(self) -> FactoryState { self.factory }
    pub fn storage(self) -> Storage { self.storage }
    pub fn dimensions(self) -> (usize, usize) {
        match self.storage {
            Storage::F32 { layout, .. } => (layout.rows(), layout.columns()),
            Storage::FiniteHalf { layout, .. } => (layout.rows(), layout.columns()),
            Storage::RequiredSpecialized { rows, columns } => (rows, columns),
        }
    }

    pub fn allocate_backing(self, fill: CoefficientFill)
        -> Result<CoefficientBacking, AllocationError>
    {
        match (self.storage, fill) {
            (Storage::F32 { layout, .. }, CoefficientFill::F32(word)) => {
                let mut words = Vec::new();
                words.try_reserve_exact(layout.storage_words()).map_err(AllocationError::Capacity)?;
                words.resize(layout.storage_words(), word);
                Ok(CoefficientBacking::F32(words))
            }
            (Storage::FiniteHalf { layout, .. }, CoefficientFill::Half(word)) => {
                let mut words = Vec::new();
                words.try_reserve_exact(layout.storage_words()).map_err(AllocationError::Capacity)?;
                words.resize(layout.storage_words(), word);
                Ok(CoefficientBacking::Half(words))
            }
            (Storage::RequiredSpecialized { .. }, _) => Err(AllocationError::RequiredSpecialized),
            _ => Err(AllocationError::BackingFormat),
        }
    }

    pub fn bind_complete(self, backing: CoefficientBacking)
        -> Result<BoundMatrix, BindingFailure>
    { self.bind(None, backing) }

    pub fn bind_row_major(self, logical: &[u32], backing: CoefficientBacking)
        -> Result<BoundMatrix, BindingFailure>
    { self.bind(Some(logical), backing) }

    fn bind(self, logical: Option<&[u32]>, backing: CoefficientBacking)
        -> Result<BoundMatrix, BindingFailure>
    {
        let kernel = match (self.storage, backing) {
            (Storage::F32 { layout, cache }, CoefficientBacking::F32(words)) => {
                let bound = if let Some(values) = logical {
                    matrix_f32::Matrix::from_row_major_bits(layout.rows(), layout.columns(),
                        values, cache, words)
                } else {
                    matrix_f32::Matrix::from_complete_backing(layout.rows(), layout.columns(), cache, words)
                };
                match bound {
                    Ok(matrix) => Kernel::F32(matrix),
                    Err(failure) => return Err(BindingFailure {
                        error: BindingError::F32(failure.error), plan: self,
                        backing: CoefficientBacking::F32(failure.all_coefficient_words),
                    }),
                }
            }
            (Storage::FiniteHalf { layout, cache }, CoefficientBacking::Half(words)) => {
                let bound = if let Some(values) = logical {
                    matrix_half::Matrix::from_row_major_bits(layout.rows(), layout.columns(),
                        values, cache, words)
                } else {
                    matrix_half::Matrix::from_complete_backing(layout.rows(), layout.columns(), cache, words)
                };
                match bound {
                    Ok(matrix) => Kernel::Half(matrix),
                    Err(failure) => return Err(BindingFailure {
                        error: BindingError::Half(failure.error), plan: self,
                        backing: CoefficientBacking::Half(failure.all_coefficient_words),
                    }),
                }
            }
            (Storage::RequiredSpecialized { .. }, backing) => return Err(BindingFailure {
                error: BindingError::RequiredSpecialized, plan: self, backing,
            }),
            (_, backing) => return Err(BindingFailure {
                error: BindingError::BackingFormat, plan: self, backing,
            }),
        };
        Ok(BoundMatrix { plan: self, kernel })
    }
}

impl BoundMatrix {
    pub fn plan(&self) -> Plan { self.plan }

    pub fn compiler_path(&self) -> matrix_f32::CompilerPath {
        match &self.kernel {
            Kernel::F32(matrix) => matrix.compiler_path(),
            Kernel::Half(matrix) => matrix.compiler_path(),
        }
    }

    pub fn visit_calls(&self, operation: matrix_f32::Operation,
        mut visitor: impl FnMut(matrix_f32::KernelCall))
    {
        match &self.kernel {
            Kernel::F32(matrix) => { for call in matrix.calls(operation) { visitor(call); } }
            Kernel::Half(matrix) => { for call in matrix.calls(operation) { visitor(call); } }
        }
    }

    pub fn execute(&self, input: &[f32], destination: &mut [u32], row_offset: usize,
        operation: matrix_f32::Operation) -> Result<(), ExecutionError>
    {
        match &self.kernel {
            Kernel::F32(matrix) => matrix.execute(input, destination, row_offset, operation)
                .map_err(ExecutionError::F32),
            Kernel::Half(matrix) => matrix.execute(input, destination, row_offset, operation)
                .map_err(ExecutionError::Half),
        }
    }

    pub fn execute_vectors(&self, input: &crate::vector_f32::Backing,
        destination: &mut crate::vector_f32::Backing, operation: matrix_f32::Operation)
        -> Result<(), ExecutionError>
    {
        let (rows, columns) = self.plan.dimensions();
        if input.logical_width() != columns || destination.logical_width() != rows {
            return Err(ExecutionError::Vector(crate::vector_f32::Error::Width));
        }
        let values = input.logical_bits().iter().map(|&v| f32::from_bits(v)).collect::<Vec<_>>();
        self.execute(&values, destination.full_bits_mut(), 0, operation)
    }

    pub fn execute_rows(&self, inputs: &[Vec<f32>], destinations: &mut [Vec<u32>],
        operation: matrix_f32::Operation) -> Result<(), BatchError>
    {
        match &self.kernel {
            Kernel::F32(matrix) => matrix.execute_rows(inputs, destinations, operation)
                .map_err(BatchError::F32),
            Kernel::Half(matrix) => matrix.execute_rows(inputs, destinations, operation)
                .map_err(BatchError::Half),
        }
    }

    pub fn execute_read_rows(&self, inputs: &dyn crate::logical_read_rows::ReadRows,
        destinations: &mut [Vec<u32>], operation: matrix_f32::Operation) -> Result<(), BatchError>
    {
        match &self.kernel {
            Kernel::F32(matrix) => matrix.execute_read_rows(inputs, destinations, operation)
                .map_err(BatchError::F32),
            Kernel::Half(matrix) => matrix.execute_read_rows(inputs, destinations, operation)
                .map_err(BatchError::Half),
        }
    }

    pub fn execute_read_backings(&self, inputs: &dyn crate::logical_read_rows::ReadRows,
        destinations: &mut [crate::vector_f32::Backing], operation: matrix_f32::Operation)
        -> Result<(), BatchError>
    {
        match &self.kernel {
            Kernel::F32(matrix) => matrix.execute_read_backings(inputs, destinations, operation)
                .map_err(BatchError::F32),
            Kernel::Half(matrix) => matrix.execute_read_backings(inputs, destinations, operation)
                .map_err(BatchError::Half),
        }
    }

    pub fn into_backing(self) -> (Plan, CoefficientBacking) {
        let backing = match self.kernel {
            Kernel::F32(matrix) => CoefficientBacking::F32(matrix.into_coefficient_words()),
            Kernel::Half(matrix) => CoefficientBacking::Half(matrix.into_coefficient_words()),
        };
        (self.plan, backing)
    }
}
