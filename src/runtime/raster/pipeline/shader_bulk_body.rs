use super::{blend,prefix,Cause,Completed,Condition,Control,Entry,Failure,Instruction as Op,
    Kind,Normal,Phase,Progress,Services,Stop};

pub fn source<S:Services>(host:&mut S,frame:&mut S::Frame,entry:Entry<S::Identity>)
    ->Result<Completed<S::Identity>,Failure<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>>
{
    let mut p=prefix::begin(entry);
    let mut pending=prefix::Pending::new();
    let result=(||->Result<Normal<S::Identity>,Cause<S::Error>> {
        if entry.kind!=Kind::A27fc10 {return Err(Cause::Stopped(Stop::WrongEntry));}
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
        macro_rules! jump {($rva:expr,$target:expr)=>{{
            p.attempted_rva=$rva;host.bulk_jump(frame,&entry,$rva,$target).map_err(Cause::Provider)?;
        }};}
        use Op::*;
        p.phase=Phase::BlendSetup;
        for &(rva,op) in &[(0x27fc73,RaxFromQwordR1448),(0x27fc77,RaxFromSignedDwordRax18),
            (0x27fc7b,RcxFromSignedEbp),(0x27fc7e,ImulRcxRax),(0x27fc82,AddRcxQwordR1470),
            (0x27fc86,ImulEbxDwordR1458),(0x27fc8b,SarEbx8),(0x27fc8e,IncEsi),(0x27fc90,CmpEbxFe)] {ins!(rva,op);}
        let unscaled=branch!(0x27fc96,SignedGreaterEqual,0x27fd80,0x27fc9c);
        if !unscaled {ins!(0x27fc9c,NopDwordRax);}

        if unscaled {blend_unscaled(host,frame,&mut p)?;}else{blend_scaled(host,frame,&mut p)?;jump!(0x27fd76,0x27fe3b);}
        super::finish(host,frame,&mut p)
    })();
    match result {Ok(normal)=>Ok(Completed {progress:p,normal}),Err(cause)=>Err(pending.failure(p,cause))}
}

pub(super) fn emit<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>,rva:u32,op:blend::Op)
    ->Result<(),Cause<S::Error>>
{
    p.phase=Phase::Blend;p.attempted_rva=rva;host.blend_instruction(frame,rva,op).map_err(Cause::Provider)?;
    p.completed_instructions=p.completed_instructions.wrapping_add(1);Ok(())
}
pub(super) fn loop_tail<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>,pcs:[u32;4],target:u32,fallthrough:u32)
    ->Result<bool,Cause<S::Error>>
{
    for (rva,op) in pcs[..3].iter().copied().zip([Op::AddRcxRax,Op::DecEsi,Op::CmpEsi1]) {
        p.attempted_rva=rva;host.bulk_instruction(frame,&p.entry,rva,op).map_err(Cause::Provider)?;
        p.completed_instructions=p.completed_instructions.wrapping_add(1);
    }
    let control=Control {rva:pcs[3],condition:Condition::SignedGreater,taken:target,fallthrough};
    p.phase=Phase::Branch;p.attempted_rva=pcs[3];p.last_control=Some(control);p.last_control_taken=None;
    let taken=host.bulk_branch(frame,&p.entry,control).map_err(Cause::Provider)?;p.last_control_taken=Some(taken);Ok(taken)
}
fn blend_scaled<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)->Result<(),Cause<S::Error>> {
    use blend::{Op as B,R};
    let pcs:[u32;44]=[
        0x27fca6,0x27fca9,0x27fcb0,0x27fcb4,0x27fcb8,0x27fcbf,0x27fcc2,0x27fcc8,
        0x27fccb,0x27fcce,0x27fcd2,0x27fcd9,0x27fcdc,0x27fce2,0x27fce5,0x27fce7,
        0x27fcea,0x27fcf1,0x27fcf5,0x27fcf9,0x27fd00,0x27fd03,0x27fd06,0x27fd0c,
        0x27fd10,0x27fd13,0x27fd19,0x27fd1c,0x27fd1f,0x27fd23,0x27fd2a,0x27fd30,
        0x27fd33,0x27fd36,0x27fd3d,0x27fd40,0x27fd44,0x27fd4b,0x27fd51,0x27fd54,
        0x27fd57,0x27fd5b,0x27fd62,0x27fd65];
    let ops=blend::packed_partial(R::Ebx,B::LoadDwordRcx(R::Edx),B::StoreDwordRcx(R::R10d));
    loop {
        emit(host,frame,p,0x27fca0,B::LoadDwordRdi(R::Edx))?;emit(host,frame,p,0x27fca2,B::AddRdiFour)?;
        for (rva,op) in pcs.into_iter().zip(ops) {emit(host,frame,p,rva,op)?;}
        if !loop_tail(host,frame,p,[0x27fd68,0x27fd6b,0x27fd6d,0x27fd70],0x27fca0,0x27fd76)? {return Ok(());}
    }
}
fn blend_unscaled<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>)->Result<(),Cause<S::Error>> {
    use blend::{Op as B,R::*};use B::*;
    let pcs:[u32;30]=[0x27fdaa,0x27fdac,0x27fdaf,0x27fdb6,0x27fdba,0x27fdbe,0x27fdc5,0x27fdc8,
        0x27fdcb,0x27fdd1,0x27fdd5,0x27fdd8,0x27fdde,0x27fde1,0x27fde4,0x27fde8,
        0x27fdef,0x27fdf5,0x27fdf8,0x27fdfb,0x27fe02,0x27fe05,0x27fe09,0x27fe10,
        0x27fe16,0x27fe19,0x27fe1c,0x27fe20,0x27fe27,0x27fe2a];
    let common=blend::packed_partial(Ebx,LoadDwordRcx(Edx),StoreDwordRcx(R10d));
    loop {
        for &(rva,op) in &[(0x27fd80,LoadDwordRdi(Edx)),(0x27fd82,AddRdiFour),(0x27fd86,Mov(R8d,Edx)),
            (0x27fd89,And(R8d,0xff00ff)),(0x27fd90,Mov(R9d,Edx)),(0x27fd93,Shr(R9d,8)),
            (0x27fd97,And(R9d,0xff00ff)),(0x27fd9e,Shr(Edx,24)),(0x27fda1,Imm(R10d,0x100)),(0x27fda7,Sub(R10d,Edx))] {
            emit(host,frame,p,rva,op)?;
        }

        for (rva,op) in pcs.into_iter().zip(common[14..].iter().copied()) {emit(host,frame,p,rva,op)?;}
        if !loop_tail(host,frame,p,[0x27fe2d,0x27fe30,0x27fe32,0x27fe35],0x27fd80,0x27fe3b)? {return Ok(());}
    }
}
