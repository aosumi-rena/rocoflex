use crate::{conditional_gate::{Batch, BorrowedProjection, GateError, Projection},
    condition_ncwn_handoff::NetworkParameters,
    ltv_coefficient_math::ScalarUpper, model::ModelNode,
    moda7_projection::Moda7Projection, moda7_value_services,
    ncwn_value_binding as ncwn,
    paired_condition_worker::NcwnWorker, paired_projection_module::ProjectionModule,
    paired_scalar_packing::CurrentRows,
    sequence_parameters as sequence, sequence_value_binding::Linear,
    sequence_userdata_binding as sequence_values,
    sequence_value_reset::ResetChildrenThenArchiveCache,
    streaming_value_binding,
    userdata::{self, Role}, vocoder_value_binding as paired};

pub type SequenceParts<I> = sequence_values::Parts<I, ResetChildrenThenArchiveCache>;

pub enum ProjectionInput<I> { Sequence(SequenceParts<I>), Bank(BankInput) }
pub use sequence_values::BankInput;

pub enum ProjectionOwner<'n, 'data, I: BorrowedProjection> {
    Sequence(sequence::Bound<'n, 'data, I,
        Moda7Projection<moda7_value_services::ValueServices>, sequence_values::BankOwner<'n, 'data>,
        Linear, ResetChildrenThenArchiveCache>),
    Bank(sequence_values::BankOwner<'n, 'data>),
}
impl<I: BorrowedProjection> Projection for ProjectionOwner<'_, '_, I> {
    fn reset(&mut self) -> Result<(), GateError> {
        match self { Self::Sequence(bound) => bound.projection.reset(), Self::Bank(owner) => owner.reset() }
    }
    fn forward(&mut self, input: &Batch) -> Result<Batch, GateError> {
        match self { Self::Sequence(bound) => bound.projection.forward(input), Self::Bank(owner) => owner.forward(input) }
    }
}
pub type Module<'n, 'data, I: BorrowedProjection> = ProjectionModule<ProjectionOwner<'n, 'data, I>>;
pub type Worker = NcwnWorker<ncwn::Gate, streaming_value_binding::Projection,
    streaming_value_binding::Projection, streaming_value_binding::Projection>;

pub struct NetworkInput {
    pub gates: Vec<ncwn::GatePolicy>, pub skips: Vec<streaming_value_binding::Policy>,
    pub residuals: Vec<streaming_value_binding::Policy>,
    pub condition_head: streaming_value_binding::Policy,
}
pub struct Runtime<R, O, T, U: ScalarUpper, P> {

    pub hop470: i32,
    pub disabled_cursor512: i32,
    pub providers: paired::Providers<R, O, T, U, P>, pub scratch: paired::Scratch,
    pub packing_output: Batch,
    pub continuation: crate::paired_adopt::pipeline::call::Continuation,
}
pub struct Inputs<I, R, O, T, U: ScalarUpper, P> {
    pub upsampler40: ProjectionInput<I>, pub downsampler48: ProjectionInput<I>,
    pub subgroup64: NetworkInput, pub projection56: ProjectionInput<I>,
    pub runtime: Runtime<R, O, T, U, P>,
}
pub enum ProjectionFailure<'n, 'data, I> {
    Sequence(sequence_values::Failure<'n, 'data, I, ResetChildrenThenArchiveCache>),
    Bank(sequence_values::BankFailure<'n, 'data>),
}
pub enum Cause<'n, 'data, I: BorrowedProjection, R, O: CurrentRows, T, U: ScalarUpper, P> {
    Missing(Role),
    Projection { role: Role, failure: ProjectionFailure<'n, 'data, I> },
    NetworkDecode(userdata::Error), Network(ncwn::Failure<'n, 'data>),
    Constructor(paired::Failure<Module<'n, 'data, I>, R, Worker,
        Module<'n, 'data, I>, Module<'n, 'data, I>, O, T, U, P>),
}
struct Work<'n, 'data, I: BorrowedProjection, R, O, T, U: ScalarUpper, P> {
    userdata: &'n userdata::Decoded<'data>,
    up_input: Option<ProjectionInput<I>>, down_input: Option<ProjectionInput<I>>,
    worker_input: Option<NetworkInput>, post_input: Option<ProjectionInput<I>>,
    runtime: Option<Runtime<R, O, T, U, P>>,
    up: Option<ProjectionOwner<'n, 'data, I>>, down: Option<ProjectionOwner<'n, 'data, I>>,
    worker: Option<ncwn::Bound<'n, 'data>>, post: Option<ProjectionOwner<'n, 'data, I>>,
    worker_parameters: Option<NetworkParameters<'n, 'data>>,
}

