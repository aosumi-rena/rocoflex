use super::{blend,prefix,BodyOutcome,Cause,Completed,Condition,Control,Entry,Failure,
    Instruction as Op,Kind,Normal,Phase,Progress,RequiredBody,Services,Stop};

pub fn source<S:Services>(host:&mut S,frame:&mut S::Frame,entry:Entry<S::Identity>)
    ->Result<BodyOutcome<S::Identity>,Failure<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    let mut p=prefix::begin(entry);
    let mut pending=prefix::Pending::new();
    let result=(||->Result<Normal<S::Identity>,Cause<S::Error>> {
        if entry.kind!=Kind::B2816b0 {return Err(Cause::Stopped(Stop::WrongEntry));}
        prefix::source(host,frame,&mut p,&mut pending)?;
        macro_rules! ins {($rva:expr,$op:expr)=>{{
            p.attempted_rva=$rva;host.bulk_instruction(frame,&entry,$rva,$op).map_err(Cause::Provider)?;
            p.completed_instructions=p.completed_instructions.wrapping_add(1);
        }};}
        macro_rules! branch {($rva:expr,$condition:ident,$taken:expr,$fallthrough:expr)=>{{
            p.phase=Phase::Branch;p.attempted_rva=$rva;
            let control=Control {rva:$rva,condition:Condition::$condition,taken:$taken,fallthrough:$fallthrough};
            p.last_control=Some(control);p.last_control_taken=None;
            let taken=host.bulk_branch(frame,&entry,control).map_err(Cause::Provider)?;
            p.last_control_taken=Some(taken);taken
        }};}
        use Op::*;
        p.phase=Phase::BlendSetup;
        for &(rva,op) in &[(0x281713,RaxFromQwordR1448),(0x281717,RcxFromQwordR1470),
            (0x28171b,EdxFromDwordRax18),(0x28171e,EaxFromEdx),(0x281720,ImulEaxEbp),
            (0x281723,R8FromSignedEax),(0x281726,ImulEbxDwordR1458),(0x28172b,SarEbx8),
            (0x28172e,RaxFromSignedEdx),(0x281731,CmpEbxFe)] {ins!(rva,op);}
        if branch!(0x281737,SignedGreaterEqual,0x281820,0x28173d) {return unscaled(host,frame,&mut p);}
        ins!(0x28173d,CmpEdx1);
        if branch!(0x281740,NotEqual,0x2818df,0x281746) {
            return scaled_nonunit(host,frame,&mut p);
        }

        for &(rva,op) in &[(0x281746,RdxFromSignedEbp),(0x281749,MovzxEdxByteRcxRdxPlus1),
            (0x28174e,AddRcxR8),(0x281751,AddRcxTwo),(0x281755,IncEsi),
            (0x281757,NopWordRaxRax)] {ins!(rva,op);}

        scaled_unit(host,frame,&mut p)
    })();
    match result {
        Ok(normal)=>Ok(BodyOutcome::Returned(Completed {progress:p,normal})),
        Err(cause)=>Err(pending.failure(p,cause)),
    }
}

pub(super) fn resume<S:Services>(host:&mut S,frame:&mut S::Frame,required:RequiredBody<S::Identity>)
    ->Result<BodyOutcome<S::Identity>,Failure<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    let RequiredBody {mut progress,next_rva}=required;
    let (expected,taken,expected_attempt)=match next_rva {
        0x281760=>(Control {rva:0x281740,condition:Condition::NotEqual,taken:0x2818df,fallthrough:0x281746},false,0x281757),
        0x2818df=>(Control {rva:0x281740,condition:Condition::NotEqual,taken:0x2818df,fallthrough:0x281746},true,0x281740),
        0x281820=>(Control {rva:0x281737,condition:Condition::SignedGreaterEqual,taken:0x281820,fallthrough:0x28173d},true,0x281737),
        0x281840=>(Control {rva:0x281823,condition:Condition::NotEqual,taken:0x2819af,fallthrough:0x281829},false,0x28183a),
        0x2819af=>(Control {rva:0x281823,condition:Condition::NotEqual,taken:0x2819af,fallthrough:0x281829},true,0x281823),
        _=>return Ok(BodyOutcome::Required(RequiredBody {progress,next_rva})),
    };
    let pending=prefix::Pending::new();
    let result=(||->Result<Normal<S::Identity>,Cause<S::Error>> {
        if progress.entry.kind!=Kind::B2816b0||progress.phase!=Phase::RequiredBody
            ||progress.last_control!=Some(expected)||progress.last_control_taken!=Some(taken)
            ||progress.attempted_rva!=expected_attempt {
            return Err(Cause::Stopped(Stop::WrongEntry));
        }
        host.validate_bulk_body_continuation(frame,&progress.entry,next_rva).map_err(Cause::Provider)?;
        match next_rva {
            0x281760=>scaled_unit(host,frame,&mut progress),
            0x2818df=>scaled_nonunit(host,frame,&mut progress),
            0x281820=>unscaled(host,frame,&mut progress),
            0x281840=>unscaled_unit(host,frame,&mut progress),
            0x2819af=>unscaled_nonunit(host,frame,&mut progress),
            _=>Err(Cause::Stopped(Stop::WrongEntry)),
        }
    })();
    match result {
        Ok(normal)=>Ok(BodyOutcome::Returned(Completed {progress,normal})),
        Err(cause)=>Err(pending.failure(progress,cause)),
    }
}

