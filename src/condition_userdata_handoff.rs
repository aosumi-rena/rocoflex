use crate::{cepstral::{self, queue}, cepstral_queue_model as queues, cepstral_userdata_binding,
    condition, condition_dct, condition_history::ConditionHistory,
    condition_ncwn_handoff as stages, condition_parameters, condition_phase::{Provider, SourceWordTrig},
    condition_phase_words as holder, condition_value_binding as values,
    conditioned_stages, conditional_gate::{Batch, BorrowedProjection, GateError, Projection}, dct_parameters, dct_value_binding,
    modm0_logical_sequence::RequiredReset, ncwn_value_binding::{self, Network},
    quantized_gru::{self, GenericVectorActivations}, root_conditional_handoff as root,
    sequence_userdata_binding as sequences, userdata};

type Dct = condition_dct::Provider;
type Phase = Provider<SourceWordTrig>;
type Activations = GenericVectorActivations;
type Runtime = condition_parameters::Bound<Dct, Phase, Activations>;
type ConditionRun = holder::ConditionRun<Dct, Phase, Activations>;
type RecurrentRun = holder::RecurrentRun<Dct, Phase, Activations>;
type HolderCompleted = holder::Completed<Dct, Phase, Activations>;

pub struct Metadata<'n, 'data> {
    userdata: &'n userdata::Decoded<'data>, dct: dct_parameters::Decoded<'data>,
    phase_retirements: Vec<holder::PhaseGrowthRetirement>,
}
impl<'n, 'data> Metadata<'n, 'data> {
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn dct_parameters(&self) -> &dct_parameters::Decoded<'data> { &self.dct }
    pub fn phase_retirements(&self) -> &[holder::PhaseGrowthRetirement] { &self.phase_retirements }

    pub fn into_parts(self) -> (&'n userdata::Decoded<'data>, dct_parameters::Decoded<'data>,
        Vec<holder::PhaseGrowthRetirement>)
    {
        (self.userdata, self.dct, self.phase_retirements)
    }
}

pub struct Carried<'n, 'data, C> { child: C, metadata: Metadata<'n, 'data> }
impl<'n, 'data, C> Carried<'n, 'data, C> {
    pub fn child(&self) -> &C { &self.child }
    pub fn metadata(&self) -> &Metadata<'n, 'data> { &self.metadata }
}
pub struct BindingFailure<'n, 'data> {
    userdata: &'n userdata::Decoded<'data>, cause: values::UserdataFailure<'data>,
}

pub struct InitialBindingFailure<'n, 'data> {
    userdata: &'n userdata::Decoded<'data>, cause: values::InitialUserdataFailure<'data>,
}
impl<'n, 'data> InitialBindingFailure<'n, 'data> {
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn cause(&self) -> &values::InitialUserdataFailure<'data> { &self.cause }
}

enum BeforeBeginCause {
    ArchiveCapacity(std::collections::TryReserveError),
    Growth(holder::PhaseGrowthError),
}
enum GrowingPhaseFailureOwner<'n, 'data> {
    BeforeBegin {
        upstream: Carried<'n, 'data, Runtime>, state: holder::State,
        input: Vec<Vec<f32>>, status: u8, cause: BeforeBeginCause,
    },
    Begin(Carried<'n, 'data, holder::StartFailure<Dct, Phase, Activations>>),
}

