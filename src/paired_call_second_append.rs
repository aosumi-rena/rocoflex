use super::*;
use crate::ltv_second_refill::{self as refill, append};
use retirement::second_append as second;

impl<A, B, U: ScalarUpper, W, S, P: features::Frames, G: features::Gaussian,
    M: post::Module, R: input::Rows, H, O: packing::CurrentRows, E: retirement::Erasure>
    Carried<retirement::Completed<A, B, U, join::History<W, S, P, G, M, R, History<H>>, O, E>, Adopted> {
    pub fn into_second_append(self, signal: refill::State)
        -> Result<Carried<second::Prepared<A, B, U, join::History<W, S, P, G, M, R, H>, O, E>, CompletedCall>,
            Refused<append::StartFailure<second::Suspended<A, B, U, join::History<W, S, P, G, M, R, H>, O, E>>, CompletedCall>> {

        let (retired, complete) = self.child.split_held(|held| held.split_input_held(History::finish));
        let held = CompletedCall { adopted: self.held, complete };
        match retired.into_second_append(signal) {
            Ok(child) => Ok(Carried { child, held }),
            Err(start) => Err(Refused { start, held }),
        }
    }
}
impl<A, B, U: ScalarUpper, H, O, E> Carried<second::Prepared<A, B, U, H, O, E>, CompletedCall> {
    pub fn poll(&mut self, budget: usize) -> append::Poll<'_> { self.child.poll(budget) }
    pub fn into_completed(self) -> Result<Carried<second::Completed<A, B, U, H, O, E>, CompletedCall>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<A, B, U: ScalarUpper, H, O, E> Carried<second::Completed<A, B, U, H, O, E>, CompletedCall> {
    pub fn into_parts(self) -> (second::CompletedParts<A, B, U, H, O, E>, CompletedCall) {
        (self.child.into_parts(), self.held)
    }

    pub fn into_refill<C>(self, coefficients: crate::cepstral::queue::Completed<C>, request: refill::Request)
        -> Result<Carried<refill::Prepared<C, second::History<A, B, U, H, O, E>>, CompletedCall>,
            Refused<refill::StartFailure<C, second::History<A, B, U, H, O, E>>, CompletedCall>> {
        match self.child.into_refill(coefficients, request) {
            Ok(child) => Ok(Carried { child, held: self.held }),
            Err(start) => Err(Refused { start, held: self.held }),
        }
    }
}
impl<C, H> Carried<refill::Prepared<C, H>, CompletedCall> {
    pub fn poll(&mut self, budget: usize) -> refill::Poll<'_> { self.child.poll(budget) }
    pub fn into_completed(self) -> Result<Carried<refill::Completed<C, H>, CompletedCall>, Self> {
        self.finish(|child| child.into_completed())
    }
}
impl<C, H> Carried<refill::Completed<C, H>, CompletedCall> {
    pub fn into_parts(self) -> (refill::CompletedParts<C, H>, CompletedCall) {
        (self.child.into_parts(), self.held)
    }
}
