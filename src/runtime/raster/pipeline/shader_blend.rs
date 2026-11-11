use super::{Cause,Failure,Path,Phase,Required,Stop,sample::Site};
#[path = "shader_partial_blend.rs"] pub mod partial;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum R {Edx,R8d,R9d,R10d,R11d,R13d,Esi,Ebx}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Mov(R,R),MovzxLowByte(R,R),Imm(R,u32),And(R,u32),Imul(R,R),Shr(R,u8),Shl(R,u8),Sub(R,R),Add(R,R),Or(R,R),Neg(R),
    LoadDwordRcxRax(R),LoadByteRcxRaxPlus(R,u8),StoreDwordRcxRax(R),StoreByteRcxRaxPlus(R,u8),
    LoadDwordRaxRcx(R),LoadByteRaxRcxPlus(R,u8),StoreDwordRaxRcx(R),StoreByteRaxRcxPlus(R,u8),
    LoadDwordRdi(R),LoadByteRdiPlus(R,u8),AddRdiFour,LoadDwordRcx(R),StoreDwordRcx(R),
    LoadByteRcxPlus(R,i8),StoreByteRcxPlus(R,i8),
}

pub(super) fn packed_partial(factor:R,load:Op,store:Op)->[Op;44] {
    use Op::*;use R::*;
    [Mov(R8d,Edx),And(R8d,0xff00ff),Imul(R8d,factor),Shr(R8d,8),And(R8d,0xff00ff),
     Shr(Edx,8),And(Edx,0xff00ff),Imul(Edx,factor),Mov(R9d,Edx),Shr(R9d,8),And(R9d,0xff00ff),
     Shr(Edx,24),Imm(R10d,0x100),Sub(R10d,Edx),load,Mov(R11d,Edx),And(R11d,0xff00ff),
     Imul(R11d,R10d),Shr(R11d,8),And(R11d,0xff00ff),Add(R11d,R8d),Shr(Edx,8),And(Edx,0xff00ff),
     Imul(Edx,R10d),Shr(Edx,8),And(Edx,0xff00ff),Add(Edx,R9d),Mov(R8d,R11d),Shr(R8d,8),And(R8d,0x10001),
     Imm(R9d,0x100),Sub(R9d,R8d),Or(R9d,R11d),And(R9d,0xff00ff),Mov(R8d,Edx),Shr(R8d,8),And(R8d,0x10001),
     Imm(R10d,0x100),Sub(R10d,R8d),Or(R10d,Edx),Shl(R10d,8),And(R10d,0xff00ff00),Or(R10d,R9d),store]
}
pub trait Services:super::sample::Services {

    fn blend_instruction(&mut self,frame:&mut Self::Frame,rva:u32,op:Op)->Result<(),Self::Error>;
}

