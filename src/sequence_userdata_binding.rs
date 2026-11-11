use std::collections::{TryReserveError, VecDeque};
use crate::{conditional_gate::{Batch, BorrowedProjection, GateError, Projection},
    logical_read_rows::ReadRows, model::ModelNode,
    moda7_projection::{self, Moda7Projection}, moda7_value_services,
    modl0_float_parameters as float, modl0_quantized_parameters as quantized,
    modm0_logical_sequence::{CachePolicy, RequiredReset},
    sequence_parameters::{self as sequence, Decoded, Kind, Mismatch},
    sequence_value_binding::Linear,
    streaming_bank_parameters as bank, streaming_matrix::NoPostprocess,
    streaming_parameters, streaming_rows::ValueRows, streaming_value_binding,
    streaming_value_services::NoPacked};

pub type Activation = Moda7Projection<moda7_value_services::ValueServices>;
pub type Child<'n, 'data, I> = sequence::Child<I, Activation, BankOwner<'n, 'data>, Linear>;
pub struct BankInput {
    pub admission: bank::BankAdmission, pub policies: Vec<bank::TapPolicy<NoPostprocess>>,
    pub inputs: streaming_parameters::BindingInputs,
    pub state: streaming_parameters::InitialState,
    pub rows: ValueRows, pub packed: Option<NoPacked>,
}
pub enum BankFailure<'n, 'data> {
    Decode { cause: String, input: BankInput },
    Binding { node: &'n ModelNode<'data>, admission: bank::BankAdmission,
        failure: bank::Failure<NoPostprocess, ValueRows, NoPacked> },
}
pub struct BankOwner<'n, 'data> {
    node: &'n ModelNode<'data>, admission: bank::BankAdmission,
    bound: streaming_parameters::Bound<streaming_value_binding::Tap, ValueRows, NoPacked>,
}
impl<'n, 'data> BankOwner<'n, 'data> {
    pub fn node(&self) -> &'n ModelNode<'data> { self.node }
    pub fn admission(&self) -> &bank::BankAdmission { &self.admission }
    pub fn bound(&self) -> &streaming_parameters::Bound<streaming_value_binding::Tap, ValueRows, NoPacked> { &self.bound }
}
impl Projection for BankOwner<'_, '_> {
    fn reset(&mut self) -> Result<(), GateError> { self.bound.module.reset() }
    fn forward(&mut self, input: &Batch) -> Result<Batch, GateError> { self.bound.module.forward(input) }
}
impl BorrowedProjection for BankOwner<'_, '_> {
    fn forward_borrowed(&mut self, input: &dyn ReadRows) -> Result<Batch, GateError> { self.bound.module.forward_borrowed(input) }
}

pub fn bind_bank_node<'n, 'data>(node: &'n ModelNode<'data>, input: BankInput)
    -> Result<BankOwner<'n, 'data>, BankFailure<'n, 'data>>
{
    let admission = bank::BankAdmission { configuration: input.admission.configuration,
        input_width: input.admission.input_width, output_width: input.admission.output_width };
    let parameters = match bank::decode_source_bank(node, admission) {
        Ok(parameters) => parameters,
        Err(cause) => return Err(BankFailure::Decode { cause, input }),
    };
    let BankInput { admission, policies, inputs, state, rows, packed } = input;
    match bank::bind(bank::Parts { parameters, policies, inputs, state, rows, packed }) {
        Ok(bound) => Ok(BankOwner { node, admission, bound }),
        Err(failure) => Err(BankFailure::Binding { node, admission, failure }),
    }
}

