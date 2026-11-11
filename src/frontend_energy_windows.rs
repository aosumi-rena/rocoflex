use std::{collections::TryReserveError, convert::Infallible, vec::IntoIter};
use super::energy_reduction_math::{self as reduction, SourceEnergyMath, SourceEnergyFailure, SourceEnergyProgress};
use crate::frontend_energy_window_math as window_math;
use crate::{frontend_gain_log_rows as log_rows, frontend_gain_statistics as statistics,
    frontend_gain_log_suffix_math as suffix};

pub struct SourceEnergyWindow { pub samples: Vec<f32>, pub seed_bits: u32 }
pub struct SourceEnergyWindows { pub windows: Vec<SourceEnergyWindow> }

pub struct SourceWindowStorage { pub windows: Vec<Vec<f32>> }
pub struct SourceEnergyRow {
    window: Vec<f32>, seed_bits: u32, energy_bits: u32, progress: SourceEnergyProgress,
}
impl SourceEnergyRow {
    pub fn window(&self) -> &[f32] { &self.window }
    pub fn seed_bits(&self) -> u32 { self.seed_bits }
    pub fn energy_bits(&self) -> u32 { self.energy_bits }
    pub fn progress(&self) -> &SourceEnergyProgress { &self.progress }
    pub fn into_parts(self) -> (Vec<f32>, u32, u32, SourceEnergyProgress) {
        (self.window, self.seed_bits, self.energy_bits, self.progress)
    }
}
pub struct SourceEnergyRowsCompleted { math: SourceEnergyMath, history: Vec<SourceEnergyRow> }
impl SourceEnergyRowsCompleted {
    pub fn math(&self) -> &SourceEnergyMath { &self.math }
    pub fn history(&self) -> &[SourceEnergyRow] { &self.history }
    pub fn into_parts(self) -> (SourceEnergyMath, Vec<SourceEnergyRow>) { (self.math, self.history) }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceEnergyStorageSlot { History, Energy }
pub enum SourceEnergyRowsCause {
    Count { actual: usize, expected: usize },
    Storage { slot: SourceEnergyStorageSlot, error: TryReserveError }, Reduction,
}
enum MathOwner { Ready(SourceEnergyMath), Failed(SourceEnergyFailure) }
pub struct SourceEnergyRowsFailure {
    row: usize, pending: IntoIter<SourceEnergyWindow>, energy: Vec<f32>, history: Vec<SourceEnergyRow>,
    owner: MathOwner, cause: SourceEnergyRowsCause,
}
impl SourceEnergyRowsFailure {
    pub fn row(&self) -> usize { self.row }

