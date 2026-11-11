use std::collections::VecDeque;
use crate::{cepstral::queue, conditional_gate::Batch, ltv_caller_second_values as graph};
pub use graph::{ReadyForReceiverSelection, SecondSignalProgress, SecondSignalStage};
#[path = "ltv_second_refill_services.rs"]
mod services;
#[path = "ltv_second_append.rs"]
pub mod append;

pub struct Parts { pub width: usize, pub fifo: VecDeque<Option<Vec<f32>>> }
pub struct State { parts: Parts }
pub struct BindingFailure { pub parts: Parts }
impl State {
    pub fn from_explicit_parts(parts: Parts) -> Result<Self, BindingFailure> {
        if parts.width == 0 || parts.width > i32::MAX as usize || parts.fifo.len() > i32::MAX as usize
            || parts.fifo.iter().any(|row| row.as_ref().is_none_or(|row| row.len() != parts.width)) {
            return Err(BindingFailure { parts });
        }
        Ok(Self { parts })
    }
    pub fn parts(&self) -> &Parts { &self.parts }
    pub fn into_parts(self) -> Parts { self.parts }
}
pub struct Request {
    pub selection: queue::refill::Selection,
    pub signal_output: Batch, pub coefficient_output: Batch,

    pub signal_status: u8, pub coefficient_status: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { WrongQueue, SignalCountChanged, CoefficientCountChanged }
pub struct StartFailure<P, H> {
    pub cause: Domain, pub signal: State, pub coefficients: queue::Completed<P>, pub held: H, pub request: Request,
}
pub enum Coefficients<P> {
    Before(queue::Completed<P>), Active(queue::refill::WithMixer<P>),
    After(queue::refill::WithMixerCompleted<P>), Refused(queue::refill::WithMixerStartFailure<P>), Vacant,
}
pub enum ServiceError {
    Allocation(std::collections::TryReserveError), CountDomain(i64), Control(&'static str),
    CoefficientStart(queue::refill::Domain), Coefficient,
}
pub struct Trace {
    pub selection: queue::refill::Selection, pub old_status: u8,
    pub old_count: Option<usize>, pub released_old: Vec<Vec<f32>>,
    pub pending_retirement: Option<Vec<f32>>, pub reset_end: bool,
    pub snapshot_count: Option<usize>, pub moved: usize,
    pub fresh_count_before_erase: Option<usize>, pub erased: Option<usize>,
}
struct Owner<P, H> {
    held: H, signal: State, output: Batch, coefficients: Coefficients<P>,
    coefficient_output: Option<Batch>, coefficient_status: u8, trace: Trace,
}
pub struct Prepared<P, H> {
    owner: Owner<P, H>, work: graph::SecondSignalWork,
    failure: Option<graph::SecondSignalError<ServiceError>>, ready: Option<ReadyForReceiverSelection>,
}
pub enum Poll<'a> { Pending(SecondSignalProgress), Complete, Failed(&'a graph::SecondSignalError<ServiceError>) }
pub struct Completed<P, H> { prepared: Prepared<P, H> }
pub struct CompletedParts<P, H> {
    pub held: H, pub signal: State, pub coefficients: queue::refill::WithMixerCompleted<P>,
    pub input: Batch, pub trace: Trace, pub readiness: ReadyForReceiverSelection,
}
pub struct SecondHistory { pub signal: State, pub trace: Trace, pub readiness: ReadyForReceiverSelection }
pub struct SecondTrace { pub trace: Trace, pub readiness: ReadyForReceiverSelection }
impl<D, P, A, X, Y, C> crate::cepstral_queue_model::Completed<D, P, A, X, Y, C> {

    pub fn into_second_signal_refill(self, signal: State, request: Request)
        -> Result<Prepared<C, crate::cepstral_model::Upstream<D, P, A, X, Y>>,
            StartFailure<C, crate::cepstral_model::Upstream<D, P, A, X, Y>>>
    {
        let (upstream, coefficients) = self.into_parts();
        Prepared::from_completed_queues(signal, coefficients, upstream, request)
    }
}
impl<P, H> Prepared<P, H> {

