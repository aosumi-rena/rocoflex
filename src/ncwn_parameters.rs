use std::collections::{TryReserveError, VecDeque};
use crate::{condition_history::ConditionHistory,
    conditional_gate::{Auxiliary, ConditionalGate, GateConditionalWork, GateExplicitState,
        GateStateRefusal, Projection, VectorKernels},
    conditional_ncwn::{Block, ConditionalNcwn, ConditionedTransform, NcwnConditionalWork,
        NcwnConfig, NcwnExplicitState, NcwnStateRefusal},
    model::ModelNode, model_tags::decoded_hashes, root_binding::{BindingError, GateConfig, GatePlan}};

const MODULE: u32 = 0x7fca41ff;

const OBSERVED_20: [u64; 3] = [0xb4b47af0170a2c31, 0xf4a5eed3928cbe78, 0x5a73eed7f90cb153];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site { Network, Gates, Skips, Residuals }
#[derive(Debug)]
pub enum Error {
    Operator { site: Site, cause: String },
    Shape { site: Site, bytes: usize, children: usize },
    Configuration, GateSignature { gate: usize, kind: u32, tag: u64, bytes: usize },
    GateConfiguration { gate: usize, cause: BindingError },
    GateDomain { gate: usize }, GateWidth { gate: usize },
    ListCounts { gates: usize, skips: usize, residuals: usize },
    Allocation(TryReserveError),
}

#[derive(Clone, Copy)]
pub struct ProjectionNode<'n, 'data> {
    node: &'n ModelNode<'data>, input_width: usize, output_width: usize,
}
impl<'n, 'data> ProjectionNode<'n, 'data> {
    pub fn node(&self) -> &'n ModelNode<'data> { self.node }
    pub fn input_width(&self) -> usize { self.input_width }
    pub fn output_width(&self) -> usize { self.output_width }

    pub fn decode_source_bank(&self)
        -> Result<crate::streaming_bank_parameters::Decoded, ProjectionDecodeError>
    {
        let config = crate::projected_conv_binding::ProjectedConvConfig::parse(self.node.data)
            .map_err(ProjectionDecodeError::Configuration)?;
        let configuration = [config.taps as u32, config.stride as u32, config.padding as u32,
            config.dilation as u32, config.fifth_word as u32];
        crate::streaming_bank_parameters::decode_source_bank(self.node,
            crate::streaming_bank_parameters::BankAdmission { configuration,
                input_width: self.input_width, output_width: self.output_width })
            .map_err(ProjectionDecodeError::Parameters)
    }
}
#[derive(Debug)]
pub enum ProjectionDecodeError {
    Configuration(crate::projected_conv_binding::ConvBindingError), Parameters(String),
}
pub struct Gate<'n, 'data> { node: &'n ModelNode<'data>, config: GateConfig, plan: GatePlan }
impl<'n, 'data> Gate<'n, 'data> {
    pub fn node(&self) -> &'n ModelNode<'data> { self.node }
    pub fn configuration(&self) -> GateConfig { self.config }
    pub fn plan(&self) -> GatePlan { self.plan }
    pub fn main_projection(&self) -> ProjectionNode<'n, 'data> {
        ProjectionNode { node: &self.node.children[self.plan.main_projection_child],
            input_width: self.config.input_width as usize, output_width: 2 * self.config.half_width as usize }
    }
    pub fn condition_projection(&self) -> Option<ProjectionNode<'n, 'data>> {
        self.plan.condition_projection_child.and_then(|index| self.node.children.get(index))
            .map(|node| ProjectionNode { node, input_width: self.config.condition_width as usize,
                output_width: 2 * self.config.half_width as usize })
    }
    pub fn auxiliary(&self) -> Option<&'n ModelNode<'data>> {
        self.plan.auxiliary_child.and_then(|index| self.node.children.get(index))
    }

    pub fn bind_initialized<P: Projection, C: Projection, A: Auxiliary, K: VectorKernels>(
        &self, parts: GateParts<P, C, A, K>)
        -> Result<ConditionalGate<P, C, A, K>, GateStateRefusal<P, C, A, K>>
    {
        let GateParts { main, condition, auxiliary, kernels, history, needs_reset, pending_conditional_work } = parts;
        ConditionalGate::from_explicit_state(GateExplicitState { config: self.config,
            main, condition, auxiliary, kernels, history, needs_reset, pending_conditional_work })
    }
}
pub struct GateParts<P, C, A, K> {
    pub main: P, pub condition: C, pub auxiliary: Option<A>, pub kernels: K,
    pub history: ConditionHistory<Vec<f32>>, pub needs_reset: bool,
    pub pending_conditional_work: Option<GateConditionalWork>,
}
pub struct Layer<'n, 'data> {
    gate: Gate<'n, 'data>, skip: &'n ModelNode<'data>, residual: &'n ModelNode<'data>,
}
impl<'n, 'data> Layer<'n, 'data> {
    pub fn gate(&self) -> &Gate<'n, 'data> { &self.gate }
    pub fn skip(&self) -> ProjectionNode<'n, 'data> {
        ProjectionNode { node: self.skip, input_width: self.gate.config.half_width as usize,
            output_width: self.gate.config.half_width as usize }
    }
    pub fn residual(&self) -> ProjectionNode<'n, 'data> {
        ProjectionNode { node: self.residual, input_width: self.gate.config.input_width as usize,
            output_width: self.gate.config.half_width as usize }
    }
}
pub struct Decoded<'n, 'data> {
    node: &'n ModelNode<'data>, config: NcwnConfig,
    layers: Vec<Layer<'n, 'data>>, condition_head: &'n ModelNode<'data>,
}
impl<'n, 'data> Decoded<'n, 'data> {
    pub fn node(&self) -> &'n ModelNode<'data> { self.node }
    pub fn configuration(&self) -> NcwnConfig { self.config }
    pub fn layers(&self) -> &[Layer<'n, 'data>] { &self.layers }
    pub fn condition_head(&self) -> ProjectionNode<'n, 'data> {
        ProjectionNode { node: self.condition_head, input_width: self.config.condition_width,
            output_width: self.config.output_width }
    }