pub struct Failure<'n, 'data, I: BorrowedProjection, R, O: CurrentRows, T, U: ScalarUpper, P> {
    cause: Cause<'n, 'data, I, R, O, T, U, P>,
    work: Work<'n, 'data, I, R, O, T, U, P>,
}
impl<'n, 'data, I: BorrowedProjection, R, O: CurrentRows, T, U: ScalarUpper, P>
    Failure<'n, 'data, I, R, O, T, U, P>
{
    pub fn cause(&self) -> &Cause<'n, 'data, I, R, O, T, U, P> { &self.cause }
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.work.userdata }
    pub fn upsampler(&self) -> Option<&ProjectionOwner<'n, 'data, I>> { self.work.up.as_ref() }
    pub fn downsampler(&self) -> Option<&ProjectionOwner<'n, 'data, I>> { self.work.down.as_ref() }
    pub fn subgroup(&self) -> Option<&ncwn::Bound<'n, 'data>> { self.work.worker.as_ref() }
    pub fn post_projection(&self) -> Option<&ProjectionOwner<'n, 'data, I>> { self.work.post.as_ref() }
    pub fn subgroup_parameters(&self) -> Option<&NetworkParameters<'n, 'data>> { self.work.worker_parameters.as_ref() }
}
pub struct Bound<'n, 'data, I: BorrowedProjection, R, O, T, U: ScalarUpper, P> {
    userdata: &'n userdata::Decoded<'data>, subgroup_parameters: NetworkParameters<'n, 'data>,
    paired: paired::Bound<Module<'n, 'data, I>, R, Worker,
        Module<'n, 'data, I>, Module<'n, 'data, I>, O, T, U, P>,
}

pub struct Metadata<'n, 'data> {
    userdata: &'n userdata::Decoded<'data>, subgroup_parameters: NetworkParameters<'n, 'data>,
    reset: crate::paired_constructor_reset_values::ResetComplete,
    rate: Option<crate::vocoder_rate::Prepared>,
}
impl<'n, 'data> Metadata<'n, 'data> {
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn subgroup_parameters(&self) -> &NetworkParameters<'n, 'data> { &self.subgroup_parameters }
    pub fn reset_completion(&self) -> &crate::paired_constructor_reset_values::ResetComplete { &self.reset }
    pub fn rate(&self) -> Option<&crate::vocoder_rate::Prepared> { self.rate.as_ref() }
}
impl<'n, 'data, I: BorrowedProjection, R, O, T, U: ScalarUpper, P>
    Bound<'n, 'data, I, R, O, T, U, P>
{
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn subgroup_parameters(&self) -> &NetworkParameters<'n, 'data> { &self.subgroup_parameters }
    pub fn paired(&self) -> &paired::Bound<Module<'n, 'data, I>, R, Worker,
        Module<'n, 'data, I>, Module<'n, 'data, I>, O, T, U, P> { &self.paired }

    pub fn into_parts(self) -> (crate::paired_adopt::pipeline::call::Bound<
        Module<'n, 'data, I>, R, Worker, Module<'n, 'data, I>, Module<'n, 'data, I>, O, T, U, P,
        crate::paired_features::StandardGaussian>, Metadata<'n, 'data>)
    {
        let Self { userdata, subgroup_parameters, paired } = self;
        let (call, reset) = paired.into_parts();
        (call, Metadata { userdata, subgroup_parameters, reset, rate: None })
    }
}