    pub fn from_completed_queues(signal: State, coefficients: queue::Completed<P>, held: H, request: Request)
        -> Result<Self, StartFailure<P, H>>
    {
        let selection = request.selection;
        let cause = if selection.queue() != queue::refill::Queue::At328 { Some(Domain::WrongQueue) }
            else if u64::from(selection.request_word()) != signal.parts.fifo.len() as u64 { Some(Domain::SignalCountChanged) }
            else if selection.loaded_count() != coefficients.queues().parts().queued[1].len() as u64 {
                Some(Domain::CoefficientCountChanged)
            } else { None };
        if let Some(cause) = cause { return Err(StartFailure { cause, signal, coefficients, held, request }); }
        let trace = Trace { selection, old_status: request.signal_output.status, old_count: None,
            released_old: Vec::new(), pending_retirement: None, reset_end: false,
            snapshot_count: None, moved: 0, fresh_count_before_erase: None, erased: None };
        Ok(Self { owner: Owner { held, signal, output: request.signal_output,
            coefficients: Coefficients::Before(coefficients), coefficient_output: Some(request.coefficient_output),
            coefficient_status: request.coefficient_status, trace },
            work: graph::SecondSignalWork::after_explicit_selection(selection.signed_loop_count(), request.signal_status),
            failure: None, ready: None })
    }
    pub fn held(&self) -> &H { &self.owner.held }
    pub fn signal(&self) -> &State { &self.owner.signal }
    pub fn output(&self) -> &Batch { &self.owner.output }
    pub fn coefficients(&self) -> &Coefficients<P> { &self.owner.coefficients }
    pub fn trace(&self) -> &Trace { &self.owner.trace }
    pub fn progress(&self) -> SecondSignalProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&graph::SecondSignalError<ServiceError>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> Poll<'_> {
        if self.failure.is_none() && self.ready.is_none() {
            match graph::advance(&mut self.work, &mut self.owner, budget) {
                Ok(graph::SecondSignalAdvance::Paused) => {},
                Ok(graph::SecondSignalAdvance::ReadyForReceiverSelection(ready)) => self.ready = Some(ready),
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { Poll::Failed(error) }
        else if self.ready.is_some() { Poll::Complete }
        else { Poll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<P, H>, Self> {
        if self.failure.is_some() || self.ready.is_none() || !matches!(&self.owner.coefficients, Coefficients::After(_)) {
            return Err(self);
        }
        Ok(Completed { prepared: self })
    }
}
impl<P, H> Completed<P, H> {
    pub fn prepared(&self) -> &Prepared<P, H> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<P, H> {
        let ready = self.prepared.ready.expect("private second-refill completion marker");
        let owner = self.prepared.owner;
        let coefficients = match owner.coefficients { Coefficients::After(completed) => completed,
            _ => unreachable!("private coefficient completion checked before handback") };
        CompletedParts { held: owner.held, signal: owner.signal, coefficients,
            input: owner.output, trace: owner.trace, readiness: ready }
    }
}
impl<D, P, A, X, Y, C> Completed<C, crate::cepstral_model::Upstream<D, P, A, X, Y>> {

    pub fn into_filter<M, F, K: crate::ltv_filter::PairedKernel, O>(self,
        state: crate::ltv_filter::State<M, F, K>, frame: i32, observer: Option<O>)
        -> Result<crate::ltv_filter_model::Run<D, P, A, X, Y, C, M, F, K, O>,
            crate::ltv_filter_model::StartFailure<D, P, A, X, Y, C, M, F, K, O>>
    {
        let parts = self.into_parts();
        let refill = crate::cepstral_queue_model::Refilled::from_completed_parts(parts.held, parts.coefficients);
        let second = SecondHistory { signal: parts.signal, trace: parts.trace, readiness: parts.readiness };
        refill.into_second_filter(state, parts.input, frame, observer, second)
    }
    pub fn into_filter_without_observer<M, F, K: crate::ltv_filter::PairedKernel>(self,
        state: crate::ltv_filter::State<M, F, K>, frame: i32)
        -> Result<crate::ltv_filter_model::Run<D, P, A, X, Y, C, M, F, K, std::convert::Infallible>,
            crate::ltv_filter_model::StartFailure<D, P, A, X, Y, C, M, F, K, std::convert::Infallible>>
    {
        self.into_filter(state, frame, None)
    }
}
