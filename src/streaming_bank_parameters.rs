use std::collections::TryReserveError;
use crate::{bank::Conv, model::ModelNode, projected_conv_binding::SharedMatrixProfile,
    quantized_matrix::{OverwriteFold, QuantizedMatrix}, streaming_matrix::Postprocess,
    streaming_parameters::{self as parameters, BindingInputs, InitialState},
    streaming_quantized_matrix::{self as quantized, QuantizedValueTap},
    streaming_value_services::{RequiredPacked, RequiredRows}};

pub struct BankAdmission {
    pub configuration: [u32; 5], pub input_width: usize, pub output_width: usize,
}
pub struct Decoded {
    configuration: [u8; 20], taps: Vec<QuantizedMatrix>, bias: Vec<f32>,
}
impl Decoded {
    pub fn configuration(&self) -> &[u8; 20] { &self.configuration }
    pub fn taps(&self) -> &[QuantizedMatrix] { &self.taps }
    pub fn bias(&self) -> &[f32] { &self.bias }
}

pub fn decode_source_bank(node: &ModelNode<'_>, admission: BankAdmission) -> crate::Result<Decoded> {
    let decoded = Conv::load(node, admission.configuration, admission.input_width, admission.output_width)?;
    let configuration = node.data.try_into().expect("bank loader admitted exact twenty bytes");
    let (taps, bias) = decoded.into_parameter_owners();
    Ok(Decoded { configuration, taps, bias })
}

pub struct TapPolicy<P> {
    pub overwrite_fold: OverwriteFold, pub postprocess: Option<P>,
    pub compatible_profile: Option<SharedMatrixProfile>,
}
pub struct Parts<P, F, R> {
    pub parameters: Decoded, pub policies: Vec<TapPolicy<P>>, pub inputs: BindingInputs,
    pub state: InitialState,
    pub rows: F, pub packed: Option<R>,
}
pub enum Cause {
    ConfigurationMismatch, PolicyCount { taps: usize, policies: usize },
    ChildCountOverflow, Allocation(TryReserveError),
}
pub enum Failure<P: Postprocess, F: RequiredRows, R: RequiredPacked> {
    Preparation { cause: Cause, parts: Parts<P, F, R> },
    Binding(parameters::Failure<QuantizedValueTap<P>, F, R>),
}

pub fn bind<P: Postprocess, F: RequiredRows, R: RequiredPacked>(parts: Parts<P, F, R>)
    -> Result<parameters::Bound<QuantizedValueTap<P>, F, R>, Failure<P, F, R>> {
    if parts.inputs.configuration.as_slice() != parts.parameters.configuration.as_slice() {
        return Err(Failure::Preparation { cause: Cause::ConfigurationMismatch, parts });
    }
    let taps = parts.parameters.taps.len();
    if parts.policies.len() != taps {
        return Err(Failure::Preparation { cause: Cause::PolicyCount { taps, policies: parts.policies.len() }, parts });
    }

    let count = match taps.checked_add(1) {
        Some(count) => count,
        None => return Err(Failure::Preparation { cause: Cause::ChildCountOverflow, parts }),
    };
    let mut children = Vec::new();
    if let Err(error) = children.try_reserve_exact(count) {
        return Err(Failure::Preparation { cause: Cause::Allocation(error), parts });
    }
    let Parts { parameters: Decoded { configuration: _, taps, bias }, policies, inputs,
        state, rows, packed } = parts;
    for (matrix, policy) in taps.into_iter().zip(policies) {
        let TapPolicy { overwrite_fold, postprocess, compatible_profile } = policy;
        let value = QuantizedValueTap::from_explicit_product_parts(quantized::Parts { matrix, overwrite_fold, postprocess });
        children.push(parameters::Child::Matrix { value, compatible_profile });
    }
    children.push(parameters::Child::Bias(bias));
    parameters::bind(parameters::Parts { inputs, children, state, rows, packed }).map_err(Failure::Binding)
}
