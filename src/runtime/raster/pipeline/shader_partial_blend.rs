use super::{Services,Op,R};
use super::super::{Cause,Failure,Path,Phase,Required,Stop,sample::Site,point};

pub fn pixel<S:Services>(host:&mut S,required:Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let Required {mut state,next_rva}=required;
    let result=(||->Result<u32,Cause<S::Error>> {
        if state.progress.phase!=Phase::Complete {return Err(Cause::Stopped(Stop::WrongEntry));}
        let history=state.progress.sample_history.ok_or(Cause::Stopped(Stop::MissingSampleHistory))?;
        let (path,final_pixel)=match (state.progress.path,history.returned.target.site,next_rva) {
            (Some(Path::A),Site::APartial,0x27ab39)=>(Path::A,false),
            (Some(Path::A),Site::AFinalPartial,0x27a808)=>(Path::A,true),
            (Some(Path::B),Site::BPartial,0x27b08e)=>(Path::B,false),
            (Some(Path::B),Site::BFinalPartial,0x27ad95)=>(Path::B,true),
            _=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        let owners=state.shader.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        state.progress.attempted_rva=next_rva;
        host.validate_sample_history(&mut state.shader.frame,&history).map_err(Cause::Provider)?;
        host.validate_point_entry(&mut state.shader.frame,&owners,path,next_rva).map_err(Cause::Provider)?;
        state.progress.phase=Phase::Blend;
        use Op::*;use R::*;

        let factor=if final_pixel {Esi}else{R13d};
        match path {
            Path::A=>{
                let pcs:[u32;44]=if final_pixel {
                    [0x27a808,0x27a80b,0x27a812,0x27a816,0x27a81a,0x27a821,0x27a824,0x27a82a,
                     0x27a82d,0x27a830,0x27a834,0x27a83b,0x27a83e,0x27a844,0x27a847,0x27a84a,
                     0x27a84d,0x27a854,0x27a858,0x27a85c,0x27a863,0x27a866,0x27a869,0x27a86f,
                     0x27a873,0x27a876,0x27a87c,0x27a87f,0x27a882,0x27a886,0x27a88d,0x27a893,
                     0x27a896,0x27a899,0x27a8a0,0x27a8a3,0x27a8a7,0x27a8ae,0x27a8b4,0x27a8b7,
                     0x27a8ba,0x27a8be,0x27a8c5,0x27a8c8]
                }else{
                    [0x27ab39,0x27ab3c,0x27ab43,0x27ab47,0x27ab4b,0x27ab52,0x27ab55,0x27ab5b,
                     0x27ab5f,0x27ab62,0x27ab66,0x27ab6d,0x27ab70,0x27ab76,0x27ab79,0x27ab7c,
                     0x27ab7f,0x27ab86,0x27ab8a,0x27ab8e,0x27ab95,0x27ab98,0x27ab9b,0x27aba1,
                     0x27aba5,0x27aba8,0x27abae,0x27abb1,0x27abb4,0x27abb8,0x27abbf,0x27abc5,
                     0x27abc8,0x27abcb,0x27abd2,0x27abd5,0x27abd9,0x27abe0,0x27abe6,0x27abe9,
                     0x27abec,0x27abf0,0x27abf7,0x27abfa]
                };
                let ops=super::packed_partial(factor,LoadDwordRaxRcx(Edx),StoreDwordRaxRcx(R10d));
                for (rva,op) in pcs.into_iter().zip(ops) {
                    state.progress.attempted_rva=rva;host.blend_instruction(&mut state.shader.frame,rva,op).map_err(Cause::Provider)?;
                    state.progress.completed_instructions=state.progress.completed_instructions.wrapping_add(1);
                }
            },
            Path::B=>{
                let pcs:[u32;41]=if final_pixel {
                    [0x27ad95,0x27ad98,0x27ad9c,0x27ada3,0x27ada7,0x27adaa,0x27adae,0x27adb5,
                     0x27adbb,0x27adbe,0x27adc1,0x27adc7,0x27adcb,0x27add1,0x27add4,0x27adda,
                     0x27adde,0x27ade2,0x27ade5,0x27ade8,0x27adec,0x27adef,0x27adf2,0x27adf7,
                     0x27adfd,0x27ae01,0x27ae04,0x27ae08,0x27ae0c,0x27ae13,0x27ae16,0x27ae19,
                     0x27ae1c,0x27ae22,0x27ae28,0x27ae2b,0x27ae2e,0x27ae33,0x27ae36,0x27ae39,0x27ae3d]
                }else{
                    [0x27b08e,0x27b091,0x27b095,0x27b09c,0x27b0a0,0x27b0a3,0x27b0a7,0x27b0ae,
                     0x27b0b4,0x27b0b8,0x27b0bb,0x27b0c1,0x27b0c5,0x27b0cb,0x27b0ce,0x27b0d4,
                     0x27b0d8,0x27b0dc,0x27b0df,0x27b0e2,0x27b0e6,0x27b0e9,0x27b0ec,0x27b0f1,
                     0x27b0f7,0x27b0fb,0x27b0fe,0x27b102,0x27b106,0x27b10d,0x27b110,0x27b113,
                     0x27b116,0x27b11c,0x27b122,0x27b125,0x27b128,0x27b12d,0x27b130,0x27b133,0x27b137]
                };
                let ops:[Op;41]=[Mov(R8d,Edx),Shr(R8d,8),And(R8d,0xff00ff),Imul(R8d,factor),Mov(R9d,R8d),Shr(R9d,8),And(R9d,0xff00ff),
                    And(Edx,0xff00ff),Imul(Edx,factor),Shr(Edx,8),And(Edx,0xff00ff),Shr(R8d,24),Imm(R10d,0x100),Sub(R10d,R8d),
                    LoadByteRaxRcxPlus(R8d,1),Imul(R8d,R10d),Shr(R8d,8),Add(R8d,R9d),Mov(R9d,R8d),Shr(R9d,8),Neg(R9d),Or(R9d,R8d),
                    LoadByteRaxRcxPlus(R8d,0),LoadByteRaxRcxPlus(R11d,2),Shl(R11d,16),Or(R11d,R8d),Imul(R11d,R10d),Shr(R11d,8),
                    And(R11d,0xff00ff),Add(R11d,Edx),Mov(Edx,R11d),Shr(Edx,8),And(Edx,0x10001),Imm(R8d,0x100),Sub(R8d,Edx),Or(R8d,R11d),
                    StoreByteRaxRcxPlus(R9d,1),Mov(Edx,R8d),Shr(Edx,16),StoreByteRaxRcxPlus(Edx,2),StoreByteRaxRcxPlus(R8d,0)];
                for (rva,op) in pcs.into_iter().zip(ops) {
                    state.progress.attempted_rva=rva;host.blend_instruction(&mut state.shader.frame,rva,op).map_err(Cause::Provider)?;
                    state.progress.completed_instructions=state.progress.completed_instructions.wrapping_add(1);
                }
            },
        }
        if final_pixel {

            state.progress.phase=Phase::Complete;return Ok(match path {Path::A=>0x27a8cc,Path::B=>0x27ae41});
        }
        let (test,jg,positive,jump,residual)=match path {
            Path::A=>(0x27abfe,0x27ac01,0x27aacc,0x27ac07,0x27aae7),
            Path::B=>(0x27b13b,0x27b13e,0x27b021,0x27b144,0x27b03c),
        };
        state.progress.attempted_rva=test;
        host.point_instruction(&mut state.shader.frame,test,point::Instruction::TestR14d).map_err(Cause::Provider)?;
        state.progress.completed_instructions=state.progress.completed_instructions.wrapping_add(1);
        let control=point::Control {rva:jg,condition:point::Condition::SignedGreater,taken:positive,fallthrough:jump};
        state.progress.phase=Phase::PointBranch;state.progress.attempted_rva=jg;
        state.progress.point_control=Some(control);state.progress.point_control_taken=None;
        let taken=host.point_branch(&mut state.shader.frame,control).map_err(Cause::Provider)?;
        state.progress.point_control_taken=Some(taken);
        let next=if taken {positive}else{
            state.progress.phase=Phase::Jump;state.progress.attempted_rva=jump;
            host.setup_jump(&mut state.shader.frame,jump,residual).map_err(Cause::Provider)?;residual
        };
        state.progress.phase=Phase::Complete;Ok(next)
    })();
    match result {Ok(next_rva)=>Ok(Required {state,next_rva}),Err(cause)=>Err(Failure {state,cause})}
}
