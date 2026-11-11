use crate::{cepstral::queue, cepstral_model};

pub struct StartFailure<D, P, A, X, Y, M> {
    pub upstream: cepstral_model::Upstream<D, P, A, X, Y>,
    pub queues: queue::StartFailure<M>,
}
pub struct Run<D, P, A, X, Y, M> {
    upstream: cepstral_model::Upstream<D, P, A, X, Y>, queues: queue::Run<M>,
}
pub struct Completed<D, P, A, X, Y, M> {
    upstream: cepstral_model::Upstream<D, P, A, X, Y>, queues: queue::Completed<M>,
}
impl<D, P, A, X, Y, M> cepstral_model::Completed<D, P, A, X, Y, M> {
    pub fn into_output_queues(self, state: queue::State)
        -> Result<Run<D, P, A, X, Y, M>, StartFailure<D, P, A, X, Y, M>>
    {
        let (upstream, mixer) = self.into_parts();
        match state.begin(mixer) {
            Ok(queues) => Ok(Run { upstream, queues }),
            Err(queues) => Err(StartFailure { upstream, queues }),
        }
    }
}
impl<D, P, A, X, Y, M> Run<D, P, A, X, Y, M> {
    pub fn upstream(&self) -> &cepstral_model::Upstream<D, P, A, X, Y> { &self.upstream }
    pub fn queues(&self) -> &queue::Run<M> { &self.queues }
    pub fn poll(&mut self, budget: usize) -> queue::Poll<'_> { self.queues.poll(budget) }
    pub fn into_completed(self) -> Result<Completed<D, P, A, X, Y, M>, Self> {
        match self.queues.into_completed() {
            Ok(queues) => Ok(Completed { upstream: self.upstream, queues }),
            Err(queues) => Err(Self { upstream: self.upstream, queues }),
        }
    }
}
impl<D, P, A, X, Y, M> Completed<D, P, A, X, Y, M> {
    pub fn upstream(&self) -> &cepstral_model::Upstream<D, P, A, X, Y> { &self.upstream }
    pub fn queues(&self) -> &queue::Completed<M> { &self.queues }
    pub fn into_parts(self) -> (cepstral_model::Upstream<D, P, A, X, Y>, queue::Completed<M>) {
        (self.upstream, self.queues)
    }

    pub fn begin_refill(self, selection: queue::refill::Selection,
        output: crate::conditional_gate::Batch, raw_status: u8)
        -> Result<Refilling<D, P, A, X, Y, M>, RefillStartFailure<D, P, A, X, Y, M>>
    {
        match self.queues.begin_refill(selection, output, raw_status) {
            Ok(refill) => Ok(Refilling { upstream: self.upstream, refill }),
            Err(refill) => Err(RefillStartFailure { upstream: self.upstream, refill }),
        }
    }
}

pub struct RefillStartFailure<D, P, A, X, Y, M> {
    pub upstream: cepstral_model::Upstream<D, P, A, X, Y>,
    pub refill: queue::refill::WithMixerStartFailure<M>,
}
pub struct Refilling<D, P, A, X, Y, M> {
    upstream: cepstral_model::Upstream<D, P, A, X, Y>,
    refill: queue::refill::WithMixer<M>,
}
pub struct Refilled<D, P, A, X, Y, M> {
    upstream: cepstral_model::Upstream<D, P, A, X, Y>,
    refill: queue::refill::WithMixerCompleted<M>,
}
impl<D, P, A, X, Y, M> Refilling<D, P, A, X, Y, M> {
    pub fn upstream(&self) -> &cepstral_model::Upstream<D, P, A, X, Y> { &self.upstream }
    pub fn refill(&self) -> &queue::refill::WithMixer<M> { &self.refill }
    pub fn poll(&mut self, budget: usize) -> queue::refill::Poll<'_> { self.refill.poll(budget) }
    pub fn into_completed(self) -> Result<Refilled<D, P, A, X, Y, M>, Self> {
        match self.refill.into_completed() {
            Ok(refill) => Ok(Refilled { upstream: self.upstream, refill }),
            Err(refill) => Err(Self { upstream: self.upstream, refill }),
        }
    }
}
impl<D, P, A, X, Y, M> Refilled<D, P, A, X, Y, M> {

    pub(crate) fn from_completed_parts(upstream: cepstral_model::Upstream<D, P, A, X, Y>,
        refill: queue::refill::WithMixerCompleted<M>) -> Self { Self { upstream, refill } }
    pub fn upstream(&self) -> &cepstral_model::Upstream<D, P, A, X, Y> { &self.upstream }
    pub fn refill(&self) -> &queue::refill::WithMixerCompleted<M> { &self.refill }
    pub fn into_parts(self) -> (cepstral_model::Upstream<D, P, A, X, Y>, queue::refill::WithMixerCompleted<M>) {
        (self.upstream, self.refill)
    }
}
