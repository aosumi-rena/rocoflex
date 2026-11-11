use std::collections::TryReserveError;
use crate::{row_prepare_values, streaming_rows::ValueRows,
    streaming_value_services::RequiredRows};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Idle, Admission, TrimBack, Create, Admit, Install, Retire, Complete }
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub stage: Stage, pub requested_rows: Option<usize>, pub width: Option<usize>,
    pub index: Option<usize>, pub halted: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetiredKind { Trimmed, Replaced }
pub struct RetiredRow { index: usize, kind: RetiredKind, values: Vec<f32> }
impl RetiredRow {
    pub fn index(&self) -> usize { self.index }
    pub fn kind(&self) -> RetiredKind { self.kind }
    pub fn values(&self) -> &[f32] { &self.values }
}
pub enum Error {

    PriorFailure, Count(usize), Width(usize), MissingRow { index: usize },
    Factory(crate::streaming_rows::Error),
    Capacity { stage: Stage, cause: TryReserveError },
    CreatedWidth { expected: usize, actual: usize },
    Index { index: usize, actual_count: usize },
}

pub struct FactoryRows {
    factory: ValueRows, retired: Vec<RetiredRow>,
    pending_donor: Option<Vec<f32>>, pending_retirement: Option<RetiredRow>,
    progress: Progress,
}
impl FactoryRows {
    pub(super) fn new(factory: ValueRows) -> Self {
        Self { factory, retired: Vec::new(), pending_donor: None, pending_retirement: None,
            progress: Progress { stage: Stage::Idle, requested_rows: None, width: None,
                index: None, halted: false } }
    }
    pub fn factory(&self) -> &ValueRows { &self.factory }
    pub fn retired(&self) -> &[RetiredRow] { &self.retired }
    pub fn pending_donor(&self) -> Option<&[f32]> { self.pending_donor.as_deref() }
    pub fn pending_retirement(&self) -> Option<&RetiredRow> { self.pending_retirement.as_ref() }
    pub fn progress(&self) -> Progress { self.progress }
    fn reserve_retired(&mut self, stage: Stage) -> Result<(), Error> {
        self.retired.try_reserve(1).map_err(|cause| Error::Capacity { stage, cause })
    }
    pub(super) fn prepare(&mut self, rows: &mut Vec<Option<Vec<f32>>>, count: usize, width: usize)
        -> Result<(), Error>
    {
        if self.progress.halted { return Err(Error::PriorFailure); }

        self.progress.halted = true;
        self.progress.stage = Stage::Admission;
        self.progress.requested_rows = Some(count);
        self.progress.width = Some(width);
        self.progress.index = None;
        if count > i32::MAX as usize { return Err(Error::Count(count)); }
        if width == 0 || width > i32::MAX as usize { return Err(Error::Width(width)); }
        row_prepare_values::prepare(&mut Preparation { owner: self, rows }, count, width)?;
        self.progress.stage = Stage::Complete;
        self.progress.index = None;
        self.progress.halted = false;
        Ok(())
    }
}
struct Preparation<'a> { owner: &'a mut FactoryRows, rows: &'a mut Vec<Option<Vec<f32>>> }
impl row_prepare_values::Services for Preparation<'_> {
    type Error = Error;
    fn current_count(&self) -> usize { self.rows.len() }
    fn retire_current_back(&mut self) -> Result<(), Error> {
        let index = self.rows.len() - 1;
        self.owner.progress.stage = Stage::TrimBack;
        self.owner.progress.index = Some(index);

        if self.rows[index].is_none() {
            let _ = self.rows.pop().expect("selected null back slot");
            return Ok(());
        }

        self.owner.reserve_retired(Stage::TrimBack)?;
        let values = self.rows.pop().expect("selected back slot").expect("checked live row");
        self.owner.retired.push(RetiredRow { index, kind: RetiredKind::Trimmed, values });
        Ok(())
    }
    fn row_matches(&self, index: usize, width: usize) -> bool {
        self.rows.get(index).and_then(Option::as_ref).is_some_and(|row| row.len() == width)
    }
    fn create_admit_install(&mut self, index: usize, width: usize) -> Result<(), Error> {
        self.owner.progress.index = Some(index);
        let count = self.rows.len();
        if index > count { return Err(Error::Index { index, actual_count: count }); }
        self.owner.progress.stage = Stage::Create;
        self.owner.pending_donor = Some(self.owner.factory.create_uncleared(width, false, false)
            .map_err(Error::Factory)?);
        self.owner.progress.stage = Stage::Admit;
        let actual = self.owner.pending_donor.as_ref().expect("actual factory donor").len();
        if actual != width { return Err(Error::CreatedWidth { expected: width, actual }); }
        self.owner.progress.stage = Stage::Install;
        if index == count {

            self.rows.try_reserve(1).map_err(|cause| Error::Capacity { stage: Stage::Install, cause })?;
            self.rows.push(self.owner.pending_donor.take());
        } else {
            let old = std::mem::replace(&mut self.rows[index], self.owner.pending_donor.take());

            if let Some(old) = old {
                self.owner.pending_retirement = Some(RetiredRow { index, kind: RetiredKind::Replaced, values: old });
                self.owner.progress.stage = Stage::Retire;

                self.owner.reserve_retired(Stage::Retire)?;
                self.owner.retired.push(self.owner.pending_retirement.take().expect("actual displaced row"));
            }
        }
        Ok(())
    }
}
