use std::collections::{TryReserveError, VecDeque};
use crate::{row_pool_values::{pool_put_plan, PoolPutPlan, RowGrowthError, SourceRowGrowthPlan},
    streaming_projection::{Pool, Row, RowRefusal},
    streaming_value_services::{RequiredRows, ValuePool}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retirement { ArchiveOwnedValues }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy { pub allocation_word: u32, pub retirement: Retirement }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation { Create, Grow(Pool), ClearHistory, Construct(Pool), RetirePool(Pool), RetireRow(Pool) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopKind { Flags, Width, Growth, GrowthState, CounterOverflow, Capacity }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoppedAt { pub operation: Operation, pub kind: StopKind }
pub enum Error {
    PriorFailure, Flags { first: bool, second: bool }, Width(usize),
    Growth(RowGrowthError), GrowthState, CounterOverflow,
    Capacity { operation: Operation, cause: TryReserveError },
}
enum RetiredOwners {
    Row { pool: Pool, row: Row }, History(VecDeque<Option<Row>>),
    Pool { pool: Pool, state: ValuePool },
}
pub struct Retired { owners: RetiredOwners }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetiredKind { Row(Pool), History, Pool(Pool) }
impl Retired {
    pub fn kind(&self) -> RetiredKind {
        match &self.owners { RetiredOwners::Row { pool, .. } => RetiredKind::Row(*pool),
            RetiredOwners::History(_) => RetiredKind::History,
            RetiredOwners::Pool { pool, .. } => RetiredKind::Pool(*pool) }
    }
    pub fn row(&self) -> Option<&[f32]> {
        match &self.owners { RetiredOwners::Row { row, .. } => Some(row), _ => None }
    }
    pub fn history(&self) -> Option<&VecDeque<Option<Row>>> {
        match &self.owners { RetiredOwners::History(history) => Some(history), _ => None }
    }
    pub fn pool(&self) -> Option<&ValuePool> {
        match &self.owners { RetiredOwners::Pool { state, .. } => Some(state), _ => None }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress { pub created_rows: usize, pub retired_events: usize, pub stopped: Option<StoppedAt> }
pub struct ReadyParts { pub policy: Policy, pub created_rows: usize, pub retired: Vec<Retired> }
pub struct ValueRows {
    policy: Policy, created_rows: usize, retired: Vec<Retired>, stopped: Option<StoppedAt>,
}
impl ValueRows {

    pub fn from_explicit_policy(policy: Policy) -> Self {
        Self { policy, created_rows: 0, retired: Vec::new(), stopped: None }
    }
    pub fn policy(&self) -> Policy { self.policy }
    pub fn progress(&self) -> Progress {
        Progress { created_rows: self.created_rows, retired_events: self.retired.len(), stopped: self.stopped }
    }
    pub fn retired(&self) -> &[Retired] { &self.retired }
    pub fn into_ready_parts(self) -> Result<ReadyParts, Self> {
        if self.stopped.is_some() { return Err(self); }
        Ok(ReadyParts { policy: self.policy, created_rows: self.created_rows, retired: self.retired })
    }
    fn ready(&self) -> Result<(), Error> {
        if self.stopped.is_some() { Err(Error::PriorFailure) } else { Ok(()) }
    }
    fn fail(&mut self, operation: Operation, error: Error) -> Error {
        let kind = match &error {
            Error::PriorFailure => return error,
            Error::Flags { .. } => StopKind::Flags, Error::Width(_) => StopKind::Width,
            Error::Growth(_) => StopKind::Growth, Error::GrowthState => StopKind::GrowthState,
            Error::CounterOverflow => StopKind::CounterOverflow, Error::Capacity { .. } => StopKind::Capacity,
        };
        self.stopped.get_or_insert(StoppedAt { operation, kind });
        error
    }
    fn width(&mut self, operation: Operation, width: usize) -> Result<(), Error> {
        if width == 0 || width > i32::MAX as usize {
            return Err(self.fail(operation, Error::Width(width)));
        }
        Ok(())
    }
    fn reserve_retired(&mut self, operation: Operation) -> Result<(), Error> {
        match self.policy.retirement {
            Retirement::ArchiveOwnedValues => self.retired.try_reserve(1)
                .map_err(|cause| self.fail(operation, Error::Capacity { operation, cause })),
        }
    }
}
impl RequiredRows for ValueRows {
    type Error = Error;
    fn create_uncleared(&mut self, width: usize, first: bool, second: bool) -> Result<Row, Error> {
        self.ready()?;
        let operation = Operation::Create;
        if first || second { return Err(self.fail(operation, Error::Flags { first, second })); }
        self.width(operation, width)?;
        let next = self.created_rows.checked_add(1)
            .ok_or_else(|| self.fail(operation, Error::CounterOverflow))?;
        let row = crate::value_row_allocation::allocate_logical_row(width, self.policy.allocation_word)
            .map_err(|cause| self.fail(operation, match cause {
                crate::value_row_allocation::LogicalError::Width(width) => Error::Width(width),
                crate::value_row_allocation::LogicalError::Capacity(cause) => Error::Capacity { operation, cause },
            }))?;
        self.created_rows = next;
        Ok(row)
    }
    fn grow_pool_slots(&mut self, pool: Pool, storage: &mut ValuePool, plan: SourceRowGrowthPlan)
        -> Result<usize, Error>
    {
        self.ready()?;
        let operation = Operation::Grow(pool);
        let expected = pool_put_plan(storage.cached.len(), storage.source_capacity, storage.maximum)
            .map_err(|cause| self.fail(operation, Error::Growth(cause)))?;
        if expected != PoolPutPlan::Grow(plan) { return Err(self.fail(operation, Error::GrowthState)); }
        let additional = plan.new_capacity() - storage.cached.len();
        storage.cached.try_reserve_exact(additional)
            .map_err(|cause| self.fail(operation, Error::Capacity { operation, cause }))?;

        Ok(plan.new_capacity())
    }
    fn delete_row(&mut self, pool: Pool, row: Row) -> Result<(), RowRefusal<Error>> {
        if let Err(reason) = self.ready() { return Err(RowRefusal { reason, row }); }
        if let Err(reason) = self.reserve_retired(Operation::RetireRow(pool)) {
            return Err(RowRefusal { reason, row });
        }
        self.retired.push(Retired { owners: RetiredOwners::Row { pool, row } });
        Ok(())
    }
    fn clear_history_6490(&mut self, history: &mut VecDeque<Option<Row>>) -> Result<(), Error> {
        self.ready()?;
        self.reserve_retired(Operation::ClearHistory)?;
        self.retired.push(Retired { owners: RetiredOwners::History(std::mem::take(history)) });
        Ok(())
    }
    fn construct_pool_30a00(&mut self, pool: Pool, width: usize, initial: u32, maximum: u32)
        -> Result<ValuePool, Error>
    {
        self.ready()?;
        let operation = Operation::Construct(pool);
        self.width(operation, width)?;
        let mut cached = Vec::new();
        cached.try_reserve_exact(initial as usize)
            .map_err(|cause| self.fail(operation, Error::Capacity { operation, cause }))?;
        Ok(ValuePool { width, maximum: u64::from(maximum), source_capacity: initial as usize, cached })
    }
    fn retire_replaced_pool(&mut self, pool: Pool, old: &mut ValuePool) -> Result<(), Error> {
        self.ready()?;
        self.reserve_retired(Operation::RetirePool(pool))?;
        let empty = ValuePool { width: old.width, maximum: old.maximum,
            source_capacity: old.source_capacity, cached: Vec::new() };
        let state = std::mem::replace(old, empty);
        self.retired.push(Retired { owners: RetiredOwners::Pool { pool, state } });
        Ok(())
    }
}
