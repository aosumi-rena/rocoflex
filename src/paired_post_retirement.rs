use super::{packing, ScalarUpper};
use crate::{paired_retirement_values as graph, paired_scalar_packing::CurrentRows};
use crate::paired_features::FeatureRow;
use std::collections::VecDeque;
#[path = "paired_post_second_append.rs"]
pub mod second_append;
pub use graph::{PositivePrefix, PrefixEnd, RetirementComplete, RetirementProgress, RetirementStage};

pub struct CurrentFeatures<'a> {
    pub rows: &'a mut VecDeque<FeatureRow>,
    pub logical_cursor_count: &'a mut usize,
    pub cursor_words: &'a mut Vec<u32>,
}

pub trait Erasure {
    type Error;

    type Context;
    fn prepare_positive(&mut self, request: PositivePrefix, rows: &VecDeque<FeatureRow>,
        logical_cursor_count: usize, cursor_words: &[u32], context: &mut Self::Context) -> Result<(), Self::Error>;
    fn erase_positive(&mut self, request: PositivePrefix, current: CurrentFeatures<'_>,
        context: &mut Self::Context) -> Result<(), Self::Error>;
    fn admit_current_cursors(&mut self, rows: &VecDeque<FeatureRow>, logical_cursor_count: usize,
        cursor_words: &[u32], context: &mut Self::Context) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { CursorHalted, CursorStorage }
pub struct StartFailure<A, B, U: ScalarUpper, H, P: CurrentRows, R> {
    pub cause: Domain, pub packed: packing::Packed<A, B, U, H, P>, pub erasure: R,
}
pub enum Error<E> { Service(E), CurrentCursorStorage }
struct Owner<A, B, U: ScalarUpper, H, P, R> {
    continuation: packing::Continuation<A, B, U, H, P>, erasure: R,
}
pub struct Prepared<A, B, U: ScalarUpper, H, P, R: Erasure> {
    owner: Owner<A, B, U, H, P, R>, work: graph::RetirementWork,
    failure: Option<graph::RetirementError<Error<R::Error>>>, complete: Option<RetirementComplete>,
}
pub enum Poll<'a, E> { Pending(RetirementProgress), Complete, Failed(&'a graph::RetirementError<Error<E>>) }
pub struct Completed<A, B, U: ScalarUpper, H, P, R: Erasure> { prepared: Prepared<A, B, U, H, P, R> }
pub struct CompletedParts<A, B, U: ScalarUpper, H, P, R> {
    pub continuation: packing::Continuation<A, B, U, H, P>, pub erasure: R,
    pub retirement: RetirementComplete,
}
impl<A, B, U: ScalarUpper, H, P: CurrentRows> packing::Packed<A, B, U, H, P> {

    pub fn into_retirement<R: Erasure<Context = H>>(self, erasure: R)
        -> Result<Prepared<A, B, U, H, P, R>, StartFailure<A, B, U, H, P, R>>
    {
        let p = self.post().state().parts();
        if p.cursor_halted { return Err(StartFailure { cause: Domain::CursorHalted, packed: self, erasure }); }
        let Some(words) = p.cursor_words.get(..p.cursor_count) else {
            return Err(StartFailure { cause: Domain::CursorStorage, packed: self, erasure });
        };
        let work = graph::RetirementWork::after_explicit_snapshot(p.noise_rows.len() as u64, words);
        Ok(Prepared { owner: Owner { continuation: self.into_continuation(), erasure },
            work, failure: None, complete: None })
    }
}
impl<A, B, U: ScalarUpper, H, P, R: Erasure> Prepared<A, B, U, H, P, R> {
    pub fn continuation(&self) -> &packing::Continuation<A, B, U, H, P> { &self.owner.continuation }
    pub fn erasure(&self) -> &R { &self.owner.erasure }
    pub fn progress(&self) -> RetirementProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&graph::RetirementError<Error<R::Error>>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> Poll<'_, R::Error> where R: Erasure<Context = H> {
        if self.failure.is_none() && self.complete.is_none() {
            match graph::advance(&mut self.work, &mut self.owner, budget) {
                Ok(graph::RetirementAdvance::Paused) => {},
                Ok(graph::RetirementAdvance::Complete(marker)) => {
                    if matches!(&marker, RetirementComplete::Retired(_)) {
                        self.owner.continuation.post.parts.cursor_halted = false;
                    }
                    self.complete = Some(marker);
                }
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.complete.is_some() { Poll::Complete }
        else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<A, B, U, H, P, R>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) }
        else { Ok(Completed { prepared: self }) }
    }
}
impl<A, B, U: ScalarUpper, H, P, R: Erasure> Completed<A, B, U, H, P, R> {
    pub fn prepared(&self) -> &Prepared<A, B, U, H, P, R> { &self.prepared }

    pub(crate) fn split_held<T, X>(self, split: impl FnOnce(H) -> (T, X))
        -> (Completed<A, B, U, T, P, R>, X) {
        let Prepared { owner, work, failure, complete } = self.prepared;
        let Owner { continuation, erasure } = owner;
        let packing::Continuation { post, packing, input240, held, post_trace,
            packing_trace, post_completion, packing_completion } = continuation;
        let (held, extra) = split(held);
        (Completed { prepared: Prepared { owner: Owner { erasure,
            continuation: packing::Continuation { post, packing, input240, held,
                post_trace, packing_trace, post_completion, packing_completion } },
            work, failure, complete } }, extra)
    }

    pub fn into_parts(self) -> CompletedParts<A, B, U, H, P, R> {
        let owner = self.prepared.owner;
        CompletedParts { continuation: owner.continuation, erasure: owner.erasure,
            retirement: self.prepared.complete.expect("private retirement completion") }
    }
}
impl<A, B, U: ScalarUpper, H, P, R: Erasure<Context = H>> graph::RetirementServices for Owner<A, B, U, H, P, R> {
    type Error = Error<R::Error>;
    fn prepare_positive_prefix(&mut self, request: PositivePrefix) -> Result<(), Self::Error> {
        let p = &mut self.continuation.post.parts;
        self.erasure.prepare_positive(request, &p.noise_rows, p.cursor_count, &p.cursor_words,
            &mut self.continuation.held).map_err(Error::Service)?;
        p.cursor_halted = true;
        Ok(())
    }
    fn erase_positive_prefix(&mut self, request: PositivePrefix) -> Result<(), Self::Error> {
        let p = &mut self.continuation.post.parts;
        let result = self.erasure.erase_positive(request, CurrentFeatures {
            rows: &mut p.noise_rows, logical_cursor_count: &mut p.cursor_count, cursor_words: &mut p.cursor_words },
            &mut self.continuation.held);

        p.cursor_halted = true;
        result.map_err(Error::Service)
    }
    fn current_cursor_words_after_erase(&mut self) -> Result<&mut [u32], Self::Error> {
        let p = &mut self.continuation.post.parts;
        self.erasure.admit_current_cursors(&p.noise_rows, p.cursor_count, &p.cursor_words,
            &mut self.continuation.held).map_err(Error::Service)?;
        p.cursor_words.get_mut(..p.cursor_count).ok_or(Error::CurrentCursorStorage)
    }
}