pub struct GrowingPhaseFailure<'n, 'data> {
    owner: GrowingPhaseFailureOwner<'n, 'data>, spare_word: u32,
}
pub enum GrowingPhaseCause<'a> {
    ArchiveCapacity(&'a std::collections::TryReserveError),
    Growth(&'a holder::PhaseGrowthError),
    Begin(&'a holder::StartFailure<Dct, Phase, Activations>),
}
impl<'n, 'data> GrowingPhaseFailure<'n, 'data> {
    pub fn metadata(&self) -> &Metadata<'n, 'data> {
        match &self.owner {
            GrowingPhaseFailureOwner::BeforeBegin { upstream, .. } => upstream.metadata(),
            GrowingPhaseFailureOwner::Begin(failure) => failure.metadata(),
        }
    }
    pub fn spare_word(&self) -> u32 { self.spare_word }
    pub fn cause(&self) -> GrowingPhaseCause<'_> {
        match &self.owner {
            GrowingPhaseFailureOwner::BeforeBegin { cause: BeforeBeginCause::ArchiveCapacity(cause), .. } =>
                GrowingPhaseCause::ArchiveCapacity(cause),
            GrowingPhaseFailureOwner::BeforeBegin { cause: BeforeBeginCause::Growth(cause), .. } =>
                GrowingPhaseCause::Growth(cause),
            GrowingPhaseFailureOwner::Begin(failure) => GrowingPhaseCause::Begin(failure.child()),
        }
    }
    pub fn runtime_before_begin(&self) -> Option<&Runtime> {
        match &self.owner {
            GrowingPhaseFailureOwner::BeforeBegin { upstream, .. } => Some(upstream.child()),
            GrowingPhaseFailureOwner::Begin(_) => None,
        }
    }
    pub fn state(&self) -> &holder::State {
        match &self.owner {
            GrowingPhaseFailureOwner::BeforeBegin { state, .. } => state,
            GrowingPhaseFailureOwner::Begin(failure) => match failure.child() {
                holder::StartFailure::RequiredPhaseCapacity { state, .. }
                | holder::StartFailure::Capacity { state, .. }
                | holder::StartFailure::Condition { state, .. } => state,
            },
        }
    }
    pub fn input(&self) -> &[Vec<f32>] {
        match &self.owner {
            GrowingPhaseFailureOwner::BeforeBegin { input, .. } => input,
            GrowingPhaseFailureOwner::Begin(failure) => match failure.child() {
                holder::StartFailure::RequiredPhaseCapacity { input, .. }
                | holder::StartFailure::Capacity { input, .. } => input,
                holder::StartFailure::Condition { failure, .. } => &failure.input,
            },
        }
    }
    pub fn status(&self) -> u8 {
        match &self.owner {
            GrowingPhaseFailureOwner::BeforeBegin { status, .. } => *status,
            GrowingPhaseFailureOwner::Begin(failure) => match failure.child() {
                holder::StartFailure::RequiredPhaseCapacity { status, .. }
                | holder::StartFailure::Capacity { status, .. } => *status,
                holder::StartFailure::Condition { failure, .. } => failure.status,
            },
        }
    }
}

pub struct NetworkRequest {
    pub gates: Vec<ncwn_value_binding::GatePolicy>,
    pub skips: Vec<crate::streaming_value_binding::Policy>,
    pub residuals: Vec<crate::streaming_value_binding::Policy>,
    pub condition_head: crate::streaming_value_binding::Policy,
}
impl NetworkRequest {
    fn with_parameters<'n, 'data>(self, parameters: crate::ncwn_parameters::Decoded<'n, 'data>)
        -> ncwn_value_binding::Parts<'n, 'data>
    {
        let Self { gates, skips, residuals, condition_head } = self;
        ncwn_value_binding::Parts { parameters, gates, skips, residuals, condition_head }
    }
}
pub struct StageInputs {
    pub pending: ConditionHistory<Vec<f32>>, pub stage_input: Batch, pub incoming_worker_status: u8,
}
pub enum NetworkCause<'n, 'data> {
    Decode { role: userdata::Role, cause: userdata::Error },
    Binding { role: userdata::Role, failure: ncwn_value_binding::Failure<'n, 'data> },
    Stages(stages::Failure<'n, 'data, Dct, Phase, Activations>),
}
struct NetworkWork<'n, 'data> {
    upstream: Option<HolderCompleted>, first: Option<ncwn_value_binding::Bound<'n, 'data>>,
    requests: [Option<NetworkRequest>; 2], inputs: Option<StageInputs>,
}

pub struct NetworkFailure<'n, 'data> {
    metadata: Metadata<'n, 'data>, work: NetworkWork<'n, 'data>, cause: NetworkCause<'n, 'data>,
}
impl<'n, 'data> NetworkFailure<'n, 'data> {
    pub fn metadata(&self) -> &Metadata<'n, 'data> { &self.metadata }
    pub fn cause(&self) -> &NetworkCause<'n, 'data> { &self.cause }
    pub fn holder(&self) -> Option<&HolderCompleted> { self.work.upstream.as_ref() }
    pub fn first_initialized(&self) -> Option<&ncwn_value_binding::Bound<'n, 'data>> { self.work.first.as_ref() }
    pub fn remaining_requests(&self) -> &[Option<NetworkRequest>; 2] { &self.work.requests }
    pub fn stage_inputs(&self) -> Option<&StageInputs> { self.work.inputs.as_ref() }
}
enum CepstralFailureOwner<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    Initialization {
        upstream: Carried<'n, 'data, stages::Completed<'n, 'data, Dct, Phase, Activations>>,
        failure: cepstral_userdata_binding::Failure<'n, 'data, I, R>,
    },
    Root(Carried<'n, 'data, root::Failure<'n, 'data, Dct, Phase, Activations,
        cepstral_userdata_binding::BandProjection<'n, 'data, I, R>>>),
}

