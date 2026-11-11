use super::{Cause,Condition,Control,Instruction as Op,I32,R64,
    Next,Phase,Progress,Required,Saved,Sensitive,Services,Stop,Xmm};
use super::super::{NormalReturn,Returned,Output};
#[path = "shader_sample_bilinear.rs"] mod bilinear;

#[must_use]
pub enum Outcome<I,R,O> {
    Returned {progress:Progress<I>,normal:NormalReturn<I,R,O>},
}
#[must_use = "SAME borrowed Frame and ALL partial RET/output packets must remain in original provider"]
pub struct Failure<I,R,O,E> {
    pub progress:Progress<I>,pub cause:Cause<E>,
    pub normal:Option<NormalReturn<I,R,O>>,
    pub observed_return:Option<super::super::inverse::Observation>,
    pub partial_return:Option<Returned<I,R>>,pub partial_output:Option<Output<I,O>>,
}
pub fn continue_after_prefix<S:Services>(host:&mut S,frame:&mut S::Frame,required:Required<S::Identity>)
    ->Result<Outcome<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket>,
        Failure<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    let Required {progress:mut p,next}=required;
    let mut normal_return=None;let mut observed_return=None;let mut partial_return=None;let mut partial_output=None;
    let result=(||->Result<(),Cause<S::Error>> {
        if next!=Next::AfterAddss27ff10||p.phase!=Phase::RequiredRemainder||p.attempted_rva!=0x27ff10 {
            return Err(Cause::Stopped(Stop::WrongContinuation));
        }
        let entry=p.entry;
        host.validate_helper_resume(frame,&p,0x27ff15).map_err(Cause::Provider)?;
        macro_rules! ins {($rva:expr,$op:expr)=>{{
            p.attempted_rva=$rva;
            host.helper_instruction(frame,&entry,$rva,$op).map_err(Cause::Provider)?;
            p.completed_instructions=p.completed_instructions.wrapping_add(1);
        }};}
        macro_rules! branch {($rva:expr,$condition:ident,$taken:expr,$fallthrough:expr)=>{{
            p.phase=Phase::Branch;p.attempted_rva=$rva;
            let control=Control {rva:$rva,condition:Condition::$condition,taken:$taken,fallthrough:$fallthrough};
            p.last_control=Some(control);p.last_control_taken=None;
            let taken=host.helper_branch(frame,&entry,control).map_err(Cause::Provider)?;
            p.last_control_taken=Some(taken);taken
        }};}
        macro_rules! sensitive {($slot:expr,$op:expr)=>{{super::sensitive(host,frame,&mut p,$slot,$op)?;}};}
        use Op::*;use I32::*;
        p.phase=Phase::Remainder;
        for &(rva,op) in &[
            (0x27ff15,StoreRcx32 {offset:0x20,source:Eax}),(0x27ff18,StoreRcx32 {offset:0x28,source:Edx}),
            (0x27ff1b,Add32(R10d,R8d)),(0x27ff1e,StoreRcx32 {offset:0x18,source:R10d}),(0x27ff22,Test32(Edx)),
        ] {ins!(rva,op);}
        if !branch!(0x27ff24,SignedGreater,0x27ff31,0x27ff26) {
            for &(rva,op) in &[(0x27ff26,Add32(Edx,R9d)),(0x27ff29,StoreRcx32 {offset:0x28,source:Edx}),
                (0x27ff2c,Dec32(Eax)),(0x27ff2e,StoreRcx32 {offset:0x20,source:Eax})] {ins!(rva,op);}
        }
        ins!(0x27ff31,Sub32(Edx,R9d));ins!(0x27ff34,StoreRcx32 {offset:0x24,source:Edx});
        ins!(0x27ff37,Mulss(Xmm::X1,Xmm::X2));
        sensitive!(3,Sensitive::CvttEaxXmm1At27ff3b);
        ins!(0x27ff3f,Mulss(Xmm::X0,Xmm::X2));
        sensitive!(4,Sensitive::CvttR10dXmm0At27ff43);
        ins!(0x27ff48,StoreRcx32 {offset:0x30,source:R9d});ins!(0x27ff4c,SubEaxR10d);ins!(0x27ff4f,Cdq);
        sensitive!(5,Sensitive::IdivR9dAt27ff50);
        for &(rva,op) in &[(0x27ff53,StoreRcx32 {offset:0x34,source:Eax}),(0x27ff56,StoreRcx32 {offset:0x3c,source:Edx}),
            (0x27ff59,Add32(R8d,R10d)),(0x27ff5c,StoreRcx32 {offset:0x2c,source:R8d}),(0x27ff60,Test32(Edx))] {ins!(rva,op);}
        if !branch!(0x27ff62,SignedGreater,0x27ff6f,0x27ff64) {
            for &(rva,op) in &[(0x27ff64,Add32(Edx,R9d)),(0x27ff67,StoreRcx32 {offset:0x3c,source:Edx}),
                (0x27ff6a,Dec32(Eax)),(0x27ff6c,StoreRcx32 {offset:0x34,source:Eax})] {ins!(rva,op);}
        }
        ins!(0x27ff6f,Sub32(Edx,R9d));ins!(0x27ff72,StoreRcx32 {offset:0x38,source:Edx});ins!(0x27ff75,Inc32(R9d));
        p.attempted_rva=0x27ff78;host.helper_jump(frame,&entry,0x27ff78,0x27ffb0).map_err(Cause::Provider)?;

        loop {
            p.phase=Phase::PointLoop;
            for &(rva,op) in &[
                (0x27ffb0,LoadRcx32 {destination:R11d,offset:0x18}),(0x27ffb4,LoadRcx32 {destination:Edx,offset:0x24}),
                (0x27ffb7,LoadRcx32 {destination:Eax,offset:0x20}),(0x27ffba,Add32(Eax,R11d)),
                (0x27ffbd,AddRcx32 {destination:Edx,offset:0x28}),(0x27ffc0,StoreRcx32 {offset:0x24,source:Edx}),
                (0x27ffc3,StoreRcx32 {offset:0x18,source:Eax}),
            ] {ins!(rva,op);}

            if !branch!(0x27ffc6,SignedLessEqual,0x27ffd3,0x27ffc8) {
                for &(rva,op) in &[(0x27ffc8,SubRcx32 {destination:Edx,offset:0x1c}),
                    (0x27ffcb,StoreRcx32 {offset:0x24,source:Edx}),(0x27ffce,Inc32(Eax)),
                    (0x27ffd0,StoreRcx32 {offset:0x18,source:Eax})] {ins!(rva,op);}
            }
            for &(rva,op) in &[
                (0x27ffd3,LoadRcx32 {destination:Esi,offset:0x2c}),(0x27ffd6,LoadRcx32 {destination:Edx,offset:0x38}),
                (0x27ffd9,LoadRcx32 {destination:Eax,offset:0x34}),(0x27ffdc,Add32(Eax,Esi)),
                (0x27ffde,AddRcx32 {destination:Edx,offset:0x3c}),(0x27ffe1,StoreRcx32 {offset:0x38,source:Edx}),
                (0x27ffe4,StoreRcx32 {offset:0x2c,source:Eax}),
            ] {ins!(rva,op);}
            if !branch!(0x27ffe7,SignedLessEqual,0x27fff4,0x27ffe9) {
                for &(rva,op) in &[(0x27ffe9,SubRcx32 {destination:Edx,offset:0x30}),
                    (0x27ffec,StoreRcx32 {offset:0x38,source:Edx}),(0x27ffef,Inc32(Eax)),
                    (0x27fff1,StoreRcx32 {offset:0x2c,source:Eax})] {ins!(rva,op);}
            }
            for &(rva,op) in &[(0x27fff4,Mov32(Eax,R11d)),(0x27fff7,Sar32(Eax,8)),(0x27fffa,RdiFromQwordRcx50),
                (0x27fffe,LoadRdi32 {destination:R10d,offset:0x1c}),(0x280002,LoadRdi32 {destination:Ebp,offset:0x20}),
                (0x280005,Cdq)] {ins!(rva,op);}
            sensitive!(6,Sensitive::IdivR10dAt280006);
            for &(rva,op) in &[(0x280009,Mov32(R8d,Edx)),(0x28000c,Mov32(Eax,Esi)),(0x28000e,Sar32(Eax,8)),
                (0x280011,Mov32(Ebx,Edx)),(0x280013,Sar32(Ebx,31)),(0x280016,And32(Ebx,R10d)),(0x280019,Cdq)] {ins!(rva,op);}
            sensitive!(7,Sensitive::IdivEbpAt28001a);
            for &(rva,op) in &[(0x28001c,Add32(Ebx,R8d)),(0x28001f,Mov32(Eax,Edx)),(0x280021,Sar32(Eax,31)),
                (0x280024,And32(Eax,Ebp)),(0x280026,Add32(Eax,Edx)),(0x280028,CmpRcx5cZero)] {ins!(rva,op);}
            let nearest=if branch!(0x28002c,Equal,0x27ff80,0x280032) {true}else{
                ins!(0x280032,CmpRcx32 {left:Ebx,offset:0x60});
                if branch!(0x280035,UnsignedAboveEqual,0x27ff80,0x28003b) {true}else{
                    ins!(0x28003b,CmpRcx32 {left:Eax,offset:0x64});
                    branch!(0x28003e,UnsignedAboveEqual,0x27ff80,0x280044)
                }
            };
            if !nearest {
                bilinear::source(host,frame,&mut p)?;
            }else{
            p.phase=Phase::Nearest;
            for &(rva,op) in &[
                (0x27ff80,Cdqe),(0x27ff82,SignedRdi32 {destination:R64::Rdx,offset:0x14}),
                (0x27ff86,Imul64(R64::Rdx,R64::Rax)),(0x27ff8a,AddRdxQwordRdi),(0x27ff8d,SignedEbxToRax),
                (0x27ff90,SignedRdi32 {destination:R64::R8,offset:0x18}),(0x27ff94,Imul64(R64::R8,R64::Rax)),
                (0x27ff98,EaxFromDwordR8Rdx),(0x27ff9c,DwordR14FromEax),
            ] {ins!(rva,op);}
            }

            ins!(0x27ff9f,AddR14Four);ins!(0x27ffa3,Dec32(R9d));ins!(0x27ffa6,CmpR9d1);
            if branch!(0x27ffaa,SignedLessEqual,0x2801a3,0x27ffb0) {break;}
        }
        p.phase=Phase::Epilogue;
        for &(rva,op) in &[
            (0x2801a3,RestoreAlignedXmm {destination:Xmm::X6,rsp_offset:0x10}),
            (0x2801a8,RestoreAlignedXmm {destination:Xmm::X7,rsp_offset:0x20}),
            (0x2801ad,RestoreAlignedXmm {destination:Xmm::X8,rsp_offset:0x30}),(0x2801b3,AddRsp48),
            (0x2801b7,Pop(Saved::Rbx)),(0x2801b8,Pop(Saved::Rbp)),(0x2801b9,Pop(Saved::Rdi)),(0x2801ba,Pop(Saved::Rsi)),
            (0x2801bb,Pop(Saved::R12)),(0x2801bd,Pop(Saved::R13)),(0x2801bf,Pop(Saved::R14)),(0x2801c1,Pop(Saved::R15)),
        ] {ins!(rva,op);}
        p.phase=Phase::Ret;p.attempted_rva=0x2801c3;
        match host.actual_helper_ret_2801c3(frame,&entry) {
            Ok(normal)=>{observed_return=Some(normal.observed);normal_return=Some(normal);},
            Err(super::super::CallFailure {error,observed_normal_return,returned,output})=>{
                p.phase=Phase::RetStopped;observed_return=observed_normal_return;partial_return=returned;partial_output=output;
                return Err(Cause::Provider(error));
            }
        }
        p.phase=Phase::ValidateRet;
        let normal=normal_return.as_ref().ok_or(Cause::Stopped(Stop::MissingReturn))?;
        let returned=normal.returned.as_ref().ok_or(Cause::Stopped(Stop::MissingReturn))?;
        let output=normal.output.as_ref().ok_or(Cause::Stopped(Stop::MissingOutput))?;
        if returned.key.target!=entry.target {return Err(Cause::Stopped(Stop::ReturnMismatch));}
        if output.key.target!=entry.target {return Err(Cause::Stopped(Stop::OutputMismatch));}
        if returned.key.callee_frame!=entry.callee_frame||output.key.callee_frame!=entry.callee_frame {
            return Err(Cause::Stopped(Stop::CalleeMismatch));
        }
        if returned.key.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
        host.validate_actual_helper_ret(frame,&entry,returned).map_err(Cause::Provider)?;
        host.validate_sample_return(frame,returned).map_err(Cause::Provider)?;
        host.validate_sample_output(frame,output).map_err(Cause::Provider)?;
        host.validate_sample_continuation(frame,&entry.target,entry.target.site.return_rva()).map_err(Cause::Provider)?;
        p.phase=Phase::Returned;Ok(())
    })();
    match result {
        Ok(())=>match normal_return.take() {
            Some(normal)=>Ok(Outcome::Returned {progress:p,normal}),
            None=>Err(Failure {progress:p,cause:Cause::Stopped(Stop::MissingReturn),normal:None,observed_return,partial_return,partial_output}),
        },
        Err(cause)=>Err(Failure {progress:p,cause,normal:normal_return,observed_return,partial_return,partial_output}),
    }
}

