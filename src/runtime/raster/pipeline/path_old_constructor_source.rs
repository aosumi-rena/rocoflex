use super::{comparison,old212200_source::{Instruction,Opcode}};
#[derive(Clone,Copy,Debug)]
pub enum Error {Scalar(comparison::Request),Packed(comparison::PackedRequest)}
pub fn comparison_instruction(request:comparison::Request)->Result<Option<Instruction>,Error> {
    let comparison::Site::AffineIdentity {coefficient}=request.site else {return Ok(None);};
    if request.relation!=comparison::Relation::OrderedEqual {return Err(Error::Scalar(request));}
    let pc=match coefficient {
        2 if request.left==0=>0x211ba6,
        3 if request.left==0=>0x211bb7,
        4 if request.right==0x3f800000=>0x211bcf,
        5 if request.left==0=>0x211bdd,
        _=>return Err(Error::Scalar(request)),
    };
    Ok(Some(Instruction {pc,opcode:Opcode::Ucomiss,reversed:false}))
}
pub fn packed_instruction(request:comparison::PackedRequest)->Result<Instruction,Error> {
    let comparison::Site::AffineIdentity {coefficient}=request.site else {return Err(Error::Packed(request));};
    if request.left[1..]!=[0,0,0] {return Err(Error::Packed(request));}

    let pc=match coefficient {
        1 if request.right==[0;4]=>0x211b93,
        0 if request.right==[0x3f800000,0,0,0]=>0x211b9f,
        _=>return Err(Error::Packed(request)),
    };
    Ok(Instruction {pc,opcode:Opcode::CmpeqpsXmm,reversed:false})
}