pub struct CepstralFailure<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{ owner: CepstralFailureOwner<'n, 'data, I, R> }
impl<'n, 'data, I, R> CepstralFailure<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    pub fn metadata(&self) -> &Metadata<'n, 'data> {
        match &self.owner {
            CepstralFailureOwner::Initialization { upstream, .. } => upstream.metadata(),
            CepstralFailureOwner::Root(failure) => failure.metadata(),
        }
    }
    pub fn completed_stages(&self) -> Option<&stages::Completed<'n, 'data, Dct, Phase, Activations>> {
        match &self.owner {
            CepstralFailureOwner::Initialization { upstream, .. } => Some(upstream.child()),
            CepstralFailureOwner::Root(_) => None,
        }
    }
    pub fn initialization_failure(&self) -> Option<&cepstral_userdata_binding::Failure<'n, 'data, I, R>> {
        match &self.owner {
            CepstralFailureOwner::Initialization { failure, .. } => Some(failure),
            CepstralFailureOwner::Root(_) => None,
        }
    }
    pub fn root_failure(&self) -> Option<&root::Failure<'n, 'data, Dct, Phase, Activations,
        cepstral_userdata_binding::BandProjection<'n, 'data, I, R>>>
    {
        match &self.owner {
            CepstralFailureOwner::Initialization { .. } => None,
            CepstralFailureOwner::Root(failure) => Some(failure.child()),
        }
    }
}
impl<'n, 'data> BindingFailure<'n, 'data> {
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn cause(&self) -> &values::UserdataFailure<'data> { &self.cause }
}

pub fn bind_userdata<'n, 'data>(userdata: &'n userdata::Decoded<'data>, shifts: [u32; 6],
    dct_policy: dct_value_binding::Policy, initialization: values::Initialization)
    -> Result<Carried<'n, 'data, Runtime>, BindingFailure<'n, 'data>>
{
    match values::bind_userdata(userdata, shifts, dct_policy, initialization) {
        Ok(values::Bound { dct_parameters: dct, runtime: child }) =>
            Ok(Carried { child, metadata: Metadata { userdata, dct, phase_retirements: Vec::new() } }),
        Err(cause) => Err(BindingFailure { userdata, cause }),
    }
}

