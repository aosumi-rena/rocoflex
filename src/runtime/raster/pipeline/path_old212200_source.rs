use super::{comparison::{self,CurveKind as K,NearPhase as N,Relation as R},curve_source::MaximumRequest};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Opcode {Ucomiss,Cmpeqss,CmpeqpsXmm,Maxss}
#[derive(Clone,Copy,Debug)]
pub struct Instruction {pub pc:u32,pub opcode:Opcode,pub reversed:bool}
#[derive(Clone,Copy,Debug)]
pub enum Error {Request(comparison::Request),Maximum(MaximumRequest)}

fn near_index(kind:K,pair:u8)->Option<usize> {
    let count=match kind {K::Line|K::Quadratic=>4,K::Cubic=>8};
    if pair<count {Some(pair as usize)}else {None}
}
pub fn comparison_instruction(request:comparison::Request)->Result<Option<Instruction>,Error> {
    let instruction=match request.site {
        comparison::Site::IteratorToken {phase,stacked,..}=>{
            use comparison::TokenPhase as T;
            let (marker,pc)=match phase {
                T::InitialClose=>(0x47c35280,if stacked {0x2122cd}else {0x212368}),
                T::QuadraticAfterRead=>(0x47c35180,if stacked {0x2122ff}else {0x21245f}),
                T::CubicAfterRead=>(0x47c35200,if stacked {0x212522}else {0x2125ba}),
                T::LineAfterTransform=>(0x47c35080,0x212719),
                T::QuadraticAfterTransform=>(0x47c35180,0x212728),
                T::CubicAfterTransform=>(0x47c35200,0x2128ae),
                T::CloseBeforeMove=>(0x47c35280,0x212da5),
            };
            if request.relation!=R::OrderedEqual||request.right!=marker {
                return Err(Error::Request(request));
            }
            Instruction {pc,opcode:Opcode::Ucomiss,reversed:false}
        },
        comparison::Site::IteratorPeekClose {..}=>{
            if request.relation!=R::OrderedEqual||request.right!=0x47c35280 {return Err(Error::Request(request));}
            Instruction {pc:0x2133ae,opcode:Opcode::Ucomiss,reversed:false}
        },
        comparison::Site::CurveDistance {kind,half}=>{
            if request.relation!=R::OrderedGreater {return Err(Error::Request(request));}
            let pc=match (kind,half) {(K::Quadratic,0)=>0x212c82,(K::Cubic,0)=>0x2129da,(K::Cubic,1)=>0x212a0e,
                _=>return Err(Error::Request(request))};
            Instruction {pc,opcode:Opcode::Ucomiss,reversed:false}
        },
        comparison::Site::CurveNear {kind,pair,phase}=>{
            let index=near_index(kind,pair).ok_or(Error::Request(request))?;
            let (pc,opcode,reversed)=match phase {
                N::ExceptionalEqual=>{
                    if request.relation!=R::OrderedEqual {return Err(Error::Request(request));}
                    let pc=match kind {
                        K::Line=>[0x2123ac,0x2122a0,0x2133e8,0x213423][index],
                        K::Quadratic=>[0x212815,0x21284d,0x212b4c,0x212c58][index],
                        K::Cubic=>[0x212a3b,0x212a73,0x212e41,0x212e75,0x212f39,0x212f6c,0x21303f,0x213077][index],
                    };
                    let mask=matches!((kind,pair),(K::Line,3)|(K::Quadratic,3)|(K::Cubic,7));
                    (pc,if mask {Opcode::Cmpeqss}else {Opcode::Ucomiss},kind==K::Cubic&&pair==7)
                },
                N::AbsoluteThreshold=>{
                    if request.relation!=R::OrderedLessEqual||request.right!=0x00800000 {return Err(Error::Request(request));}
                    let pc=match kind {
                        K::Line=>[0x21258a,0x212409,0x21344d,0x213491][index],
                        K::Quadratic=>[0x212877,0x212aee,0x212bf5,0x212d70][index],
                        K::Cubic=>[0x212a9f,0x212dea,0x212ea2,0x212ee7,0x212f9e,0x212fe5,0x2131c5,0x21320e][index],
                    };(pc,Opcode::Ucomiss,true)
                },
                N::RelativeThreshold=>{
                    if request.relation!=R::OrderedGreaterEqual {return Err(Error::Request(request));}
                    let pc=match kind {
                        K::Line=>[0x2125ac,0x21242b,0x21346b,0x2134af][index],
                        K::Quadratic=>[0x2128a0,0x212b14,0x212c1f,0x212d9a][index],
                        K::Cubic=>[0x212ac8,0x212e10,0x212ec8,0x212f0d,0x212fc3,0x213009,0x2131eb,0x213233][index],
                    };(pc,Opcode::Ucomiss,false)
                },
            };Instruction {pc,opcode,reversed}
        },
        _=>return Ok(None),
    };Ok(Some(instruction))
}
pub fn maximum_instruction(request:MaximumRequest)->Result<Instruction,Error> {
    let index=near_index(request.kind,request.pair).ok_or(Error::Maximum(request))?;
    let pc=match request.kind {
        K::Line=>[0x2125a0,0x21241f,0x21345f,0x2134a3][index],
        K::Quadratic=>[0x212894,0x212b08,0x212c13,0x212d8e][index],
        K::Cubic=>[0x212abc,0x212e04,0x212ebc,0x212f01,0x212fb7,0x212ffb,0x2131df,0x213225][index],
    };

    Ok(Instruction {pc,opcode:Opcode::Maxss,reversed:true})
}
