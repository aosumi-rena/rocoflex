use super::*;
use crate::{header_append_values as append, paired_interpolation as interpolation,
    ltv_coefficient_math::{Provider, ScalarUpper}};
pub use append::{AppendComplete, AppendProgress};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendDomain { OutputCount }
pub struct AppendStartFailure<W, S: interpolation::Storage, U: ScalarUpper, P: paired_features::Frames,
    G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H> {
    pub cause: AppendDomain, pub state: State<W>, pub interpolation: interpolation::Completed<S, U, P, G, M, R, H>,
}
pub struct AppendTrace {
    pub original_count: usize, pub original_status: u8, pub appended: usize, pub donor_finished: bool,
}
pub enum AppendServiceError { Allocation(std::collections::TryReserveError), Control(&'static str) }
struct AppendOwner<W, S, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    state: State<W>, suspended: interpolation::OutputSuspended<S, U>, output336: Batch,
    upstream: paired_features::Completed<P, G, M, R, H>, interpolation_trace: interpolation::Trace,
    interpolation_complete: interpolation::InterpolationComplete, trace: AppendTrace,
}
pub struct Prepared<W, S: interpolation::Storage, U: ScalarUpper, P: paired_features::Frames,
    G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H> {
    owner: AppendOwner<W, S, U, P, G, M, R, H>, work: append::AppendWork,
    failure: Option<append::AppendError<AppendServiceError>>, complete: Option<AppendComplete>,
}
pub enum AppendPoll<'a> { Pending(AppendProgress), Complete, Failed(&'a append::AppendError<AppendServiceError>) }
pub struct Completed<W, S: interpolation::Storage, U: ScalarUpper, P: paired_features::Frames,
    G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H> { prepared: Prepared<W, S, U, P, G, M, R, H> }
pub struct CompletedParts<W, S, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> {
    pub state: State<W>, pub interpolation: interpolation::State<S, U>,
    pub upstream: paired_features::Completed<P, G, M, R, H>,
    pub interpolation_trace: interpolation::Trace, pub interpolation_complete: interpolation::InterpolationComplete,
    pub trace: AppendTrace, pub appended: AppendComplete,
}
impl<W> State<W> {
    pub fn append_interpolation<S: interpolation::Storage, U: ScalarUpper, P: paired_features::Frames,
        G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H>(mut self,
        completed: interpolation::Completed<S, U, P, G, M, R, H>)
        -> Result<Prepared<W, S, U, P, G, M, R, H>, AppendStartFailure<W, S, U, P, G, M, R, H>> {
        let count = completed.prepared().state().parts().output336.rows.len();
        if count != completed.prepared().progress().written {
            return Err(AppendStartFailure { cause: AppendDomain::OutputCount, state: self, interpolation: completed });
        }
        let interpolation::CompletedParts { state, upstream, trace: interpolation_trace,
            interpolation: interpolation_complete } = completed.into_parts();
        let (suspended, output336) = state.suspend_output_for_append();
        let original_status = output336.status;
        self.halted = true;
        Ok(Prepared { owner: AppendOwner { state: self, suspended, output336, upstream,
            interpolation_trace, interpolation_complete,
            trace: AppendTrace { original_count: count, original_status, appended: 0, donor_finished: false } },
            work: append::AppendWork::after_explicit_admission(count), failure: None, complete: None })
    }
}
impl<W, S: interpolation::Storage, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> Prepared<W, S, U, P, G, M, R, H> {
    pub fn state(&self) -> &State<W> { &self.owner.state }
    pub fn output336(&self) -> &Batch { &self.owner.output336 }
    pub fn upstream(&self) -> &paired_features::Completed<P, G, M, R, H> { &self.owner.upstream }
    pub fn interpolation_storage(&self) -> &S { self.owner.suspended.storage() }
    pub fn interpolation_math(&self) -> &Provider<U> { self.owner.suspended.math() }
    pub fn interpolation_history(&self) -> Option<&[f32]> { self.owner.suspended.history() }
    pub fn interpolation_metadata(&self) -> (u64, i32, i32, u8) { self.owner.suspended.metadata() }
    pub fn interpolation_trace(&self) -> &interpolation::Trace { &self.owner.interpolation_trace }
    pub fn trace(&self) -> &AppendTrace { &self.owner.trace }
    pub fn progress(&self) -> AppendProgress { self.work.progress() }
    pub fn failure(&self) -> Option<&append::AppendError<AppendServiceError>> { self.failure.as_ref() }
    pub fn poll(&mut self, budget: usize) -> AppendPoll<'_> {
        if self.failure.is_none() && self.complete.is_none() {
            match append::advance(&mut self.work, &mut self.owner, budget) {
                Ok(append::AppendAdvance::Paused) => {},
                Ok(append::AppendAdvance::Complete(marker)) => {
                    self.owner.state.halted = false; self.complete = Some(marker);
                }
                Err(error) => self.failure = Some(error),
            }
        }
        if let Some(error) = &self.failure { AppendPoll::Failed(error) }
        else if self.complete.is_some() { AppendPoll::Complete } else { AppendPoll::Pending(self.work.progress()) }
    }
    pub fn into_completed(self) -> Result<Completed<W, S, U, P, G, M, R, H>, Self> {
        if self.failure.is_some() || self.complete.is_none() { Err(self) } else { Ok(Completed { prepared: self }) }
    }
}
impl<W, S: interpolation::Storage, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> Completed<W, S, U, P, G, M, R, H> {
    pub fn prepared(&self) -> &Prepared<W, S, U, P, G, M, R, H> { &self.prepared }
    pub fn into_parts(self) -> CompletedParts<W, S, U, P, G, M, R, H> {
        let o = self.prepared.owner;
        CompletedParts { state: o.state, interpolation: o.suspended.restore_completed(o.output336),
            upstream: o.upstream, interpolation_trace: o.interpolation_trace,
            interpolation_complete: o.interpolation_complete, trace: o.trace,
            appended: self.prepared.complete.expect("private append completion") }
    }

