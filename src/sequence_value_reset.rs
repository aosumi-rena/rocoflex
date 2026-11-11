use std::collections::TryReserveError;
use crate::{conditional_gate::{Batch, GateError, Projection},
    modm0_logical_sequence::RequiredReset,
    sequence_parameters::{Child, Decoded, Kind}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopAt { Layout, Child(usize), BatchArchive, HistoryArchive }
pub enum Error {
    PriorStop(StopAt), Layout { expected: usize, actual: usize },
    Kind { index: usize, expected: Kind, actual: Kind },
    Child { index: usize, reason: GateError },
    Capacity { at: StopAt, cause: TryReserveError },
}
pub struct RetiredCache { batches: Vec<Batch> }
impl RetiredCache { pub fn batches(&self) -> &[Batch] { &self.batches } }

pub(crate) struct CacheArchive { retired: Vec<RetiredCache> }
pub(crate) enum CacheArchiveError { Batches(TryReserveError), History(TryReserveError) }
impl CacheArchive {
    pub(crate) fn new() -> Self { Self { retired: Vec::new() } }
    pub(crate) fn retired(&self) -> &[RetiredCache] { &self.retired }
    pub(crate) fn retire(&mut self, intermediates: &mut Vec<Batch>) -> Result<(), CacheArchiveError> {
        if intermediates.is_empty() { return Ok(()); }
        let mut batches = Vec::new();
        batches.try_reserve_exact(intermediates.len()).map_err(CacheArchiveError::Batches)?;
        self.retired.try_reserve(1).map_err(CacheArchiveError::History)?;

        batches.extend(intermediates.drain(..));
        self.retired.push(RetiredCache { batches });
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress { pub completed_children: usize, pub retired_caches: usize, pub stopped: Option<StopAt> }

pub struct ResetChildrenThenArchiveCache {
    expected: Vec<Kind>, completed_children: usize, archive: CacheArchive, stopped: Option<StopAt>,
}
impl ResetChildrenThenArchiveCache {

    pub fn for_parameters(parameters: &Decoded<'_, '_>) -> Result<Self, TryReserveError> {
        let mut expected = Vec::new();
        expected.try_reserve_exact(parameters.children().len())?;
        expected.extend(parameters.children().iter().map(|node| node.kind()));
        Ok(Self { expected, completed_children: 0, archive: CacheArchive::new(), stopped: None })
    }
    pub fn progress(&self) -> Progress {
        Progress { completed_children: self.completed_children, retired_caches: self.archive.retired().len(), stopped: self.stopped }
    }
    pub fn retired(&self) -> &[RetiredCache] { self.archive.retired() }
    fn capacity(&mut self, at: StopAt, cause: TryReserveError) -> Error {
        self.stopped = Some(at);
        Error::Capacity { at, cause }
    }
}
impl<I: Projection, A: Projection, C: Projection, L: Projection>
    RequiredReset<Child<I, A, C, L>, Child<I, A, C, L>> for ResetChildrenThenArchiveCache
{
    type Error = Error;
    fn reset(&mut self, first: &mut Child<I, A, C, L>, following: &mut [Child<I, A, C, L>],
        intermediates: &mut Vec<Batch>) -> Result<(), Error>
    {
        if let Some(at) = self.stopped { return Err(Error::PriorStop(at)); }
        self.completed_children = 0;
        let count = following.len() + 1;
        if self.expected.len() != count {
            self.stopped = Some(StopAt::Layout);
            return Err(Error::Layout { expected: self.expected.len(), actual: count });
        }

        if let Some((index, expected, actual)) = std::iter::once(&*first).chain(following.iter())
            .zip(&self.expected).enumerate().find_map(|(index, (child, expected))|
                (child.kind() != *expected).then_some((index, *expected, child.kind())))
        {
            self.stopped = Some(StopAt::Layout);
            return Err(Error::Kind { index, expected, actual });
        }

        self.stopped = Some(StopAt::Child(0));
        if let Err(reason) = first.reset() {
            return Err(Error::Child { index: 0, reason });
        }
        self.stopped = None;
        self.completed_children = 1;
        for (ordinal, child) in following.iter_mut().enumerate() {
            let index = ordinal + 1;
            self.stopped = Some(StopAt::Child(index));
            if let Err(reason) = child.reset() {
                return Err(Error::Child { index, reason });
            }
            self.stopped = None;
            self.completed_children = index + 1;
        }

        self.archive.retire(intermediates).map_err(|error| match error {
            CacheArchiveError::Batches(cause) => self.capacity(StopAt::BatchArchive, cause),
            CacheArchiveError::History(cause) => self.capacity(StopAt::HistoryArchive, cause),
        })
    }
}
