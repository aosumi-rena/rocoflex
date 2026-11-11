use std::collections::VecDeque;
use crate::{conditional_gate::Batch, paired_condition_values as graph, paired_features,
    paired_input, paired_post::Module};
pub use graph::{ConditionComplete, ConditionProgress, CountError, WorkCount};
#[path = "paired_condition_services.rs"]
mod services;
#[path = "paired_condition_append.rs"]
pub mod interpolation_append;

pub struct Borrowed176<'a> {
    queue: &'a VecDeque<paired_features::FeatureRow>, indices: &'a [Option<usize>], status: u8,
}
impl Borrowed176<'_> {
    pub fn row_count(&self) -> usize { self.indices.len() }
    pub fn status(&self) -> u8 { self.status }
    pub fn row(&self, ordinal: usize) -> Option<&[f32]> {
        self.indices.get(ordinal).copied().flatten().and_then(|i| self.queue.get(i))
            .map(paired_features::FeatureRow::samples)
    }
}

pub trait Worker {
    type Error;
    fn forward176(&mut self, input: Borrowed176<'_>, conditions208: &mut Batch,
        retained240: &mut Batch) -> Result<(), Self::Error>;
}
pub struct Parts<W> {
    pub worker64: W, pub cursor_index508: i32, pub ended517: u8,
    pub cursor_words: Vec<u32>, pub cursor_count: usize,
    pub cursor_halted: bool,

    pub input176_indices: Vec<Option<usize>>, pub input176_len: usize, pub input176_status: u8,

    pub conditions432: VecDeque<Option<Vec<f32>>>, pub condition208: Batch, pub retained240: Batch,
}
pub struct State<W> { parts: Parts<W>, halted: bool }
pub(crate) struct SharedSuspended<W> {
    worker64: W, cursor_index508: i32, ended517: u8,
    input176_indices: Vec<Option<usize>>, input176_len: usize, input176_status: u8,
    conditions432: VecDeque<Option<Vec<f32>>>, condition208: Batch, halted: bool,
}
impl<W> SharedSuspended<W> {
    pub(crate) fn worker(&self) -> &W { &self.worker64 }
    pub(crate) fn restore(self, retained240: Batch, cursor_count: usize, cursor_words: Vec<u32>, cursor_halted: bool) -> State<W> {
        State { parts: Parts { worker64: self.worker64, cursor_index508: self.cursor_index508, ended517: self.ended517,
            cursor_words, cursor_count, cursor_halted, input176_indices: self.input176_indices,
            input176_len: self.input176_len, input176_status: self.input176_status,
            conditions432: self.conditions432, condition208: self.condition208, retained240 }, halted: self.halted }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { CursorStorage, CursorIndex, CursorHalted, Ended, BorrowStorage, Count(CountError),
    Nonempty176, Nonempty208, Capacity176 { required: usize }, Capacity208 { required: usize }, MissingCondition(usize) }
pub struct BindingFailure<W> { pub cause: Domain, pub parts: Parts<W> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdoptError { PriorFailure, Header }
pub struct StartFailure<W, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    pub cause: Domain, pub state: State<W>, pub upstream: paired_features::Completed<P, G, M, R, H>,
}
impl<W> State<W> {
    pub fn from_explicit_parts(parts: Parts<W>) -> Result<Self, BindingFailure<W>> {
        let cause = if parts.cursor_halted { Some(Domain::CursorHalted) }
            else if parts.cursor_count > parts.cursor_words.len() { Some(Domain::CursorStorage) }
            else if parts.cursor_index508 < 0 { Some(Domain::CursorIndex) }
            else if parts.ended517 > 1 { Some(Domain::Ended) }
            else if parts.input176_len > parts.input176_indices.len() { Some(Domain::BorrowStorage) }
            else { None };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }
        Ok(Self { parts, halted: false })
    }
    pub fn parts(&self) -> &Parts<W> { &self.parts }
    pub fn is_halted(&self) -> bool { self.halted }
    pub fn into_parts(self) -> Parts<W> { self.parts }
    pub(crate) fn adopt_advanced_prefix_ended517(&mut self, ended: u8) -> Result<(), AdoptError> {
        if self.halted { return Err(AdoptError::PriorFailure); }
        if self.parts.ended517 != 0 || ended > 1 { return Err(AdoptError::Header); }
        self.parts.ended517 = ended; Ok(())
    }
    pub(crate) fn suspend_shared(self) -> (SharedSuspended<W>, Batch, usize, Vec<u32>, bool) {
        let Parts { worker64, cursor_index508, ended517, cursor_words, cursor_count, cursor_halted,
            input176_indices, input176_len, input176_status, conditions432, condition208, retained240 } = self.parts;
        (SharedSuspended { worker64, cursor_index508, ended517, input176_indices, input176_len,
            input176_status, conditions432, condition208, halted: self.halted },
            retained240, cursor_count, cursor_words, cursor_halted)
    }
    pub fn begin<P: paired_features::Frames, G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H>(
        mut self, upstream: paired_features::Completed<P, G, M, R, H>)
        -> Result<Prepared<W, P, G, M, R, H>, StartFailure<W, P, G, M, R, H>> where W: Worker {
        let index = self.parts.cursor_index508 as usize;
        if index >= self.parts.cursor_count {
            return Err(StartFailure { cause: Domain::CursorIndex, state: self, upstream });
        }
        let cursor = self.parts.cursor_words[index] as i32;
        let selected = match graph::select_work_count(upstream.prepared().state().parts().queue400.len() as u64,
            self.parts.conditions432.len() as u64, cursor, self.parts.ended517) {
            Ok(count) => count,
            Err(error) => return Err(StartFailure { cause: Domain::Count(error), state: self, upstream }),
        };

        if selected.work != 0 {
            let p = &self.parts;
            let cause = if p.input176_len != 0 { Some(Domain::Nonempty176) }
                else if !p.condition208.rows.is_empty() { Some(Domain::Nonempty208) }
                else if selected.work > p.input176_indices.len() { Some(Domain::Capacity176 { required: selected.work }) }
                else if selected.work > p.condition208.rows.capacity() { Some(Domain::Capacity208 { required: selected.work }) }
                else { (0..selected.work).find(|&i| p.conditions432.get(i).is_none_or(Option::is_none)).map(Domain::MissingCondition) };
            if let Some(cause) = cause { return Err(StartFailure { cause, state: self, upstream }); }
            self.halted = true;
        }
        let ended = self.parts.ended517;
        Ok(Prepared { owner: Owner { state: self, upstream, index, selected,
            trace: Trace { bound: None, moved: 0, erased: 0, worker_completed: false,
                zeroed176: false, cursor_after: None, finished176: false,
                retired208: Vec::new(), pending_retirement: None } },
            work: graph::ConditionWork::after_explicit_admission(selected.work, cursor, ended),
            failure: None, complete: None })
    }
}
pub struct Trace {
    pub bound: Option<(usize, usize)>, pub moved: usize, pub erased: usize,
    pub worker_completed: bool, pub zeroed176: bool, pub cursor_after: Option<u32>, pub finished176: bool,