    pub fn pending(&self) -> &[SourceEnergyWindow] { self.pending.as_slice() }
    pub fn energy(&self) -> &[f32] { &self.energy }
    pub fn history(&self) -> &[SourceEnergyRow] { &self.history }
    pub fn math(&self) -> &SourceEnergyMath {
        match &self.owner { MathOwner::Ready(math) => math, MathOwner::Failed(failure) => failure.math() }
    }
    pub fn current_failure(&self) -> Option<&SourceEnergyFailure> {
        match &self.owner { MathOwner::Ready(_) => None, MathOwner::Failed(failure) => Some(failure) }
    }
    pub fn cause(&self) -> &SourceEnergyRowsCause { &self.cause }
}
pub(super) fn reduce_windows(request: SourceEnergyWindows, expected: usize, math: SourceEnergyMath)
    -> Result<(Vec<f32>, SourceEnergyRowsCompleted), SourceEnergyRowsFailure>
{
    match reduce_with_choice(request, expected, math, Supplied) {
        Ok((energy, completed)) => Ok((energy, SourceEnergyRowsCompleted {
            math: completed.math, history: completed.history })),
        Err(failure) => {
            let BatchFailure { row, pending, storage: AppendEnergy { energy }, history, owner } = failure;
            let (owner, cause) = match owner {
                BatchOwner::Ready { math, cause: ReadyCause::Rows(cause), .. } => (MathOwner::Ready(math), cause),
                BatchOwner::Reduction { failure, .. } => (MathOwner::Failed(failure), SourceEnergyRowsCause::Reduction),
                BatchOwner::Window { failure, .. } => match failure {},
                BatchOwner::Continuation { failure, .. } => match failure {},

                BatchOwner::Ready { .. } => unreachable!(),
            };
            Err(SourceEnergyRowsFailure { row, pending, energy, history, owner, cause })
        }
    }
}

pub struct SourceWindowReceipt {
    request: window_math::SourceWindowRequest,
    progress: window_math::SourceWindowProgress,
    reports: [Option<window_math::SourceWindowAttempt>; 4],
}
impl SourceWindowReceipt {
    pub fn request(&self) -> window_math::SourceWindowRequest { self.request }
    pub fn progress(&self) -> &window_math::SourceWindowProgress { &self.progress }
    pub fn reports(&self) -> &[Option<window_math::SourceWindowAttempt>; 4] { &self.reports }
    pub fn into_parts(self) -> (window_math::SourceWindowRequest, window_math::SourceWindowProgress,
        [Option<window_math::SourceWindowAttempt>; 4]) { (self.request, self.progress, self.reports) }
}
pub struct SourceWindowEnergyRow { reduction: SourceEnergyRow, window: SourceWindowReceipt }
impl SourceWindowEnergyRow {
    pub fn reduction(&self) -> &SourceEnergyRow { &self.reduction }
    pub fn window_receipt(&self) -> &SourceWindowReceipt { &self.window }
    pub fn into_parts(self) -> (SourceEnergyRow, SourceWindowReceipt) { (self.reduction, self.window) }
}
pub struct SourceWindowEnergyRowsCompleted {
    window_math: window_math::SourceWindowMath, math: SourceEnergyMath, history: Vec<SourceWindowEnergyRow>,
}
impl SourceWindowEnergyRowsCompleted {
    pub fn window_math(&self) -> &window_math::SourceWindowMath { &self.window_math }
    pub fn math(&self) -> &SourceEnergyMath { &self.math }
    pub fn history(&self) -> &[SourceWindowEnergyRow] { &self.history }
    pub fn into_parts(self) -> (window_math::SourceWindowMath, SourceEnergyMath, Vec<SourceWindowEnergyRow>) {
        (self.window_math, self.math, self.history)
    }
}
pub enum SourceWindowEnergyRowsError<'a> {
    Rows(&'a SourceEnergyRowsCause), Ordinal { last: usize },
    SourceCount { requested: usize, backing: usize },
    Window(&'a window_math::SourceWindowFailure), Reduction(&'a SourceEnergyFailure),
}
pub struct SourceWindowEnergyRowsFailure { failure: BatchFailure<Constructed> }
impl SourceWindowEnergyRowsFailure {
    pub fn row(&self) -> usize { self.failure.row }
    pub fn pending(&self) -> &[SourceEnergyWindow] { self.failure.pending.as_slice() }
    pub fn energy(&self) -> &[f32] { &self.failure.storage.energy }
    pub fn history(&self) -> &[SourceWindowEnergyRow] { &self.failure.history }
    pub fn source(&self) -> &[f32] {
        match &self.failure.owner {
            BatchOwner::Ready { choice, .. } | BatchOwner::Reduction { choice, .. } | BatchOwner::Continuation { choice, .. } => &choice.source,
            BatchOwner::Window { failure, .. } => failure.failure.source(),
        }
    }
    pub fn window_math(&self) -> &window_math::SourceWindowMath {
        match &self.failure.owner {
            BatchOwner::Ready { choice, .. } | BatchOwner::Reduction { choice, .. } | BatchOwner::Continuation { choice, .. } => &choice.math,
            BatchOwner::Window { failure, .. } => failure.failure.math(),
        }
    }
    pub fn math(&self) -> &SourceEnergyMath {
        match &self.failure.owner {
            BatchOwner::Ready { math, .. } | BatchOwner::Window { math, .. } | BatchOwner::Continuation { math, .. } => math,
            BatchOwner::Reduction { failure, .. } => failure.math(),
        }
    }
    pub fn current_window_failure(&self) -> Option<&window_math::SourceWindowFailure> {
        match &self.failure.owner { BatchOwner::Window { failure, .. } => Some(&failure.failure), _ => None }
    }
    pub fn current_window_receipt(&self) -> Option<&SourceWindowReceipt> {
        match &self.failure.owner { BatchOwner::Reduction { receipt, .. } => Some(receipt), _ => None }
    }
    pub fn current_energy_failure(&self) -> Option<&SourceEnergyFailure> {
        match &self.failure.owner { BatchOwner::Reduction { failure, .. } => Some(failure), _ => None }
    }
    pub fn current_seed_bits(&self) -> Option<u32> {
        match &self.failure.owner {
            BatchOwner::Ready { .. } => self.failure.pending.as_slice().first().map(|row| row.seed_bits),
            BatchOwner::Window { failure, .. } => Some(failure.seed_bits),
            BatchOwner::Reduction { failure, .. } => Some(failure.seed_bits()),
            BatchOwner::Continuation { failure, .. } => match *failure {},
        }
    }
    pub fn error(&self) -> SourceWindowEnergyRowsError<'_> {
        match &self.failure.owner {
            BatchOwner::Ready { cause: ReadyCause::Rows(cause), .. } => SourceWindowEnergyRowsError::Rows(cause),
            BatchOwner::Ready { cause: ReadyCause::Ordinal { last }, .. } => SourceWindowEnergyRowsError::Ordinal { last: *last },
            BatchOwner::Ready { cause: ReadyCause::SourceCount { requested, backing }, .. } =>
                SourceWindowEnergyRowsError::SourceCount { requested: *requested, backing: *backing },
            BatchOwner::Ready { cause: ReadyCause::LogStorage(_), .. } => unreachable!(),
            BatchOwner::Window { failure, .. } => SourceWindowEnergyRowsError::Window(&failure.failure),
            BatchOwner::Reduction { failure, .. } => SourceWindowEnergyRowsError::Reduction(failure),
            BatchOwner::Continuation { failure, .. } => match *failure {},
        }
    }
}
pub(super) struct ConstructedRows {
    pub(super) energy: Vec<f32>, pub(super) source: Vec<f32>, pub(super) rows: SourceWindowEnergyRowsCompleted,
}
pub(super) fn construct_and_reduce_windows(source: Vec<f32>, source_count: usize,
    request: SourceEnergyWindows, expected: usize, window_math: window_math::SourceWindowMath, math: SourceEnergyMath)
    -> Result<ConstructedRows, SourceWindowEnergyRowsFailure>
{
    match reduce_with_choice(request, expected, math, Constructed { source, source_count, math: window_math }) {
        Ok((energy, completed)) => Ok(ConstructedRows { energy, source: completed.choice.source,
            rows: SourceWindowEnergyRowsCompleted { window_math: completed.choice.math,
                math: completed.math, history: completed.history } }),
        Err(failure) => Err(SourceWindowEnergyRowsFailure { failure }),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEnergyArrayInitializationStage { Extent, Energy, Metric }
pub enum SourceEnergyArrayInitializationCause {

    Extent { count: usize }, Storage(TryReserveError),
}
pub struct SourceEnergyArrayInitializationFailure {
    count: usize, stage: SourceEnergyArrayInitializationStage, cause: SourceEnergyArrayInitializationCause,
    energy: Option<Vec<f32>>, metric: Option<Vec<f32>>,
}
impl SourceEnergyArrayInitializationFailure {
    pub fn count(&self) -> usize { self.count }
    pub fn stage(&self) -> SourceEnergyArrayInitializationStage { self.stage }
    pub fn cause(&self) -> &SourceEnergyArrayInitializationCause { &self.cause }
    pub fn energy(&self) -> Option<&[f32]> { self.energy.as_deref() }
    pub fn metric(&self) -> Option<&[f32]> { self.metric.as_deref() }
}
fn initialize_energy_arrays(count: usize) -> Result<InitializedEnergy, SourceEnergyArrayInitializationFailure> {
    if count == 0 || count.checked_mul(std::mem::size_of::<f32>()).filter(|bytes| *bytes <= isize::MAX as usize).is_none() {
        return Err(SourceEnergyArrayInitializationFailure { count, stage: SourceEnergyArrayInitializationStage::Extent,
            cause: SourceEnergyArrayInitializationCause::Extent { count }, energy: None, metric: None });
    }
    let mut energy = Vec::new();
    if let Err(error) = energy.try_reserve_exact(count) {
        return Err(SourceEnergyArrayInitializationFailure { count, stage: SourceEnergyArrayInitializationStage::Energy,
            cause: SourceEnergyArrayInitializationCause::Storage(error), energy: Some(energy), metric: None });
    }

    energy.resize(count, f32::from_bits(0));
    let mut metric = Vec::new();
    if let Err(error) = metric.try_reserve_exact(count) {
        return Err(SourceEnergyArrayInitializationFailure { count, stage: SourceEnergyArrayInitializationStage::Metric,
            cause: SourceEnergyArrayInitializationCause::Storage(error), energy: Some(energy), metric: Some(metric) });
    }
    metric.resize(count, f32::from_bits(0));
    Ok(InitializedEnergy { energy, metric })
}
pub(super) struct InitializedRowLogs<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub(super) request: statistics::SourceGainStatisticsRequest,
    pub(super) gated_math: crate::frontend_gated_energy_math::SourceGatedEnergyMath,
    pub(super) math: crate::source_scalar::SourceF64ValueMath, pub(super) services: L,
    pub(super) history: Vec<log_rows::SourceGainLogRowReceipt<L>>,
}
struct InitializedRowLogFailure<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    request: statistics::SourceGainStatisticsRequest, gated_math: crate::frontend_gated_energy_math::SourceGatedEnergyMath,
    history: Vec<log_rows::SourceGainLogRowReceipt<L>>, failure: log_rows::SourceGainLogRowFailure<L>,
}
impl<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> RowContinuation<InitializedEnergy> for InitializedRowLogs<L> {
    type Error = InitializedRowLogFailure<L>;
    fn reserve(&mut self) -> Result<(), TryReserveError> { self.history.try_reserve(1) }
    fn apply(self, row: usize, storage: &mut InitializedEnergy) -> Result<Self, Self::Error> {
        let Self { request, gated_math, math, services, mut history } = self;
        let selected = log_rows::SourceGainRowLogRequest { row_dword: row as u32, pc: 0x55298c,
            input_energy_bits: storage.energy[row].to_bits() };
        match log_rows::run_gain_log_row(selected, suffix::RowConstants {
            xmm10_multiply_bits: request.multiply_bits, xmm11_add_bits: request.add_bits }, &mut storage.metric[row], math, services) {
            Ok(completed) => {
                let log_rows::SourceGainLogRowParts { math, services, receipt } = completed.into_parts();

                history.push(receipt);
                Ok(Self { request, gated_math, math, services, history })
            }
            Err(failure) => Err(InitializedRowLogFailure { request, gated_math, history, failure }),
        }
    }
}
enum InitializedRowsOwner<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    Initialization { source: Vec<f32>, source_count: usize, request: SourceWindowStorage,
        window_math: window_math::SourceWindowMath, math: SourceEnergyMath, logs: InitializedRowLogs<L>, failure: SourceEnergyArrayInitializationFailure },
    Batch(BatchFailure<Constructed, InitializedEnergy, InitializedRowLogs<L>>),
}
pub struct SourceInitializedWindowEnergyRowsFailure<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> { owner: InitializedRowsOwner<L> }
pub enum SourceInitializedWindowEnergyRowsError<'a, L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    Initialization(&'a SourceEnergyArrayInitializationFailure), Rows(&'a SourceEnergyRowsCause), Ordinal { last: usize },
    SourceCount { requested: usize, backing: usize },
    Window(&'a window_math::SourceWindowFailure), Reduction(&'a SourceEnergyFailure),
    LogHistory(&'a TryReserveError), Log(&'a log_rows::SourceGainLogRowFailure<L>),
}
impl<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> SourceInitializedWindowEnergyRowsFailure<L> {
    pub fn row(&self) -> usize { match &self.owner { InitializedRowsOwner::Initialization { .. } => 0,
        InitializedRowsOwner::Batch(failure) => failure.row } }
    pub fn pending(&self) -> &[Vec<f32>] { match &self.owner {
        InitializedRowsOwner::Initialization { request, .. } => &request.windows,
        InitializedRowsOwner::Batch(failure) => failure.pending.as_slice(),
    } }
    pub fn energy(&self) -> Option<&[f32]> { match &self.owner {
        InitializedRowsOwner::Initialization { failure, .. } => failure.energy(),
        InitializedRowsOwner::Batch(failure) => Some(&failure.storage.energy),
    } }
    pub fn metric(&self) -> Option<&[f32]> { match &self.owner {
        InitializedRowsOwner::Initialization { failure, .. } => failure.metric(),
        InitializedRowsOwner::Batch(failure) => Some(&failure.storage.metric),
    } }
    pub fn history(&self) -> Option<&[SourceWindowEnergyRow]> { match &self.owner {
        InitializedRowsOwner::Initialization { .. } => None,
        InitializedRowsOwner::Batch(failure) => Some(&failure.history),
    } }
    pub fn source(&self) -> &[f32] { match &self.owner {
        InitializedRowsOwner::Initialization { source, .. } => source,
        InitializedRowsOwner::Batch(failure) => match &failure.owner {
            BatchOwner::Ready { choice, .. } | BatchOwner::Reduction { choice, .. } | BatchOwner::Continuation { choice, .. } => &choice.source,
            BatchOwner::Window { failure, .. } => failure.failure.source(),
        },
    } }
    pub fn source_count(&self) -> usize { match &self.owner {
        InitializedRowsOwner::Initialization { source_count, .. } => *source_count,
        InitializedRowsOwner::Batch(failure) => match &failure.owner {
            BatchOwner::Ready { choice, .. } | BatchOwner::Reduction { choice, .. } | BatchOwner::Continuation { choice, .. } => choice.source_count,
            BatchOwner::Window { failure, .. } => failure.failure.request().source_count,
        },
    } }
    pub fn window_math(&self) -> &window_math::SourceWindowMath { match &self.owner {
        InitializedRowsOwner::Initialization { window_math, .. } => window_math,
        InitializedRowsOwner::Batch(failure) => match &failure.owner {
            BatchOwner::Ready { choice, .. } | BatchOwner::Reduction { choice, .. } | BatchOwner::Continuation { choice, .. } => &choice.math,
            BatchOwner::Window { failure, .. } => failure.failure.math(),
        },
    } }
    pub fn math(&self) -> &SourceEnergyMath { match &self.owner {
        InitializedRowsOwner::Initialization { math, .. } => math,
        InitializedRowsOwner::Batch(failure) => match &failure.owner {
            BatchOwner::Ready { math, .. } | BatchOwner::Window { math, .. } | BatchOwner::Continuation { math, .. } => math,
            BatchOwner::Reduction { failure, .. } => failure.math(),
        },
    } }
    pub fn current_window_failure(&self) -> Option<&window_math::SourceWindowFailure> { match &self.owner {
        InitializedRowsOwner::Batch(BatchFailure { owner: BatchOwner::Window { failure, .. }, .. }) => Some(&failure.failure), _ => None,
    } }
    pub fn current_window_receipt(&self) -> Option<&SourceWindowReceipt> { match &self.owner {
        InitializedRowsOwner::Batch(BatchFailure { owner: BatchOwner::Reduction { receipt, .. }, .. }) => Some(receipt),
        InitializedRowsOwner::Batch(BatchFailure { owner: BatchOwner::Continuation { .. }, history, .. }) =>
            history.last().map(|row| row.window_receipt()), _ => None,
    } }
    pub fn current_energy_failure(&self) -> Option<&SourceEnergyFailure> { match &self.owner {
        InitializedRowsOwner::Batch(BatchFailure { owner: BatchOwner::Reduction { failure, .. }, .. }) => Some(failure), _ => None,
    } }