pub struct QuantizedAllocationWorkspace {
    pub additive: crate::value_row_allocation::BackingPolicy,
    pub calls: crate::modl0_value_services::Calls,
    pub overwrite_fold: crate::quantized_matrix::OverwriteFold,
    pub lifecycle: crate::modl0_value_services::LifecyclePolicy,
    pub needs_reset: bool,
}
pub enum ChildInput<I> {
    Moda5(I),
    Moda7 { workspace: moda7_value_services::Parts, needs_reset: bool },
    Modl1(BankInput),
    Modl0Float(float::Workspace),
    Modl0FloatAllocated(float::AllocationWorkspace),
    Modl0Quantized { shift: u32, workspace: quantized::Workspace },
    Modl0QuantizedAllocated { shift: u32, workspace: QuantizedAllocationWorkspace },
}
impl<I> ChildInput<I> {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Moda5(_) => Kind::Moda5, Self::Moda7 { .. } => Kind::Moda7,
            Self::Modl1(_) => Kind::Modl1,
            Self::Modl0Float(_) | Self::Modl0FloatAllocated(_)
            | Self::Modl0Quantized { .. } | Self::Modl0QuantizedAllocated { .. } => Kind::Modl0,
        }
    }
}
pub struct Parts<I, R> {
    pub children: VecDeque<ChildInput<I>>, pub intermediates: Vec<Batch>,
    pub reset: R, pub cache_policy: CachePolicy, pub needs_reset: bool,
}
pub enum ChildFailure<'n, 'data> {
    Bank(BankFailure<'n, 'data>),
    FloatDecode { cause: String, workspace: float::Workspace },
    Float(float::Failure),
    FloatAllocationDecode { cause: String, workspace: float::AllocationWorkspace },
    FloatAllocation(float::AllocationFailure),
    QuantizedDecode { cause: String, shift: u32, workspace: quantized::Workspace },
    Quantized(quantized::Failure),
    QuantizedAllocationDecode { cause: String, shift: u32, workspace: QuantizedAllocationWorkspace },
    QuantizedAllocation(quantized::AllocationFailure),
}
pub enum PreparationCause { Metadata(Mismatch), Allocation(TryReserveError) }

pub struct Stopped<'n, 'data, I, R> {
    index: usize, parameters: Decoded<'n, 'data>, initialized: Vec<Child<'n, 'data, I>>,
    current: ChildFailure<'n, 'data>, remaining: Parts<I, R>,
}
impl<'n, 'data, I, R> Stopped<'n, 'data, I, R> {
    pub fn index(&self) -> usize { self.index }
    pub fn parameters(&self) -> &Decoded<'n, 'data> { &self.parameters }
    pub fn initialized(&self) -> &[Child<'n, 'data, I>] { &self.initialized }
    pub fn current(&self) -> &ChildFailure<'n, 'data> { &self.current }
    pub fn remaining(&self) -> &Parts<I, R> { &self.remaining }
}
pub enum Failure<'n, 'data, I, R> {
    Decode { node: &'n ModelNode<'data>, cause: sequence::DecodeError, parts: Parts<I, R> },
    Preparation { cause: PreparationCause, parameters: Decoded<'n, 'data>, parts: Parts<I, R> },
    Stopped(Stopped<'n, 'data, I, R>),
    Sequence(sequence::BindingFailure<'n, 'data, I, Activation, BankOwner<'n, 'data>, Linear, R>),
}

