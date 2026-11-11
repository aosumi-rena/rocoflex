use std::collections::TryReserveError;
use crate::{backend_factory::SourceFloatFactory,
    ncwn_parameters::{ProjectionDecodeError, ProjectionNode},
    projected_conv_binding::PROJECTED_CONV_VTABLE_RVA,
    streaming_bank_parameters as bank, streaming_matrix::NoPostprocess,
    streaming_module::Module, streaming_parameters::{self as parameters, BindingInputs, InitialState},
    streaming_quantized_matrix::QuantizedValueTap,
    streaming_rows::{Policy as RowPolicy, ValueRows},
    streaming_value_services::{NoPacked, ValueServices}};

pub type Tap = QuantizedValueTap<NoPostprocess>;
pub type Projection = Module<ValueServices<Tap, ValueRows, NoPacked>>;
pub struct Policy {
    pub factory: SourceFloatFactory, pub flags224: u8, pub rows: RowPolicy,
    pub taps: Vec<bank::TapPolicy<NoPostprocess>>,
}
pub enum Failure {
    Decode { cause: ProjectionDecodeError, policy: Policy },
    ConfigurationAllocation { cause: TryReserveError, parameters: bank::Decoded, policy: Policy },
    Binding(bank::Failure<NoPostprocess, ValueRows, NoPacked>),
}

pub fn bind_from_model(node: ProjectionNode<'_, '_>, policy: Policy)
    -> Result<parameters::Bound<Tap, ValueRows, NoPacked>, Failure>
{
    let decoded = match node.decode_source_bank() {
        Ok(decoded) => decoded,
        Err(cause) => return Err(Failure::Decode { cause, policy }),
    };
    let mut configuration = Vec::new();
    if let Err(cause) = configuration.try_reserve_exact(decoded.configuration().len()) {
        return Err(Failure::ConfigurationAllocation { cause, parameters: decoded, policy });
    }
    configuration.extend_from_slice(decoded.configuration());
    let Policy { factory, flags224, rows, taps } = policy;
    bank::bind(bank::Parts::<NoPostprocess, ValueRows, NoPacked> {
        parameters: decoded, policies: taps,
        inputs: BindingInputs { vtable_rva: PROJECTED_CONV_VTABLE_RVA, configuration, factory },
        state: InitialState::Constructor { flags224 },
        rows: ValueRows::from_explicit_policy(rows), packed: None,
    }).map_err(Failure::Binding)
}