pub fn bind_initial_generic_float_clear1_userdata<'n, 'data>(
    userdata: &'n userdata::Decoded<'data>, shifts: [u32; 6],
    dct_policy: dct_value_binding::Policy, initialization: values::InitialGenericFloatClear1)
    -> Result<Carried<'n, 'data, Runtime>, InitialBindingFailure<'n, 'data>>
{
    match values::bind_initial_generic_float_clear1_userdata(userdata, shifts, dct_policy, initialization) {
        Ok(values::Bound { dct_parameters: dct, runtime: child }) =>
            Ok(Carried { child, metadata: Metadata { userdata, dct, phase_retirements: Vec::new() } }),
        Err(cause) => Err(InitialBindingFailure { userdata, cause }),
    }
}
impl<'n, 'data> Carried<'n, 'data, Runtime> {

    pub fn begin_growing_phase(mut self, state: holder::State, input: Vec<Vec<f32>>,
        status: u8, spare_word: u32)
        -> Result<Carried<'n, 'data, ConditionRun>, GrowingPhaseFailure<'n, 'data>>
    {
        let mut state = state;
        if input.len() > state.full_backing().len() {
            if let Err(cause) = self.metadata.phase_retirements.try_reserve(1) {
                return Err(GrowingPhaseFailure { spare_word,
                    owner: GrowingPhaseFailureOwner::BeforeBegin { upstream: self, state,
                        input, status, cause: BeforeBeginCause::ArchiveCapacity(cause) } });
            }
            state = match state.grow_phase_capacity(input.len(), spare_word) {
                Ok((state, retirement)) => {
                    self.metadata.phase_retirements.push(retirement);
                    state
                },
                Err((cause, state)) => return Err(GrowingPhaseFailure { spare_word,
                    owner: GrowingPhaseFailureOwner::BeforeBegin { upstream: self, state,
                        input, status, cause: BeforeBeginCause::Growth(cause) } }),
            };
        }
        self.begin(state, input, status).map_err(|failure| GrowingPhaseFailure {
            owner: GrowingPhaseFailureOwner::Begin(failure), spare_word })
    }

    pub fn begin_initial_5c590(self, input: Vec<Vec<f32>>, status: u8, spare_word: u32)
        -> Result<Carried<'n, 'data, ConditionRun>, GrowingPhaseFailure<'n, 'data>>
    {
        self.begin_growing_phase(holder::State::from_initial_5c590_zeroed(), input, status, spare_word)
    }

    pub fn begin(self, state: holder::State, input: Vec<Vec<f32>>, status: u8)
        -> Result<Carried<'n, 'data, ConditionRun>,
            Carried<'n, 'data, holder::StartFailure<Dct, Phase, Activations>>>
    {
        match state.begin(self.child, input, status) {
            Ok(child) => Ok(Carried { child, metadata: self.metadata }),
            Err(child) => Err(Carried { child, metadata: self.metadata }),
        }
    }
}
pub enum RecurrentFailure<'n, 'data> {

    Incomplete(Carried<'n, 'data, ConditionRun>),
    Rejected(Carried<'n, 'data, holder::RecurrentStartFailure<Dct, Phase, Activations>>),
}
impl<'n, 'data> Carried<'n, 'data, ConditionRun> {
    pub fn poll(&mut self, row_budget: usize)
        -> condition::Poll<'_, <Dct as condition::Dct>::Error, <Phase as condition::PhaseServices>::Error>
    { self.child.poll(row_budget) }

    pub fn into_recurrent(self) -> Result<Carried<'n, 'data, RecurrentRun>, RecurrentFailure<'n, 'data>> {
        match self.child.into_recurrent() {
            Ok(child) => Ok(Carried { child, metadata: self.metadata }),
            Err(holder::RecurrentStartFailure::Incomplete(child)) =>
                Err(RecurrentFailure::Incomplete(Carried { child, metadata: self.metadata })),
            Err(child) => Err(RecurrentFailure::Rejected(Carried { child, metadata: self.metadata })),
        }
    }
}
impl<'n, 'data> Carried<'n, 'data, RecurrentRun> {
    pub fn poll(&mut self, row_budget: usize)
        -> quantized_gru::Poll<'_, <Activations as quantized_gru::Activations>::Error>
    { self.child.poll(row_budget) }