fn initialize_child<'n, 'data, I>(node: &'n ModelNode<'data>, input: ChildInput<I>)
    -> Result<Child<'n, 'data, I>, ChildFailure<'n, 'data>>
{
    Ok(match input {
        ChildInput::Moda5(owner) => sequence::Child::Moda5(owner),
        ChildInput::Moda7 { workspace, needs_reset } => sequence::Child::Moda7(
            Moda7Projection::from_explicit_parts(moda7_projection::Parts {
                services: moda7_value_services::ValueServices::from_explicit_parts(workspace), needs_reset })),
        ChildInput::Modl1(input) => sequence::Child::Modl1(bind_bank_node(node, input).map_err(ChildFailure::Bank)?),
        ChildInput::Modl0Float(workspace) => {
            let parameters = match float::decode_float_node(node) {
                Ok(parameters) => parameters,
                Err(cause) => return Err(ChildFailure::FloatDecode { cause, workspace }),
            };
            sequence::Child::Modl0(Linear::Float(float::bind_initialized(float::Parts { parameters, workspace }).map_err(ChildFailure::Float)?))
        }
        ChildInput::Modl0FloatAllocated(workspace) => {
            let parameters = match float::decode_float_node(node) {
                Ok(parameters) => parameters,
                Err(cause) => return Err(ChildFailure::FloatAllocationDecode { cause, workspace }),
            };
            sequence::Child::Modl0(Linear::Float(float::bind_allocated(float::AllocationParts { parameters, workspace }).map_err(ChildFailure::FloatAllocation)?))
        }
        ChildInput::Modl0Quantized { shift, workspace } => {
            let parameters = match quantized::decode_quantized_node(node, shift) {
                Ok(parameters) => parameters,
                Err(cause) => return Err(ChildFailure::QuantizedDecode { cause, shift, workspace }),
            };
            sequence::Child::Modl0(Linear::Quantized(quantized::bind_initialized(quantized::Parts { parameters, workspace }).map_err(ChildFailure::Quantized)?))
        }
        ChildInput::Modl0QuantizedAllocated { shift, workspace } => {
            let parameters = match quantized::decode_quantized_node(node, shift) {
                Ok(parameters) => parameters,
                Err(cause) => return Err(ChildFailure::QuantizedAllocationDecode { cause, shift, workspace }),
            };
            let QuantizedAllocationWorkspace { additive, calls, overwrite_fold, lifecycle, needs_reset } = workspace;
            sequence::Child::Modl0(Linear::Quantized(quantized::bind_allocated(quantized::AllocationParts {
                parameters, additive, calls, overwrite_fold, lifecycle, needs_reset }).map_err(ChildFailure::QuantizedAllocation)?))
        }
    })
}

pub fn bind_node<'n, 'data, I: BorrowedProjection, R>(node: &'n ModelNode<'data>, mut parts: Parts<I, R>)
    -> Result<sequence::Bound<'n, 'data, I, Activation, BankOwner<'n, 'data>, Linear, R>, Failure<'n, 'data, I, R>>
where R: RequiredReset<Child<'n, 'data, I>, Child<'n, 'data, I>>
{
    let parameters = match sequence::decode_node(node) {
        Ok(parameters) => parameters,
        Err(cause) => return Err(Failure::Decode { node, cause, parts }),
    };
    let count = parameters.children().len();
    if parts.children.len() != count {
        return Err(Failure::Preparation { cause: PreparationCause::Metadata(Mismatch::Count {
            expected: count, actual: parts.children.len() }), parameters, parts });
    }
    if let Some((index, expected, actual)) = parameters.children().iter().zip(&parts.children).enumerate()
        .find_map(|(index, (expected, actual))| (expected.kind() != actual.kind()).then_some((index, expected.kind(), actual.kind())))
    {
        return Err(Failure::Preparation { cause: PreparationCause::Metadata(Mismatch::Kind { index, expected, actual }), parameters, parts });
    }
    let mut initialized = Vec::new();
    if let Err(cause) = initialized.try_reserve_exact(count) {
        return Err(Failure::Preparation { cause: PreparationCause::Allocation(cause), parameters, parts });
    }
    while let Some(input) = parts.children.pop_front() {
        let index = initialized.len();
        match initialize_child(parameters.children()[index].node(), input) {
            Ok(child) => initialized.push(child),
            Err(current) => return Err(Failure::Stopped(Stopped { index, parameters, initialized, current, remaining: parts })),
        }
    }
    let Parts { children: _, intermediates, reset, cache_policy, needs_reset } = parts;
    parameters.bind_initialized(sequence::Parts { children: initialized, intermediates, reset, cache_policy, needs_reset })
        .map_err(Failure::Sequence)
}
