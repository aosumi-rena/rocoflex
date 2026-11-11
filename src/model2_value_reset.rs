use crate::{
    conditional_gate::{Batch, GateError, Projection},
    moda7_value_services::ValueServices as Activation,
    model2_logical_chain::LaterChild,
    modl0_projection::Modl0Projection,
    modl0_value_services::ValueServices as Linear,
    modm0_logical_sequence::RequiredReset,
    sequence_value_reset::{CacheArchive, CacheArchiveError, RetiredCache},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopAt { Layout, First, Middle, Last, BatchArchive, HistoryArchive }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {

    pub completed_children: u8,
    pub retired_caches: usize,
    pub stopped: Option<StopAt>,
}

pub enum Error {
    PriorStop { at: StopAt },
    Layout { following: usize },
    Child { at: StopAt, reason: GateError },
    Capacity { at: StopAt, cause: std::collections::TryReserveError },
}

pub struct ResetChildrenThenArchiveIntermediates {
    progress: Progress,
    archive: CacheArchive,
}
impl ResetChildrenThenArchiveIntermediates {
    pub fn new() -> Self {
        Self { progress: Progress { completed_children: 0, retired_caches: 0, stopped: None },
            archive: CacheArchive::new() }
    }
    pub fn progress(&self) -> Progress { self.progress }
    pub fn retired(&self) -> &[RetiredCache] { self.archive.retired() }

    fn child_failed(&mut self, at: StopAt, reason: GateError) -> Error {
        self.progress.stopped = Some(at);
        Error::Child { at, reason }
    }
}

impl RequiredReset<Modl0Projection<Linear>, LaterChild<Activation, Linear>>
    for ResetChildrenThenArchiveIntermediates
{
    type Error = Error;

    fn reset(
        &mut self,
        first: &mut Modl0Projection<Linear>,
        following: &mut [LaterChild<Activation, Linear>],
        intermediates: &mut Vec<Batch>,
    ) -> Result<(), Error> {
        if let Some(at) = self.progress.stopped {
            return Err(Error::PriorStop { at });
        }
        self.progress.completed_children = 0;
        let count = following.len();

        let [LaterChild::Moda7(middle), LaterChild::Modl0(last)] = following else {
            self.progress.stopped = Some(StopAt::Layout);
            return Err(Error::Layout { following: count });
        };

        self.progress.stopped = Some(StopAt::First);
        if let Err(reason) = first.reset() {
            return Err(self.child_failed(StopAt::First, reason));
        }
        self.progress.completed_children = 1;
        self.progress.stopped = Some(StopAt::Middle);
        if let Err(reason) = middle.reset() {
            return Err(self.child_failed(StopAt::Middle, reason));
        }
        self.progress.completed_children = 2;
        self.progress.stopped = Some(StopAt::Last);
        if let Err(reason) = last.reset() {
            return Err(self.child_failed(StopAt::Last, reason));
        }
        self.progress.completed_children = 3;
        if let Err(error) = self.archive.retire(intermediates) {
            let (at, cause) = match error {
                CacheArchiveError::Batches(cause) => (StopAt::BatchArchive, cause),
                CacheArchiveError::History(cause) => (StopAt::HistoryArchive, cause),
            };
            self.progress.stopped = Some(at);
            return Err(Error::Capacity { at, cause });
        }
        self.progress.retired_caches = self.archive.retired().len();
        self.progress.stopped = None;
        Ok(())
    }
}
