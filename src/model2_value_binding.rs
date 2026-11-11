use crate::{
    conditional_gate::Batch,
    model2_logical_chain::{self as chain, LaterChild},
    model2_value_reset::ResetChildrenThenArchiveIntermediates,
    moda7_projection::{self, Moda7Projection},
    moda7_value_services::ValueServices as Activation,
    modl0_projection::Modl0Projection,
    modl0_value_services::ValueServices as Linear,
    modm0_logical_sequence::{CachePolicy, LogicalSequence},
};

pub struct Context {
    pub middle: moda7_projection::Parts<Activation>,
    pub intermediates: Vec<Batch>,
    pub reset: ResetChildrenThenArchiveIntermediates,
    pub cache_policy: CachePolicy,
    pub needs_reset: bool,
}
impl Context {
    pub fn bind(self, first: Modl0Projection<Linear>, last: Modl0Projection<Linear>)
        -> Result<LogicalSequence<Modl0Projection<Linear>, LaterChild<Activation, Linear>,
                ResetChildrenThenArchiveIntermediates>,
            chain::Refusal<Linear, Activation, Linear, ResetChildrenThenArchiveIntermediates>>
    {
        let Self { middle, intermediates, reset, cache_policy, needs_reset } = self;
        let middle = Moda7Projection::from_explicit_parts(middle);
        chain::bind_initialized(chain::Parts {
            first, middle, last, intermediates, reset, cache_policy, needs_reset,
        })
    }
}
