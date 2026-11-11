use std::collections::{TryReserveError, VecDeque};
use crate::{conditional_gate::Batch, paired_input, paired_interpolation, paired_scalar_packing,
    row_prepare_values, streaming_rows::ValueRows,
    streaming_value_services::RequiredRows, value_row_allocation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputStage { Idle, Admission, TrimBack, Create, AdmitRow, Install, Retire, Complete }
#[derive(Clone, Copy, Debug)]
pub struct InputProgress {
    pub stage: InputStage, pub requested_rows: Option<usize>,
    pub index: Option<usize>, pub halted: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetiredKind { Trimmed, Replaced }
pub struct RetiredRow { index: usize, kind: RetiredKind, row: Vec<f32> }
impl RetiredRow {
    pub fn index(&self) -> usize { self.index }
    pub fn kind(&self) -> RetiredKind { self.kind }
    pub fn samples(&self) -> &[f32] { &self.row }
}
pub enum InputError {
    PriorFailure, Count(usize), Width(usize), Factory(crate::streaming_rows::Error),
    Capacity { stage: InputStage, cause: TryReserveError },
    CreatedWidth { expected: usize, actual: usize },
    Index { index: usize, actual_count: usize },
}
pub struct InputReadyParts { pub factory: ValueRows, pub retired: Vec<RetiredRow> }
struct OwnedRows {
    factory: ValueRows, retired: Vec<RetiredRow>,
    pending_donor: Option<Vec<f32>>, pending_retirement: Option<RetiredRow>,
    progress: InputProgress,
}
impl OwnedRows {
    fn from_explicit_factory(factory: ValueRows) -> Result<Self, ValueRows> {
        if factory.progress().stopped.is_some() { return Err(factory); }
        Ok(Self { factory, retired: Vec::new(), pending_donor: None, pending_retirement: None,
            progress: InputProgress { stage: InputStage::Idle, requested_rows: None, index: None, halted: false } })
    }
    fn into_ready_parts(self) -> Result<(ValueRows, Vec<RetiredRow>), Self> {
        if self.progress.halted || self.pending_donor.is_some() || self.pending_retirement.is_some()
            || self.factory.progress().stopped.is_some() { return Err(self); }
        Ok((self.factory, self.retired))
    }
    fn reserve_retired(&mut self, stage: InputStage) -> Result<(), InputError> {
        self.retired.try_reserve(1).map_err(|cause| InputError::Capacity { stage, cause })
    }
    fn prepare(&mut self, destination: &mut Batch, required_rows: usize, width: usize) -> Result<(), InputError> {
        if self.progress.halted { return Err(InputError::PriorFailure); }

        self.progress.halted = true;
        self.progress.stage = InputStage::Admission;
        self.progress.requested_rows = Some(required_rows);
        self.progress.index = None;
        if required_rows > i32::MAX as usize { return Err(InputError::Count(required_rows)); }
        if width == 0 || width > i32::MAX as usize { return Err(InputError::Width(width)); }
        row_prepare_values::prepare(&mut Preparation { owner: self, destination }, required_rows, width)?;
        self.progress.stage = InputStage::Complete;
        self.progress.index = None;
        self.progress.halted = false;
        Ok(())
    }
    fn create_donor(&mut self, width: usize) -> Result<(), InputError> {
        self.progress.stage = InputStage::Create;
        self.pending_donor = Some(self.factory.create_uncleared(width, false, false)
            .map_err(InputError::Factory)?);
        self.progress.stage = InputStage::AdmitRow;
        let actual = self.pending_donor.as_ref().expect("actual created donor").len();
        if actual != width { return Err(InputError::CreatedWidth { expected: width, actual }); }
        Ok(())
    }
    fn create_history(&mut self, width: usize) -> Result<Vec<f32>, InputError> {
        if self.progress.halted { return Err(InputError::PriorFailure); }
        self.progress.halted = true;
        self.progress.requested_rows = None;
        self.progress.index = None;

        self.create_donor(width)?;
        let row = self.pending_donor.take().expect("actual admitted history donor");
        self.progress.stage = InputStage::Complete;
        self.progress.halted = false;
        Ok(row)
    }
}

pub struct InputRows { rows: OwnedRows }
impl InputRows {
    pub fn from_explicit_factory(factory: ValueRows) -> Result<Self, ValueRows> {
        OwnedRows::from_explicit_factory(factory).map(|rows| Self { rows })
    }
    pub fn factory(&self) -> &ValueRows { &self.rows.factory }
    pub fn progress(&self) -> InputProgress { self.rows.progress }
    pub fn retired(&self) -> &[RetiredRow] { &self.rows.retired }
    pub fn pending_donor(&self) -> Option<&[f32]> { self.rows.pending_donor.as_deref() }
    pub fn pending_retirement(&self) -> Option<&RetiredRow> { self.rows.pending_retirement.as_ref() }
    pub fn into_ready_parts(self) -> Result<InputReadyParts, Self> {
        match self.rows.into_ready_parts() {
            Ok((factory, retired)) => Ok(InputReadyParts { factory, retired }),
            Err(rows) => Err(Self { rows }),
        }
    }
}
impl paired_input::Rows for InputRows {
    type Error = InputError;
    fn prepare112(&mut self, destination: &mut Batch, required_rows: usize) -> Result<(), InputError> {
        self.rows.prepare(destination, required_rows, 1)
    }
}

pub struct InterpolationReadyParts { pub factory: ValueRows, pub retired: Vec<RetiredRow> }

pub struct InterpolationStorage { rows: OwnedRows }
impl InterpolationStorage {
    pub fn from_explicit_factory(factory: ValueRows) -> Result<Self, ValueRows> {
        OwnedRows::from_explicit_factory(factory).map(|rows| Self { rows })
    }
    pub fn factory(&self) -> &ValueRows { &self.rows.factory }
    pub fn progress(&self) -> InputProgress { self.rows.progress }
    pub fn retired(&self) -> &[RetiredRow] { &self.rows.retired }
    pub fn pending_donor(&self) -> Option<&[f32]> { self.rows.pending_donor.as_deref() }
    pub fn pending_retirement(&self) -> Option<&RetiredRow> { self.rows.pending_retirement.as_ref() }
    pub fn into_ready_parts(self) -> Result<InterpolationReadyParts, Self> {
        match self.rows.into_ready_parts() {
            Ok((factory, retired)) => Ok(InterpolationReadyParts { factory, retired }),
            Err(rows) => Err(Self { rows }),
        }
    }
}
impl paired_interpolation::Storage for InterpolationStorage {
    type Error = InputError;
    fn create_history(&mut self, width: usize) -> Result<Vec<f32>, InputError> {
        self.rows.create_history(width)
    }
    fn prepare336(&mut self, output: &mut Batch, count: usize, width: usize) -> Result<(), InputError> {
        self.rows.prepare(output, count, width)
    }
}

struct Preparation<'a> { owner: &'a mut OwnedRows, destination: &'a mut Batch }
impl row_prepare_values::Services for Preparation<'_> {
    type Error = InputError;
    fn current_count(&self) -> usize { self.destination.rows.len() }
    fn retire_current_back(&mut self) -> Result<(), InputError> {
        let index = self.destination.rows.len() - 1;
        self.owner.progress.stage = InputStage::TrimBack;
        self.owner.progress.index = Some(index);

        self.owner.reserve_retired(InputStage::TrimBack)?;
        let row = self.destination.rows.pop().expect("shared preparation selected a live back row");
        self.owner.retired.push(RetiredRow { index, kind: RetiredKind::Trimmed, row });
        Ok(())
    }
    fn row_matches(&self, index: usize, width: usize) -> bool {
        self.destination.rows.get(index).is_some_and(|row| row.len() == width)
    }
    fn create_admit_install(&mut self, index: usize, width: usize) -> Result<(), InputError> {
        self.owner.progress.index = Some(index);
        self.owner.create_donor(width)?;
        let count = self.destination.rows.len();
        if index > count { return Err(InputError::Index { index, actual_count: count }); }
        self.owner.progress.stage = InputStage::Install;
        if index == count {

            self.destination.rows.try_reserve(1)
                .map_err(|cause| InputError::Capacity { stage: InputStage::Install, cause })?;
            self.destination.rows.push(self.owner.pending_donor.take().expect("retained append donor"));
        } else {
            let donor = self.owner.pending_donor.take().expect("retained replacement donor");
            let row = std::mem::replace(&mut self.destination.rows[index], donor);
            self.owner.pending_retirement = Some(RetiredRow { index, kind: RetiredKind::Replaced, row });
            self.owner.progress.stage = InputStage::Retire;

            self.owner.reserve_retired(InputStage::Retire)?;
            self.owner.retired.push(self.owner.pending_retirement.take().expect("retained replaced row"));
        }
        Ok(())
    }
}

pub enum CurrentSource {
    Supplied(VecDeque<Vec<f32>>),
    Allocate { width: usize, allocation_word: u32, remaining: usize },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurrentAttempt { Supplied, Allocate { width: usize, allocation_word: u32 } }
#[derive(Clone, Copy, Debug)]
pub struct CurrentProgress { pub acquired: usize, pub attempted: Option<CurrentAttempt>, pub halted: bool }
pub enum CurrentError { PriorFailure, Exhausted, Allocation(value_row_allocation::LogicalError) }
pub struct CurrentReadyParts { pub source: CurrentSource, pub acquired: usize }
pub struct CurrentRows { source: CurrentSource, progress: CurrentProgress }
impl CurrentRows {

    pub fn from_explicit_source(source: CurrentSource) -> Self {
        Self { source, progress: CurrentProgress { acquired: 0, attempted: None, halted: false } }
    }
    pub fn source(&self) -> &CurrentSource { &self.source }
    pub fn progress(&self) -> CurrentProgress { self.progress }
    pub fn into_ready_parts(self) -> Result<CurrentReadyParts, Self> {
        if self.progress.halted { return Err(self); }
        Ok(CurrentReadyParts { source: self.source, acquired: self.progress.acquired })
    }
}
impl paired_scalar_packing::CurrentRows for CurrentRows {
    type Error = CurrentError;
    fn acquire_current(&mut self) -> Result<Vec<f32>, CurrentError> {
        if self.progress.halted { return Err(CurrentError::PriorFailure); }
        self.progress.halted = true;
        let result = match &mut self.source {
            CurrentSource::Supplied(rows) => {
                self.progress.attempted = Some(CurrentAttempt::Supplied);
                rows.pop_front().ok_or(CurrentError::Exhausted)
            }
            CurrentSource::Allocate { width, allocation_word, remaining } => {
                self.progress.attempted = Some(CurrentAttempt::Allocate { width: *width, allocation_word: *allocation_word });
                if *remaining == 0 { return Err(CurrentError::Exhausted); }

                *remaining -= 1;
                value_row_allocation::allocate_logical_row(*width, *allocation_word).map_err(CurrentError::Allocation)
            }
        };
        let row = result?;

        self.progress.acquired += 1;
        self.progress.halted = false;
        Ok(row)
    }
}
