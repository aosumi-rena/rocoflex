use super::super::{ByteAddress as A,Cause,Instruction as Op,I32::*,R64,Phase,Progress,Services};

fn at(base:R64,index:Option<R64>,offset:u8)->A {A {base,index,offset}}

pub(super) fn source<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)
    ->Result<(),Cause<S::Error>>
{
    let entry=p.entry;

    host.validate_helper_resume(frame,p,0x280044).map_err(Cause::Provider)?;
    p.phase=Phase::Bilinear;
    use Op::*;
    let plan:&[(u32,Op)]=&[
        (0x280044,MovzxLowByte32 {destination:R10d,source:Esi}),
        (0x280048,StoreRsp32 {offset:0,source:R10d}),(0x28004c,MovzxLowByte32 {destination:Esi,source:R11d}),
        (0x280050,StoreRsp64 {offset:8,source:R64::R14}),
        (0x280055,Signed32To64 {destination:R64::R14,source:Eax}),
        (0x280058,SignedRdi32 {destination:R64::R11,offset:0x14}),(0x28005c,Imul64(R64::R14,R64::R11)),
        (0x280060,AddR14QwordRdi),(0x280063,Signed32To64 {destination:R64::Rbx,source:Ebx}),
        (0x280066,SignedRdi32 {destination:R64::Rdi,offset:0x18}),(0x28006a,Imul64(R64::Rbx,R64::Rdi)),
        (0x28006e,Lea64(R64::Rdx,R64::R14,R64::Rbx)),(0x280072,Imm32(R8d,0x100)),
        (0x280078,Sub32(R8d,Esi)),(0x28007b,Imm32(R15d,0x100)),(0x280081,Sub32(R15d,R10d)),
        (0x280084,Mov32(R10d,R15d)),(0x280087,Imul32(R10d,R8d)),
        (0x28008b,LoadByte32 {destination:R12d,address:at(R64::Rbx,Some(R64::R14),0)}),(0x280090,Imul32(R12d,R10d)),
        (0x280094,Imul32(R15d,Esi)),
        (0x280098,LoadByte32 {destination:Ebp,address:at(R64::Rdi,Some(R64::Rdx),0)}),(0x28009c,Imul32(Ebp,R15d)),
        (0x2800a0,Add32(Ebp,R12d)),
        (0x2800a3,LoadByte32 {destination:R13d,address:at(R64::Rbx,Some(R64::R14),1)}),(0x2800a9,Imul32(R13d,R10d)),
        (0x2800ad,LoadByte32 {destination:R12d,address:at(R64::Rdi,Some(R64::Rdx),1)}),(0x2800b3,Imul32(R12d,R15d)),
        (0x2800b7,Add32(R12d,R13d)),(0x2800ba,StoreRsp32 {offset:4,source:R9d}),
        (0x2800bf,LoadByte32 {destination:R9d,address:at(R64::Rbx,Some(R64::R14),2)}),(0x2800c5,Imul32(R9d,R10d)),
        (0x2800c9,LoadByte32 {destination:R13d,address:at(R64::Rdi,Some(R64::Rdx),2)}),(0x2800cf,Imul32(R13d,R15d)),
        (0x2800d3,Add32(R13d,R9d)),
        (0x2800d6,LoadByte32 {destination:R9d,address:at(R64::Rbx,Some(R64::R14),3)}),(0x2800dc,Imul32(R9d,R10d)),
        (0x2800e0,LoadByte32 {destination:Eax,address:at(R64::Rdi,Some(R64::Rdx),3)}),(0x2800e5,Imul32(Eax,R15d)),
        (0x2800e9,Add32(Eax,R9d)),(0x2800ec,Add64(R64::Rdx,R64::Rdi)),
        (0x2800ef,LoadRsp32 {destination:R9d,offset:0}),(0x2800f3,Imul32(Esi,R9d)),
        (0x2800f7,LoadByte32 {destination:R10d,address:at(R64::R11,Some(R64::Rdx),0)}),(0x2800fc,Imul32(R10d,Esi)),
        (0x280100,Add32(R10d,Ebp)),
        (0x280103,LoadByte32 {destination:R15d,address:at(R64::R11,Some(R64::Rdx),1)}),(0x280109,Imul32(R15d,Esi)),
        (0x28010d,Add32(R15d,R12d)),
        (0x280110,LoadByte32 {destination:R14d,address:at(R64::R11,Some(R64::Rdx),2)}),(0x280116,Imul32(R14d,Esi)),
        (0x28011a,Add32(R14d,R13d)),
        (0x28011d,LoadByte32 {destination:Ebx,address:at(R64::R11,Some(R64::Rdx),3)}),(0x280123,Imul32(Ebx,Esi)),
        (0x280126,Add32(Ebx,Eax)),(0x280128,Add64(R64::Rdx,R64::R11)),(0x28012b,Sub64(R64::Rdx,R64::Rdi)),
        (0x28012e,Imul32(R8d,R9d)),
        (0x280132,LoadByte32 {destination:Eax,address:at(R64::Rdx,None,0)}),(0x280135,Imul32(Eax,R8d)),
        (0x280139,Add32(Eax,R10d)),(0x28013c,AddImm32(Eax,0x8000)),
        (0x280141,LoadByte32 {destination:R9d,address:at(R64::Rdx,None,1)}),(0x280146,Imul32(R9d,R8d)),
        (0x28014a,Add32(R9d,R15d)),(0x28014d,AddImm32(R9d,0x8000)),
        (0x280154,LoadByte32 {destination:R10d,address:at(R64::Rdx,None,2)}),(0x280159,Imul32(R10d,R8d)),
        (0x28015d,Add32(R10d,R14d)),(0x280160,AddImm32(R10d,0x8000)),
        (0x280167,LoadRsp64 {destination:R64::R14,offset:8}),
        (0x28016c,LoadByte32 {destination:Edx,address:at(R64::Rdx,None,3)}),(0x280170,Imul32(Edx,R8d)),
        (0x280174,Add32(Edx,Ebx)),(0x280176,AddImm32(Edx,0x8000)),
        (0x28017c,Shr32(Eax,16)),(0x28017f,Shr32(R9d,16)),(0x280183,Shr32(R10d,16)),(0x280187,Shr32(Edx,16)),
        (0x28018a,StoreR14Byte {offset:0,source:Eax}),(0x28018d,StoreR14Byte {offset:1,source:R9d}),
        (0x280191,LoadRsp32 {destination:R9d,offset:4}),
        (0x280196,StoreR14Byte {offset:2,source:R10d}),(0x28019a,StoreR14Byte {offset:3,source:Edx}),
    ];

    for &(rva,op) in plan {
        p.attempted_rva=rva;host.helper_instruction(frame,&entry,rva,op).map_err(Cause::Provider)?;
        p.completed_instructions=p.completed_instructions.wrapping_add(1);
    }
    p.attempted_rva=0x28019e;
    host.helper_jump(frame,&entry,0x28019e,0x27ff9f).map_err(Cause::Provider)
}
