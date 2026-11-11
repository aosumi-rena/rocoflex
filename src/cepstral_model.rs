use crate::{cepstral, conditioned_model, conditioned_stages,
    conditional_gate::{Batch, Projection}};

pub struct Upstream<D, P, A, X, Y> {
    condition_and_gru: conditioned_model::Upstream<D, P, A>,
    stages: conditioned_stages::State<X, Y>,
    stage_trace_after_move: conditioned_stages::Trace,
}
pub struct UpstreamParts<D, P, A, X, Y> {
    pub condition_and_gru: conditioned_model::Upstream<D, P, A>,
    pub stages: conditioned_stages::State<X, Y>, pub stage_trace_after_move: conditioned_stages::Trace,
}
impl<D, P, A, X, Y> Upstream<D, P, A, X, Y> {
    pub fn condition_and_gru(&self) -> &conditioned_model::Upstream<D, P, A> { &self.condition_and_gru }
    pub fn stages(&self) -> &conditioned_stages::State<X, Y> { &self.stages }
    pub fn stage_trace_after_move(&self) -> &conditioned_stages::Trace { &self.stage_trace_after_move }

    pub fn into_parts(self) -> UpstreamParts<D, P, A, X, Y> {
        UpstreamParts { condition_and_gru: self.condition_and_gru, stages: self.stages,
            stage_trace_after_move: self.stage_trace_after_move }
    }
}
pub struct StartFailure<D, P, A, X, Y, M> {
    pub upstream: Upstream<D, P, A, X, Y>, pub mixer: cepstral::StartFailure<M>,
}
pub struct Run<D, P, A, X, Y, M: Projection> {
    upstream: Upstream<D, P, A, X, Y>, mixer: cepstral::Run<M>,
}
pub struct Completed<D, P, A, X, Y, M> {
    upstream: Upstream<D, P, A, X, Y>, mixer: cepstral::Completed<M>,
}
impl<D, P, A, X, Y> conditioned_model::Completed<D, P, A, X, Y> {
    pub fn into_cepstral<M: Projection>(self, state: cepstral::State<M>)
        -> Result<Run<D, P, A, X, Y, M>, StartFailure<D, P, A, X, Y, M>>
    {
        let (condition_and_gru, stages) = self.into_parts();
        let (stages, mut trace) = stages.into_parts();

        let input = Batch { rows: std::mem::take(&mut trace.output.rows), status: trace.output.status };
        let upstream = Upstream { condition_and_gru, stages, stage_trace_after_move: trace };
        match state.begin(input) {
            Ok(mixer) => Ok(Run { upstream, mixer }),
            Err(mixer) => Err(StartFailure { upstream, mixer }),
        }
    }
}
impl<D, P, A, X, Y, M: Projection> Run<D, P, A, X, Y, M> {
    pub fn upstream(&self) -> &Upstream<D, P, A, X, Y> { &self.upstream }
    pub fn mixer(&self) -> &cepstral::Run<M> { &self.mixer }
    pub fn poll(&mut self, budget: usize) -> cepstral::Poll<'_> { self.mixer.poll(budget) }
    pub fn into_completed(self) -> Result<Completed<D, P, A, X, Y, M>, Self> {
        match self.mixer.into_completed() {
            Ok(mixer) => Ok(Completed { upstream: self.upstream, mixer }),
            Err(mixer) => Err(Self { upstream: self.upstream, mixer }),
        }
    }
}
impl<D, P, A, X, Y, M> Completed<D, P, A, X, Y, M> {
    pub fn upstream(&self) -> &Upstream<D, P, A, X, Y> { &self.upstream }
    pub fn mixer(&self) -> &cepstral::Completed<M> { &self.mixer }
    pub fn packets(&self) -> &[Batch; 2] { self.mixer.packets() }
    pub fn into_parts(self) -> (Upstream<D, P, A, X, Y>, cepstral::Completed<M>) { (self.upstream, self.mixer) }
}
