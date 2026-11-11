use crate::{
    model2_logical_chain::{self as chain, LaterChild},
    model2_value_binding::Context,
    model2_value_reset::ResetChildrenThenArchiveIntermediates,
    moda7_value_services::ValueServices as Activation,
    modl0_projection::Modl0Projection,
    modl0_quantized_parameters as linear,
    modl0_value_services::ValueServices as Linear,
    modm0_logical_sequence::LogicalSequence,
    quantized_matrix::OverwriteFold,
    timbre::{QuantizedLinear, QuantizedParameterOwnerError},
};

pub struct Parameters { pub first: QuantizedLinear, pub last: QuantizedLinear }
pub struct Workspace {
    pub first: linear::Workspace,
    pub last: linear::Workspace,
    pub context: Context,
}
pub struct Parts { pub parameters: Parameters, pub workspace: Workspace }
pub enum Failure {
    Disconnected { parts: Parts },

    HeadFold { parts: Parts },
    FirstParameters { cause: QuantizedParameterOwnerError, parts: Parts },

    LastParameters {
        cause: QuantizedParameterOwnerError,
        first: linear::Decoded,
        last: QuantizedLinear,
        workspace: Workspace,
    },
    FirstBinding { failure: linear::Failure, last: linear::Parts, context: Context },
    LastBinding { first: linear::Bound, failure: linear::Failure, context: Context },
    Sequence {
        logical_bias: [Vec<f32>; 2],
        refusal: chain::Refusal<Linear, Activation, Linear, ResetChildrenThenArchiveIntermediates>,
    },
}
pub struct Bound {

    pub logical_bias: [Vec<f32>; 2],
    pub chain: LogicalSequence<Modl0Projection<Linear>, LaterChild<Activation, Linear>,
        ResetChildrenThenArchiveIntermediates>,
}

pub fn bind_initialized(parts: Parts) -> Result<Bound, Failure> {
    if parts.parameters.first.out_dim != parts.parameters.last.in_dim {
        return Err(Failure::Disconnected { parts });
    }
    let Parts { parameters: Parameters { first, last }, workspace } = parts;
    let first = match linear::Decoded::from_owned_linear(first) {
        Ok(first) => first,
        Err((cause, first)) => return Err(Failure::FirstParameters {
            cause, parts: Parts { parameters: Parameters { first, last }, workspace },
        }),
    };
    let last = match linear::Decoded::from_owned_linear(last) {
        Ok(last) => last,
        Err((cause, last)) => return Err(Failure::LastParameters { cause, first, last, workspace }),
    };
    let Workspace { first: first_workspace, last: last_workspace, context } = workspace;
    let last = linear::Parts { parameters: last, workspace: last_workspace };
    let first = match linear::bind_initialized(linear::Parts {
        parameters: first, workspace: first_workspace,
    }) {
        Ok(first) => first,
        Err(failure) => return Err(Failure::FirstBinding { failure, last, context }),
    };
    let last = match linear::bind_initialized(last) {
        Ok(last) => last,
        Err(failure) => return Err(Failure::LastBinding { first, failure, context }),
    };
    let linear::Bound { logical_bias: first_bias, projection: first } = first;
    let linear::Bound { logical_bias: last_bias, projection: last } = last;
    let logical_bias = [first_bias, last_bias];
    match context.bind(first, last) {
        Ok(chain) => Ok(Bound { logical_bias, chain }),
        Err(refusal) => Err(Failure::Sequence { logical_bias, refusal }),
    }
}

fn bind_saturating_head(parameters: Parameters, workspace: Workspace) -> Result<Bound, Failure> {
    if workspace.first.overwrite_fold != OverwriteFold::SaturateEach
        || workspace.last.overwrite_fold != OverwriteFold::SaturateEach
    {
        return Err(Failure::HeadFold { parts: Parts { parameters, workspace } });
    }
    bind_initialized(Parts { parameters, workspace })
}

impl crate::timbre::TimbreHead {

    pub fn into_initialized_value_chain(self, workspace: Workspace) -> Result<Bound, Failure> {
        let (first, last) = self.into_linear_owners();
        bind_saturating_head(Parameters { first, last }, workspace)
    }
}
impl crate::pitch_score::PitchScore {

    pub fn into_initialized_value_chain(self, workspace: Workspace) -> Result<Bound, Failure> {
        let (first, last) = self.into_linear_owners();
        bind_saturating_head(Parameters { first, last }, workspace)
    }
}
