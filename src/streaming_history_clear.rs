use std::collections::VecDeque;
use crate::{condition_deque_clear_values as graph, streaming_projection::Row};
pub use graph::{ClearComplete, ClearError, ClearProgress, ClearStage};

pub trait RequiredHistoryEffects {
    type Error;

    fn retire_current_back(&mut self, history: &mut VecDeque<Option<Row>>) -> Result<(), Self::Error>;

    fn finish_current_back(&mut self, history: &mut VecDeque<Option<Row>>) -> Result<(), Self::Error>;

    fn finish_storage(&mut self, history: &mut VecDeque<Option<Row>>) -> Result<(), Self::Error>;
}
pub struct HistoryClear<S> { services: S, work: graph::ClearWork }
struct Bound<'a, S> { history: &'a mut VecDeque<Option<Row>>, services: &'a mut S }
impl<S: RequiredHistoryEffects> graph::ClearServices for Bound<'_, S> {
    type Error = S::Error;
    fn current_live_count(&self) -> usize { self.history.len() }
    fn retire_current_back(&mut self) -> Result<(), Self::Error> {
        self.services.retire_current_back(self.history)
    }
    fn finish_current_back(&mut self) -> Result<(), Self::Error> {
        self.services.finish_current_back(self.history)
    }
    fn finish_storage(&mut self) -> Result<(), Self::Error> {
        self.services.finish_storage(self.history)
    }
}
impl<S: RequiredHistoryEffects> HistoryClear<S> {

    pub fn after_explicit_admission(services: S) -> Self {
        Self { services, work: graph::ClearWork::after_admission() }
    }
    pub fn services(&self) -> &S { &self.services }
    pub fn progress(&self) -> ClearProgress { self.work.progress() }

    pub fn clear(&mut self, history: &mut VecDeque<Option<Row>>) -> Result<ClearComplete, ClearError<S::Error>> {
        let mut bound = Bound { history, services: &mut self.services };
        loop {
            match graph::advance_clear(&mut self.work, &mut bound, usize::MAX) {
                Ok(graph::ClearAdvance::Paused) => {},
                Ok(graph::ClearAdvance::Complete(marker)) => return Ok(marker),
                Err(error) => return Err(error),
            }
        }
    }
    pub fn into_completed_services(self) -> Result<S, Self> {
        let progress = self.work.progress();
        if progress.failed || progress.stage != ClearStage::Complete { Err(self) } else { Ok(self.services) }
    }
}
