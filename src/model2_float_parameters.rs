use crate::{
    model::ModelNode,
    model_tags::decoded_hashes,
    model2_logical_chain::{self as chain, LaterChild},
    model2_value_binding::Context,
    model2_value_reset::ResetChildrenThenArchiveIntermediates,
    moda7_value_services::ValueServices as Activation,
    modl0_float_parameters as linear,
    modl0_projection::Modl0Projection,
    modl0_value_services::ValueServices as Linear,
    modm0_logical_sequence::LogicalSequence,
};

pub struct Decoded {
    first: linear::Decoded,
    last: linear::Decoded,
}
impl Decoded {
    pub fn first(&self) -> &linear::Decoded { &self.first }
    pub fn last(&self) -> &linear::Decoded { &self.last }
}

pub fn decode_float_chain(node: &ModelNode<'_>) -> crate::Result<Decoded> {
    node.require(0x7fca41ff, b"modm0", decoded_hashes("modm0"))?;
    if !node.data.is_empty() || node.children.len() != 3 {
        return Err("float model chain requires exactly Modl0, Moda7, Modl0".into());
    }
    let activation = &node.children[1];
    activation.require(0x7fca41ff, b"moda7", decoded_hashes("moda7"))?;
    if !activation.data.is_empty() || !activation.children.is_empty() {
        return Err("float model activation must be an empty Moda7 leaf".into());
    }
    let first = linear::decode_float_node(&node.children[0])?;
    let last = linear::decode_float_node(&node.children[2])?;
    if first.output_width() != last.input_width() {
        return Err("float model chain linear dimensions do not connect through Moda7".into());
    }
    Ok(Decoded { first, last })
}

pub enum Initialization {
    Initialized(linear::Workspace), Allocate(linear::AllocationWorkspace),
}
pub struct Parts {
    pub parameters: Decoded,
    pub first: Initialization,
    pub last: Initialization,
    pub context: Context,
}
pub enum LinearFailure {
    Initialized(linear::Failure), Allocation(linear::AllocationFailure),
}
pub enum Failure {
    First { failure: LinearFailure, last_parameters: linear::Decoded,
        last: Initialization, context: Context },

    Last { first: linear::Bound, failure: LinearFailure, context: Context },

    Sequence {
        parameters: Decoded,
        refusal: chain::Refusal<Linear, Activation, Linear, ResetChildrenThenArchiveIntermediates>,
    },
}
pub struct Bound {
    pub parameters: Decoded,
    pub chain: LogicalSequence<Modl0Projection<Linear>, LaterChild<Activation, Linear>,
        ResetChildrenThenArchiveIntermediates>,
}

pub fn bind(parts: Parts) -> Result<Bound, Failure> {
    let Parts { parameters: Decoded { first, last }, first: first_workspace,
        last: last_workspace, context } = parts;
    let first = match bind_linear(first, first_workspace) {
        Ok(first) => first,
        Err(failure) => return Err(Failure::First {
            failure, last_parameters: last, last: last_workspace, context }),
    };
    let last = match bind_linear(last, last_workspace) {
        Ok(last) => last,
        Err(failure) => return Err(Failure::Last { first, failure, context }),
    };
    let linear::Bound { parameters: first_parameters, projection: first } = first;
    let linear::Bound { parameters: last_parameters, projection: last } = last;
    let parameters = Decoded { first: first_parameters, last: last_parameters };
    match context.bind(first, last) {
        Ok(chain) => Ok(Bound { parameters, chain }),
        Err(refusal) => Err(Failure::Sequence { parameters, refusal }),
    }
}

fn bind_linear(parameters: linear::Decoded, initialization: Initialization)
    -> Result<linear::Bound, LinearFailure>
{
    match initialization {
        Initialization::Initialized(workspace) => linear::bind_initialized(
            linear::Parts { parameters, workspace }).map_err(LinearFailure::Initialized),
        Initialization::Allocate(workspace) => linear::bind_allocated(
            linear::AllocationParts { parameters, workspace }).map_err(LinearFailure::Allocation),
    }
}
