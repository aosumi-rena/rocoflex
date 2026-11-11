use crate::{
    condition_history::ConditionHistory,
    condition_phase_words,
    conditioned_model,
    conditioned_stages::{self, Batch},
    conditional_gate::GateError,
    ncwn_parameters,
    ncwn_value_binding::{self, Network, ProjectionInputs},
    quantized_gru::Activations,
};

pub struct NetworkParameters<'n, 'data> {
    pub parameters: ncwn_parameters::Decoded<'n, 'data>,
    pub projection_inputs: Vec<ProjectionInputs>,
}
pub struct Parts<'n, 'data, D, P, A> {

    pub upstream: condition_phase_words::Completed<D, P, A>,
    pub first: ncwn_value_binding::Bound<'n, 'data>,
    pub second: ncwn_value_binding::Bound<'n, 'data>,
    pub pending: ConditionHistory<Vec<f32>>,

    pub stage_input: Batch,

    pub incoming_worker_status: u8,
}
pub enum Failure<'n, 'data, D, P, A> {
    Stages {
        upstream: condition_phase_words::Completed<D, P, A>,
        parameters: [NetworkParameters<'n, 'data>; 2],
        failure: conditioned_stages::BindingFailure<Network, Network>,
        stage_input: Batch,
        incoming_worker_status: u8,
    },
    Start {
        phase: condition_phase_words::State,
        parameters: [NetworkParameters<'n, 'data>; 2],
        failure: conditioned_model::StartFailure<D, P, A, Network, Network>,
    },
}
pub struct Run<'n, 'data, D, P, A> {
    phase: condition_phase_words::State,
    parameters: [NetworkParameters<'n, 'data>; 2],
    run: conditioned_model::Run<D, P, A, Network, Network>,
}
pub struct Completed<'n, 'data, D, P, A> {
    phase: condition_phase_words::State,
    parameters: [NetworkParameters<'n, 'data>; 2],
    completed: conditioned_model::Completed<D, P, A, Network, Network>,
}

pub fn begin<'n, 'data, D, P, A: Activations>(parts: Parts<'n, 'data, D, P, A>)
    -> Result<Run<'n, 'data, D, P, A>, Failure<'n, 'data, D, P, A>>
{
    let Parts { upstream, first, second, pending, stage_input, incoming_worker_status } = parts;
    let ncwn_value_binding::Bound { parameters: first_parameters,
        network: first, projection_inputs: first_inputs } = first;
    let ncwn_value_binding::Bound { parameters: second_parameters,
        network: second, projection_inputs: second_inputs } = second;
    let parameters = [NetworkParameters { parameters: first_parameters, projection_inputs: first_inputs },
        NetworkParameters { parameters: second_parameters, projection_inputs: second_inputs }];
    let stages = match conditioned_stages::State::from_explicit_state(first, second, pending) {
        Ok(stages) => stages,
        Err(failure) => return Err(Failure::Stages {
            upstream, parameters, failure, stage_input, incoming_worker_status }),
    };
    let condition_phase_words::Completed { state: phase, condition } = upstream;
    match condition.into_stages(stages, stage_input, incoming_worker_status) {
        Ok(run) => Ok(Run { phase, parameters, run }),
        Err(failure) => Err(Failure::Start { phase, parameters, failure }),
    }
}

impl<'n, 'data, D, P, A> Run<'n, 'data, D, P, A> {
    pub fn phase_state(&self) -> &condition_phase_words::State { &self.phase }
    pub fn network_parameters(&self) -> &[NetworkParameters<'n, 'data>; 2] { &self.parameters }
    pub fn upstream(&self) -> &conditioned_model::Upstream<D, P, A> { self.run.upstream() }
    pub fn stages(&self) -> &conditioned_stages::Run<Network, Network> { self.run.stages() }
    pub fn poll(&mut self, effect_budget: usize)
        -> conditioned_stages::Poll<'_, GateError, GateError>
    {
        self.run.poll(effect_budget)
    }
    pub fn into_completed(self) -> Result<Completed<'n, 'data, D, P, A>, Self> {
        let Self { phase, parameters, run } = self;
        match run.into_completed() {
            Ok(completed) => Ok(Completed { phase, parameters, completed }),
            Err(run) => Err(Self { phase, parameters, run }),
        }
    }
}
impl<'n, 'data, D, P, A> Completed<'n, 'data, D, P, A> {
    pub fn phase_state(&self) -> &condition_phase_words::State { &self.phase }
    pub fn network_parameters(&self) -> &[NetworkParameters<'n, 'data>; 2] { &self.parameters }
    pub fn upstream(&self) -> &conditioned_model::Upstream<D, P, A> { self.completed.upstream() }
    pub fn stages(&self) -> &conditioned_stages::Completed<Network, Network> { self.completed.stages() }
    pub fn output(&self) -> &Batch { self.completed.output() }
    pub fn into_parts(self) -> (condition_phase_words::State, [NetworkParameters<'n, 'data>; 2],
        conditioned_model::Completed<D, P, A, Network, Network>)
    {
        (self.phase, self.parameters, self.completed)
    }
}