    pub fn current_seed_bits(&self) -> Option<u32> { match &self.owner {
        InitializedRowsOwner::Batch(BatchFailure { owner: BatchOwner::Reduction { failure, .. }, .. }) => Some(failure.seed_bits()),
        InitializedRowsOwner::Batch(BatchFailure { owner: BatchOwner::Continuation { .. }, history, .. }) =>
            history.last().map(|row| row.reduction().seed_bits()), _ => None,
    } }
    fn logs(&self) -> Result<&InitializedRowLogs<L>, &InitializedRowLogFailure<L>> { match &self.owner {
        InitializedRowsOwner::Initialization { logs, .. } => Ok(logs),
        InitializedRowsOwner::Batch(failure) => match &failure.owner {
            BatchOwner::Ready { continuation, .. } | BatchOwner::Window { continuation, .. }
            | BatchOwner::Reduction { continuation, .. } => Ok(continuation),
            BatchOwner::Continuation { failure, .. } => Err(failure),
        },
    } }
    pub fn log_request(&self) -> statistics::SourceGainStatisticsRequest { match self.logs() {
        Ok(logs) => logs.request, Err(failure) => failure.request,
    } }
    pub fn gated_math(&self) -> &crate::frontend_gated_energy_math::SourceGatedEnergyMath { match self.logs() {
        Ok(logs) => &logs.gated_math, Err(failure) => &failure.gated_math,
    } }
    pub fn suffix_math(&self) -> &crate::source_scalar::SourceF64ValueMath { match self.logs() {
        Ok(logs) => &logs.math, Err(failure) => failure.failure.math(),
    } }
    pub fn log_services(&self) -> &L { match self.logs() {
        Ok(logs) => &logs.services, Err(failure) => failure.failure.services(),
    } }
    pub fn log_history(&self) -> &[log_rows::SourceGainLogRowReceipt<L>] { match self.logs() {
        Ok(logs) => &logs.history, Err(failure) => &failure.history,
    } }
    pub fn current_log_failure(&self) -> Option<&log_rows::SourceGainLogRowFailure<L>> { self.logs().err().map(|failure| &failure.failure) }
    pub fn initialization_failure(&self) -> Option<&SourceEnergyArrayInitializationFailure> { match &self.owner {
        InitializedRowsOwner::Initialization { failure, .. } => Some(failure), _ => None,
    } }
    pub fn error(&self) -> SourceInitializedWindowEnergyRowsError<'_, L> { match &self.owner {
        InitializedRowsOwner::Initialization { failure, .. } => SourceInitializedWindowEnergyRowsError::Initialization(failure),
        InitializedRowsOwner::Batch(failure) => match &failure.owner {
            BatchOwner::Ready { cause: ReadyCause::Rows(cause), .. } => SourceInitializedWindowEnergyRowsError::Rows(cause),
            BatchOwner::Ready { cause: ReadyCause::Ordinal { last }, .. } => SourceInitializedWindowEnergyRowsError::Ordinal { last: *last },
            BatchOwner::Ready { cause: ReadyCause::SourceCount { requested, backing }, .. } =>
                SourceInitializedWindowEnergyRowsError::SourceCount { requested: *requested, backing: *backing },
            BatchOwner::Ready { cause: ReadyCause::LogStorage(error), .. } => SourceInitializedWindowEnergyRowsError::LogHistory(error),
            BatchOwner::Window { failure, .. } => SourceInitializedWindowEnergyRowsError::Window(&failure.failure),
            BatchOwner::Reduction { failure, .. } => SourceInitializedWindowEnergyRowsError::Reduction(failure),
            BatchOwner::Continuation { failure, .. } => SourceInitializedWindowEnergyRowsError::Log(&failure.failure),
        },
    } }
}
pub(super) struct InitializedConstructedRows<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    pub(super) energy: Vec<f32>, pub(super) metric: Vec<f32>, pub(super) source: Vec<f32>,
    pub(super) rows: SourceWindowEnergyRowsCompleted, pub(super) logs: InitializedRowLogs<L>,
}
pub(super) fn initialize_construct_and_reduce_windows<L: log_rows::SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices>(source: Vec<f32>, source_count: usize,
    request: SourceWindowStorage, expected: usize, window_math: window_math::SourceWindowMath, math: SourceEnergyMath, logs: InitializedRowLogs<L>)
    -> Result<InitializedConstructedRows<L>, SourceInitializedWindowEnergyRowsFailure<L>>
{
    let storage = match initialize_energy_arrays(expected) {
        Ok(storage) => storage,
        Err(failure) => return Err(SourceInitializedWindowEnergyRowsFailure { owner: InitializedRowsOwner::Initialization {
            source, source_count, request, window_math, math, logs, failure } }),
    };
    match reduce_with_storage(request.windows.into_iter(), expected, math, Constructed { source, source_count, math: window_math }, storage, logs) {
        Ok((storage, completed)) => Ok(InitializedConstructedRows { energy: storage.energy, metric: storage.metric,
            source: completed.choice.source, rows: SourceWindowEnergyRowsCompleted { window_math: completed.choice.math,
                math: completed.math, history: completed.history }, logs: completed.continuation }),
        Err(failure) => Err(SourceInitializedWindowEnergyRowsFailure { owner: InitializedRowsOwner::Batch(failure) }),
    }
}

