use crate::{conditional_gate::Batch, ltv_coefficient_math::{Provider, ScalarUpper},
    ltv_coefficient_values::CoefficientError, paired_features, paired_input, paired_post::Module,
    paired_interpolation_values as graph};
pub use graph::{InterpolationComplete, InterpolationProgress};
#[path = "paired_interpolation_services.rs"]
mod services;
#[path = "paired_interpolation_rows.rs"]
mod rows;

pub trait Storage {
    type Error;
    fn create_history(&mut self, width: usize) -> Result<Vec<f32>, Self::Error>;
    fn prepare336(&mut self, output: &mut Batch, count: usize, width: usize) -> Result<(), Self::Error>;
}
pub struct Parts<S, U: ScalarUpper> {
    pub storage: S, pub math: Provider<U>, pub history8: Option<Vec<f32>>,
    pub seen16: u64, pub factor24: i32, pub width28: i32, pub ended32: u8, pub output336: Batch,
}
pub struct State<S, U: ScalarUpper> { parts: Parts<S, U>, halted: bool }
pub(crate) struct MathSuspended<S> {
    storage: S, history8: Option<Vec<f32>>, seen16: u64, factor24: i32,
    width28: i32, ended32: u8, output336: Batch, halted: bool,
}
impl<S> MathSuspended<S> {
    pub(crate) fn storage(&self) -> &S { &self.storage }
    pub(crate) fn restore<U: ScalarUpper>(self, math: Provider<U>) -> State<S, U> {
        State { parts: Parts { storage: self.storage, math, history8: self.history8, seen16: self.seen16,
            factor24: self.factor24, width28: self.width28, ended32: self.ended32, output336: self.output336 }, halted: self.halted }
    }
}

pub(crate) struct OutputSuspended<S, U: ScalarUpper> {
    storage: S, math: Provider<U>, history8: Option<Vec<f32>>,
    seen16: u64, factor24: i32, width28: i32, ended32: u8,
}
impl<S, U: ScalarUpper> OutputSuspended<S, U> {
    pub(crate) fn storage(&self) -> &S { &self.storage }
    pub(crate) fn math(&self) -> &Provider<U> { &self.math }
    pub(crate) fn history(&self) -> Option<&[f32]> { self.history8.as_deref() }
    pub(crate) fn metadata(&self) -> (u64, i32, i32, u8) { (self.seen16, self.factor24, self.width28, self.ended32) }
    pub(crate) fn restore_completed(self, output336: Batch) -> State<S, U> {
        State { parts: Parts { storage: self.storage, math: self.math, history8: self.history8,
            seen16: self.seen16, factor24: self.factor24, width28: self.width28, ended32: self.ended32, output336 }, halted: false }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { Factor, History, AlreadyEnded, Count, Rate }
pub struct BindingFailure<S, U: ScalarUpper> { pub cause: Domain, pub parts: Parts<S, U> }
pub struct StartFailure<S, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    pub cause: Domain, pub state: State<S, U>, pub upstream: paired_features::Completed<P, G, M, R, H>,
}
impl<S, U: ScalarUpper> State<S, U> {
    pub fn from_explicit_parts(parts: Parts<S, U>) -> Result<Self, BindingFailure<S, U>> {
        let cause = if parts.factor24 <= 0 { Some(Domain::Factor) }
            else if parts.seen16 != 0 && (parts.width28 <= 0
                || parts.history8.as_ref().is_none_or(|row| row.len() != parts.width28 as usize)) { Some(Domain::History) }
            else { None };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }
        Ok(Self { parts, halted: false })
    }
    pub fn parts(&self) -> &Parts<S, U> { &self.parts }
    pub fn is_halted(&self) -> bool { self.halted }
    pub fn into_parts(self) -> Parts<S, U> { self.parts }
    pub(crate) fn suspend_math(self) -> (MathSuspended<S>, Provider<U>) {
        let Parts { storage, math, history8, seen16, factor24, width28, ended32, output336 } = self.parts;
        (MathSuspended { storage, history8, seen16, factor24, width28, ended32, output336, halted: self.halted }, math)
    }
    pub(crate) fn suspend_output_for_append(self) -> (OutputSuspended<S, U>, Batch) {
        let Parts { storage, math, history8, seen16, factor24, width28, ended32, output336 } = self.parts;
        (OutputSuspended { storage, math, history8, seen16, factor24, width28, ended32 }, output336)
    }
    pub fn begin<P: paired_features::Frames, G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H>(
        mut self, upstream: paired_features::Completed<P, G, M, R, H>)
        -> Result<Prepared<S, U, P, G, M, R, H>, StartFailure<S, U, P, G, M, R, H>> where S: Storage {
        let input = upstream.prepared().input().prepared();
        let source = input.state().parts();
        let count = input.paired_features().rows.len();
        let cause = if self.parts.ended32 != 0 { Some(Domain::AlreadyEnded) }
            else if count > i32::MAX as usize || self.parts.seen16.checked_add(count as u64).is_none() { Some(Domain::Count) }
            else if source.hop / source.groups != self.parts.factor24 as usize { Some(Domain::Rate) }
            else { None };
        if let Some(cause) = cause { return Err(StartFailure { cause, state: self, upstream }); }
        let seen = self.parts.seen16;
        let factor = self.parts.factor24;
        let ended = input.paired_features().status;
        self.halted = true; self.parts.ended32 = ended;
        Ok(Prepared { owner: Owner { state: self, upstream,
            trace: Trace { history_width: None, history_installed: false,
                retired_history: Vec::new(), pending_history: None, rejected_history: None,
                retired_output: Vec::new(), pending_output: None, preparation: None,
                arithmetic_admitted: false, row: None, copies: 0, lane_stores: 0, last_bits: None } },
            work: graph::InterpolationWork::after_explicit_prefix(count, seen, factor, ended), failure: None, complete: None })
    }
}
pub struct Trace {
    pub history_width: Option<usize>, pub history_installed: bool,

