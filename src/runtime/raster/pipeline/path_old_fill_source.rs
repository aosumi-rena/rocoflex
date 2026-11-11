use super::{comparison,old212200_source::{Instruction,Opcode}};
#[derive(Clone,Copy,Debug)]
pub struct Error {pub request:comparison::Request}
pub fn comparison_instruction(request:comparison::Request)->Result<Option<Instruction>,Error> {
    let comparison::Site::FillToken {marker,..}=request.site else {return Ok(None);};
    if request.relation!=comparison::Relation::OrderedEqual||request.right!=marker {return Err(Error {request});}

    let pc=match marker {
        0x47c35100=>0x21f9a1,
        0x47c35200=>0x21f9a8,
        0x47c35080=>0x21f9af,
        0x47c35180=>0x21f9b6,
        _=>return Err(Error {request}),
    };
    Ok(Some(Instruction {pc,opcode:Opcode::Ucomiss,reversed:false}))
}