fn unscaled<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)
    ->Result<Normal<S::Identity>,Cause<S::Error>>
{
    let entry=p.entry;
    p.phase=Phase::BlendSetup;p.attempted_rva=0x281820;
    host.bulk_instruction(frame,&entry,0x281820,Op::CmpEdx1).map_err(Cause::Provider)?;
    p.completed_instructions=p.completed_instructions.wrapping_add(1);
    let control=Control {rva:0x281823,condition:Condition::NotEqual,taken:0x2819af,fallthrough:0x281829};
    p.phase=Phase::Branch;p.attempted_rva=0x281823;p.last_control=Some(control);p.last_control_taken=None;
    let taken=host.bulk_branch(frame,&entry,control).map_err(Cause::Provider)?;p.last_control_taken=Some(taken);
    if taken {return unscaled_nonunit(host,frame,p);}
    p.phase=Phase::BlendSetup;
    for &(rva,op) in &[(0x281829,Op::RdxFromSignedEbp),(0x28182c,Op::MovzxEdxByteRcxRdxPlus1),
        (0x281831,Op::AddRcxR8),(0x281834,Op::AddRcxTwo),(0x281838,Op::IncEsi),
        (0x28183a,Op::NopWordRaxRax)] {
        p.attempted_rva=rva;host.bulk_instruction(frame,&entry,rva,op).map_err(Cause::Provider)?;
        p.completed_instructions=p.completed_instructions.wrapping_add(1);
    }
    unscaled_unit(host,frame,p)
}

fn unscaled_unit<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)
    ->Result<Normal<S::Identity>,Cause<S::Error>>
{
    use blend::{Op as B,R::*};use B::*;
    loop {

        for &(rva,op) in &[
            (0x281840,LoadByteRdiPlus(R10d,3)),(0x281845,LoadDwordRdi(R8d)),(0x281848,AddRdiFour),
            (0x28184c,Imm(R9d,0x100)),(0x281852,Sub(R9d,R10d)),(0x281855,Mov(R10d,R8d)),
            (0x281858,And(R10d,0xff00ff)),(0x28185f,LoadByteRcxPlus(R11d,-2)),
            (0x281864,LoadByteRcxPlus(Ebx,0)),(0x281867,Shl(Ebx,16)),(0x28186a,Or(Ebx,R11d)),
            (0x28186d,Imul(Ebx,R9d)),(0x281871,Shr(Ebx,8)),(0x281874,And(Ebx,0xff00ff)),
            (0x28187a,Add(Ebx,R10d)),(0x28187d,Mov(R10d,Ebx)),(0x281880,Shr(R10d,8)),
            (0x281884,And(R10d,0x10001)),(0x28188b,Imm(R11d,0x100)),(0x281891,Sub(R11d,R10d)),
            (0x281894,Or(R11d,Ebx)),(0x281897,Shr(R8d,8)),(0x28189b,And(R8d,0xff00ff)),
            (0x2818a2,MovzxLowByte(Edx,Edx)),(0x2818a5,Imul(Edx,R9d)),(0x2818a9,Shr(Edx,8)),
            (0x2818ac,Add(Edx,R8d)),(0x2818af,Mov(R8d,Edx)),(0x2818b2,Shr(R8d,8)),
            (0x2818b6,Neg(R8d)),(0x2818b9,Or(R8d,Edx)),(0x2818bc,StoreByteRcxPlus(R8d,-1)),
            (0x2818c0,Mov(Edx,R11d)),(0x2818c3,Shr(Edx,16)),(0x2818c6,StoreByteRcxPlus(Edx,0)),
            (0x2818c8,StoreByteRcxPlus(R11d,-2)),
        ] {super::body::emit(host,frame,p,rva,op)?;}
        if !super::body::loop_tail(host,frame,p,[0x2818cc,0x2818cf,0x2818d1,0x2818d4],0x281840,0x2818da)? {break;}
    }
    p.attempted_rva=0x2818da;
    host.bulk_jump(frame,&p.entry,0x2818da,0x281a5b).map_err(Cause::Provider)?;
    super::finish(host,frame,p)
}