    pub fn into_condition(self) -> Result<ConditionPrepared<W, S, U, P, G, M, R, H>,
        ConditionStartFailure<W, S, U, P, G, M, R, H>> where W: Worker {
        let parts = self.into_parts();
        let history = History { interpolation: parts.interpolation, interpolation_trace: parts.interpolation_trace,
            interpolation_complete: parts.interpolation_complete, append_trace: parts.trace, appended: parts.appended };
        match parts.state.begin(parts.upstream) {
            Ok(inner) => Ok(ConditionPrepared { inner, history }),
            Err(start) => Err(ConditionStartFailure { start, history }),
        }
    }
}
impl<W, S, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> append::AppendServices for AppendOwner<W, S, U, P, G, M, R, H> {
    type Error = AppendServiceError;
    fn append_admitted_ordinal(&mut self, ordinal: usize) -> Result<(), Self::Error> {
        if ordinal >= self.output336.rows.len() { return Err(AppendServiceError::Control("admitted336 ordinal")); }

        self.state.parts.conditions432.try_reserve(1).map_err(AppendServiceError::Allocation)?;
        let row = std::mem::take(&mut self.output336.rows[ordinal]);
        self.state.parts.conditions432.push_back(Some(row)); self.trace.appended += 1; Ok(())
    }
    fn finish_donor_header(&mut self) -> Result<(), Self::Error> {
        if self.trace.appended != self.trace.original_count || self.output336.rows.iter().any(|row| !row.is_empty()) {
            return Err(AppendServiceError::Control("zeroed336 donors before finish"));
        }

        self.output336.rows.clear(); self.output336.status = 0; self.trace.donor_finished = true; Ok(())
    }
}
pub struct History<S, U: ScalarUpper> {
    pub interpolation: interpolation::State<S, U>, pub interpolation_trace: interpolation::Trace,
    pub interpolation_complete: interpolation::InterpolationComplete, pub append_trace: AppendTrace,
    pub appended: AppendComplete,
}
pub struct ConditionStartFailure<W: Worker, S, U: ScalarUpper, P: paired_features::Frames,
    G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H> {
    pub start: super::StartFailure<W, P, G, M, R, H>, pub history: History<S, U>,
}
pub struct ConditionPrepared<W: Worker, S, U: ScalarUpper, P: paired_features::Frames,
    G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H> {
    inner: super::Prepared<W, P, G, M, R, H>, history: History<S, U>,
}
pub struct ConditionCompleted<W: Worker, S, U: ScalarUpper, P: paired_features::Frames,
    G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H> {
    inner: super::Completed<W, P, G, M, R, H>, history: History<S, U>,
}
impl<W: Worker, S, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> ConditionPrepared<W, S, U, P, G, M, R, H> {
    pub fn prepared(&self) -> &super::Prepared<W, P, G, M, R, H> { &self.inner }
    pub fn history(&self) -> &History<S, U> { &self.history }
    pub fn poll(&mut self, budget: usize) -> super::Poll<'_, W::Error> { self.inner.poll(budget) }
    pub fn into_completed(self) -> Result<ConditionCompleted<W, S, U, P, G, M, R, H>, Self> {
        let Self { inner, history } = self;
        match inner.into_completed() {
            Ok(inner) => Ok(ConditionCompleted { inner, history }),
            Err(inner) => Err(Self { inner, history }),
        }
    }
}
impl<W: Worker, S, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> ConditionCompleted<W, S, U, P, G, M, R, H> {
    pub fn prepared(&self) -> &super::Prepared<W, P, G, M, R, H> { self.inner.prepared() }
    pub fn history(&self) -> &History<S, U> { &self.history }
    pub fn into_parts(self) -> (super::CompletedParts<W, P, G, M, R, H>, History<S, U>) {
        (self.inner.into_parts(), self.history)
    }
}