    pub fn bind_initialized<G: ConditionedTransform, S: Projection, R: Projection, H: Projection>(
        self, parts: Parts<G, S, R, H>) -> Result<Bound<'n, 'data, G, S, R, H>, Failure<'n, 'data, G, S, R, H>>
    {
        if parts.blocks.len() != self.layers.len() { return Err(Failure::BlockCount { parameters: self, parts }); }
        let Parts { blocks, condition_head, condition_rows, needs_reset, pending_conditional_work } = parts;
        match ConditionalNcwn::from_explicit_state(NcwnExplicitState { config: self.config,
            blocks, condition_head, condition_rows, needs_reset, pending_conditional_work }) {
            Ok(network) => Ok(Bound { parameters: self, network }),
            Err(refusal) => Err(Failure::Network { parameters: self, refusal }),
        }
    }
}
pub struct Parts<G, S, R, H> {
    pub blocks: Vec<Block<G, S, R>>, pub condition_head: H,
    pub condition_rows: VecDeque<Vec<f32>>, pub needs_reset: bool,
    pub pending_conditional_work: Option<NcwnConditionalWork>,
}
pub enum Failure<'n, 'data, G, S, R, H> {
    BlockCount { parameters: Decoded<'n, 'data>, parts: Parts<G, S, R, H> },
    Network { parameters: Decoded<'n, 'data>, refusal: NcwnStateRefusal<G, S, R, H> },
}
pub struct Bound<'n, 'data, G, S, R, H> {
    pub parameters: Decoded<'n, 'data>, pub network: ConditionalNcwn<G, S, R, H>,
}

fn shape(node: &ModelNode<'_>, site: Site) -> Error {
    Error::Shape { site, bytes: node.data.len(), children: node.children.len() }
}
fn group(node: &ModelNode<'_>, site: Site) -> Result<(), Error> {
    node.require(MODULE, b"cmpg1", decoded_hashes("cmpg1"))
        .map_err(|cause| Error::Operator { site, cause })?;
    if !node.data.is_empty() { return Err(shape(node, site)); }
    Ok(())
}
fn gate<'n, 'data>(node: &'n ModelNode<'data>, index: usize) -> Result<Gate<'n, 'data>, Error> {
    let admitted = if node.data.len() == 12 {
        node.require(MODULE, b"_gnc1v0", decoded_hashes("_gnc1v0")).is_ok()
    } else {
        node.version() == 2 && node.kind == MODULE && node.data.len() == 20 && OBSERVED_20.contains(&node.tag)
    };
    if !admitted { return Err(Error::GateSignature { gate: index, kind: node.kind, tag: node.tag, bytes: node.data.len() }); }
    let config = GateConfig::parse(node.data).map_err(|cause| Error::GateConfiguration { gate: index, cause })?;
    if config.input_width <= 0 || config.half_width <= 0 || config.condition_width <= 0
        || config.warmup_rows < 0 || config.half_width > i32::MAX / 2
    { return Err(Error::GateDomain { gate: index }); }
    let plan = config.plan(node.children.len()).map_err(|cause| Error::GateConfiguration { gate: index, cause })?;
    Ok(Gate { node, config, plan })
}

pub fn decode_node<'n, 'data>(node: &'n ModelNode<'data>) -> Result<Decoded<'n, 'data>, Error> {
    node.require(MODULE, b"_ncwnv0", decoded_hashes("_ncwnv0"))
        .map_err(|cause| Error::Operator { site: Site::Network, cause })?;
    if node.data.len() != 12 || node.children.len() != 4 { return Err(shape(node, Site::Network)); }
    let mut words = [0i32; 3];
    for (index, word) in words.iter_mut().enumerate() {
        *word = i32::from_le_bytes(node.data[index * 4..index * 4 + 4].try_into().unwrap());
    }
    if words.iter().any(|word| *word <= 0) { return Err(Error::Configuration); }
    let config = NcwnConfig { input_width: words[0] as usize, output_width: words[1] as usize,
        condition_width: words[2] as usize };
    for (index, site) in [Site::Gates, Site::Skips, Site::Residuals].into_iter().enumerate() {
        group(&node.children[index], site)?;
    }
    let gates = &node.children[0].children;
    let skips = &node.children[1].children;
    let residuals = &node.children[2].children;
    if gates.is_empty() || gates.len() != skips.len() || gates.len() != residuals.len() {
        return Err(Error::ListCounts { gates: gates.len(), skips: skips.len(), residuals: residuals.len() });
    }
    let mut layers = Vec::new();
    layers.try_reserve_exact(gates.len()).map_err(Error::Allocation)?;
    for (index, node) in gates.iter().enumerate() {
        let gate = gate(node, index)?;
        let expected_input = if index == 0 { config.input_width } else { config.output_width };
        if gate.config.input_width as usize != expected_input || gate.config.half_width as usize != config.output_width
            || gate.config.condition_width as usize != config.condition_width
        { return Err(Error::GateWidth { gate: index }); }
        layers.push(Layer { gate, skip: &skips[index], residual: &residuals[index] });
    }
    Ok(Decoded { node, config, layers, condition_head: &node.children[3] })
}