    pub fn into_completed(self) -> Result<Carried<'n, 'data, HolderCompleted>, Self> {
        match self.child.into_completed() {
            Ok(child) => Ok(Carried { child, metadata: self.metadata }),
            Err(child) => Err(Self { child, metadata: self.metadata }),
        }
    }
}
impl<'n, 'data> Carried<'n, 'data, HolderCompleted> {

    pub fn into_userdata_conditioned_stages(self, first: NetworkRequest, second: NetworkRequest,
        pending: ConditionHistory<Vec<f32>>, stage_input: Batch, incoming_worker_status: u8)
        -> Result<Carried<'n, 'data, stages::Run<'n, 'data, Dct, Phase, Activations>>,
            NetworkFailure<'n, 'data>>
    {
        let Self { child, metadata } = self;
        let actual_userdata = metadata.userdata;
        let mut work = NetworkWork { upstream: Some(child), first: None,
            requests: [Some(first), Some(second)],
            inputs: Some(StageInputs { pending, stage_input, incoming_worker_status }) };
        let parameters = match actual_userdata.decode_conditional_network(userdata::Role::FirstConditional) {
            Ok(parameters) => parameters,
            Err(cause) => return Err(NetworkFailure { metadata, work,
                cause: NetworkCause::Decode { role: userdata::Role::FirstConditional, cause } }),
        };
        let request = work.requests[0].take().expect("original first network request");
        match ncwn_value_binding::bind(request.with_parameters(parameters)) {
            Ok(first) => work.first = Some(first),
            Err(failure) => return Err(NetworkFailure { metadata, work,
                cause: NetworkCause::Binding { role: userdata::Role::FirstConditional, failure } }),
        }
        let parameters = match actual_userdata.decode_conditional_network(userdata::Role::SecondConditional) {
            Ok(parameters) => parameters,
            Err(cause) => return Err(NetworkFailure { metadata, work,
                cause: NetworkCause::Decode { role: userdata::Role::SecondConditional, cause } }),
        };
        let request = work.requests[1].take().expect("original second network request");
        let second = match ncwn_value_binding::bind(request.with_parameters(parameters)) {
            Ok(second) => second,
            Err(failure) => return Err(NetworkFailure { metadata, work,
                cause: NetworkCause::Binding { role: userdata::Role::SecondConditional, failure } }),
        };

        let upstream = work.upstream.take().expect("original completed holder");
        let first = work.first.take().expect("initialized first network");
        let StageInputs { pending, stage_input, incoming_worker_status } =
            work.inputs.take().expect("original stage inputs");
        let held = Carried { child: upstream, metadata };
        match held.into_conditioned_stages(first, second, pending, stage_input, incoming_worker_status) {
            Ok(run) => Ok(run),
            Err(Carried { child, metadata }) => Err(NetworkFailure { metadata, work,
                cause: NetworkCause::Stages(child) }),
        }
    }

    pub fn into_conditioned_stages(self,
        first: ncwn_value_binding::Bound<'n, 'data>, second: ncwn_value_binding::Bound<'n, 'data>,
        pending: ConditionHistory<Vec<f32>>, stage_input: Batch, incoming_worker_status: u8)
        -> Result<Carried<'n, 'data, stages::Run<'n, 'data, Dct, Phase, Activations>>,
            Carried<'n, 'data, stages::Failure<'n, 'data, Dct, Phase, Activations>>>
    {
        match stages::begin(stages::Parts { upstream: self.child, first, second,
            pending, stage_input, incoming_worker_status })
        {
            Ok(child) => Ok(Carried { child, metadata: self.metadata }),
            Err(child) => Err(Carried { child, metadata: self.metadata }),
        }
    }
    pub fn into_completed_parts(self) -> (HolderCompleted, Metadata<'n, 'data>) {
        (self.child, self.metadata)
    }
}
impl<'n, 'data> Carried<'n, 'data, stages::Run<'n, 'data, Dct, Phase, Activations>> {
    pub fn poll(&mut self, effect_budget: usize) -> conditioned_stages::Poll<'_, GateError, GateError> {
        self.child.poll(effect_budget)
    }
    pub fn into_completed(self)
        -> Result<Carried<'n, 'data, stages::Completed<'n, 'data, Dct, Phase, Activations>>, Self>
    {
        match self.child.into_completed() {
            Ok(child) => Ok(Carried { child, metadata: self.metadata }),
            Err(child) => Err(Self { child, metadata: self.metadata }),
        }
    }
}
impl<'n, 'data> Carried<'n, 'data, stages::Completed<'n, 'data, Dct, Phase, Activations>> {

