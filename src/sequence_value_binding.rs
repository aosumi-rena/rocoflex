use std::collections::{TryReserveError, VecDeque};
use crate::{
    conditional_gate::{Batch, BorrowedProjection, GateError, Projection},
    logical_read_rows::ReadRows,
    moda7_projection::{self, Moda7Projection}, moda7_value_services,
    modl0_float_parameters as float, modl0_quantized_parameters as quantized,
    modl0_projection::{self, Modl0Projection}, modl0_value_services,
    modm0_logical_sequence::{CachePolicy, RequiredReset},
    sequence_parameters::{self as sequence, Decoded, Kind, Mismatch},
    streaming_module::Module, streaming_projection::Services,
};

pub enum Linear {
    Float(float::Bound),
    Quantized(quantized::Bound),
    Explicit(Modl0Projection<modl0_value_services::ValueServices>),
}
impl Projection for Linear {
    fn reset(&mut self) -> Result<(), GateError> {
        match self {
            Self::Float(bound) => bound.projection.reset(),
            Self::Quantized(bound) => bound.projection.reset(),
            Self::Explicit(projection) => projection.reset(),
        }
    }
    fn forward(&mut self, input: &Batch) -> Result<Batch, GateError> {
        match self {
            Self::Float(bound) => bound.projection.forward(input),
            Self::Quantized(bound) => bound.projection.forward(input),
            Self::Explicit(projection) => projection.forward(input),
        }
    }
}
impl BorrowedProjection for Linear {
    fn forward_borrowed(&mut self, input: &dyn ReadRows) -> Result<Batch, GateError> {
        match self {
            Self::Float(bound) => bound.projection.forward_borrowed(input),
            Self::Quantized(bound) => bound.projection.forward_borrowed(input),
            Self::Explicit(projection) => projection.forward_borrowed(input),
        }
    }
}

pub enum ChildInput<I, S: Services> {
    Moda5(I),
    Moda7 { services: moda7_value_services::Parts, needs_reset: bool },
    Modl1(Module<S>),
    Modl0Float(float::Parts),
    Modl0FloatAllocated(float::AllocationParts),
    Modl0Quantized(quantized::Parts),
    Modl0QuantizedAllocated(quantized::AllocationParts),
    Modl0Explicit { services: modl0_value_services::Parts, needs_reset: bool },
}
impl<I, S: Services> ChildInput<I, S> {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Moda5(_) => Kind::Moda5,
            Self::Moda7 { .. } => Kind::Moda7,
            Self::Modl1(_) => Kind::Modl1,
            Self::Modl0Float(_) | Self::Modl0FloatAllocated(_)
            | Self::Modl0Quantized(_) | Self::Modl0QuantizedAllocated(_)
            | Self::Modl0Explicit { .. } => Kind::Modl0,
        }
    }
}
pub struct Parts<I, S: Services, R> {
    pub children: VecDeque<ChildInput<I, S>>,
    pub intermediates: Vec<Batch>,
    pub reset: R,
    pub cache_policy: CachePolicy,
    pub needs_reset: bool,
}
pub enum PreparationCause { Metadata(Mismatch), Allocation(TryReserveError) }

pub enum ChildFailure {
    Modl0Float(float::Failure),
    Modl0FloatAllocated(float::AllocationFailure),
    Modl0Quantized(quantized::Failure),
    Modl0QuantizedAllocated(quantized::AllocationFailure),
}
pub enum Failure<'n, 'data, I, S: Services, R> {

    Preparation { cause: PreparationCause, parameters: Decoded<'n, 'data>, parts: Parts<I, S, R> },

    Child { index: usize, parameters: Decoded<'n, 'data>,
        initialized: Vec<sequence::Child<I, Moda7Projection<moda7_value_services::ValueServices>, Module<S>, Linear>>,
        current: ChildFailure, remaining: Parts<I, S, R> },

