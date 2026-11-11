use super::comparison::{PointRole,Relation,Request,Site};
use super::old212200_source::{Instruction,Opcode};
#[derive(Clone,Copy,Debug)]
pub enum Error {Relation {request:Request}}
pub fn comparison_instruction(request:Request)->Result<Option<Instruction>,Error> {
    use PointRole::*;use Site::*;

    let pc=match request.site {
        LowerX(Move)=>0x2159df,UpperX(Move)=>0x2159e4,
        LowerY(Move)=>0x2159f7,UpperY(Move)=>0x215a17,
        LowerX(FirstControl)=>0x215e25,UpperX(FirstControl)=>0x215e2e,
        LowerY(FirstControl)=>0x215e47,UpperY(FirstControl)=>0x215e4f,
        LowerX(SecondControl)=>0x215e61,UpperX(SecondControl)=>0x215e69,
        LowerY(SecondControl)=>0x215e7b,UpperY(SecondControl)=>0x215e84,
        LowerX(End)=>0x215e98,UpperX(End)=>0x215e9e,
        LowerY(End)=>0x215eb2,UpperY(End)=>0x215eb8,
        _=>return Ok(None),
    };
    if request.relation!=Relation::OrderedGreater {return Err(Error::Relation {request});}
    Ok(Some(Instruction {pc,opcode:Opcode::Ucomiss,reversed:false}))
}
