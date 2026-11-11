use super::*;
use crate::{cepstral_model, cepstral_queue_model, ltv_filter, ltv_filter_model as filter,
    ltv_second_refill as refill};
use retirement::second_append as second;

pub struct SecondOwners<A, B, U: ScalarUpper, H, O, E, D, P, C, X, Y> {
    producer: second::History<A, B, U, H, O, E>,
    coefficients: cepstral_model::Upstream<D, P, C, X, Y>,
}
impl<A, B, U: ScalarUpper, H, O, E, D, P, C, X, Y> SecondOwners<A, B, U, H, O, E, D, P, C, X, Y> {
    pub fn producer(&self) -> &second::History<A, B, U, H, O, E> { &self.producer }
    pub fn coefficient_upstream(&self) -> &cepstral_model::Upstream<D, P, C, X, Y> { &self.coefficients }
}
pub struct FilterOwners<A, B, U: ScalarUpper, H, O, E> {
    producer: second::History<A, B, U, H, O, E>, complete: CompletedCall,
}
impl<A, B, U: ScalarUpper, H, O, E> FilterOwners<A, B, U, H, O, E> {
    pub fn producer(&self) -> &second::History<A, B, U, H, O, E> { &self.producer }
    pub fn completion(&self) -> &CompletedCall { &self.complete }
}

pub struct FilteredParts<A, B, U: ScalarUpper, H, O, E, D, P, C, X, Y, Q, M, F, K, V> {
    pub filter: filter::Continuation<D, P, C, X, Y, Q, M, F, K, V>,
    pub producer: second::History<A, B, U, H, O, E>, pub complete: CompletedCall,
}
pub struct JoinedFilterParts<J, F> {
    pub producer: J, pub filter: F, pub append_trace: refill::append::Trace, pub complete: CompletedCall,
}
impl<A, B, U: ScalarUpper, H, O, E> Carried<second::Completed<A, B, U, H, O, E>, CompletedCall> {

    pub fn into_refill_with_model<D, P, C, X, Y, Q>(self,
        coefficients: cepstral_queue_model::Completed<D, P, C, X, Y, Q>, request: refill::Request)
        -> Result<Carried<refill::Prepared<Q, SecondOwners<A, B, U, H, O, E, D, P, C, X, Y>>, CompletedCall>,
            Refused<refill::StartFailure<Q, SecondOwners<A, B, U, H, O, E, D, P, C, X, Y>>, CompletedCall>> {
        let parts = self.child.into_parts();
        let (coefficients, queues) = coefficients.into_parts();
        let held = SecondOwners { producer: parts.history, coefficients };
        match refill::Prepared::from_completed_queues(parts.signal, queues, held, request) {
            Ok(child) => Ok(Carried { child, held: self.held }),
            Err(start) => Err(Refused { start, held: self.held }),
        }
    }
}
impl<A, B, U: ScalarUpper, H, O, E, D, P, C, X, Y, Q>
    Carried<refill::Completed<Q, SecondOwners<A, B, U, H, O, E, D, P, C, X, Y>>, CompletedCall> {

    pub fn into_filter<M, F, K: ltv_filter::PairedKernel, V>(self,
        state: ltv_filter::State<M, F, K>, frame: i32, observer: Option<V>)
        -> Result<Carried<filter::Run<D, P, C, X, Y, Q, M, F, K, V>, FilterOwners<A, B, U, H, O, E>>,
            Refused<filter::StartFailure<D, P, C, X, Y, Q, M, F, K, V>, FilterOwners<A, B, U, H, O, E>>> {
        let parts = self.child.into_parts();
        let refill = cepstral_queue_model::Refilled::from_completed_parts(parts.held.coefficients, parts.coefficients);
        let second = refill::SecondHistory { signal: parts.signal, trace: parts.trace, readiness: parts.readiness };
        let held = FilterOwners { producer: parts.held.producer, complete: self.held };
        match refill.into_second_filter(state, parts.input, frame, observer, second) {
            Ok(child) => Ok(Carried { child, held }),
            Err(start) => Err(Refused { start, held }),
        }
    }
}
impl<D, P, C, X, Y, Q, M: crate::ltv_synthesis::SourceCoefficientMath,
    F: crate::ltv_render::SourceLtvFft, K: ltv_filter::PairedKernel, V: ltv_filter::SpanObserver,
    A, B, U: ScalarUpper, H, O, E>
    Carried<filter::Run<D, P, C, X, Y, Q, M, F, K, V>, FilterOwners<A, B, U, H, O, E>> {
    pub fn run(self)
        -> Result<Carried<filter::Completed<D, P, C, X, Y, Q, M, F, K, V>, FilterOwners<A, B, U, H, O, E>>,
            Carried<filter::Failed<D, P, C, X, Y, Q, M, F, K, V>, FilterOwners<A, B, U, H, O, E>>> {
        match self.child.run() {
            Ok(child) => Ok(Carried { child, held: self.held }),
            Err(child) => Err(Carried { child, held: self.held }),
        }
    }
}
impl<D, P, C, X, Y, Q, M, F, K, V, A, B, U: ScalarUpper, H, O, E>
    Carried<filter::Completed<D, P, C, X, Y, Q, M, F, K, V>, FilterOwners<A, B, U, H, O, E>> {
    pub fn into_continuation(self) -> FilteredParts<A, B, U, H, O, E, D, P, C, X, Y, Q, M, F, K, V> {
        FilteredParts { filter: self.child.into_continuation(), producer: self.held.producer, complete: self.held.complete }
    }
}
impl<A, B, U: ScalarUpper, W, S, PF: features::Frames, G: features::Gaussian,
    MI: post::Module, RI: input::Rows, H, O, E, D, P, C, X, Y, Q, M, F, K, V>
    FilteredParts<A, B, U, join::History<W, S, PF, G, MI, RI, H>, O, E, D, P, C, X, Y, Q, M, F, K, V> {

    pub fn into_joined_state(self)
        -> Result<JoinedFilterParts<join::Restored<A, B, U, W, S, PF, G, MI, RI, H, O, E>,
            filter::Continuation<D, P, C, X, Y, Q, M, F, K, V>>, Self> {
        if self.producer.retired.continuation.post.parts().final304.is_none() { return Err(self); }
        let Self { filter, producer, complete } = self;
        match join::restore_retired_parts(producer.retired) {
            Ok(producer_state) => Ok(JoinedFilterParts { producer: producer_state,
                filter, append_trace: producer.append_trace, complete }),
            Err(_) => unreachable!("same completed final304 checked before pure move"),
        }
    }
}