pub fn full_pixel<S:Services>(host:&mut S,required:Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let Required {mut state,next_rva}=required;
    let result=(||->Result<u32,Cause<S::Error>> {
        if state.progress.phase!=Phase::Complete {return Err(Cause::Stopped(Stop::WrongEntry));}
        let history=state.progress.sample_history.ok_or(Cause::Stopped(Stop::MissingSampleHistory))?;
        let (path,final_pixel)=match (history.returned.target.site,state.progress.path,next_rva) {
            (Site::AFull,Some(Path::A),0x27a9ff)=>(Path::A,false),
            (Site::AFinalFull,Some(Path::A),0x27ac86)=>(Path::A,true),
            (Site::BFull,Some(Path::B),0x27af6f)=>(Path::B,false),
            (Site::BFinalFull,Some(Path::B),0x27b1c6)=>(Path::B,true),
            _=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        let owners=state.shader.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        state.progress.attempted_rva=next_rva;
        host.validate_sample_history(&mut state.shader.frame,&history).map_err(Cause::Provider)?;
        host.validate_point_entry(&mut state.shader.frame,&owners,path,next_rva).map_err(Cause::Provider)?;
        use Op::*;use R::*;
        state.progress.phase=Phase::Blend;
        match path {
            Path::A=>{
                let pcs:[u32;44]=if final_pixel {
                    [0x27ac86,0x27ac89,0x27ac90,0x27ac94,0x27ac98,0x27ac9f,0x27aca3,0x27acaa,
                     0x27acae,0x27acb1,0x27acb5,0x27acbc,0x27acc0,0x27acc6,0x27acc9,0x27accc,
                     0x27accf,0x27acd6,0x27acda,0x27acde,0x27ace5,0x27ace8,0x27aceb,0x27acf1,
                     0x27acf5,0x27acf8,0x27acfe,0x27ad01,0x27ad04,0x27ad08,0x27ad0f,0x27ad15,
                     0x27ad18,0x27ad1b,0x27ad22,0x27ad25,0x27ad29,0x27ad30,0x27ad36,0x27ad39,
                     0x27ad3c,0x27ad40,0x27ad47,0x27ad4a]
                }else{
                    [0x27a9ff,0x27aa02,0x27aa09,0x27aa0d,0x27aa11,0x27aa18,0x27aa1c,0x27aa23,
                     0x27aa27,0x27aa2a,0x27aa2e,0x27aa35,0x27aa39,0x27aa3f,0x27aa42,0x27aa45,
                     0x27aa48,0x27aa4f,0x27aa53,0x27aa57,0x27aa5e,0x27aa61,0x27aa64,0x27aa6a,
                     0x27aa6e,0x27aa71,0x27aa77,0x27aa7a,0x27aa7d,0x27aa81,0x27aa88,0x27aa8e,
                     0x27aa91,0x27aa94,0x27aa9b,0x27aa9e,0x27aaa2,0x27aaa9,0x27aaaf,0x27aab2,
                     0x27aab5,0x27aab9,0x27aac0,0x27aac3]
                };

                let ops:[Op;44]=[Mov(R8d,R9d),And(R8d,0xff00ff),Imul(R8d,Edx),Shr(R8d,8),And(R8d,0xff00ff),
                    Shr(R9d,8),And(R9d,0xff00ff),Imul(R9d,Edx),Mov(R10d,R9d),Shr(R10d,8),And(R10d,0xff00ff),
                    Shr(R9d,24),Imm(R11d,0x100),Sub(R11d,R9d),LoadDwordRcxRax(Edx),Mov(R9d,Edx),
                    And(R9d,0xff00ff),Imul(R9d,R11d),Shr(R9d,8),And(R9d,0xff00ff),Add(R9d,R8d),Shr(Edx,8),
                    And(Edx,0xff00ff),Imul(Edx,R11d),Shr(Edx,8),And(Edx,0xff00ff),Add(Edx,R10d),
                    Mov(R8d,R9d),Shr(R8d,8),And(R8d,0x10001),Imm(R10d,0x100),Sub(R10d,R8d),Or(R10d,R9d),
                    And(R10d,0xff00ff),Mov(R8d,Edx),Shr(R8d,8),And(R8d,0x10001),Imm(R9d,0x100),Sub(R9d,R8d),
                    Or(R9d,Edx),Shl(R9d,8),And(R9d,0xff00ff00),Or(R9d,R10d),StoreDwordRcxRax(R9d)];
                for (rva,op) in pcs.into_iter().zip(ops) {
                    state.progress.attempted_rva=rva;
                    host.blend_instruction(&mut state.shader.frame,rva,op).map_err(Cause::Provider)?;
                    state.progress.completed_instructions+=1;
                }
            },
            Path::B=>{
                let pcs:[u32;41]=if final_pixel {
                    [0x27b1c6,0x27b1c9,0x27b1cd,0x27b1d4,0x27b1d8,0x27b1db,0x27b1df,0x27b1e6,
                     0x27b1ec,0x27b1f0,0x27b1f3,0x27b1f9,0x27b1fd,0x27b203,0x27b206,0x27b20c,
                     0x27b210,0x27b214,0x27b217,0x27b21a,0x27b21e,0x27b221,0x27b224,0x27b229,
                     0x27b22f,0x27b233,0x27b236,0x27b23a,0x27b23e,0x27b245,0x27b248,0x27b24b,
                     0x27b24e,0x27b254,0x27b25a,0x27b25d,0x27b260,0x27b265,0x27b268,0x27b26b,0x27b26f]
                }else{
                    [0x27af6f,0x27af72,0x27af76,0x27af7d,0x27af81,0x27af84,0x27af88,0x27af8f,
                     0x27af95,0x27af99,0x27af9c,0x27afa2,0x27afa6,0x27afac,0x27afaf,0x27afb5,
                     0x27afb9,0x27afbd,0x27afc0,0x27afc3,0x27afc7,0x27afca,0x27afcd,0x27afd2,
                     0x27afd8,0x27afdc,0x27afdf,0x27afe3,0x27afe7,0x27afee,0x27aff1,0x27aff4,
                     0x27aff7,0x27affd,0x27b003,0x27b006,0x27b009,0x27b00e,0x27b011,0x27b014,0x27b018]
                };
                let ops:[Op;41]=[Mov(R9d,Edx),Shr(R9d,8),And(R9d,0xff00ff),Imul(R9d,R8d),Mov(R10d,R9d),
                    Shr(R10d,8),And(R10d,0xff00ff),And(Edx,0xff00ff),Imul(Edx,R8d),Shr(Edx,8),And(Edx,0xff00ff),
                    Shr(R9d,24),Imm(R8d,0x100),Sub(R8d,R9d),LoadByteRcxRaxPlus(R9d,1),Imul(R9d,R8d),
                    Shr(R9d,8),Add(R9d,R10d),Mov(R10d,R9d),Shr(R10d,8),Neg(R10d),Or(R10d,R9d),
                    LoadByteRcxRaxPlus(R9d,0),LoadByteRcxRaxPlus(R11d,2),Shl(R11d,16),Or(R11d,R9d),
                    Imul(R11d,R8d),Shr(R11d,8),And(R11d,0xff00ff),Add(R11d,Edx),Mov(Edx,R11d),Shr(Edx,8),
                    And(Edx,0x10001),Imm(R8d,0x100),Sub(R8d,Edx),Or(R8d,R11d),StoreByteRcxRaxPlus(R10d,1),
                    Mov(Edx,R8d),Shr(Edx,16),StoreByteRcxRaxPlus(Edx,2),StoreByteRcxRaxPlus(R8d,0)];
                for (rva,op) in pcs.into_iter().zip(ops) {
                    state.progress.attempted_rva=rva;
                    host.blend_instruction(&mut state.shader.frame,rva,op).map_err(Cause::Provider)?;
                    state.progress.completed_instructions+=1;
                }
            },
        }
        let next=match (path,final_pixel) {
            (Path::A,false)=>0x27aac7,(Path::B,false)=>0x27b01c,
            (Path::A,true)=>{
                state.progress.phase=Phase::Jump;state.progress.attempted_rva=0x27ad4e;
                host.setup_jump(&mut state.shader.frame,0x27ad4e,0x27a8cc).map_err(Cause::Provider)?;0x27a8cc
            },
            (Path::B,true)=>{
                state.progress.phase=Phase::Jump;state.progress.attempted_rva=0x27b273;
                host.setup_jump(&mut state.shader.frame,0x27b273,0x27ae41).map_err(Cause::Provider)?;0x27ae41
            },
        };
        state.progress.phase=Phase::Complete;Ok(next)
    })();
    match result {Ok(next_rva)=>Ok(Required {state,next_rva}),Err(cause)=>Err(Failure {state,cause})}
}
