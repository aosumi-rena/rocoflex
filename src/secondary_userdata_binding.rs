use crate::{conditional_gate::{Batch, GateError}, model::ModelNode,
    ncwn_parameters as parameters, ncwn_value_binding as values, userdata};
pub use crate::vocoder_userdata_binding::NetworkInput;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkChild { Child1, Child2, Child9 }
impl NetworkChild {
    pub const fn ordinal(self) -> usize {
        match self { Self::Child1 => 1, Self::Child2 => 2, Self::Child9 => 9 }
    }
}

pub struct Metadata<'u, 'data> {
    userdata: &'u userdata::Decoded<'data>, child: NetworkChild,
    node: &'u ModelNode<'data>,
}
impl<'u, 'data> Metadata<'u, 'data> {
    pub fn userdata(&self) -> &'u userdata::Decoded<'data> { self.userdata }
    pub fn child(&self) -> NetworkChild { self.child }
    pub fn node(&self) -> &'u ModelNode<'data> { self.node }
}

pub enum Cause<'u, 'data> {
    MissingChild { actual_children: usize },
    Decode(parameters::Error),
    Initialization(values::Failure<'u, 'data>),
}

pub struct Failure<'u, 'data> {
    userdata: &'u userdata::Decoded<'data>, child: NetworkChild,
    cause: Cause<'u, 'data>, untouched_inputs: Option<NetworkInput>,
}
impl<'u, 'data> Failure<'u, 'data> {
    pub fn userdata(&self) -> &'u userdata::Decoded<'data> { self.userdata }
    pub fn child(&self) -> NetworkChild { self.child }
    pub fn cause(&self) -> &Cause<'u, 'data> { &self.cause }
    pub fn untouched_inputs(&self) -> Option<&NetworkInput> { self.untouched_inputs.as_ref() }
}

pub struct Bound<'u, 'data> {
    metadata: Metadata<'u, 'data>, initialized: values::Bound<'u, 'data>,
}
impl<'u, 'data> Bound<'u, 'data> {
    pub fn metadata(&self) -> &Metadata<'u, 'data> { &self.metadata }
    pub fn initialized(&self) -> &values::Bound<'u, 'data> { &self.initialized }

    pub fn into_parts(self) -> (values::Bound<'u, 'data>, Metadata<'u, 'data>) {
        (self.initialized, self.metadata)
    }

    pub fn forward_owned(mut self, input: Batch, condition: Batch)
        -> Result<Completed<'u, 'data>, ForwardFailure<'u, 'data>>
    {
        match self.initialized.network.forward(&input, &condition) {
            Ok(output) => Ok(Completed { output, bound: self, input, condition }),
            Err(cause) => Err(ForwardFailure { cause, bound: self, input, condition }),
        }
    }
}

pub struct Completed<'u, 'data> {
    output: Batch, bound: Bound<'u, 'data>, input: Batch, condition: Batch,
}
impl<'u, 'data> Completed<'u, 'data> {
    pub fn output(&self) -> &Batch { &self.output }
    pub fn bound(&self) -> &Bound<'u, 'data> { &self.bound }
    pub fn input(&self) -> &Batch { &self.input }
    pub fn condition(&self) -> &Batch { &self.condition }
    pub fn into_parts(self) -> (Batch, Bound<'u, 'data>, Batch, Batch) {
        (self.output, self.bound, self.input, self.condition)
    }
}

pub struct ForwardFailure<'u, 'data> {
    cause: GateError, bound: Bound<'u, 'data>, input: Batch, condition: Batch,
}
impl<'u, 'data> ForwardFailure<'u, 'data> {
    pub fn cause(&self) -> &GateError { &self.cause }
    pub fn bound(&self) -> &Bound<'u, 'data> { &self.bound }
    pub fn input(&self) -> &Batch { &self.input }
    pub fn condition(&self) -> &Batch { &self.condition }
}

pub fn bind_network<'u, 'data>(userdata: &'u userdata::Decoded<'data>, child: NetworkChild,
    inputs: NetworkInput) -> Result<Bound<'u, 'data>, Failure<'u, 'data>>
{
    let secondary = userdata.secondary_module();
    let Some(node) = secondary.children.get(child.ordinal()) else {
        return Err(Failure { userdata, child, cause: Cause::MissingChild { actual_children: secondary.children.len() },
            untouched_inputs: Some(inputs) });
    };
    let parameters = match parameters::decode_node(node) {
        Ok(parameters) => parameters,
        Err(cause) => return Err(Failure { userdata, child, cause: Cause::Decode(cause), untouched_inputs: Some(inputs) }),
    };
    let NetworkInput { gates, skips, residuals, condition_head } = inputs;
    match values::bind(values::Parts { parameters, gates, skips, residuals, condition_head }) {
        Ok(initialized) => Ok(Bound { metadata: Metadata { userdata, child, node }, initialized }),
        Err(cause) => Err(Failure { userdata, child, cause: Cause::Initialization(cause), untouched_inputs: None }),
    }
}
