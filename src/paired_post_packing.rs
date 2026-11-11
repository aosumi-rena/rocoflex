use super::{Completed, CompletedParts, Module, ScalarUpper, State, Trace, PostComplete};
use crate::{conditional_gate::Batch, paired_scalar_packing as scalar_pack};

pub struct Suspended<A, B, U: ScalarUpper, H> { post: CompletedParts<A, B, U, H> }
impl<A, B, U: ScalarUpper, H> Suspended<A, B, U, H> {
    pub fn state(&self) -> &State<A, B, U> { &self.post.state }
    pub fn input240(&self) -> &Batch { &self.post.input240 }
    pub fn held(&self) -> &H { &self.post.held }
    pub fn trace(&self) -> &Trace { &self.post.trace }
    pub fn completion(&self) -> &PostComplete { &self.post.post }
}
pub enum StartFailure<A: Module, B: Module, U: ScalarUpper, H, P> {
    EndedMismatch { post: Completed<A, B, U, H>, packing: scalar_pack::State<P> },
    Refused { post: Suspended<A, B, U, H>, packing: scalar_pack::StartFailure<P> },
}
pub struct Prepared<A, B, U: ScalarUpper, H, P: scalar_pack::CurrentRows> {
    post: Suspended<A, B, U, H>, packing: scalar_pack::Prepared<P>,
}
pub struct Packed<A, B, U: ScalarUpper, H, P: scalar_pack::CurrentRows> {
    post: Suspended<A, B, U, H>, packing: scalar_pack::Completed<P>,
}
pub struct Continuation<A, B, U: ScalarUpper, H, P> {
    pub post: State<A, B, U>, pub packing: scalar_pack::State<P>,
    pub input240: Batch, pub held: H, pub post_trace: Trace, pub packing_trace: scalar_pack::Trace,
    pub post_completion: PostComplete, pub packing_completion: scalar_pack::PackingComplete,
}
pub(crate) struct OutputSuspended<A, B, U: ScalarUpper, H, P> {
    post: State<A, B, U>, packing: scalar_pack::OutputSuspended<P>, input240: Batch, held: H,
    post_trace: Trace, packing_trace: scalar_pack::Trace,
    post_completion: PostComplete, packing_completion: scalar_pack::PackingComplete,
}
impl<A, B, U: ScalarUpper, H, P> Continuation<A, B, U, H, P> {
    pub(crate) fn suspend_output(self) -> (OutputSuspended<A, B, U, H, P>, Batch) {
        let (packing, output) = self.packing.suspend_output();
        (OutputSuspended { post: self.post, packing, input240: self.input240, held: self.held,
            post_trace: self.post_trace, packing_trace: self.packing_trace,
            post_completion: self.post_completion, packing_completion: self.packing_completion }, output)
    }
}
impl<A, B, U: ScalarUpper, H, P> OutputSuspended<A, B, U, H, P> {
    pub(crate) fn post(&self) -> &State<A, B, U> { &self.post }
    pub(crate) fn held(&self) -> &H { &self.held }
    pub(crate) fn restore(self, output: Batch) -> Continuation<A, B, U, H, P> {
        Continuation { post: self.post, packing: self.packing.restore(output), input240: self.input240,
            held: self.held, post_trace: self.post_trace, packing_trace: self.packing_trace,
            post_completion: self.post_completion, packing_completion: self.packing_completion }
    }
}
impl<A: Module, B: Module, U: ScalarUpper, H> Completed<A, B, U, H> {
    pub fn into_packing<P: scalar_pack::CurrentRows>(self, packing: scalar_pack::State<P>)
        -> Result<Prepared<A, B, U, H, P>, StartFailure<A, B, U, H, P>>
    {
        if self.prepared.owner.state.parts.ended != packing.parts().ended {
            return Err(StartFailure::EndedMismatch { post: self, packing });
        }
        let mut parts = self.into_parts();
        let input = parts.state.parts.final304.take().expect("completed post owns final304");
        let post = Suspended { post: parts };
        match packing.begin(input) {
            Ok(packing) => Ok(Prepared { post, packing }),
            Err(packing) => Err(StartFailure::Refused { post, packing }),
        }
    }
}
impl<A, B, U: ScalarUpper, H, P: scalar_pack::CurrentRows> Prepared<A, B, U, H, P> {
    pub fn post(&self) -> &Suspended<A, B, U, H> { &self.post }
    pub fn packing(&self) -> &scalar_pack::Prepared<P> { &self.packing }
    pub fn poll(&mut self, budget: usize) -> scalar_pack::Poll<'_, P::Error> { self.packing.poll(budget) }
    pub fn into_completed(self) -> Result<Packed<A, B, U, H, P>, Self> {
        match self.packing.into_completed() {
            Ok(packing) => Ok(Packed { post: self.post, packing }),
            Err(packing) => Err(Self { post: self.post, packing }),
        }
    }
}
impl<A, B, U: ScalarUpper, H, P: scalar_pack::CurrentRows> Packed<A, B, U, H, P> {
    pub fn post(&self) -> &Suspended<A, B, U, H> { &self.post }
    pub fn packing(&self) -> &scalar_pack::Completed<P> { &self.packing }

    pub fn into_continuation(self) -> Continuation<A, B, U, H, P> {
        let packed = self.packing.into_parts();
        let mut post = self.post.post;
        post.state.parts.final304 = Some(packed.input);
        Continuation { post: post.state, packing: packed.state, input240: post.input240,
            held: post.held, post_trace: post.trace, packing_trace: packed.trace,
            post_completion: post.post, packing_completion: packed.packing }
    }
}
