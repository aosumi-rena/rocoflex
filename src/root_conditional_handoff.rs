use crate::{cepstral, cepstral_model, cepstral_parameters,
    condition_ncwn_handoff::{self, NetworkParameters}, condition_phase_words,
    conditional_gate::{Batch, Projection}, ncwn_value_binding::Network};

pub struct Owners<'n, 'data> {
    phase: condition_phase_words::State,
    networks: [NetworkParameters<'n, 'data>; 2],
    cepstral: cepstral_parameters::Decoded<'n, 'data>,
}
impl<'n, 'data> Owners<'n, 'data> {
    pub fn phase_state(&self) -> &condition_phase_words::State { &self.phase }
    pub fn networks(&self) -> &[NetworkParameters<'n, 'data>; 2] { &self.networks }
    pub fn cepstral_parameters(&self) -> &cepstral_parameters::Decoded<'n, 'data> { &self.cepstral }
    pub fn into_parts(self) -> (condition_phase_words::State, [NetworkParameters<'n, 'data>; 2],
        cepstral_parameters::Decoded<'n, 'data>)
    {
        (self.phase, self.networks, self.cepstral)
    }
}
pub struct Parts<'n, 'data, D, P, A, M> {
    pub upstream: condition_ncwn_handoff::Completed<'n, 'data, D, P, A>,
    pub mixer: cepstral_parameters::Bound<'n, 'data, M>,
}
pub struct Failure<'n, 'data, D, P, A, M> {
    owners: Owners<'n, 'data>,
    failure: cepstral_model::StartFailure<D, P, A, Network, Network, M>,
}
impl<'n, 'data, D, P, A, M> Failure<'n, 'data, D, P, A, M> {
    pub fn owners(&self) -> &Owners<'n, 'data> { &self.owners }
    pub fn failure(&self) -> &cepstral_model::StartFailure<D, P, A, Network, Network, M> { &self.failure }
}
pub struct Run<'n, 'data, D, P, A, M: Projection> {
    owners: Owners<'n, 'data>,
    run: cepstral_model::Run<D, P, A, Network, Network, M>,
}
pub struct Completed<'n, 'data, D, P, A, M> {
    owners: Owners<'n, 'data>,
    completed: cepstral_model::Completed<D, P, A, Network, Network, M>,
}

pub fn begin<'n, 'data, D, P, A, M: Projection>(parts: Parts<'n, 'data, D, P, A, M>)
    -> Result<Run<'n, 'data, D, P, A, M>, Failure<'n, 'data, D, P, A, M>>
{
    let Parts { upstream, mixer } = parts;
    let (phase, networks, upstream) = upstream.into_parts();
    let cepstral_parameters::Bound { parameters: cepstral, state } = mixer;
    let owners = Owners { phase, networks, cepstral };
    match upstream.into_cepstral(state) {
        Ok(run) => Ok(Run { owners, run }),
        Err(failure) => Err(Failure { owners, failure }),
    }
}
impl<'n, 'data, D, P, A, M: Projection> Run<'n, 'data, D, P, A, M> {
    pub fn owners(&self) -> &Owners<'n, 'data> { &self.owners }
    pub fn upstream(&self) -> &cepstral_model::Upstream<D, P, A, Network, Network> { self.run.upstream() }
    pub fn mixer(&self) -> &cepstral::Run<M> { self.run.mixer() }
    pub fn poll(&mut self, budget: usize) -> cepstral::Poll<'_> { self.run.poll(budget) }
    pub fn into_completed(self) -> Result<Completed<'n, 'data, D, P, A, M>, Self> {
        let Self { owners, run } = self;
        match run.into_completed() {
            Ok(completed) => Ok(Completed { owners, completed }),
            Err(run) => Err(Self { owners, run }),
        }
    }
}
impl<'n, 'data, D, P, A, M> Completed<'n, 'data, D, P, A, M> {
    pub fn owners(&self) -> &Owners<'n, 'data> { &self.owners }
    pub fn upstream(&self) -> &cepstral_model::Upstream<D, P, A, Network, Network> { self.completed.upstream() }
    pub fn mixer(&self) -> &cepstral::Completed<M> { self.completed.mixer() }
    pub fn packets(&self) -> &[Batch; 2] { self.completed.packets() }
    pub fn into_parts(self) -> (Owners<'n, 'data>, cepstral_model::Completed<D, P, A, Network, Network, M>) {
        (self.owners, self.completed)
    }
}