trait WindowRequest {
    type Seed;
    fn into_storage(self) -> (Vec<f32>, Self::Seed);
}
impl WindowRequest for SourceEnergyWindow {
    type Seed = u32;
    fn into_storage(self) -> (Vec<f32>, u32) { (self.samples, self.seed_bits) }
}
impl WindowRequest for Vec<f32> {
    type Seed = ();
    fn into_storage(self) -> (Vec<f32>, ()) { (self, ()) }
}
trait WindowChoice<R: WindowRequest = SourceEnergyWindow>: Sized {
    type Row;
    type Receipt;
    type Error;
    fn admit(&self, count: usize) -> Option<ReadyCause>;
    fn prepare(self, ordinal: usize, request: R)
        -> Result<(Vec<f32>, R::Seed, Self, Self::Receipt), Self::Error>;
    fn make_row(reduction: SourceEnergyRow, receipt: Self::Receipt) -> Self::Row;
}
struct Supplied;
impl WindowChoice for Supplied {
    type Row = SourceEnergyRow;
    type Receipt = ();
    type Error = Infallible;
    fn admit(&self, _: usize) -> Option<ReadyCause> { None }
    fn prepare(self, _: usize, request: SourceEnergyWindow) -> Result<(Vec<f32>, u32, Self, ()), Infallible> {
        Ok((request.samples, request.seed_bits, self, ()))
    }
    fn make_row(reduction: SourceEnergyRow, (): ()) -> Self::Row { reduction }
}
struct Constructed { source: Vec<f32>, source_count: usize, math: window_math::SourceWindowMath }
struct WindowRefusal<S> { failure: window_math::SourceWindowFailure, seed_bits: S }
impl<R: WindowRequest> WindowChoice<R> for Constructed {
    type Row = SourceWindowEnergyRow;
    type Receipt = SourceWindowReceipt;
    type Error = WindowRefusal<R::Seed>;
    fn admit(&self, count: usize) -> Option<ReadyCause> {
        if let Some(last) = count.checked_sub(1) {
            if u32::try_from(last).is_err() { return Some(ReadyCause::Ordinal { last }); }
        }
        if self.source_count > self.source.len() || self.source_count > i32::MAX as usize {
            return Some(ReadyCause::SourceCount { requested: self.source_count, backing: self.source.len() });
        }
        None
    }
    fn prepare(self, ordinal: usize, request: R)
        -> Result<(Vec<f32>, R::Seed, Self, SourceWindowReceipt), WindowRefusal<R::Seed>>
    {
        let Self { source, source_count, math } = self;
        let (storage, seed_bits) = request.into_storage();

        let selected = window_math::SourceWindowRequest { row_dword: ordinal as u32, source_count };
        let completed = match window_math::prepare_source_window(source, selected, storage, math) {
            Ok(completed) => completed,
            Err(failure) => return Err(WindowRefusal { failure, seed_bits }),
        };
        let window_math::SourceWindowParts { source, window, request, math, progress } = completed.into_parts();

        let reports = *math.reports();
        Ok((window, seed_bits, Self { source, source_count, math }, SourceWindowReceipt { request, progress, reports }))
    }
    fn make_row(reduction: SourceEnergyRow, window: SourceWindowReceipt) -> Self::Row {
        SourceWindowEnergyRow { reduction, window }
    }
}
enum ReadyCause {
    Rows(SourceEnergyRowsCause), Ordinal { last: usize }, SourceCount { requested: usize, backing: usize }, LogStorage(TryReserveError),
}
enum BatchOwner<C: WindowChoice<B::Request>, B: EnergyStorage = AppendEnergy, T: RowContinuation<B> = NoRowLog> {
    Ready { choice: C, math: SourceEnergyMath, continuation: T, cause: ReadyCause },
    Window { failure: C::Error, math: SourceEnergyMath, continuation: T },
    Reduction { choice: C, receipt: C::Receipt, failure: SourceEnergyFailure, continuation: T },
    Continuation { choice: C, math: SourceEnergyMath, failure: T::Error },
}
trait EnergyStorage {
    type Request: WindowRequest;
    fn reserve(&mut self) -> Result<(), TryReserveError>;
    fn seed(&self, row: usize, supplied: &<Self::Request as WindowRequest>::Seed) -> u32;
    fn commit<R>(&mut self, row: usize, bits: u32, history: &mut Vec<R>, record: R);
}
struct AppendEnergy { energy: Vec<f32> }
impl EnergyStorage for AppendEnergy {
    type Request = SourceEnergyWindow;
    fn reserve(&mut self) -> Result<(), TryReserveError> { self.energy.try_reserve(1) }
    fn seed(&self, _: usize, supplied: &u32) -> u32 { *supplied }
    fn commit<R>(&mut self, _: usize, bits: u32, history: &mut Vec<R>, record: R) {

        history.push(record); self.energy.push(f32::from_bits(bits));
    }
}
struct InitializedEnergy { energy: Vec<f32>, metric: Vec<f32> }
impl EnergyStorage for InitializedEnergy {
    type Request = Vec<f32>;