pub struct RatedRuntime<R, O, T, U: ScalarUpper, P> {
    pub disabled_cursor512: i32,
    pub providers: paired::Providers<R, O, T, U, P>, pub scratch: paired::Scratch,
    pub packing_output: Batch,
    pub continuation: crate::paired_adopt::pipeline::call::Continuation,
}
pub struct RatedInputs<I, R, O, T, U: ScalarUpper, P> {
    pub upsampler40: ProjectionInput<I>, pub downsampler48: ProjectionInput<I>,
    pub subgroup64: NetworkInput, pub projection56: ProjectionInput<I>,
    pub runtime: RatedRuntime<R, O, T, U, P>,
}
pub enum RatedFailure<'n, 'data, I: BorrowedProjection, R, O: CurrentRows, T, U: ScalarUpper, P> {

    Configuration { userdata: &'n userdata::Decoded<'data>, rate: crate::vocoder_rate::Prepared,
        inputs: RatedInputs<I, R, O, T, U, P> },
    Initialization { rate: crate::vocoder_rate::Prepared,
        failure: Failure<'n, 'data, I, R, O, T, U, P> },
}
pub struct RatedBound<'n, 'data, I: BorrowedProjection, R, O, T, U: ScalarUpper, P> {
    rate: crate::vocoder_rate::Prepared,
    initialized: Bound<'n, 'data, I, R, O, T, U, P>,
}
impl<'n, 'data, I: BorrowedProjection, R, O, T, U: ScalarUpper, P>
    RatedBound<'n, 'data, I, R, O, T, U, P>
{
    pub fn rate(&self) -> &crate::vocoder_rate::Prepared { &self.rate }
    pub fn initialized(&self) -> &Bound<'n, 'data, I, R, O, T, U, P> { &self.initialized }
    pub fn into_parts(self) -> (crate::paired_adopt::pipeline::call::Bound<
        Module<'n, 'data, I>, R, Worker, Module<'n, 'data, I>, Module<'n, 'data, I>, O, T, U, P,
        crate::paired_features::StandardGaussian>, Metadata<'n, 'data>)
    {
        let (call, mut metadata) = self.initialized.into_parts();
        metadata.rate = Some(self.rate);
        (call, metadata)
    }
}

pub fn initialize_with_rate<'n, 'data, I, R, O, T, U, P>(userdata: &'n userdata::Decoded<'data>,
    rate: crate::vocoder_rate::Prepared, inputs: RatedInputs<I, R, O, T, U, P>)
    -> Result<RatedBound<'n, 'data, I, R, O, T, U, P>, RatedFailure<'n, 'data, I, R, O, T, U, P>>
where I: BorrowedProjection, R: crate::paired_input::Rows,
    O: CurrentRows, T: crate::paired_interpolation::Storage, U: ScalarUpper, P: crate::paired_features::Frames
{
    if rate.configuration() != userdata.primary().vocoder() {
        return Err(RatedFailure::Configuration { userdata, rate, inputs });
    }
    let hop470 = rate.rate().hop1136();
    let RatedInputs { upsampler40, downsampler48, subgroup64, projection56,
        runtime: RatedRuntime { disabled_cursor512, providers, scratch, packing_output, continuation } } = inputs;
    match initialize(userdata, Inputs { upsampler40, downsampler48, subgroup64, projection56,
        runtime: Runtime { hop470, disabled_cursor512, providers, scratch, packing_output, continuation } })
    {
        Ok(initialized) => Ok(RatedBound { rate, initialized }),
        Err(failure) => Err(RatedFailure::Initialization { rate, failure }),
    }
}

fn initialize_projection<'n, 'data, I: BorrowedProjection>(
    node: &'n ModelNode<'data>, input: ProjectionInput<I>)
    -> Result<ProjectionOwner<'n, 'data, I>, ProjectionFailure<'n, 'data, I>>
{
    match input {
        ProjectionInput::Sequence(input) => sequence_values::bind_node(node, input)
            .map(ProjectionOwner::Sequence).map_err(ProjectionFailure::Sequence),
        ProjectionInput::Bank(input) => sequence_values::bind_bank_node(node, input)
            .map(ProjectionOwner::Bank).map_err(ProjectionFailure::Bank),
    }
}

