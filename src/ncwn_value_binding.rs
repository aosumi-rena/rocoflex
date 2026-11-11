use std::collections::{TryReserveError, VecDeque};
use crate::{activation::SourceGateKernels, condition_history::ConditionHistory,
    conditional_gate::{Auxiliary, Batch, BatchAuxiliary, ConditionalGate, GateError,
        GateExplicitState, GateStateRefusal},
    conditional_ncwn::{Block, ConditionalNcwn}, ncwn_parameters as parameters,
    streaming_parameters::BindingInputs, streaming_value_binding as projection};

pub enum AbsentAuxiliary {}
impl Auxiliary for AbsentAuxiliary {
    fn reset(&mut self) -> Result<(), GateError> { match *self {} }
    fn forward_row(&mut self, _: &mut [f32]) -> Result<(), GateError> { match *self {} }
}
impl BatchAuxiliary for AbsentAuxiliary {
    fn forward_batch_in_place(&mut self, _: &mut Batch) -> Result<(), GateError> { match *self {} }
}
pub type Gate = ConditionalGate<projection::Projection, projection::Projection, AbsentAuxiliary, SourceGateKernels>;
pub type Network = ConditionalNcwn<Gate, projection::Projection, projection::Projection, projection::Projection>;
pub struct GatePolicy {
    pub main: projection::Policy, pub condition: projection::Policy, pub kernels: SourceGateKernels,
}