    pub fn into_userdata_cepstral<I: BorrowedProjection, R>(self,
        inputs: cepstral_userdata_binding::Inputs<I, R>)
        -> Result<Carried<'n, 'data, root::Run<'n, 'data, Dct, Phase, Activations,
                cepstral_userdata_binding::BandProjection<'n, 'data, I, R>>>,
            CepstralFailure<'n, 'data, I, R>>
    where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
    {
        let actual_userdata = self.metadata.userdata;
        let mixer = match cepstral_userdata_binding::bind_userdata(actual_userdata, inputs) {
            Ok(mixer) => mixer,
            Err(failure) => return Err(CepstralFailure {
                owner: CepstralFailureOwner::Initialization { upstream: self, failure } }),
        };
        self.into_cepstral(mixer).map_err(|failure| CepstralFailure {
            owner: CepstralFailureOwner::Root(failure) })
    }

    pub fn into_cepstral<I: BorrowedProjection, R>(self,
        mixer: cepstral_userdata_binding::Bound<'n, 'data, I, R>)
        -> Result<Carried<'n, 'data, root::Run<'n, 'data, Dct, Phase, Activations,
                cepstral_userdata_binding::BandProjection<'n, 'data, I, R>>>,
            Carried<'n, 'data, root::Failure<'n, 'data, Dct, Phase, Activations,
                cepstral_userdata_binding::BandProjection<'n, 'data, I, R>>>>
    where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
    {
        match mixer.into_root_conditional(self.child) {
            Ok(child) => Ok(Carried { child, metadata: self.metadata }),
            Err(child) => Err(Carried { child, metadata: self.metadata }),
        }
    }

    pub fn into_completed_parts(self)
        -> (stages::Completed<'n, 'data, Dct, Phase, Activations>, Metadata<'n, 'data>)
    { (self.child, self.metadata) }
}

impl<'n, 'data, C: Projection> Carried<'n, 'data, root::Run<'n, 'data, Dct, Phase, Activations, C>> {
    pub fn poll(&mut self, budget: usize) -> cepstral::Poll<'_> { self.child.poll(budget) }

    pub fn into_completed(self)
        -> Result<Carried<'n, 'data, root::Completed<'n, 'data, Dct, Phase, Activations, C>>, Self>
    {
        match self.child.into_completed() {
            Ok(child) => Ok(Carried { child, metadata: self.metadata }),
            Err(child) => Err(Self { child, metadata: self.metadata }),
        }
    }
}
impl<'n, 'data, C> Carried<'n, 'data, root::Completed<'n, 'data, Dct, Phase, Activations, C>> {

    pub fn begin_queues(self, state: queue::State)
        -> Result<crate::paired_adopt::pipeline::Carried<
                queues::Run<Dct, Phase, Activations, Network, Network, C>,
                (root::Owners<'n, 'data>, Metadata<'n, 'data>)>,
            crate::paired_adopt::pipeline::Refused<
                queues::StartFailure<Dct, Phase, Activations, Network, Network, C>,
                (root::Owners<'n, 'data>, Metadata<'n, 'data>)>>
    {
        crate::root_cepstral_handoff::begin_queues_holding(self.child, state, self.metadata)
    }

    pub fn into_completed_parts(self)
        -> (root::Completed<'n, 'data, Dct, Phase, Activations, C>, Metadata<'n, 'data>)
    { (self.child, self.metadata) }
}
