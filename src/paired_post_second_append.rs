use super::{Completed as Retired, CompletedParts as RetiredParts, Erasure, RetirementComplete};
use crate::{conditional_gate::Batch, ltv_coefficient_math::ScalarUpper,
    ltv_second_refill::{self as refill, append}, paired_post::{packing, State},
    paired_scalar_packing::CurrentRows};

pub struct Suspended<A, B, U: ScalarUpper, H, P, R> {
    continuation: packing::OutputSuspended<A, B, U, H, P>, erasure: R, retirement: RetirementComplete,
}
impl<A, B, U: ScalarUpper, H, P, R> Suspended<A, B, U, H, P, R> {
    pub fn post(&self) -> &State<A, B, U> { self.continuation.post() }
    pub fn held(&self) -> &H { self.continuation.held() }
    pub fn erasure(&self) -> &R { &self.erasure }
    pub fn retirement(&self) -> &RetirementComplete { &self.retirement }
    fn restore(self, source: Batch) -> RetiredParts<A, B, U, H, P, R> {
        RetiredParts { continuation: self.continuation.restore(source), erasure: self.erasure,
            retirement: self.retirement }
    }
}
pub struct Prepared<A, B, U: ScalarUpper, H, P, R> {
    append: append::Prepared<Suspended<A, B, U, H, P, R>>,
}
pub struct Completed<A, B, U: ScalarUpper, H, P, R> {
    append: append::Completed<Suspended<A, B, U, H, P, R>>,
}
pub struct History<A, B, U: ScalarUpper, H, P, R> {
    pub retired: RetiredParts<A, B, U, H, P, R>, pub append_trace: append::Trace,
}
pub struct CompletedParts<A, B, U: ScalarUpper, H, P, R> {
    pub signal: refill::State, pub history: History<A, B, U, H, P, R>,
}
impl<A, B, U: ScalarUpper, H, P: CurrentRows, R: Erasure> Retired<A, B, U, H, P, R> {

    pub fn into_second_append(self, signal: refill::State)
        -> Result<Prepared<A, B, U, H, P, R>, append::StartFailure<Suspended<A, B, U, H, P, R>>>
    {
        let retired = self.into_parts();
        let (continuation, output) = retired.continuation.suspend_output();
        let held = Suspended { continuation, erasure: retired.erasure, retirement: retired.retirement };
        signal.append_explicit_returned_values(output, held).map(|append| Prepared { append })
    }
}
impl<A, B, U: ScalarUpper, H, P, R> Prepared<A, B, U, H, P, R> {

    pub fn append(&self) -> &append::Prepared<Suspended<A, B, U, H, P, R>> { &self.append }
    pub fn poll(&mut self, budget: usize) -> append::Poll<'_> { self.append.poll(budget) }
    pub fn into_completed(self) -> Result<Completed<A, B, U, H, P, R>, Self> {
        match self.append.into_completed() {
            Ok(append) => Ok(Completed { append }), Err(append) => Err(Self { append }),
        }
    }
}
impl<A, B, U: ScalarUpper, H, P, R> Completed<A, B, U, H, P, R> {
    pub fn append(&self) -> &append::Completed<Suspended<A, B, U, H, P, R>> { &self.append }
    pub fn into_parts(self) -> CompletedParts<A, B, U, H, P, R> {
        let parts = self.append.into_parts();
        CompletedParts { signal: parts.signal, history: History {
            retired: parts.held.restore(parts.source), append_trace: parts.trace } }
    }

    pub fn into_refill<C>(self, coefficients: crate::cepstral::queue::Completed<C>, request: refill::Request)
        -> Result<refill::Prepared<C, History<A, B, U, H, P, R>>,
            refill::StartFailure<C, History<A, B, U, H, P, R>>>
    {
        let parts = self.into_parts();
        refill::Prepared::from_completed_queues(parts.signal, coefficients, parts.history, request)
    }
}