pub struct Parts<'n, 'data> {
    pub parameters: parameters::Decoded<'n, 'data>,
    pub gates: Vec<GatePolicy>, pub skips: Vec<projection::Policy>,
    pub residuals: Vec<projection::Policy>, pub condition_head: projection::Policy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site { Main(usize), Condition(usize), Skip(usize), Residual(usize), ConditionHead }
pub struct ProjectionInputs { pub site: Site, pub inputs: BindingInputs }
pub struct Bound<'n, 'data> {
    pub parameters: parameters::Decoded<'n, 'data>, pub network: Network,
    pub projection_inputs: Vec<ProjectionInputs>,
}
pub enum Admission {
    ListCounts { layers: usize, gates: usize, skips: usize, residuals: usize },
    AuxiliaryRequired { layer: usize }, SizeOverflow, Allocation(TryReserveError),
}
pub enum GateFailure {
    MissingCondition { policy: GatePolicy },
    Main { failure: projection::Failure, condition: projection::Policy, kernels: SourceGateKernels },
    Condition { main: projection::Projection, failure: projection::Failure, kernels: SourceGateKernels },
    Adoption(GateStateRefusal<projection::Projection, projection::Projection, AbsentAuxiliary, SourceGateKernels>),
    Reset { gate: Gate, error: GateError },
}
pub enum Cause {
    Gate { layer: usize, failure: GateFailure },
    Projection { site: Site, failure: projection::Failure },
}
pub enum Failure<'n, 'data> {
    Admission { cause: Admission, parts: Parts<'n, 'data> },
    Stopped(Stopped<'n, 'data>),
    Network {
        failure: parameters::Failure<'n, 'data, Gate, projection::Projection, projection::Projection, projection::Projection>,
        projection_inputs: Vec<ProjectionInputs>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completed { pub gates: usize, pub skips: usize, pub residuals: usize, pub condition_head: bool }
struct Work<'n, 'data> {
    parameters: parameters::Decoded<'n, 'data>,
    pending_gates: std::vec::IntoIter<GatePolicy>, pending_skips: std::vec::IntoIter<projection::Policy>,
    pending_residuals: std::vec::IntoIter<projection::Policy>, pending_head: Option<projection::Policy>,
    gates: Vec<Gate>, skips: Vec<projection::Projection>, residuals: Vec<projection::Projection>,
    head: Option<projection::Projection>, blocks: Vec<Block<Gate, projection::Projection, projection::Projection>>,
    inputs: Vec<ProjectionInputs>,
}

pub struct Stopped<'n, 'data> { work: Work<'n, 'data>, cause: Cause }
impl<'n, 'data> Stopped<'n, 'data> {
    pub fn cause(&self) -> &Cause { &self.cause }
    pub fn parameters(&self) -> &parameters::Decoded<'n, 'data> { &self.work.parameters }
    pub fn projection_inputs(&self) -> &[ProjectionInputs] { &self.work.inputs }
    pub fn completed(&self) -> Completed {
        Completed { gates: self.work.gates.len(), skips: self.work.skips.len(),
            residuals: self.work.residuals.len(), condition_head: self.work.head.is_some() }
    }
}

fn bind_projection(node: parameters::ProjectionNode<'_, '_>, policy: projection::Policy,
    site: Site, inputs: &mut Vec<ProjectionInputs>) -> Result<projection::Projection, projection::Failure>
{
    let bound = projection::bind_from_model(node, policy)?;

    inputs.push(ProjectionInputs { site, inputs: bound.inputs });
    Ok(bound.module)
}
fn bind_gate(parameter: &parameters::Gate<'_, '_>, policy: GatePolicy,
    layer: usize, inputs: &mut Vec<ProjectionInputs>) -> Result<Gate, GateFailure>
{
    let Some(condition_node) = parameter.condition_projection() else {
        return Err(GateFailure::MissingCondition { policy });
    };
    let GatePolicy { main, condition, kernels } = policy;
    let main = match bind_projection(parameter.main_projection(), main, Site::Main(layer), inputs) {
        Ok(main) => main,
        Err(failure) => return Err(GateFailure::Main { failure, condition, kernels }),
    };
    let condition = match bind_projection(condition_node, condition, Site::Condition(layer), inputs) {
        Ok(condition) => condition,
        Err(failure) => return Err(GateFailure::Condition { main, failure, kernels }),
    };

    let mut gate = ConditionalGate::from_explicit_state(GateExplicitState {
        config: parameter.configuration(), main, condition, auxiliary: None,
        kernels, history: ConditionHistory::from_explicit_pending(VecDeque::new()),
        needs_reset: true, pending_conditional_work: None,
    }).map_err(GateFailure::Adoption)?;
    if let Err(error) = gate.reset() { return Err(GateFailure::Reset { gate, error }); }
    Ok(gate)
}

pub fn bind<'n, 'data>(parts: Parts<'n, 'data>) -> Result<Bound<'n, 'data>, Failure<'n, 'data>> {
    let count = parts.parameters.layers().len();
    if parts.gates.len() != count || parts.skips.len() != count || parts.residuals.len() != count {
        return Err(Failure::Admission { cause: Admission::ListCounts { layers: count,
            gates: parts.gates.len(), skips: parts.skips.len(), residuals: parts.residuals.len() }, parts });
    }
    if let Some(layer) = parts.parameters.layers().iter().position(|layer| layer.gate().configuration().has_auxiliary) {
        return Err(Failure::Admission { cause: Admission::AuxiliaryRequired { layer }, parts });
    }
    let Some(input_count) = count.checked_mul(4).and_then(|n| n.checked_add(1)) else {
        return Err(Failure::Admission { cause: Admission::SizeOverflow, parts });
    };
    let (mut gates, mut skips, mut residuals, mut blocks, mut inputs) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    if let Err(cause) = gates.try_reserve_exact(count) {
        return Err(Failure::Admission { cause: Admission::Allocation(cause), parts });
    }
    if let Err(cause) = skips.try_reserve_exact(count) {
        return Err(Failure::Admission { cause: Admission::Allocation(cause), parts });
    }
    if let Err(cause) = residuals.try_reserve_exact(count) {
        return Err(Failure::Admission { cause: Admission::Allocation(cause), parts });
    }
    if let Err(cause) = blocks.try_reserve_exact(count) {
        return Err(Failure::Admission { cause: Admission::Allocation(cause), parts });
    }
    if let Err(cause) = inputs.try_reserve_exact(input_count) {
        return Err(Failure::Admission { cause: Admission::Allocation(cause), parts });
    }
    let Parts { parameters, gates: gate_policies, skips: skip_policies, residuals: residual_policies, condition_head } = parts;
    let mut work = Work { parameters, pending_gates: gate_policies.into_iter(), pending_skips: skip_policies.into_iter(),
        pending_residuals: residual_policies.into_iter(), pending_head: Some(condition_head),
        gates, skips, residuals, blocks, inputs, head: None };
    for layer in 0..count {
        let policy = work.pending_gates.next().expect("preflighted gate policy count");
        match bind_gate(work.parameters.layers()[layer].gate(), policy, layer, &mut work.inputs) {
            Ok(gate) => work.gates.push(gate),
            Err(failure) => return Err(Failure::Stopped(Stopped { work, cause: Cause::Gate { layer, failure } })),
        }
    }
    for layer in 0..count {
        let policy = work.pending_skips.next().expect("preflighted skip policy count");
        let site = Site::Skip(layer);
        match bind_projection(work.parameters.layers()[layer].skip(), policy, site, &mut work.inputs) {
            Ok(skip) => work.skips.push(skip),
            Err(failure) => return Err(Failure::Stopped(Stopped { work, cause: Cause::Projection { site, failure } })),
        }
    }
    for layer in 0..count {
        let policy = work.pending_residuals.next().expect("preflighted residual policy count");
        let site = Site::Residual(layer);
        match bind_projection(work.parameters.layers()[layer].residual(), policy, site, &mut work.inputs) {
            Ok(residual) => work.residuals.push(residual),
            Err(failure) => return Err(Failure::Stopped(Stopped { work, cause: Cause::Projection { site, failure } })),
        }
    }
    let policy = work.pending_head.take().expect("original head policy");
    match bind_projection(work.parameters.condition_head(), policy, Site::ConditionHead, &mut work.inputs) {
        Ok(head) => work.head = Some(head),
        Err(failure) => return Err(Failure::Stopped(Stopped { work,
            cause: Cause::Projection { site: Site::ConditionHead, failure } })),
    }
    let gates = std::mem::take(&mut work.gates);
    let skips = std::mem::take(&mut work.skips);
    let residuals = std::mem::take(&mut work.residuals);
    for ((gate, skip), residual) in gates.into_iter().zip(skips).zip(residuals) {
        work.blocks.push(Block::from_explicit_queues(gate, skip, residual, VecDeque::new(), VecDeque::new(), None));
    }
    let Work { parameters, blocks, head, inputs, .. } = work;
    match parameters.bind_initialized(parameters::Parts { blocks,
        condition_head: head.expect("completed condition head"), condition_rows: VecDeque::new(),
        needs_reset: false, pending_conditional_work: None }) {
        Ok(bound) => Ok(Bound { parameters: bound.parameters, network: bound.network, projection_inputs: inputs }),
        Err(failure) => Err(Failure::Network { failure, projection_inputs: inputs }),
    }
}
