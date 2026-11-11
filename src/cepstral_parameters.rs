use std::collections::TryReserveError;
use crate::{cepstral, model::ModelNode, model_tags::decoded_hashes};

const MODULE: u32 = 0x7fca41ff;
const OBSERVED_V2: u64 = 0xc666b3679006cb9f;
#[derive(Debug)]
pub enum DecodeError {
    Version(u32), Operator { kind: u32, tag: u64 }, HeaderLength(usize),
    Mode(u32), BandCount(u32), Extent,
    ConfigurationLength { expected: usize, actual: usize },
    OuterChildren(usize), Group(String),
    GroupShape { bytes: usize, children: usize, expected: usize }, Allocation(TryReserveError),
}

#[derive(Clone, Copy)]
pub struct BandRecord<'data> { bytes: &'data [u8] }
impl<'data> BandRecord<'data> {
    pub fn bytes(&self) -> &'data [u8] { self.bytes }
    pub fn words(&self) -> [u32; 4] {
        std::array::from_fn(|index| u32::from_le_bytes(self.bytes[index * 4..index * 4 + 4].try_into().unwrap()))
    }
}
pub struct Band<'n, 'data> { record: BandRecord<'data>, projection: &'n ModelNode<'data> }
impl<'n, 'data> Band<'n, 'data> {
    pub fn configuration(&self) -> BandRecord<'data> { self.record }
    pub fn projection(&self) -> &'n ModelNode<'data> { self.projection }
    pub fn decode_sequence(&self) -> Result<crate::sequence_parameters::Decoded<'n, 'data>, crate::sequence_parameters::DecodeError> {
        crate::sequence_parameters::decode_node(self.projection)
    }
}
pub struct Decoded<'n, 'data> {
    node: &'n ModelNode<'data>, mode: u32, bands: Vec<Band<'n, 'data>>,
}
impl<'n, 'data> Decoded<'n, 'data> {
    pub fn node(&self) -> &'n ModelNode<'data> { self.node }
    pub fn mode(&self) -> u32 { self.mode }
    pub fn bands(&self) -> &[Band<'n, 'data>] { &self.bands }

    pub fn initialize_values<P>(self, inputs: crate::cepstral_value_binding::Inputs<P>)
        -> Result<Bound<'n, 'data, P>, crate::cepstral_value_binding::Failure<'n, 'data, P>>
    {
        crate::cepstral_value_binding::initialize(self, inputs)
    }

    pub fn bind_initialized<P>(self, parts: cepstral::Parts<P>)
        -> Result<Bound<'n, 'data, P>, BindingFailure<'n, 'data, P>>
    {
        if parts.mode80 != self.mode || parts.band_count84 != self.bands.len() || parts.bands.len() != self.bands.len() {
            return Err(BindingFailure::Metadata { parameters: self, parts });
        }
        match cepstral::State::from_explicit_parts(parts) {
            Ok(state) => Ok(Bound { parameters: self, state }),
            Err(refusal) => Err(BindingFailure::State { parameters: self, refusal }),
        }
    }

    pub fn bind_factory_initialized<P>(self, parts: cepstral::Parts<P>,
        factories: [crate::streaming_rows::ValueRows; 2])
        -> Result<Bound<'n, 'data, P>, FactoryBindingFailure<'n, 'data, P>>
    {
        if parts.mode80 != self.mode || parts.band_count84 != self.bands.len() || parts.bands.len() != self.bands.len() {
            return Err(FactoryBindingFailure { owner: FactoryFailureOwner::Metadata { parameters: self, parts, factories } });
        }
        match cepstral::State::from_explicit_factory_parts(parts, factories) {
            Ok(state) => Ok(Bound { parameters: self, state }),
            Err(refusal) => Err(FactoryBindingFailure { owner: FactoryFailureOwner::State { parameters: self, refusal } }),
        }
    }
}
pub struct Bound<'n, 'data, P> { pub parameters: Decoded<'n, 'data>, pub state: cepstral::State<P> }
pub enum BindingFailure<'n, 'data, P> {
    Metadata { parameters: Decoded<'n, 'data>, parts: cepstral::Parts<P> },
    State { parameters: Decoded<'n, 'data>, refusal: cepstral::BindingFailure<P> },
}
enum FactoryFailureOwner<'n, 'data, P> {
    Metadata { parameters: Decoded<'n, 'data>, parts: cepstral::Parts<P>,
        factories: [crate::streaming_rows::ValueRows; 2] },
    State { parameters: Decoded<'n, 'data>, refusal: cepstral::FactoryBindingFailure<P> },
}

pub struct FactoryBindingFailure<'n, 'data, P> { owner: FactoryFailureOwner<'n, 'data, P> }
impl<'n, 'data, P> FactoryBindingFailure<'n, 'data, P> {
    pub fn parameters(&self) -> &Decoded<'n, 'data> {
        match &self.owner { FactoryFailureOwner::Metadata { parameters, .. }
            | FactoryFailureOwner::State { parameters, .. } => parameters }
    }
    pub fn metadata_parts(&self) -> Option<&cepstral::Parts<P>> {
        match &self.owner { FactoryFailureOwner::Metadata { parts, .. } => Some(parts), _ => None }
    }
    pub fn metadata_factories(&self) -> Option<&[crate::streaming_rows::ValueRows; 2]> {
        match &self.owner { FactoryFailureOwner::Metadata { factories, .. } => Some(factories), _ => None }
    }
    pub fn state_refusal(&self) -> Option<&cepstral::FactoryBindingFailure<P>> {
        match &self.owner { FactoryFailureOwner::State { refusal, .. } => Some(refusal), _ => None }
    }
}

pub fn decode_observed_mode2<'n, 'data>(node: &'n ModelNode<'data>) -> Result<Decoded<'n, 'data>, DecodeError> {
    if node.version() != 2 { return Err(DecodeError::Version(node.version())); }
    if node.kind != MODULE || node.tag != OBSERVED_V2 {
        return Err(DecodeError::Operator { kind: node.kind, tag: node.tag });
    }
    if node.data.len() < 8 { return Err(DecodeError::HeaderLength(node.data.len())); }
    let mode = u32::from_le_bytes(node.data[..4].try_into().unwrap());
    let count = u32::from_le_bytes(node.data[4..8].try_into().unwrap());
    if mode != 2 { return Err(DecodeError::Mode(mode)); }
    if count == 0 || count > i32::MAX as u32 { return Err(DecodeError::BandCount(count)); }
    let count = count as usize;
    let expected = count.checked_mul(16).and_then(|bytes| bytes.checked_add(8)).ok_or(DecodeError::Extent)?;
    if node.data.len() != expected {
        return Err(DecodeError::ConfigurationLength { expected, actual: node.data.len() });
    }
    if node.children.len() != 1 { return Err(DecodeError::OuterChildren(node.children.len())); }
    let group = &node.children[0];
    group.require(MODULE, b"cmpg1", decoded_hashes("cmpg1")).map_err(DecodeError::Group)?;
    if !group.data.is_empty() || group.children.len() != count {
        return Err(DecodeError::GroupShape { bytes: group.data.len(), children: group.children.len(), expected: count });
    }
    let mut bands = Vec::new();
    bands.try_reserve_exact(count).map_err(DecodeError::Allocation)?;
    for (index, projection) in group.children.iter().enumerate() {
        let start = 8 + index * 16;
        bands.push(Band { record: BandRecord { bytes: &node.data[start..start + 16] }, projection });
    }
    Ok(Decoded { node, mode, bands })
}