fn unscaled_nonunit<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)
    ->Result<Normal<S::Identity>,Cause<S::Error>>
{
    p.phase=Phase::BlendSetup;
    for &(rva,op) in &[(0x2819af,Op::AddRcxR8),(0x2819b2,Op::AddRcxTwo),
        (0x2819b6,Op::IncEsi),(0x2819b8,Op::NopDwordRaxRax)] {
        p.attempted_rva=rva;host.bulk_instruction(frame,&p.entry,rva,op).map_err(Cause::Provider)?;
        p.completed_instructions=p.completed_instructions.wrapping_add(1);
    }
    use blend::{Op as B,R::*};use B::*;
    loop {

        for &(rva,op) in &[
            (0x2819c0,LoadByteRdiPlus(R9d,3)),(0x2819c5,LoadDwordRdi(Edx)),(0x2819c7,AddRdiFour),
            (0x2819cb,Imm(R8d,0x100)),(0x2819d1,Sub(R8d,R9d)),(0x2819d4,Mov(R9d,Edx)),
            (0x2819d7,And(R9d,0xff00ff)),(0x2819de,LoadByteRcxPlus(R10d,-2)),
            (0x2819e3,LoadByteRcxPlus(R11d,0)),(0x2819e7,Shl(R11d,16)),(0x2819eb,Or(R11d,R10d)),
            (0x2819ee,Imul(R11d,R8d)),(0x2819f2,Shr(R11d,8)),(0x2819f6,And(R11d,0xff00ff)),
            (0x2819fd,Add(R11d,R9d)),(0x281a00,Mov(R9d,R11d)),(0x281a03,Shr(R9d,8)),
            (0x281a07,And(R9d,0x10001)),(0x281a0e,Imm(R10d,0x100)),(0x281a14,Sub(R10d,R9d)),
            (0x281a17,Or(R10d,R11d)),(0x281a1a,Shr(Edx,8)),(0x281a1d,And(Edx,0xff00ff)),
            (0x281a23,LoadByteRcxPlus(R9d,-1)),(0x281a28,Imul(R9d,R8d)),
            (0x281a2c,Shr(R9d,8)),(0x281a30,Add(R9d,Edx)),(0x281a33,Mov(Edx,R9d)),
            (0x281a36,Shr(Edx,8)),(0x281a39,Neg(Edx)),(0x281a3b,Or(Edx,R9d)),
            (0x281a3e,StoreByteRcxPlus(Edx,-1)),(0x281a41,Mov(Edx,R10d)),
            (0x281a44,Shr(Edx,16)),(0x281a47,StoreByteRcxPlus(Edx,0)),
            (0x281a49,StoreByteRcxPlus(R10d,-2)),
        ] {super::body::emit(host,frame,p,rva,op)?;}
        if !super::body::loop_tail(host,frame,p,[0x281a4d,0x281a50,0x281a52,0x281a55],0x2819c0,0x281a5b)? {break;}
    }

    super::finish(host,frame,p)
}

fn scaled_unit<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)
    ->Result<Normal<S::Identity>,Cause<S::Error>>
{
    use blend::{Op as B,R::*};use B::*;
    loop {

        for &(rva,op) in &[
            (0x281760,LoadDwordRdi(R8d)),(0x281763,AddRdiFour),
            (0x281767,Mov(R9d,R8d)),(0x28176a,Shr(R9d,8)),(0x28176e,And(R9d,0xff00ff)),
            (0x281775,Imul(R9d,Ebx)),(0x281779,Mov(R10d,R9d)),(0x28177c,Shr(R10d,8)),
            (0x281780,And(R10d,0xff00ff)),(0x281787,And(R8d,0xff00ff)),
            (0x28178e,Imul(R8d,Ebx)),(0x281792,Shr(R8d,8)),(0x281796,And(R8d,0xff00ff)),
            (0x28179d,Shr(R9d,24)),(0x2817a1,Imm(R11d,0x100)),(0x2817a7,Sub(R11d,R9d)),
            (0x2817aa,MovzxLowByte(Edx,Edx)),(0x2817ad,Imul(Edx,R11d)),
            (0x2817b1,Shr(Edx,8)),(0x2817b4,Add(Edx,R10d)),(0x2817b7,Mov(R9d,Edx)),
            (0x2817ba,Shr(R9d,8)),(0x2817be,Neg(R9d)),(0x2817c1,Or(R9d,Edx)),
            (0x2817c4,LoadByteRcxPlus(Edx,-2)),(0x2817c8,LoadByteRcxPlus(R10d,0)),
            (0x2817cc,Shl(R10d,16)),(0x2817d0,Or(R10d,Edx)),(0x2817d3,Imul(R10d,R11d)),
            (0x2817d7,Shr(R10d,8)),(0x2817db,And(R10d,0xff00ff)),(0x2817e2,Add(R10d,R8d)),
            (0x2817e5,Mov(Edx,R10d)),(0x2817e8,Shr(Edx,8)),(0x2817eb,And(Edx,0x10001)),
            (0x2817f1,Imm(R8d,0x100)),(0x2817f7,Sub(R8d,Edx)),(0x2817fa,Or(R8d,R10d)),
            (0x2817fd,StoreByteRcxPlus(R9d,-1)),(0x281801,Mov(Edx,R8d)),
            (0x281804,Shr(Edx,16)),(0x281807,StoreByteRcxPlus(Edx,0)),
            (0x281809,StoreByteRcxPlus(R8d,-2)),
        ] {super::body::emit(host,frame,p,rva,op)?;}
        if !super::body::loop_tail(host,frame,p,[0x28180d,0x281810,0x281812,0x281815],0x281760,0x28181b)? {break;}
    }
    p.attempted_rva=0x28181b;
    host.bulk_jump(frame,&p.entry,0x28181b,0x281a5b).map_err(Cause::Provider)?;
    super::finish(host,frame,p)
}

