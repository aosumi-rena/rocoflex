use super::{Operation,Sequence};
use crate::{Result,model::{ModelNode,ModelTree},model_tags::decoded_hashes};

const MODULE:u32=0x7fca41ff;
const COMPONENT:u32=0x7fca40ff;
const ALTERNATE_ROOT:u64=0xc3b2e0d90e2fdb1b;
const WRAPPERS:[u64;2]=[0x9694a11c1f6aa94c,0xbd2a83ac685a6552];
const WRAPPER_CONFIGURATION:[u8;4]=[0,1,0,2];

struct Inner {sequence:Sequence,input_width:usize,hidden_width:usize,output_width:usize}
impl Inner {
    fn from_wrapper(node:&ModelNode<'_>,tag:u64)->Result<Self> {
        if node.kind!=MODULE || node.tag!=tag || node.data!=WRAPPER_CONFIGURATION.as_slice()
            || node.children.len()!=1 {
            return Err("Alternate inner network wrapper topology differs".into());
        }
        let body=&node.children[0];
        body.require(MODULE,b"modm0",decoded_hashes("modm0"))?;
        if !body.data.is_empty() || body.children.len()!=3 {
            return Err("Alternate inner network requires three stored stages".into());
        }
        let activation=&body.children[1];
        activation.require(MODULE,b"moda7",decoded_hashes("moda7"))?;
        if !activation.data.is_empty() || !activation.children.is_empty() {
            return Err("Alternate inner network requires an empty SiLU stage".into());
        }
        for linear in [&body.children[0],&body.children[2]] {
            linear.require(MODULE,b"modl0",decoded_hashes("modl0"))?;
            if !linear.data.is_empty() || linear.children.len()!=2 {
                return Err("Alternate inner network linear topology differs".into());
            }
            linear.children[0].require(COMPONENT,b"prim4",decoded_hashes("prim4"))?;
            linear.children[1].require(COMPONENT,b"prim1",decoded_hashes("prim1"))?;
            if !linear.children[0].children.is_empty() || !linear.children[1].children.is_empty() {
                return Err("Alternate inner network parameters must be leaves".into());
            }
        }

        let sequence=Sequence::load(body)?;
        let (input_width,hidden_width,output_width)=match sequence.operations.as_slice() {
            [Operation::Linear(first,_),Operation::Silu,Operation::Linear(last,_)]
                if first.output_width==last.input_width=>
                    (first.input_width,first.output_width,last.output_width),
            _=>return Err("Alternate inner network matrix dimensions do not connect".into()),
        };
        Ok(Self {sequence,input_width,hidden_width,output_width})
    }
    fn widths(&self)->(usize,usize,usize) {(self.input_width,self.hidden_width,self.output_width)}
    fn forward_row(&mut self,row:&[f32])->Result<Vec<f32>> {
        if row.len()!=self.input_width || row.iter().any(|value|!value.is_finite()) {
            return Err("Alternate inner network input dimensions or values differ".into());
        }
        self.sequence.forward(row)
    }
}

pub(crate) struct AlternateInnerPair {first:Inner,second:Inner}
impl AlternateInnerPair {
    pub(crate) fn from_userdata(bytes:&[u8])->Result<Self> {
        let tree=ModelTree::parse(bytes)?;
        if tree.version!=2 {return Err("Alternate inner networks require DNNIv2".into());}
        tree.root.require(COMPONENT,b"cmpg1",decoded_hashes("cmpg1"))?;
        if !tree.root.data.is_empty() || tree.root.children.len()!=2 {
            return Err("Alternate synthesis envelope topology differs".into());
        }
        let root=&tree.root.children[1];
        if root.kind!=MODULE || root.tag!=ALTERNATE_ROOT || root.data.len()!=16 || root.children.len()!=12 {
            return Err("Alternate synthesis root topology differs".into());
        }
        let first=Inner::from_wrapper(&root.children[5],WRAPPERS[0])?;
        let second=Inner::from_wrapper(&root.children[6],WRAPPERS[1])?;
        Ok(Self {first,second})
    }

    pub(crate) fn is_256_384_512(&self)->bool {
        self.first.widths()==(256,384,512) && self.second.widths()==(256,384,512)
    }

    pub(crate) fn forward_row(&mut self,row:&[f32])->Result<(Vec<f32>,Vec<f32>)> {
        let first=self.first.forward_row(row)?;
        let second=self.second.forward_row(row)?;
        Ok((first,second))
    }
}