    fn reserve(&mut self) -> Result<(), TryReserveError> { Ok(()) }
    fn seed(&self, row: usize, _: &()) -> u32 { self.energy[row].to_bits() }
    fn commit<R>(&mut self, row: usize, bits: u32, history: &mut Vec<R>, record: R) {
        self.energy[row] = f32::from_bits(bits); history.push(record);
    }
}
trait RowContinuation<B: EnergyStorage>: Sized {
    type Error;
    fn reserve(&mut self) -> Result<(), TryReserveError>;
    fn apply(self, row: usize, storage: &mut B) -> Result<Self, Self::Error>;
}
struct NoRowLog;
impl<B: EnergyStorage> RowContinuation<B> for NoRowLog {
    type Error = Infallible;
    fn reserve(&mut self) -> Result<(), TryReserveError> { Ok(()) }
    fn apply(self, _: usize, _: &mut B) -> Result<Self, Infallible> { Ok(self) }
}
struct BatchFailure<C: WindowChoice<B::Request>, B: EnergyStorage = AppendEnergy, T: RowContinuation<B> = NoRowLog> {
    row: usize, pending: IntoIter<B::Request>, storage: B, history: Vec<C::Row>, owner: BatchOwner<C, B, T>,
}
struct BatchCompleted<C: WindowChoice<R>, R: WindowRequest = SourceEnergyWindow, T = NoRowLog> {
    choice: C, math: SourceEnergyMath, history: Vec<C::Row>, continuation: T,
}
fn reduce_with_choice<C: WindowChoice>(request: SourceEnergyWindows, expected: usize,
    math: SourceEnergyMath, choice: C) -> Result<(Vec<f32>, BatchCompleted<C>), BatchFailure<C>>
{
    reduce_with_storage(request.windows.into_iter(), expected, math, choice, AppendEnergy { energy: Vec::new() }, NoRowLog)
        .map(|(storage, completed)| (storage.energy, completed))
}
fn reduce_with_storage<C: WindowChoice<B::Request>, B: EnergyStorage, T: RowContinuation<B>>(mut pending: IntoIter<B::Request>, expected: usize,
    mut math: SourceEnergyMath, mut choice: C, mut storage: B, mut continuation: T)
    -> Result<(B, BatchCompleted<C, B::Request, T>), BatchFailure<C, B, T>>
{
    let mut history = Vec::new(); let mut row = 0;
    if pending.len() != expected {
        let actual = pending.len();
        return Err(BatchFailure { row, pending, storage, history, owner: BatchOwner::Ready { choice, math, continuation,
            cause: ReadyCause::Rows(SourceEnergyRowsCause::Count { actual, expected }) } });
    }
    if let Some(cause) = choice.admit(expected) {
        return Err(BatchFailure { row, pending, storage, history, owner: BatchOwner::Ready { choice, math, continuation, cause } });
    }
    while pending.len() != 0 {

        if let Err(error) = history.try_reserve(1) {
            return Err(BatchFailure { row, pending, storage, history, owner: BatchOwner::Ready { choice, math, continuation,
                cause: ReadyCause::Rows(SourceEnergyRowsCause::Storage { slot: SourceEnergyStorageSlot::History, error }) } });
        }
        if let Err(error) = storage.reserve() {
            return Err(BatchFailure { row, pending, storage, history, owner: BatchOwner::Ready { choice, math, continuation,
                cause: ReadyCause::Rows(SourceEnergyRowsCause::Storage { slot: SourceEnergyStorageSlot::Energy, error }) } });
        }
        if let Err(error) = continuation.reserve() {
            return Err(BatchFailure { row, pending, storage, history, owner: BatchOwner::Ready {
                choice, math, continuation, cause: ReadyCause::LogStorage(error) } });
        }
        let current = match pending.next() { Some(current) => current, None => unreachable!() };
        let (window, supplied_seed, next_choice, receipt) = match choice.prepare(row, current) {
            Ok(prepared) => prepared,
            Err(failure) => return Err(BatchFailure { row, pending, storage, history,
                owner: BatchOwner::Window { failure, math, continuation } }),
        };

        let seed_bits = storage.seed(row, &supplied_seed);
        let completed = match reduction::reduce_source_window(window, seed_bits, math) {
            Ok(completed) => completed,
            Err(failure) => return Err(BatchFailure { row, pending, storage, history,
                owner: BatchOwner::Reduction { choice: next_choice, receipt, failure, continuation } }),
        };
        let (window, energy_bits, next_math, progress) = completed.into_parts();
        let record = <C as WindowChoice<B::Request>>::make_row(SourceEnergyRow { window, seed_bits, energy_bits, progress }, receipt);
        storage.commit(row, energy_bits, &mut history, record);
        continuation = match continuation.apply(row, &mut storage) {
            Ok(next) => next,
            Err(failure) => return Err(BatchFailure { row, pending, storage, history,
                owner: BatchOwner::Continuation { choice: next_choice, math: next_math, failure } }),
        };
        math = next_math; choice = next_choice; row += 1;
    }
    Ok((storage, BatchCompleted { choice, math, history, continuation }))
}
