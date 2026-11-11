use super::Target;
#[path = "shader_sample_remainder.rs"] pub mod remainder;
#[path = "shader_sample_invoke.rs"] mod invocation;
pub use invocation::{source as invoke,Completed as InvocationCompleted,Failure as InvocationFailure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Saved {R15,R14,R13,R12,Rsi,Rdi,Rbp,Rbx}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Xmm {X0,X1,X2,X3,X4,X5,X6,X7,X8}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I32 {Eax,Edx,R8d,R9d,R10d,R11d,R12d,R13d,R14d,R15d,Esi,Ebx,Ebp}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum R64 {Rax,Rdx,R8,Rbx,Rdi,R11,R14}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteAddress {pub base:R64,pub index:Option<R64>,pub offset:u8}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Push(Saved),SubRsp48,SaveAlignedXmm {source:Xmm,rsp_offset:u8},R14FromRdx,
    Cvtsi2ssRcx {destination:Xmm,offset:u8},Cvtsi2ss {destination:Xmm,source:I32},
    MovssRcx {destination:Xmm,offset:u8},Addss(Xmm,Xmm),Xorps(Xmm,Xmm),
    Mulss(Xmm,Xmm),Movaps(Xmm,Xmm),MulssRcx {destination:Xmm,offset:u8},
    R8dFromDwordRcx44,

    Xmm2FromRipNext27fef4Plus3a20cc,
    DwordRcx1cFromR9d,SubEaxR10d,Cdq,
    StoreRcx32 {offset:u8,source:I32},LoadRcx32 {destination:I32,offset:u8},
    Mov32(I32,I32),Add32(I32,I32),Sub32(I32,I32),AddRcx32 {destination:I32,offset:u8},
    SubRcx32 {destination:I32,offset:u8},Dec32(I32),Inc32(I32),Test32(I32),
    Sar32(I32,u8),And32(I32,I32),CmpRcx32 {left:I32,offset:u8},CmpRcx5cZero,CmpR9d1,
    RdiFromQwordRcx50,LoadRdi32 {destination:I32,offset:u8},Cdqe,
    SignedRdi32 {destination:R64,offset:u8},SignedEbxToRax,Imul64(R64,R64),
    AddRdxQwordRdi,AddR14QwordRdi,EaxFromDwordR8Rdx,DwordR14FromEax,AddR14Four,
    RestoreAlignedXmm {destination:Xmm,rsp_offset:u8},AddRsp48,Pop(Saved),
    MovzxLowByte32 {destination:I32,source:I32},StoreRsp32 {offset:u8,source:I32},
    LoadRsp32 {destination:I32,offset:u8},StoreRsp64 {offset:u8,source:R64},LoadRsp64 {destination:R64,offset:u8},
    Signed32To64 {destination:R64,source:I32},Add64(R64,R64),Sub64(R64,R64),Lea64(R64,R64,R64),
    Imm32(I32,u32),Imul32(I32,I32),Shr32(I32,u8),AddImm32(I32,u32),
    LoadByte32 {destination:I32,address:ByteAddress},StoreR14Byte {offset:u8,source:I32},
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sensitive {CvttEaxXmm4,CvttR10dXmm3,IdivR9d,
    CvttEaxXmm1At27ff3b,CvttR10dXmm0At27ff43,IdivR9dAt27ff50,IdivR10dAt280006,IdivEbpAt28001a}
impl Sensitive {
    pub fn rva(self)->u32 {match self {Self::CvttEaxXmm4=>0x27fef8,Self::CvttR10dXmm3=>0x27ff00,Self::IdivR9d=>0x27ff0d,
        Self::CvttEaxXmm1At27ff3b=>0x27ff3b,Self::CvttR10dXmm0At27ff43=>0x27ff43,Self::IdivR9dAt27ff50=>0x27ff50,
        Self::IdivR10dAt280006=>0x280006,Self::IdivEbpAt28001a=>0x28001a}}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry<I> {
    pub target:Target<I>,pub callee_frame:I,pub callee_context:I,
    pub callee_stack:I,pub actual_call_receipt:I,pub receipt_owner:I,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {pub raw_eax:u32,pub raw_edx:u32,pub raw_r10d:u32,pub rflags:u64,pub mxcsr:u32}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt<I> {
    pub entry:Entry<I>,pub instruction:Sensitive,pub identity:I,pub previous:Option<I>,pub observed:Observation,
}
pub struct NormalInstruction<I> {pub observed:Observation,pub retained:Option<Receipt<I>>}
pub struct InstructionFailure<I,E> {
    pub error:E,pub observed_normal_completion:Option<Observation>,pub retained:Option<Receipt<I>>,
}
pub trait Services:super::Services {

    fn enter_sample_27fe50(&mut self,frame:&mut Self::Frame,target:&Target<Self::Identity>)
        ->Result<Entry<Self::Identity>,Self::Error>;

    fn validate_helper_entry(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>)->Result<(),Self::Error>;

    fn helper_instruction(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,rva:u32,op:Instruction)
        ->Result<(),Self::Error>;

    fn actual_helper_sensitive(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,instruction:Sensitive)
        ->Result<NormalInstruction<Self::Identity>,InstructionFailure<Self::Identity,Self::Error>>;
    fn validate_helper_sensitive(&mut self,frame:&mut Self::Frame,receipt:&Receipt<Self::Identity>)
        ->Result<(),Self::Error>;

    fn validate_helper_resume(&mut self,frame:&mut Self::Frame,progress:&Progress<Self::Identity>,at:u32)
        ->Result<(),Self::Error>;

    fn helper_branch(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,control:Control)
        ->Result<bool,Self::Error>;
    fn helper_jump(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,rva:u32,target:u32)
        ->Result<(),Self::Error>;

    fn actual_helper_ret_2801c3(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>)
        ->Result<super::NormalReturn<Self::Identity,Self::SampleReturnPacket,Self::SampleOutputPacket>,
            super::CallFailure<Self::Identity,Self::SampleReturnPacket,Self::SampleOutputPacket,Self::Error>>;

    fn validate_actual_helper_ret(&mut self,frame:&mut Self::Frame,entry:&Entry<Self::Identity>,
        returned:&super::Returned<Self::Identity,Self::SampleReturnPacket>)->Result<(),Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {SignedGreater,SignedLessEqual,Equal,UnsignedAboveEqual}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control {pub rva:u32,pub condition:Condition,pub taken:u32,pub fallthrough:u32}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {Admission,Prefix,Sensitive,SensitiveStopped,ValidateSensitive,AfterSensitive,RequiredRemainder,
    Remainder,PointLoop,Branch,Nearest,Bilinear,Epilogue,Ret,RetStopped,ValidateRet,Returned}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {EntryMismatch,MissingReceipt,ReceiptMismatch,ObservationMismatch,WrongContinuation,
    MissingReturn,MissingOutput,ReturnMismatch,OutputMismatch,CalleeMismatch}
pub enum Cause<E> {Provider(E),Stopped(Stop)}
#[derive(Clone, Copy, Debug)]
pub struct Progress<I> {
    pub entry:Entry<I>,pub phase:Phase,pub attempted_rva:u32,pub completed_instructions:usize,

    pub sensitive:[Option<Receipt<I>>;8],pub observed:Option<Observation>,pub last_sensitive:Option<Receipt<I>>,
    pub last_control:Option<Control>,pub last_control_taken:Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {

    AfterAddss27ff10,
}
#[must_use = "remainder/whole normal return/native output STILL REQUIRED on SAME Frame"]
pub struct Required<I> {pub progress:Progress<I>,pub next:Next}
#[must_use = "original borrowed Frame MUST remain owned by actual call provider on failure"]
pub struct Failure<I,E> {pub progress:Progress<I>,pub cause:Cause<E>}

pub fn prefix<S:Services>(host:&mut S,frame:&mut S::Frame,entry:Entry<S::Identity>)
    ->Result<Required<S::Identity>,Failure<S::Identity,S::Error>>
{
    let mut p=Progress {entry,phase:Phase::Admission,attempted_rva:0x27fe50,
        completed_instructions:0,sensitive:[None;8],observed:None,last_sensitive:None,last_control:None,last_control_taken:None};
    let result=(||->Result<(),Cause<S::Error>> {
        if entry.callee_context!=entry.target.callee_context||entry.target.callee_module!=entry.target.owners.source_module {
            return Err(Cause::Stopped(Stop::EntryMismatch));
        }
        host.validate_helper_entry(frame,&entry).map_err(Cause::Provider)?;
        macro_rules! ins {($rva:expr,$op:expr)=>{{
            p.attempted_rva=$rva;
            host.helper_instruction(frame,&entry,$rva,$op).map_err(Cause::Provider)?;
            p.completed_instructions+=1;
        }};}
        macro_rules! sensitive {($slot:expr,$instruction:expr)=>{{
            sensitive(host,frame,&mut p,$slot,$instruction)?;
        }};}
        use Instruction::*;use Xmm::*;
        p.phase=Phase::Prefix;
        for (rva,source) in [(0x27fe50,Saved::R15),(0x27fe52,Saved::R14),(0x27fe54,Saved::R13),(0x27fe56,Saved::R12),
            (0x27fe58,Saved::Rsi),(0x27fe59,Saved::Rdi),(0x27fe5a,Saved::Rbp),(0x27fe5b,Saved::Rbx)] {ins!(rva,Push(source));}
        let plan:&[(u32,Instruction)]=&[
            (0x27fe5c,SubRsp48),(0x27fe60,SaveAlignedXmm {source:X8,rsp_offset:0x30}),
            (0x27fe66,SaveAlignedXmm {source:X7,rsp_offset:0x20}),(0x27fe6b,SaveAlignedXmm {source:X6,rsp_offset:0x10}),
            (0x27fe70,R14FromRdx),(0x27fe73,Cvtsi2ssRcx {destination:X2,offset:0x68}),
            (0x27fe78,Cvtsi2ss {destination:X0,source:I32::R8d}),(0x27fe7d,MovssRcx {destination:X1,offset:0x40}),
            (0x27fe82,Addss(X0,X1)),(0x27fe86,Addss(X2,X1)),(0x27fe8a,Xorps(X1,X1)),
            (0x27fe8d,Cvtsi2ss {destination:X1,source:I32::R9d}),(0x27fe92,Addss(X1,X0)),
            (0x27fe96,MovssRcx {destination:X5,offset:4}),(0x27fe9b,Mulss(X5,X2)),
            (0x27fe9f,MovssRcx {destination:X4,offset:0}),(0x27fea3,MovssRcx {destination:X6,offset:8}),
            (0x27fea8,Movaps(X3,X4)),(0x27feab,Mulss(X3,X0)),(0x27feaf,Addss(X3,X5)),(0x27feb3,Addss(X3,X6)),
            (0x27feb7,MovssRcx {destination:X7,offset:0xc}),(0x27febc,MulssRcx {destination:X2,offset:0x10}),
            (0x27fec1,Mulss(X0,X7)),(0x27fec5,Addss(X0,X2)),(0x27fec9,MovssRcx {destination:X8,offset:0x14}),
            (0x27fecf,Addss(X0,X8)),(0x27fed4,Mulss(X4,X1)),(0x27fed8,Addss(X4,X5)),(0x27fedc,Addss(X4,X6)),
            (0x27fee0,Mulss(X1,X7)),(0x27fee4,Addss(X1,X2)),(0x27fee8,R8dFromDwordRcx44),
            (0x27feec,Xmm2FromRipNext27fef4Plus3a20cc),(0x27fef4,Mulss(X4,X2)),
        ];
        for &(rva,op) in plan {ins!(rva,op);}
        sensitive!(0,Sensitive::CvttEaxXmm4);
        ins!(0x27fefc,Mulss(X3,X2));
        sensitive!(1,Sensitive::CvttR10dXmm3);
        ins!(0x27ff05,DwordRcx1cFromR9d);ins!(0x27ff09,SubEaxR10d);ins!(0x27ff0c,Cdq);
        sensitive!(2,Sensitive::IdivR9d);
        ins!(0x27ff10,Addss(X1,X8));
        p.phase=Phase::RequiredRemainder;Ok(())
    })();
    match result {Ok(())=>Ok(Required {progress:p,next:Next::AfterAddss27ff10}),Err(cause)=>Err(Failure {progress:p,cause})}
}

pub(super) fn sensitive<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>,slot:usize,instruction:Sensitive)
    ->Result<(),Cause<S::Error>>
{
    let entry=p.entry;p.phase=Phase::Sensitive;p.attempted_rva=instruction.rva();p.observed=None;
    let normal=match host.actual_helper_sensitive(frame,&entry,instruction) {
        Ok(normal)=>normal,
        Err(InstructionFailure {error,observed_normal_completion,retained})=>{
            p.phase=Phase::SensitiveStopped;p.observed=observed_normal_completion;p.sensitive[slot]=retained;
            return Err(Cause::Provider(error));
        }
    };
    p.observed=Some(normal.observed);p.sensitive[slot]=normal.retained;
    let receipt=normal.retained.ok_or(Cause::Stopped(Stop::MissingReceipt))?;
    if receipt.entry!=entry||receipt.instruction!=instruction||receipt.previous!=p.last_sensitive.map(|r|r.identity) {
        return Err(Cause::Stopped(Stop::ReceiptMismatch));
    }
    if receipt.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
    p.phase=Phase::ValidateSensitive;host.validate_helper_sensitive(frame,&receipt).map_err(Cause::Provider)?;
    p.last_sensitive=Some(receipt);p.completed_instructions=p.completed_instructions.wrapping_add(1);p.phase=Phase::AfterSensitive;Ok(())
}