pub fn initialize<'n, 'data, I, R, O, T, U, P>(userdata: &'n userdata::Decoded<'data>,
    inputs: Inputs<I, R, O, T, U, P>)
    -> Result<Bound<'n, 'data, I, R, O, T, U, P>, Failure<'n, 'data, I, R, O, T, U, P>>
where I: BorrowedProjection, R: crate::paired_input::Rows,
    O: CurrentRows, T: crate::paired_interpolation::Storage, U: ScalarUpper, P: crate::paired_features::Frames
{
    let Inputs { upsampler40, downsampler48, subgroup64, projection56, runtime } = inputs;
    let mut work = Work { userdata, up_input: Some(upsampler40), down_input: Some(downsampler48),
        worker_input: Some(subgroup64), post_input: Some(projection56), runtime: Some(runtime),
        up: None, down: None, worker: None, post: None, worker_parameters: None };
    macro_rules! fail { ($cause:expr) => { return Err(Failure { cause: $cause, work }) }; }
    let Some(node) = userdata.role(Role::Upsampler) else { fail!(Cause::Missing(Role::Upsampler)); };
    match initialize_projection(node, work.up_input.take().expect("original upsampler request")) {
        Ok(owner) => work.up = Some(owner),
        Err(failure) => fail!(Cause::Projection { role: Role::Upsampler, failure }),
    }
    let Some(node) = userdata.role(Role::Downsampler) else { fail!(Cause::Missing(Role::Downsampler)); };
    match initialize_projection(node, work.down_input.take().expect("original downsampler request")) {
        Ok(owner) => work.down = Some(owner),
        Err(failure) => fail!(Cause::Projection { role: Role::Downsampler, failure }),
    }
    let parameters = match userdata.decode_conditional_network(Role::SubgroupConditional) {
        Ok(parameters) => parameters,
        Err(cause) => fail!(Cause::NetworkDecode(cause)),
    };
    let NetworkInput { gates, skips, residuals, condition_head } = work.worker_input.take().expect("original network policies");
    match ncwn::bind(ncwn::Parts { parameters, gates, skips, residuals, condition_head }) {
        Ok(bound) => work.worker = Some(bound),
        Err(failure) => fail!(Cause::Network(failure)),
    }
    let Some(node) = userdata.role(Role::SubgroupProjection) else { fail!(Cause::Missing(Role::SubgroupProjection)); };
    match initialize_projection(node, work.post_input.take().expect("original post projection request")) {
        Ok(owner) => work.post = Some(owner),
        Err(failure) => fail!(Cause::Projection { role: Role::SubgroupProjection, failure }),
    }
    let ncwn::Bound { parameters, network, projection_inputs } = work.worker.take().expect("same completed network initializer");
    work.worker_parameters = Some(NetworkParameters { parameters, projection_inputs });
    let Runtime { hop470, disabled_cursor512, providers, scratch, packing_output, continuation } = work.runtime.take().expect("same runtime inputs");

    let plan = userdata.primary().descriptor();
    let configuration = paired::Configuration { hop: hop470, groups: plan.count,
        enabled: plan.subgroup_flag, disabled_cursor512 };
    let parts = paired::Parts { configuration,
        modules: paired::Modules {
            module40: ProjectionModule::from_explicit_projection(work.up.take().expect("same upsampler")),
            module48: ProjectionModule::from_explicit_projection(work.down.take().expect("same downsampler")),
            module56: ProjectionModule::from_explicit_projection(work.post.take().expect("same post projection")),
            worker64: NcwnWorker::from_explicit_module(network),
        }, providers, scratch, packing_output, continuation };
    match paired::initialize(parts) {
        Ok(paired) => Ok(Bound { userdata, subgroup_parameters: work.worker_parameters.take().expect("same original network metadata"), paired }),
        Err(failure) => fail!(Cause::Constructor(failure)),
    }
}
