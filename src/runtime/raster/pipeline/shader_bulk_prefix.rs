use super::{sample,Cause,Condition,Control,Entry,Failure,Instruction as Op,Kind,
    Phase,Progress,Resource,Services,Stop};

pub(super) fn begin<I:Copy>(entry:Entry<I>)->Progress<I> {
    Progress {entry,phase:Phase::Entry,attempted_rva:entry.kind.callee_rva(),completed_instructions:0,
        last_control:None,last_control_taken:None,resources:[None,None],sample_target:None,sample_observed:None,
        sample_history:entry.prior_sample_history,sample_retention_attempt:None,normal:None,helper_progress:None}
}

pub(super) struct Pending<I,R,O,E> {
    normal:Option<sample::NormalReturn<I,R,O>>,
    returned:Option<sample::Returned<I,R>>,output:Option<sample::Output<I,O>>,
    helper_failure:Option<sample::helper::InvocationFailure<I,R,O,E>>,
}
impl<I,R,O,E> Pending<I,R,O,E> {
    pub(super) fn new()->Self {Self {normal:None,returned:None,output:None,helper_failure:None}}
    pub(super) fn failure(self,progress:Progress<I>,cause:Cause<E>)->Failure<I,R,O,E> {
        Failure {progress,cause,sample_normal:self.normal,sample_return:self.returned,
            sample_output:self.output,helper_failure:self.helper_failure}
    }
}

