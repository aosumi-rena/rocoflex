use crate::{condition, condition_gru, conditioned_stages::{self, Batch, Stage},
    quantized_gru::{Activations, QuantizedGru}};

pub struct Upstream<D, P, A> {
    condition: condition::Condition<D, P>, feature_input: Vec<Vec<f32>>, phase_words: Vec<u32>,
    condition_work: condition::Work, recurrent: QuantizedGru<A>, intermediate: Vec<Vec<f32>>, status: u8,
}
pub struct UpstreamParts<D, P, A> {
    pub condition: condition::Condition<D, P>, pub feature_input: Vec<Vec<f32>>, pub phase_words: Vec<u32>,
    pub condition_work: condition::Work, pub recurrent: QuantizedGru<A>,
    pub intermediate: Vec<Vec<f32>>, pub status: u8,
}
impl<D, P, A> Upstream<D, P, A> {
    pub fn condition(&self) -> &condition::Condition<D, P> { &self.condition }
    pub fn feature_input(&self) -> &[Vec<f32>] { &self.feature_input }
    pub fn phase_words(&self) -> &[u32] { &self.phase_words }
    pub fn condition_work(&self) -> &condition::Work { &self.condition_work }
    pub fn recurrent(&self) -> &QuantizedGru<A> { &self.recurrent }
    pub fn intermediate(&self) -> &[Vec<f32>] { &self.intermediate }
    pub fn status(&self) -> u8 { self.status }

    pub fn into_parts(self) -> UpstreamParts<D, P, A> {
        UpstreamParts { condition: self.condition, feature_input: self.feature_input,
            phase_words: self.phase_words, condition_work: self.condition_work,
            recurrent: self.recurrent, intermediate: self.intermediate, status: self.status }
    }
}
pub struct StartFailure<D, P, A, X, Y> {
    pub upstream: Upstream<D, P, A>, pub stages: conditioned_stages::StartFailure<X, Y>,
}
pub struct Run<D, P, A, X: Stage, Y: Stage> {
    upstream: Upstream<D, P, A>, stages: conditioned_stages::Run<X, Y>,
}
pub struct Completed<D, P, A, X, Y> {
    upstream: Upstream<D, P, A>, stages: conditioned_stages::Completed<X, Y>,
}

impl<D, P, A: Activations> condition_gru::Completed<D, P, A> {

    pub fn into_stages<X: Stage, Y: Stage>(self, state: conditioned_stages::State<X, Y>,
        stage_input: Batch, incoming_worker_status: u8)
        -> Result<Run<D, P, A, X, Y>, StartFailure<D, P, A, X, Y>>
    {
        let upstream = Upstream { condition: self.condition, feature_input: self.input,
            phase_words: self.phase_words, condition_work: self.work, recurrent: self.recurrent.model,
            intermediate: self.recurrent.input, status: self.recurrent.status };
        let holder = Batch { rows: self.recurrent.rows, status: upstream.status };
        match state.begin(stage_input, holder, incoming_worker_status) {
            Ok(stages) => Ok(Run { upstream, stages }),
            Err(stages) => Err(StartFailure { upstream, stages }),
        }
    }
}
impl<D, P, A, X: Stage, Y: Stage> Run<D, P, A, X, Y> {
    pub fn upstream(&self) -> &Upstream<D, P, A> { &self.upstream }
    pub fn stages(&self) -> &conditioned_stages::Run<X, Y> { &self.stages }
    pub fn poll(&mut self, effect_budget: usize) -> conditioned_stages::Poll<'_, X::Error, Y::Error> {
        self.stages.poll(effect_budget)
    }
    pub fn into_completed(self) -> Result<Completed<D, P, A, X, Y>, Self> {
        match self.stages.into_completed() {
            Ok(stages) => Ok(Completed { upstream: self.upstream, stages }),
            Err(stages) => Err(Self { upstream: self.upstream, stages }),
        }
    }
}
impl<D, P, A, X, Y> Completed<D, P, A, X, Y> {
    pub fn upstream(&self) -> &Upstream<D, P, A> { &self.upstream }
    pub fn stages(&self) -> &conditioned_stages::Completed<X, Y> { &self.stages }
    pub fn output(&self) -> &Batch { self.stages.output() }
    pub fn into_parts(self) -> (Upstream<D, P, A>, conditioned_stages::Completed<X, Y>) {
        (self.upstream, self.stages)
    }
}
