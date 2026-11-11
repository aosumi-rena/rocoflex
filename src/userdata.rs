use crate::{condition_parameters, dct_parameters, model::{ModelNode, ModelTree},
    model_tags::decoded_hashes, model2_operator_binding::{Ltvfs1Config13, OperatorBindingError},
    root_binding::{BindingError, DescriptorPlan, RootConfig18, VocoderConfigV2}};

const COMPONENT: u32 = 0x7fca40ff;
const MODULE: u32 = 0x7fca41ff;

const PRIMARY_18: u64 = 0x82ae1c455e65f4f5;
const SECONDARY_16: u64 = 0xc3b2e0d90e2fdb1b;
const VOCODER_V2: u64 = 0xeb8a194b072d26c9;
const CEPSTRAL: u64 = 0xc666b3679006cb9f;
const FILTER: u64 = 0x35bd434052b63ee4;
const CONDITION: u64 = 0x6cd86b223291c04b;
const DCT: u64 = 0x2c8d896b0b3ead8f;
const DCT_MATRICES: [u64; 2] = [0xce8a79eed68f751b, 0x618fd1bc101c6bed];

pub struct Loaded { bytes: Vec<u8> }
impl Loaded {
    pub fn read(path: &std::path::Path, maximum_bytes: u64) -> crate::Result<Self> {
        crate::model::read_bytes(path, maximum_bytes).map(|bytes| Self { bytes })
    }
    pub fn byte_count(&self) -> usize { self.bytes.len() }
    pub(crate) fn bytes(&self) -> &[u8] { &self.bytes }
    pub fn decode(&self) -> Result<Decoded<'_>, Error> { decode_observed_v2(&self.bytes) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Vocoder, PreModule, FirstConditional, SecondConditional, Projection112,
    Vector120, Cepstral, Filter136, Filter144, ExtraFilter,
    Upsampler, Downsampler, SubgroupConditional, SubgroupProjection, Conditioning,
}
impl Role {
    fn slot(self) -> usize { match self {
        Self::Vocoder => 0, Self::PreModule => 1, Self::FirstConditional => 2,
        Self::SecondConditional => 3, Self::Projection112 => 4, Self::Vector120 => 5,
        Self::Cepstral => 6, Self::Filter136 => 7, Self::Filter144 => 8,
        Self::ExtraFilter => 9, Self::Upsampler => 10, Self::Downsampler => 11,
        Self::SubgroupConditional => 12, Self::SubgroupProjection => 13, Self::Conditioning => 14,
    } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site { Envelope, Primary, Secondary, Role(Role), Dct, DctMatrix(usize) }
#[derive(Debug)]
pub enum Error {
    Tree(String), Version(u32),
    Operator { site: Site, cause: String },
    Signature { site: Site, kind: u32, tag: u64 },
    Shape { site: Site, bytes: usize, children: usize },
    Missing(Role), Root(BindingError), Vocoder(BindingError),
    Filter { role: Role, cause: OperatorBindingError },
    Condition(String), Dct(dct_parameters::DecodeError),
    ConditionalRole(Role), ConditionalNetwork { role: Role, cause: crate::ncwn_parameters::Error },
    Cepstral(crate::cepstral_parameters::DecodeError),
    SequenceRole(Role), Sequence { role: Role, cause: crate::sequence_parameters::DecodeError },
}

pub struct Primary {
    configuration: RootConfig18,
    vocoder: VocoderConfigV2,
    descriptor: DescriptorPlan,
    filters: [Option<Ltvfs1Config13>; 3],
}
impl Primary {
    pub fn configuration(&self) -> RootConfig18 { self.configuration }
    pub fn vocoder(&self) -> VocoderConfigV2 { self.vocoder }
    pub fn descriptor(&self) -> &DescriptorPlan { &self.descriptor }
    pub fn filter(&self, role: Role) -> Option<Ltvfs1Config13> {
        match role { Role::Filter136 => self.filters[0], Role::Filter144 => self.filters[1],
            Role::ExtraFilter => self.filters[2], _ => None }
    }
}
pub struct Decoded<'a> { tree: ModelTree<'a>, primary: Primary }
impl<'a> Decoded<'a> {
    pub fn tree(&self) -> &ModelTree<'a> { &self.tree }
    pub fn primary(&self) -> &Primary { &self.primary }

