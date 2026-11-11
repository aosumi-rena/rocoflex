use std::collections::VecDeque;
use crate::{conditional_gate::Batch, ltv_coefficient_math::{Provider, ScalarUpper},
    paired_post_noise as noise, paired_post_values as graph};
pub use graph::{PostComplete, PostProgress, PostStage};
#[path = "paired_post_services.rs"]
mod services;
#[path = "paired_post_packing.rs"]
pub mod packing;
#[path = "paired_post_retirement.rs"]
pub mod retirement;

pub trait Module {
    type Error;
    fn forward(&mut self, input: &mut Batch, output: &mut Batch) -> Result<(), Self::Error>;
}
pub struct Parts<A, B, U: ScalarUpper> {
    pub module56: A, pub module40: B, pub math: Provider<U>,
    pub groups: usize, pub enabled: bool, pub ended: u8,
    pub cursor_index: i32, pub cursor_count: usize, pub cursor_words: Vec<u32>, pub cursor_halted: bool,
    pub noise_rows: VecDeque<crate::paired_features::FeatureRow>, pub stage272: Batch,

    pub final304: Option<Batch>,
}
pub struct State<A, B, U: ScalarUpper> { parts: Parts<A, B, U>, halted: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { Groups, CursorStorage, CursorIndex, CursorHalted, Ended, MissingFinalOutput }
pub struct BindingFailure<A, B, U: ScalarUpper> { pub cause: Domain, pub parts: Parts<A, B, U> }
impl<A, B, U: ScalarUpper> State<A, B, U> {
    pub fn from_explicit_parts(parts: Parts<A, B, U>) -> Result<Self, BindingFailure<A, B, U>> {
        let cause = if parts.groups == 0 || parts.groups > i32::MAX as usize / 2 || parts.groups & 1 != 0 {
                Some(Domain::Groups)
            } else if parts.cursor_count > parts.cursor_words.len() { Some(Domain::CursorStorage) }
            else if parts.enabled && parts.cursor_index < 0 { Some(Domain::CursorIndex) }
            else if parts.cursor_halted { Some(Domain::CursorHalted) }
            else if parts.ended > 1 { Some(Domain::Ended) }
            else if parts.final304.is_none() { Some(Domain::MissingFinalOutput) }
            else { None };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }
        Ok(Self { parts, halted: false })
    }
    pub fn parts(&self) -> &Parts<A, B, U> { &self.parts }
    pub fn into_parts(self) -> Parts<A, B, U> { self.parts }

    pub fn begin<H>(mut self, retained240: Batch, held: H) -> Prepared<A, B, U, H>
        where A: Module, B: Module
    {
        let work = graph::PostWork::after_explicit_admission(self.parts.enabled, self.parts.groups);
        self.halted = true;
        Prepared { owner: Owner { state: self, input240: retained240, held, trace: Trace::new() },
            work, failure: None, complete: None }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct NoisePreparation {
    pub index: Option<usize>, pub count: Option<usize>, pub cursor_word: Option<u32>,
    pub fifo_count: Option<u64>, pub available: Option<i32>,
    pub environment_attempted: bool, pub start: Option<usize>, pub end: Option<usize>,
}
pub struct Trace {
    pub module56_completed: bool, pub module40_completed: bool,
    pub noise: NoisePreparation, pub admitted_row: Option<usize>,
    pub noise_width: Option<usize>, pub target_width: Option<usize>,
    pub last_lane: Option<noise::LanePartial>, pub cursor_after: Option<u32>,
}
impl Trace {
    fn new() -> Self {
        Self { module56_completed: false, module40_completed: false,
            noise: NoisePreparation { index: None, count: None, cursor_word: None, fifo_count: None,
                available: None, environment_attempted: false, start: None, end: None },
            admitted_row: None, noise_width: None, target_width: None, last_lane: None, cursor_after: None }
    }
}
pub enum ServiceError<A, B> {
    Module56(A), Module40(B), Cursor, Count, Input, Width,
    Environment(crate::matrix_f32::Error),
    Lane(noise::LaneFailure<noise::Error<std::convert::Infallible>>),
    Control(&'static str),
}
struct Owner<A, B, U: ScalarUpper, H> { state: State<A, B, U>, input240: Batch, held: H, trace: Trace }
pub struct Prepared<A: Module, B: Module, U: ScalarUpper, H> {
    owner: Owner<A, B, U, H>, work: graph::PostWork,
    failure: Option<graph::PostError<ServiceError<A::Error, B::Error>>>, complete: Option<PostComplete>,
}
pub enum Poll<'a, A, B> { Pending(PostProgress), Complete, Failed(&'a graph::PostError<ServiceError<A, B>>) }
pub struct Completed<A: Module, B: Module, U: ScalarUpper, H> { prepared: Prepared<A, B, U, H> }
pub struct CompletedParts<A, B, U: ScalarUpper, H> {
    pub state: State<A, B, U>, pub input240: Batch, pub held: H, pub trace: Trace, pub post: PostComplete,
}
impl<A: Module, B: Module, U: ScalarUpper, H> Prepared<A, B, U, H> {
    pub fn state(&self) -> &State<A, B, U> { &self.owner.state }
    pub fn input240(&self) -> &Batch { &self.owner.input240 }
    pub fn held(&self) -> &H { &self.owner.held }
    pub fn trace(&self) -> &Trace { &self.owner.trace }
    pub fn progress(&self) -> PostProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&graph::PostError<ServiceError<A::Error, B::Error>>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> Poll<'_, A::Error, B::Error> {
        if self.failure.is_none() && self.complete.is_none() {
            match graph::advance(&mut self.work, &mut self.owner, budget) {
                Ok(graph::PostAdvance::Paused) => {},
                Ok(graph::PostAdvance::Complete(marker)) => {
                    self.complete = Some(marker); self.owner.state.halted = false;
                }
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.complete.is_some() { Poll::Complete }
        else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<A, B, U, H>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) }
        else { Ok(Completed { prepared: self }) }
    }
}
impl<A: Module, B: Module, U: ScalarUpper, H> Completed<A, B, U, H> {
    pub fn prepared(&self) -> &Prepared<A, B, U, H> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<A, B, U, H> {
        let owner = self.prepared.owner;
        CompletedParts { state: owner.state, input240: owner.input240, held: owner.held,
            trace: owner.trace, post: self.prepared.complete.expect("private post completion") }
    }
}
