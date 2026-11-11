use crate::{Result,model::ModelNode};

pub(super) struct Codebook { columns:Vec<[f32;64]> }
impl Codebook {
    pub(super) fn from_node(node:&ModelNode<'_>)->Result<Self> {
        if !node.children.is_empty() || !super::tagged(node,"prim0")
            || node.data.len()!=8+64*128*4 {
            return Err("Expected decoder PRIM0 codebook64x128".into());
        }
        let rows=u32::from_le_bytes(node.data[..4].try_into().unwrap());
        let choices=u32::from_le_bytes(node.data[4..8].try_into().unwrap());
        if rows!=64 || choices!=128 {return Err("Decoder codebook dimensions differ".into());}
        let mut columns=vec![[0.;64];128];
        for (row,values) in node.data[8..].chunks_exact(128*4).enumerate() {
            for (column,bytes) in values.chunks_exact(4).enumerate() {
                let value=f32::from_le_bytes(bytes.try_into().unwrap());
                if !value.is_finite() {return Err("Nonfinite decoder codebook coefficient".into());}
                columns[column][row]=value;
            }
        }
        Ok(Self {columns})
    }
    pub(super) fn column(&self,index:usize)->Result<&[f32;64]> {
        self.columns.get(index).ok_or_else(||"Decoder codebook index out of range".into())
    }
}