    Sequence(sequence::BindingFailure<'n, 'data, I,
        Moda7Projection<moda7_value_services::ValueServices>, Module<S>, Linear, R>),
}

fn initialize_child<I, S: Services>(input: ChildInput<I, S>)
    -> Result<sequence::Child<I, Moda7Projection<moda7_value_services::ValueServices>, Module<S>, Linear>, ChildFailure>
{
    Ok(match input {
        ChildInput::Moda5(owner) => sequence::Child::Moda5(owner),
        ChildInput::Moda7 { services, needs_reset } => sequence::Child::Moda7(
            Moda7Projection::from_explicit_parts(moda7_projection::Parts {
                services: moda7_value_services::ValueServices::from_explicit_parts(services), needs_reset,
            })),
        ChildInput::Modl1(module) => sequence::Child::Modl1(module),
        ChildInput::Modl0Float(parts) => sequence::Child::Modl0(Linear::Float(
            float::bind_initialized(parts).map_err(ChildFailure::Modl0Float)?)),
        ChildInput::Modl0FloatAllocated(parts) => sequence::Child::Modl0(Linear::Float(
            float::bind_allocated(parts).map_err(ChildFailure::Modl0FloatAllocated)?)),
        ChildInput::Modl0Quantized(parts) => sequence::Child::Modl0(Linear::Quantized(
            quantized::bind_initialized(parts).map_err(ChildFailure::Modl0Quantized)?)),
        ChildInput::Modl0QuantizedAllocated(parts) => sequence::Child::Modl0(Linear::Quantized(
            quantized::bind_allocated(parts).map_err(ChildFailure::Modl0QuantizedAllocated)?)),
        ChildInput::Modl0Explicit { services, needs_reset } => sequence::Child::Modl0(Linear::Explicit(
            Modl0Projection::from_explicit_parts(modl0_projection::Parts {
                services: modl0_value_services::ValueServices::from_explicit_parts(services), needs_reset,
            }))),
    })
}

pub fn bind_initialized<'n, 'data, I: BorrowedProjection, S: Services, R>(
    parameters: Decoded<'n, 'data>, mut parts: Parts<I, S, R>)
    -> Result<sequence::Bound<'n, 'data, I,
        Moda7Projection<moda7_value_services::ValueServices>, Module<S>, Linear, R>,
        Failure<'n, 'data, I, S, R>>
where R: RequiredReset<
    sequence::Child<I, Moda7Projection<moda7_value_services::ValueServices>, Module<S>, Linear>,
    sequence::Child<I, Moda7Projection<moda7_value_services::ValueServices>, Module<S>, Linear>>
{
    let count = parameters.children().len();
    if parts.children.len() != count {
        return Err(Failure::Preparation { cause: PreparationCause::Metadata(Mismatch::Count {
            expected: count, actual: parts.children.len() }), parameters, parts });
    }
    if let Some((index, expected, actual)) = parameters.children().iter().zip(&parts.children)
        .enumerate().find_map(|(index, (expected, actual))| {
            (expected.kind() != actual.kind()).then_some((index, expected.kind(), actual.kind()))
        })
    {
        return Err(Failure::Preparation { cause: PreparationCause::Metadata(Mismatch::Kind {
            index, expected, actual }), parameters, parts });
    }

    let mut initialized = Vec::new();
    if let Err(cause) = initialized.try_reserve_exact(count) {
        return Err(Failure::Preparation { cause: PreparationCause::Allocation(cause), parameters, parts });
    }
    while let Some(input) = parts.children.pop_front() {
        match initialize_child(input) {
            Ok(child) => initialized.push(child),
            Err(current) => return Err(Failure::Child {
                index: initialized.len(), parameters, initialized, current, remaining: parts,
            }),
        }
    }
    let Parts { children: _, intermediates, reset, cache_policy, needs_reset } = parts;
    parameters.bind_initialized(sequence::Parts { children: initialized, intermediates,
        reset, cache_policy, needs_reset }).map_err(Failure::Sequence)
}