    pub fn secondary_module(&self) -> &ModelNode<'a> { &self.tree.root.children[1] }
    pub fn role(&self, role: Role) -> Option<&ModelNode<'a>> {
        self.primary.descriptor.children[role.slot()]
            .and_then(|index| self.tree.root.children[0].children.get(index))
    }
    pub fn decode_conditional_network(&self, role: Role)
        -> Result<crate::ncwn_parameters::Decoded<'_, 'a>, Error>
    {
        if !matches!(role, Role::FirstConditional | Role::SecondConditional | Role::SubgroupConditional) {
            return Err(Error::ConditionalRole(role));
        }
        let node = self.role(role).ok_or(Error::Missing(role))?;
        crate::ncwn_parameters::decode_node(node).map_err(|cause| Error::ConditionalNetwork { role, cause })
    }
    pub fn decode_cepstral_parameters(&self)
        -> Result<crate::cepstral_parameters::Decoded<'_, 'a>, Error>
    {
        let node = self.role(Role::Cepstral).ok_or(Error::Missing(Role::Cepstral))?;
        crate::cepstral_parameters::decode_observed_mode2(node).map_err(Error::Cepstral)
    }
    pub fn decode_sequence(&self, role: Role) -> Result<crate::sequence_parameters::Decoded<'_, 'a>, Error> {
        if !matches!(role, Role::Projection112 | Role::SubgroupProjection) { return Err(Error::SequenceRole(role)); }
        let node = self.role(role).ok_or(Error::Missing(role))?;
        crate::sequence_parameters::decode_node(node).map_err(|cause| Error::Sequence { role, cause })
    }
    fn conditioning(&self) -> Result<&ModelNode<'a>, Error> {
        let node = self.role(Role::Conditioning).ok_or(Error::Missing(Role::Conditioning))?;
        signature(node, MODULE, CONDITION, Site::Role(Role::Conditioning))?;
        if ![4, 12].contains(&node.data.len()) || node.children.len() != 3 {
            return Err(shape(node, Site::Role(Role::Conditioning)));
        }
        Ok(node)
    }

    pub fn decode_condition_parameters(&self, shifts: [u32; 6])
        -> Result<condition_parameters::Decoded, Error>
    {
        let node = self.conditioning()?;
        condition_parameters::decode_selected(node.data, &node.children[0], &node.children[2], shifts)
            .map_err(Error::Condition)
    }

    pub fn decode_dct_parameters(&self) -> Result<dct_parameters::Decoded<'a>, Error> {
        let condition = self.conditioning()?;
        let node = &condition.children[1];
        signature(node, COMPONENT, DCT, Site::Dct)?;
        if !node.data.is_empty() || node.children.len() != 2 { return Err(shape(node, Site::Dct)); }
        for (index, matrix) in node.children.iter().enumerate() {
            if matrix.kind != COMPONENT || !DCT_MATRICES.contains(&matrix.tag) {
                return Err(Error::Signature { site: Site::DctMatrix(index), kind: matrix.kind, tag: matrix.tag });
            }
            if !matrix.children.is_empty() { return Err(shape(matrix, Site::DctMatrix(index))); }
        }
        dct_parameters::decode_selected(node.children[0].data, node.children[1].data).map_err(Error::Dct)
    }
}

fn signature(node: &ModelNode<'_>, kind: u32, tag: u64, site: Site) -> Result<(), Error> {
    if node.kind != kind || node.tag != tag { return Err(Error::Signature { site, kind: node.kind, tag: node.tag }); }
    Ok(())
}
fn shape(node: &ModelNode<'_>, site: Site) -> Error {
    Error::Shape { site, bytes: node.data.len(), children: node.children.len() }
}

pub fn decode_observed_v2(bytes: &[u8]) -> Result<Decoded<'_>, Error> {
    let tree = ModelTree::parse(bytes).map_err(Error::Tree)?;
    if tree.version != 2 { return Err(Error::Version(tree.version)); }
    tree.root.require(COMPONENT, b"cmpg1", decoded_hashes("cmpg1"))
        .map_err(|cause| Error::Operator { site: Site::Envelope, cause })?;
    if !tree.root.data.is_empty() || tree.root.children.len() != 2 { return Err(shape(&tree.root, Site::Envelope)); }
    let root = &tree.root.children[0];
    let secondary = &tree.root.children[1];
    signature(root, MODULE, PRIMARY_18, Site::Primary)?;
    signature(secondary, MODULE, SECONDARY_16, Site::Secondary)?;
    if secondary.data.len() != 16 || secondary.children.len() != 12 { return Err(shape(secondary, Site::Secondary)); }
    let configuration = RootConfig18::parse(root.data).map_err(Error::Root)?;
    let cepstral = root.children.get(configuration.cepstral_child_index()).ok_or(Error::Missing(Role::Cepstral))?;
    signature(cepstral, MODULE, CEPSTRAL, Site::Role(Role::Cepstral))?;
    if cepstral.data.len() != 56 { return Err(shape(cepstral, Site::Role(Role::Cepstral))); }
    let mode = crate::timbre::u32_at(cepstral.data, 0)
        .map_err(|cause| Error::Operator { site: Site::Role(Role::Cepstral), cause })?;
    let descriptor = configuration.bind_children(root.children.len(), mode).map_err(Error::Root)?;
    let vocoder_index = descriptor.children[Role::Vocoder.slot()].ok_or(Error::Missing(Role::Vocoder))?;
    let vocoder_node = root.children.get(vocoder_index).ok_or(Error::Missing(Role::Vocoder))?;
    signature(vocoder_node, COMPONENT, VOCODER_V2, Site::Role(Role::Vocoder))?;
    let vocoder = VocoderConfigV2::parse(vocoder_node.data).map_err(Error::Vocoder)?;
    let mut filters = [None; 3];
    for (index, role) in [Role::Filter136, Role::Filter144, Role::ExtraFilter].into_iter().enumerate() {
        if let Some(child) = descriptor.children[role.slot()] {
            let node = root.children.get(child).ok_or(Error::Missing(role))?;
            signature(node, MODULE, FILTER, Site::Role(role))?;
            if !node.children.is_empty() { return Err(shape(node, Site::Role(role))); }
            filters[index] = Some(Ltvfs1Config13::parse(node.data)
                .map_err(|cause| Error::Filter { role, cause })?.ok_or_else(|| shape(node, Site::Role(role)))?);
        }
    }
    let decoded = Decoded { tree, primary: Primary { configuration, vocoder, descriptor, filters } };
    decoded.conditioning()?;
    Ok(decoded)
}