pub(super) fn source<S:Services>(host:&mut S,frame:&mut S::Frame,p:&mut Progress<S::Identity>,
    pending:&mut Pending<S::Identity,S::SampleReturnPacket,S::SampleOutputPacket,S::Error>)
    ->Result<(),Cause<S::Error>>
{
    let entry=p.entry;
    let pc=|a,b|match entry.kind {Kind::A27fc10=>a,Kind::B2816b0=>b};
    host.validate_bulk_entry(frame,&entry).map_err(Cause::Provider)?;
    macro_rules! ins {($a:expr,$b:expr,$op:expr)=>{{
        let rva=pc($a,$b);p.attempted_rva=rva;
        host.bulk_instruction(frame,&entry,rva,$op).map_err(Cause::Provider)?;
        p.completed_instructions=p.completed_instructions.wrapping_add(1);
    }};}
    macro_rules! branch {($a:expr,$b:expr,$condition:ident,$ta:expr,$tb:expr,$fa:expr,$fb:expr)=>{{
        let rva=pc($a,$b);p.phase=Phase::Branch;p.attempted_rva=rva;
        let control=Control {rva,condition:Condition::$condition,taken:pc($ta,$tb),fallthrough:pc($fa,$fb)};
        p.last_control=Some(control);p.last_control_taken=None;
        let taken=host.bulk_branch(frame,&entry,control).map_err(Cause::Provider)?;
        p.last_control_taken=Some(taken);taken
    }};}
    macro_rules! jump {($a:expr,$b:expr,$ta:expr,$tb:expr)=>{{
        let rva=pc($a,$b);p.attempted_rva=rva;
        host.bulk_jump(frame,&entry,rva,pc($ta,$tb)).map_err(Cause::Provider)?;
    }};}
    use Op::*;use sample::helper::Saved;
    p.phase=Phase::Prefix;
    for (a,b,reg) in [(0x27fc10,0x2816b0,Saved::R14),(0x27fc12,0x2816b2,Saved::Rsi),
        (0x27fc13,0x2816b3,Saved::Rdi),(0x27fc14,0x2816b4,Saved::Rbp),(0x27fc15,0x2816b5,Saved::Rbx)] {
        ins!(a,b,Push(reg));
    }
    for &(a,b,op) in &[(0x27fc16,0x2816b6,SubRsp20),(0x27fc1a,0x2816ba,EbxFromR9d),
        (0x27fc1d,0x2816bd,EsiFromR8d),(0x27fc20,0x2816c0,EbpFromEdx),
        (0x27fc22,0x2816c2,R14FromRcx),(0x27fc25,0x2816c5,CmpDwordRcx80R8d)] {ins!(a,b,op);}
    if branch!(0x27fc2c,0x2816cc,SignedGreaterEqual,0x27fc56,0x2816f6,0x27fc2e,0x2816ce) {
        ins!(0x27fc56,0x2816f6,RdiFromQwordR1478);jump!(0x27fc5a,0x2816fa,0x27fc62,0x281702);
    }else{
        ins!(0x27fc2e,0x2816ce,RdiFromSignedEsi);

        ins!(0x27fc31,0x2816d1,QwordR1480FromRdi);ins!(0x27fc38,0x2816d8,RcxFromQwordR1478);
        let release=match entry.kind {Kind::A27fc10=>Resource::At27fc3c,Kind::B2816b0=>Resource::At2816dc};
        super::resource(host,frame,p,0,release)?;
        ins!(0x27fc41,0x2816e1,TestEdi);
        if branch!(0x27fc43,0x2816e3,Equal,0x27fc5c,0x2816fc,0x27fc45,0x2816e5) {
            ins!(0x27fc5c,0x2816fc,XorEdi);
        }else{
            ins!(0x27fc45,0x2816e5,ShlRdi2);ins!(0x27fc49,0x2816e9,RcxFromRdi);
            let allocate=match entry.kind {Kind::A27fc10=>Resource::At27fc4c,Kind::B2816b0=>Resource::At2816ec};
            super::resource(host,frame,p,1,allocate)?;

            ins!(0x27fc51,0x2816f1,RdiFromCurrentRax);jump!(0x27fc54,0x2816f4,0x27fc5e,0x2816fe);
        }
        ins!(0x27fc5e,0x2816fe,QwordR1478FromRdi);
    }
    for &(a,b,op) in &[(0x27fc62,0x281702,RcxFromR14),(0x27fc65,0x281705,RdxFromRdi),
        (0x27fc68,0x281708,R8dFromEbp),(0x27fc6b,0x28170b,R9dFromEsi)] {ins!(a,b,op);}
    let site=entry.kind.sample_site();p.phase=Phase::Sample;p.attempted_rva=site.call_rva();
    let target=host.bind_bulk_sample(frame,&entry).map_err(Cause::Provider)?;p.sample_target=Some(target);
    let origin_matches=match target.origin {
        sample::ArgumentOrigin::BulkRegisters {bulk_frame,bulk_call_receipt,current_r14,current_rdi}=>
            bulk_frame==entry.callee_frame&&bulk_call_receipt==entry.actual_call_receipt
                &&target.arguments.rcx==current_r14&&target.arguments.rdx==current_rdi,
        sample::ArgumentOrigin::ShaderWork {..}=>false,
    };
    if target.owners!=entry.owners||target.site!=site
        ||target.callee_module!=entry.owners.source_module||!origin_matches {
        return Err(Cause::Stopped(Stop::SampleTargetMismatch));
    }

    match sample::helper::invoke(host,frame,target) {
        Ok(completed)=>{p.helper_progress=Some(completed.progress);p.sample_observed=Some(completed.normal.observed);pending.normal=Some(completed.normal);},
        Err(failure)=>{
            p.phase=Phase::SampleStopped;p.helper_progress=failure.progress().copied();pending.helper_failure=Some(failure);
            return Err(Cause::Stopped(Stop::HelperStopped));
        }
    }
    let normal=pending.normal.as_ref().ok_or(Cause::Stopped(Stop::MissingReturn))?;
    let returned=normal.returned.as_ref().ok_or(Cause::Stopped(Stop::MissingReturn))?;
    let output=normal.output.as_ref().ok_or(Cause::Stopped(Stop::MissingEffects))?;
    let rk=returned.key;let ok=output.key;
    if rk.target!=target {return Err(Cause::Stopped(Stop::ReturnMismatch));}
    if ok.target!=target {return Err(Cause::Stopped(Stop::OutputMismatch));}
    if rk.callee_frame!=ok.callee_frame {return Err(Cause::Stopped(Stop::CalleeMismatch));}
    if rk.observed!=normal.observed {return Err(Cause::Stopped(Stop::ObservationMismatch));}
    host.validate_sample_return(frame,returned).map_err(Cause::Provider)?;
    host.validate_sample_output(frame,output).map_err(Cause::Provider)?;
    host.validate_sample_continuation(frame,&target,site.return_rva()).map_err(Cause::Provider)?;
    let owned=pending.normal.take().ok_or(Cause::Stopped(Stop::MissingReturn))?;
    let (returned,output)=match (owned.returned,owned.output) {
        (Some(r),Some(o))=>(r,o),(r,o)=>{pending.returned=r;pending.output=o;return Err(Cause::Stopped(Stop::MissingRetention));}
    };
    p.phase=Phase::RetainSample;
    let prior=p.sample_history;
    let retained=match host.retain_sample_packets(frame,prior.as_ref(),returned,output) {
        Ok(Some(retained))=>retained,Ok(None)=>return Err(Cause::Stopped(Stop::MissingRetention)),
        Err(sample::RetentionFailure {error,retained,returned,output})=>{
            p.sample_retention_attempt=retained;pending.returned=returned;pending.output=output;
            return Err(Cause::Provider(error));
        }
    };
    p.sample_retention_attempt=Some(retained);
    if retained.frame_owner!=entry.owners.frame||retained.returned!=rk||retained.output!=ok
        ||retained.previous!=prior.map(|r|r.node)||prior.map(|r|r.history_owner!=retained.history_owner).unwrap_or(false) {
        return Err(Cause::Stopped(Stop::RetentionMismatch));
    }
    p.sample_history=Some(retained);p.phase=Phase::SampleRetained;
    host.validate_sample_history(frame,&retained).map_err(Cause::Provider)
}
