use super::{word,vector,ModelNode,ModelTree,NcwnNetwork,AnalysisBank8,SynthesisBank8,
    SynthesisPost,HarmonicProjection,ConditioningNetwork,DenseMatrix,FloatGru};
use crate::{Result,model_tags::decoded_hashes};
const COMPONENT:u32=0x7fca40ff;
const MODULE:u32=0x7fca41ff;
const ALTERNATE_ROOT:u64=0xc3b2e0d90e2fdb1b;

fn alternate_root<'a,'data>(tree:&'a ModelTree<'data>)->Result<&'a ModelNode<'data>> {
    if tree.version!=2 {return Err("Alternate synthesis requires DNNIv2".into());}
    tree.root.require(COMPONENT,b"cmpg1",decoded_hashes("cmpg1"))?;
    if !tree.root.data.is_empty() || tree.root.children.len()!=2 {
        return Err("Alternate synthesis envelope topology differs".into());
    }
    let root=&tree.root.children[1];
    if root.kind!=MODULE || root.tag!=ALTERNATE_ROOT || root.data.len()!=16 || root.children.len()!=12 {
        return Err("Alternate synthesis root topology differs".into());
    }
    Ok(root)
}

pub(crate) struct AlternateCommon {
    pub(crate) first:NcwnNetwork,pub(crate) second:NcwnNetwork,pub(crate) subgroup:NcwnNetwork,
    pub(crate) down8:AnalysisBank8,pub(crate) up8:SynthesisBank8,pub(crate) post:SynthesisPost,
    pub(crate) conditioning:ConditioningNetwork,pub(crate) projection:HarmonicProjection,
    pub(crate) frequencies:[f32;5],
}
impl AlternateCommon {
    pub(crate) fn from_userdata(bytes:&[u8])->Result<Self> {
        let tree=ModelTree::parse(bytes)?;
        let root=alternate_root(&tree)?;

        let condition=&root.children[11];
        if condition.data.len()!=12 || condition.children.len()!=3 {
            return Err("Alternate conditioning topology differs".into());
        }
        let prefix=u16::from_le_bytes(condition.data[0..2].try_into().unwrap()) as usize;
        let period=u16::from_le_bytes(condition.data[2..4].try_into().unwrap());
        let mode=word(condition.data,4)?;
        let feature_width=word(condition.data,8)? as usize;
        if prefix==0 || period==0 || mode>1 {return Err("Invalid alternate conditioning configuration".into());}
        let values=vector(&condition.children[0],word(condition.children[0].data,0)? as usize)?;
        let dct_node=condition.children[1].children.first().ok_or("Missing alternate conditioning DCT")?;
        let dct=DenseMatrix::from_node(dct_node)?;
        let gru=FloatGru::from_node(&condition.children[2])?;
        let conditioning=ConditioningNetwork::from_parts(gru,dct,values,prefix,period,mode,feature_width)?;
        let frequency_node=&root.children[3];
        frequency_node.require(COMPONENT,b"prim1",decoded_hashes("prim1"))?;
        if !frequency_node.children.is_empty() {return Err("Alternate frequency vector must be a leaf".into());}
        let frequencies=vector(frequency_node,5)?.try_into().map_err(|_|"Alternate frequency width differs")?;
        Ok(Self {
            first:NcwnNetwork::from_node(&root.children[1])?,
            second:NcwnNetwork::from_node(&root.children[2])?,
            subgroup:NcwnNetwork::from_node(&root.children[9])?,
            up8:SynthesisBank8::from_node(&root.children[7])?,
            down8:AnalysisBank8::from_node(&root.children[8])?,
            post:SynthesisPost::from_node(&root.children[10])?,
            projection:HarmonicProjection::from_node(&root.children[4])?,
            frequencies,conditioning,
        })
    }
}