fn scaled_nonunit<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)
    ->Result<Normal<S::Identity>,Cause<S::Error>>
{
    p.phase=Phase::BlendSetup;
    for &(rva,op) in &[(0x2818df,Op::AddRcxR8),(0x2818e2,Op::AddRcxTwo),
        (0x2818e6,Op::IncEsi),(0x2818e8,Op::NopDwordRaxRax)] {
        p.attempted_rva=rva;host.bulk_instruction(frame,&p.entry,rva,op).map_err(Cause::Provider)?;
        p.completed_instructions=p.completed_instructions.wrapping_add(1);
    }
    use blend::{Op as B,R::*};use B::*;
    loop {

        for &(rva,op) in &[
            (0x2818f0,LoadDwordRdi(Edx)),(0x2818f2,AddRdiFour),
            (0x2818f6,Mov(R8d,Edx)),(0x2818f9,Shr(R8d,8)),(0x2818fd,And(R8d,0xff00ff)),
            (0x281904,Imul(R8d,Ebx)),(0x281908,Mov(R9d,R8d)),(0x28190b,Shr(R9d,8)),
            (0x28190f,And(R9d,0xff00ff)),(0x281916,And(Edx,0xff00ff)),
            (0x28191c,Imul(Edx,Ebx)),(0x28191f,Shr(Edx,8)),(0x281922,And(Edx,0xff00ff)),
            (0x281928,Shr(R8d,24)),(0x28192c,Imm(R10d,0x100)),(0x281932,Sub(R10d,R8d)),
            (0x281935,LoadByteRcxPlus(R8d,-1)),(0x28193a,Imul(R8d,R10d)),
            (0x28193e,Shr(R8d,8)),(0x281942,Add(R8d,R9d)),(0x281945,Mov(R9d,R8d)),
            (0x281948,Shr(R9d,8)),(0x28194c,Neg(R9d)),(0x28194f,Or(R9d,R8d)),
            (0x281952,LoadByteRcxPlus(R8d,-2)),(0x281957,LoadByteRcxPlus(R11d,0)),
            (0x28195b,Shl(R11d,16)),(0x28195f,Or(R11d,R8d)),(0x281962,Imul(R11d,R10d)),
            (0x281966,Shr(R11d,8)),(0x28196a,And(R11d,0xff00ff)),(0x281971,Add(R11d,Edx)),
            (0x281974,Mov(Edx,R11d)),(0x281977,Shr(Edx,8)),(0x28197a,And(Edx,0x10001)),
            (0x281980,Imm(R8d,0x100)),(0x281986,Sub(R8d,Edx)),(0x281989,Or(R8d,R11d)),
            (0x28198c,StoreByteRcxPlus(R9d,-1)),(0x281990,Mov(Edx,R8d)),
            (0x281993,Shr(Edx,16)),(0x281996,StoreByteRcxPlus(Edx,0)),
            (0x281998,StoreByteRcxPlus(R8d,-2)),
        ] {super::body::emit(host,frame,p,rva,op)?;}
        if !super::body::loop_tail(host,frame,p,[0x28199c,0x28199f,0x2819a1,0x2819a4],0x2818f0,0x2819aa)? {break;}
    }

    p.attempted_rva=0x2819aa;
    host.bulk_jump(frame,&p.entry,0x2819aa,0x281a5b).map_err(Cause::Provider)?;
    super::finish(host,frame,p)
}