    pub retired_history: Vec<Vec<f32>>, pub pending_history: Option<Vec<f32>>, pub rejected_history: Option<Vec<f32>>,
    pub retired_output: Vec<Vec<f32>>, pub pending_output: Option<Vec<f32>>,
    pub preparation: Option<usize>, pub arithmetic_admitted: bool, pub row: Option<usize>,
    pub copies: usize, pub lane_stores: usize, pub last_bits: Option<u32>,
}
pub enum ServiceError<E> {
    Create(E), Prepare(E), Math(CoefficientError),
    Input(usize), Output(usize), Width, History, WidthDomain,
    Allocation(std::collections::TryReserveError), Control(&'static str),
}
struct Owner<S, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    state: State<S, U>, upstream: paired_features::Completed<P, G, M, R, H>, trace: Trace,
}
pub struct Prepared<S: Storage, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    owner: Owner<S, U, P, G, M, R, H>, work: graph::InterpolationWork,
    failure: Option<graph::InterpolationError<ServiceError<S::Error>>>, complete: Option<InterpolationComplete>,
}
pub enum Poll<'a, E> {
    Pending(InterpolationProgress), Complete, Failed(&'a graph::InterpolationError<ServiceError<E>>),
}
pub struct Completed<S: Storage, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> { prepared: Prepared<S, U, P, G, M, R, H> }
pub struct CompletedParts<S, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    pub state: State<S, U>, pub upstream: paired_features::Completed<P, G, M, R, H>,
    pub trace: Trace, pub interpolation: InterpolationComplete,
}
impl<S: Storage, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> Prepared<S, U, P, G, M, R, H> {
    pub fn state(&self) -> &State<S, U> { &self.owner.state }
    pub fn upstream(&self) -> &paired_features::Completed<P, G, M, R, H> { &self.owner.upstream }
    pub fn trace(&self) -> &Trace { &self.owner.trace }
    pub fn progress(&self) -> InterpolationProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&graph::InterpolationError<ServiceError<S::Error>>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> Poll<'_, S::Error> {
        if self.failure.is_none() && self.complete.is_none() {
            match graph::advance(&mut self.work, &mut self.owner, budget) {
                Ok(graph::InterpolationAdvance::Paused) => {},
                Ok(graph::InterpolationAdvance::Complete(marker)) => {
                    self.owner.state.halted = false; self.complete = Some(marker);
                }
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.complete.is_some() { Poll::Complete } else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<S, U, P, G, M, R, H>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) } else { Ok(Completed { prepared: self }) }
    }
}
impl<S: Storage, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> Completed<S, U, P, G, M, R, H> {
    pub fn prepared(&self) -> &Prepared<S, U, P, G, M, R, H> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<S, U, P, G, M, R, H> {
        let owner = self.prepared.owner;
        CompletedParts { state: owner.state, upstream: owner.upstream, trace: owner.trace,
            interpolation: self.prepared.complete.expect("private interpolation completion") }
    }
}