    pub retired208: Vec<Vec<f32>>, pub pending_retirement: Option<Vec<f32>>,
}
pub enum ServiceError<E> { Worker(E), Range, Input, Cursor,
    Allocation(std::collections::TryReserveError), Control(&'static str) }
struct Owner<W, P: paired_features::Frames, G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H> {
    state: State<W>, upstream: paired_features::Completed<P, G, M, R, H>, index: usize,
    selected: WorkCount, trace: Trace,
}
pub struct Prepared<W: Worker, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    owner: Owner<W, P, G, M, R, H>, work: graph::ConditionWork,
    failure: Option<graph::ConditionError<ServiceError<W::Error>>>, complete: Option<ConditionComplete>,
}
pub enum Poll<'a, E> { Pending(ConditionProgress), Complete, Failed(&'a graph::ConditionError<ServiceError<E>>) }
pub struct Completed<W: Worker, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> { prepared: Prepared<W, P, G, M, R, H> }
pub struct CompletedParts<W, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    pub state: State<W>, pub upstream: paired_features::Completed<P, G, M, R, H>,
    pub selected: WorkCount, pub trace: Trace, pub condition: ConditionComplete,
}
impl<W: Worker, P: paired_features::Frames, G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H>
    Prepared<W, P, G, M, R, H> {
    pub fn state(&self) -> &State<W> { &self.owner.state }
    pub fn upstream(&self) -> &paired_features::Completed<P, G, M, R, H> { &self.owner.upstream }
    pub fn selected(&self) -> WorkCount { self.owner.selected }
    pub fn trace(&self) -> &Trace { &self.owner.trace }
    pub fn progress(&self) -> ConditionProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&graph::ConditionError<ServiceError<W::Error>>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> Poll<'_, W::Error> {
        if self.failure.is_none() && self.complete.is_none() {
            match graph::advance(&mut self.work, &mut self.owner, budget) {
                Ok(graph::ConditionAdvance::Paused) => {},
                Ok(graph::ConditionAdvance::Complete(marker)) => {
                    self.owner.state.halted = false; self.complete = Some(marker);
                }
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.complete.is_some() { Poll::Complete } else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<W, P, G, M, R, H>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) } else { Ok(Completed { prepared: self }) }
    }
}
impl<W: Worker, P: paired_features::Frames, G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H>
    Completed<W, P, G, M, R, H> {
    pub fn prepared(&self) -> &Prepared<W, P, G, M, R, H> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<W, P, G, M, R, H> {
        let owner = self.prepared.owner;
        CompletedParts { state: owner.state, upstream: owner.upstream, selected: owner.selected,
            trace: owner.trace, condition: self.prepared.complete.expect("private condition completion") }
    }
}
