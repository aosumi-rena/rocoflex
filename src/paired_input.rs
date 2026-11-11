use crate::{conditional_gate::Batch, paired_input_values as graph, paired_post::Module};
pub use graph::{ScalarInputComplete, ScalarInputProgress, ScalarInputStage};
#[path = "paired_input_services.rs"]
mod services;

pub trait Rows {
    type Error;
    fn prepare112(&mut self, destination: &mut Batch, required_rows: usize) -> Result<(), Self::Error>;
}
pub struct Parts<M, R> {
    pub module48: M, pub rows: R, pub hop: usize, pub groups: usize,
    pub latency496: i32, pub ended517: u8, pub input112: Batch, pub feature144: Batch,
}
pub struct State<M, R> { parts: Parts<M, R>, halted: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { Hop, Groups, Ended, AlreadyEnded, InputCount, InputStatus, Count }
pub struct BindingFailure<M, R> { pub cause: Domain, pub parts: Parts<M, R> }
pub struct StartFailure<M, R, H> {
    pub cause: Domain, pub state: State<M, R>, pub signal: Batch, pub paired_features: Batch, pub held: H,
}
impl<M, R> State<M, R> {
    pub fn from_explicit_parts(parts: Parts<M, R>) -> Result<Self, BindingFailure<M, R>> {
        let cause = if parts.hop == 0 || parts.hop > i32::MAX as usize { Some(Domain::Hop) }
            else if parts.groups < 2 || parts.groups > i32::MAX as usize || parts.groups & 1 != 0
                || parts.hop % parts.groups != 0 { Some(Domain::Groups) }
            else if parts.ended517 > 1 { Some(Domain::Ended) } else { None };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }
        Ok(Self { parts, halted: false })
    }
    pub fn parts(&self) -> &Parts<M, R> { &self.parts }
    pub fn is_halted(&self) -> bool { self.halted }
    pub fn into_parts(self) -> Parts<M, R> { self.parts }
    pub fn begin<H>(mut self, signal: Batch, paired_features: Batch, held: H)
        -> Result<Prepared<M, R, H>, StartFailure<M, R, H>> where M: Module, R: Rows
    {
        let cause = if self.parts.ended517 != 0 { Some(Domain::AlreadyEnded) }
            else if paired_features.rows.len() != signal.rows.len() { Some(Domain::InputCount) }
            else if paired_features.status != signal.status { Some(Domain::InputStatus) }
            else if signal.rows.len().checked_mul(self.parts.hop).is_none_or(|n| n > i32::MAX as usize) {
                Some(Domain::Count)
            } else { None };
        if let Some(cause) = cause { return Err(StartFailure { cause, state: self, signal, paired_features, held }); }
        let required_rows = signal.rows.len() * self.parts.hop;
        self.halted = true;
        if signal.status != 0 { self.parts.ended517 = 1; }
        let work = graph::ScalarInputWork::after_explicit_prefix(required_rows, self.parts.ended517);
        Ok(Prepared { owner: Owner { state: self, signal, paired_features, held,
            trace: Trace { preparation: None, frame: None, source_width: None,
                copied: 0, last_bits: None, module48_completed: false,
                retired_front: 0, retired_last: false, retired_rows: Vec::new(), pending_retirement: None } },
            work, failure: None, complete: None })
    }
}
pub struct Trace {
    pub preparation: Option<usize>, pub frame: Option<usize>, pub source_width: Option<usize>,
    pub copied: usize, pub last_bits: Option<u32>, pub module48_completed: bool,
    pub retired_front: usize, pub retired_last: bool,

    pub retired_rows: Vec<Vec<f32>>, pub pending_retirement: Option<Vec<f32>>,
}
pub enum ServiceError<M, R> {
    Prepare(R), Module48(M), Frame(usize), Row(usize),
    Allocation(std::collections::TryReserveError), Control(&'static str),
}
struct Owner<M, R, H> { state: State<M, R>, signal: Batch, paired_features: Batch, held: H, trace: Trace }
pub struct Prepared<M: Module, R: Rows, H> {
    owner: Owner<M, R, H>, work: graph::ScalarInputWork,
    failure: Option<graph::ScalarInputError<ServiceError<M::Error, R::Error>>>, complete: Option<ScalarInputComplete>,
}
pub enum Poll<'a, M, R> {
    Pending(ScalarInputProgress), Complete, Failed(&'a graph::ScalarInputError<ServiceError<M, R>>),
}
pub struct Completed<M: Module, R: Rows, H> { prepared: Prepared<M, R, H> }
pub struct CompletedParts<M, R, H> {
    pub state: State<M, R>, pub signal: Batch, pub paired_features: Batch, pub held: H,
    pub trace: Trace, pub scalar_input: ScalarInputComplete,
}
impl<M: Module, R: Rows, H> Prepared<M, R, H> {
    pub fn state(&self) -> &State<M, R> { &self.owner.state }
    pub fn signal(&self) -> &Batch { &self.owner.signal }
    pub fn paired_features(&self) -> &Batch { &self.owner.paired_features }
    pub fn held(&self) -> &H { &self.owner.held }
    pub fn trace(&self) -> &Trace { &self.owner.trace }
    pub fn progress(&self) -> ScalarInputProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&graph::ScalarInputError<ServiceError<M::Error, R::Error>>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> Poll<'_, M::Error, R::Error> {
        if self.failure.is_none() && self.complete.is_none() {
            match graph::advance(&mut self.work, &mut self.owner, budget) {
                Ok(graph::ScalarInputAdvance::Paused) => {},
                Ok(graph::ScalarInputAdvance::Complete(marker)) => {
                    self.owner.state.halted = false; self.complete = Some(marker);
                }
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.complete.is_some() { Poll::Complete } else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<M, R, H>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) } else { Ok(Completed { prepared: self }) }
    }
}
impl<M: Module, R: Rows, H> Completed<M, R, H> {
    pub fn prepared(&self) -> &Prepared<M, R, H> { &self.prepared }

    pub(crate) fn split_held<T, X>(self, split: impl FnOnce(H) -> (T, X)) -> (Completed<M, R, T>, X) {
        let Prepared { owner, work, failure, complete } = self.prepared;
        let Owner { state, signal, paired_features, held, trace } = owner;
        let (held, extra) = split(held);
        (Completed { prepared: Prepared { owner: Owner { state, signal, paired_features, held, trace },
            work, failure, complete } }, extra)
    }
    pub fn into_parts(self) -> CompletedParts<M, R, H> {
        let owner = self.prepared.owner;
        CompletedParts { state: owner.state, signal: owner.signal, paired_features: owner.paired_features,
            held: owner.held, trace: owner.trace,
            scalar_input: self.prepared.complete.expect("private scalar-input completion") }
    }
}
