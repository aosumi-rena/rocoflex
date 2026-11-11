use super::{Cause,Failure,Path,Phase,Required,Stop};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    R14dFromDwordRsiMinus4,EdiFromDwordRsi,EbpFromLeaRdiPlusFf,TestEdi,
    CmovnsEbpEdi,SarEbp8,R15dFromLeaRcxPlusFf,TestEcx,CmovnsR15dEcx,SarR15d8,CmpEbpR15d,
    EdxFromEdi,SubEdxEcx,ImulEdxR14d,AddEdxEax,EaxFromEdx,DecEbx,AddRsi8,EcxFromEdi,CmpEbx2,
    MovzxEcxCl,EdxFrom100,R13dFromLeaRdxRax,AddR13dFf,
    EcxFromEdx,AddEcxEax,CmovnsR13dEcx,CmpEcx100,R12FromSignedR15d,CmpEcxFf00,
    LeaRcxWork40,LeaRdxWork24,R8dFromR15d,R9dFrom1,
    R14FromLeaR14RcxScale4,IncEdx,CmpEdxDwordR1514,
    SarR13d8,SarEsi8,TestR14d,IncR15d,R8dFromEbp,SubR8dR15d,
    EdxFromR15d,R9dFromR14d,MovzxEaxDil,ImulEaxR14d,
    EsiFromLeaRaxPlusFf,TestEax,CmovnsEsiEax,CmpEax100,
    R15FromQwordWorkC8,R14FromQwordWork38,EdxFromDwordWork28,RcxFromQwordWork30,
    RdiFromSignedEbp,CmpEaxFf00,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {Equal,UnsignedBelowEqual,SignedLess,UnsignedBelow,SignedGreaterEqual,SignedLessEqual,SignedGreater}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control {pub rva:u32,pub condition:Condition,pub taken:u32,pub fallthrough:u32}
#[derive(Clone, Copy, Debug)]
pub struct FinalStoreRow<I> {
    pub history:super::sample::Retained<I>,pub store_rva:u32,pub jump_rva:Option<u32>,
    pub arrival_rva:u32,pub reload_completed:u8,
}
pub trait Services:super::Services {

    fn validate_point_entry(&mut self,frame:&mut Self::Frame,owners:&super::inverse::Owners<Self::Identity>,path:Path,rva:u32)
        ->Result<(),Self::Error>;

    fn point_instruction(&mut self,frame:&mut Self::Frame,rva:u32,op:Instruction)->Result<(),Self::Error>;

    fn point_branch(&mut self,frame:&mut Self::Frame,control:Control)->Result<bool,Self::Error>;

    fn validate_final_store_row(&mut self,frame:&mut Self::Frame,row:&FinalStoreRow<Self::Identity>,at:u32)
        ->Result<(),Self::Error>;
}

pub fn reload_after_final_store<S:Services>(host:&mut S,required:Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let Required {mut state,next_rva}=required;
    let result=(||->Result<u32,Cause<S::Error>> {
        if state.progress.phase!=Phase::Complete {return Err(Cause::Stopped(Stop::WrongEntry));}
        let history=state.progress.sample_history.ok_or(Cause::Stopped(Stop::MissingSampleHistory))?;
        let (path,store_rva,jump_rva,pcs,next)=match (state.progress.path,history.returned.target.site,next_rva) {
            (Some(Path::A),super::sample::Site::AFinalFull,0x27a8cc)=>(Path::A,0x27ad4a,Some(0x27ad4e),[0x27a8cc,0x27a8d0],0x27a8d5),
            (Some(Path::A),super::sample::Site::AFinalPartial,0x27a8cc)=>(Path::A,0x27a8c8,None,[0x27a8cc,0x27a8d0],0x27a8d5),
            (Some(Path::B),super::sample::Site::BFinalFull,0x27ae41)=>(Path::B,0x27b26f,Some(0x27b273),[0x27ae41,0x27ae45],0x27ae4a),
            (Some(Path::B),super::sample::Site::BFinalPartial,0x27ae41)=>(Path::B,0x27ae3d,None,[0x27ae41,0x27ae45],0x27ae4a),
            _=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        let owners=state.shader.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        let mut row=FinalStoreRow {history,store_rva,jump_rva,arrival_rva:next_rva,reload_completed:0};
        state.progress.attempted_rva=next_rva;state.progress.final_store_row=Some(row);
        host.validate_final_store_row(&mut state.shader.frame,&row,next_rva).map_err(Cause::Provider)?;
        host.validate_point_entry(&mut state.shader.frame,&owners,path,next_rva).map_err(Cause::Provider)?;
        state.progress.phase=Phase::Row;
        for (rva,op) in pcs.into_iter().zip([Instruction::EdxFromDwordWork28,Instruction::RcxFromQwordWork30]) {
            state.progress.attempted_rva=rva;
            host.point_instruction(&mut state.shader.frame,rva,op).map_err(Cause::Provider)?;
            state.progress.completed_instructions+=1;row.reload_completed+=1;
            state.progress.final_store_row=Some(row);
        }
        state.progress.phase=Phase::Complete;Ok(next)
    })();
    match result {Ok(next_rva)=>Ok(Required {state,next_rva}),Err(cause)=>Err(Failure {state,cause})}
}

pub fn advance_skipped_row<S:Services>(host:&mut S,required:Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let Required {mut state,next_rva}=required;
    let result=(||->Result<u32,Cause<S::Error>> {
        let (path,pcs,jge,again)=match (state.progress.path,next_rva,state.progress.phase) {
            (Some(Path::A),0x27a8d5,Phase::Complete)=>(Path::A,[0x27a8d5,0x27a8d9,0x27a8db],0x27a8df,0x27a8e5),
            (Some(Path::B),0x27ae4a,Phase::Complete)=>(Path::B,[0x27ae4a,0x27ae4e,0x27ae50],0x27ae54,0x27ae5a),
            _=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        let owners=state.shader.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        let jl=match path {Path::A=>0x27a8ef,Path::B=>0x27ae64};
        let final_jl=match path {Path::A=>(0x27ac36,0x27ac3c),Path::B=>(0x27b176,0x27b17c)};
        let from_final=state.progress.point_control==Some(Control {rva:final_jl.0,
            condition:Condition::SignedLess,taken:next_rva,fallthrough:final_jl.1})
            &&state.progress.point_control_taken==Some(true);
        let from_store=match state.progress.final_store_row {
            Some(row) if row.reload_completed==2=>{
                let (store,jump,arrival)=match (path,row.history.returned.target.site) {
                    (Path::A,super::sample::Site::AFinalFull)=>(0x27ad4a,Some(0x27ad4e),0x27a8cc),
                    (Path::A,super::sample::Site::AFinalPartial)=>(0x27a8c8,None,0x27a8cc),
                    (Path::B,super::sample::Site::BFinalFull)=>(0x27b26f,Some(0x27b273),0x27ae41),
                    (Path::B,super::sample::Site::BFinalPartial)=>(0x27ae3d,None,0x27ae41),
                    _=>return Err(Cause::Stopped(Stop::WrongEntry)),
                };
                if row.store_rva!=store||row.jump_rva!=jump||row.arrival_rva!=arrival {
                    return Err(Cause::Stopped(Stop::WrongEntry));
                }
                host.validate_final_store_row(&mut state.shader.frame,&row,next_rva).map_err(Cause::Provider)?;true
            },_=>false,
        };
        if !from_final&&!from_store {
            let key=state.progress.branch.ok_or(Cause::Stopped(Stop::MissingBranch))?;
            if key.owners!=owners||key.path!=path||key.rva!=jl||!key.taken||key.destination!=next_rva {
                return Err(Cause::Stopped(Stop::BranchMismatch));
            }
            host.validate_loop_branch(&mut state.shader.frame,&key).map_err(Cause::Provider)?;
        }
        host.validate_point_entry(&mut state.shader.frame,&owners,path,next_rva).map_err(Cause::Provider)?;
        state.progress.phase=Phase::Row;
        for (rva,op) in pcs.into_iter().zip([Instruction::R14FromLeaR14RcxScale4,Instruction::IncEdx,Instruction::CmpEdxDwordR1514]) {
            state.progress.attempted_rva=rva;
            host.point_instruction(&mut state.shader.frame,rva,op).map_err(Cause::Provider)?;
            state.progress.completed_instructions+=1;
        }
        let control=Control {rva:jge,condition:Condition::SignedGreaterEqual,taken:0x27fbdf,fallthrough:again};
        state.progress.phase=Phase::PointBranch;state.progress.attempted_rva=jge;
        state.progress.point_control=Some(control);state.progress.point_control_taken=None;
        let taken=host.point_branch(&mut state.shader.frame,control).map_err(Cause::Provider)?;
        state.progress.point_control_taken=Some(taken);state.progress.phase=Phase::Complete;

        Ok(if taken {0x27fbdf}else{again})
    })();
    match result {Ok(next_rva)=>Ok(Required {state,next_rva}),Err(cause)=>Err(Failure {state,cause})}
}

pub fn advance<S:Services>(host:&mut S,required:Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>)
    ->Result<Required<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket>,
        Failure<S::Frame,S::Identity,S::AllocationReturnPacket,S::AllocationOutputPacket,S::Error>>
{
    let Required {mut state,next_rva}=required;
    let result=(||->Result<u32,Cause<S::Error>> {
        let path=match (state.progress.path,next_rva,state.progress.phase) {
            (Some(Path::A),0x27a95d|0x27aac7|0x27aacc|0x27aae7|0x27aaf4|0x27ac10|0x27a7c6,Phase::Complete)=>Path::A,
            (Some(Path::B),0x27aecd|0x27b01c|0x27b021|0x27b03c|0x27b049|0x27b150|0x27ad53,Phase::Complete)=>Path::B,
            _=>return Err(Cause::Stopped(Stop::WrongEntry)),
        };
        let owners=state.shader.inverse.owners.ok_or(Cause::Stopped(Stop::MissingOwners))?;
        state.progress.attempted_rva=next_rva;
        host.validate_point_entry(&mut state.shader.frame,&owners,path,next_rva).map_err(Cause::Provider)?;
        macro_rules! ins {($rva:expr,$op:expr)=>{{
            state.progress.phase=Phase::Point;state.progress.attempted_rva=$rva;
            host.point_instruction(&mut state.shader.frame,$rva,$op).map_err(Cause::Provider)?;
            state.progress.completed_instructions+=1;
        }};}
        macro_rules! branch {($rva:expr,$condition:ident,$taken:expr,$fallthrough:expr)=>{{
            state.progress.phase=Phase::PointBranch;state.progress.attempted_rva=$rva;
            let control=Control {rva:$rva,condition:Condition::$condition,taken:$taken,fallthrough:$fallthrough};
            state.progress.point_control=Some(control);state.progress.point_control_taken=None;
            let taken=host.point_branch(&mut state.shader.frame,control).map_err(Cause::Provider)?;
            state.progress.point_control_taken=Some(taken);taken
        }};}
        use Instruction::*;
        let (head,equal_rva,same_pixel,different,same_tail,done,coverage,low,low_target,
            r12,threshold,jb,partial,sample_args,call)=match path {
            Path::A=>([0x27a95d,0x27a961,0x27a963,0x27a969,0x27a96b,0x27a96e,
                    0x27a971,0x27a978,0x27a97a,0x27a97e,0x27a982],0x27a985,0x27a940,0x27a987,
                [0x27a940,0x27a942,0x27a944,0x27a948,0x27a94a,0x27a94c,0x27a94e,0x27a952,0x27a954],
                (0x27a957,0x27ac10),
                [0x27a987,0x27a98a,0x27a98f,0x27a991,0x27a995,0x27a999,0x27a9a0,0x27a9a2,0x27a9a4,0x27a9a8],
                0x27a9ae,0x27aac7,0x27a9b4,0x27a9b7,0x27a9bd,0x27aaf4,
                [0x27a9c3,0x27a9c8,0x27a9cd,0x27a9d0],0x27a9d6),
            Path::B=>([0x27aecd,0x27aed1,0x27aed3,0x27aed9,0x27aedb,0x27aede,
                    0x27aee1,0x27aee8,0x27aeea,0x27aeee,0x27aef2],0x27aef5,0x27aeb0,0x27aef7,
                [0x27aeb0,0x27aeb2,0x27aeb4,0x27aeb8,0x27aeba,0x27aebc,0x27aebe,0x27aec2,0x27aec4],
                (0x27aec7,0x27b150),
                [0x27aef7,0x27aefa,0x27aeff,0x27af01,0x27af05,0x27af09,0x27af10,0x27af12,0x27af14,0x27af18],
                0x27af1e,0x27b01c,0x27af24,0x27af27,0x27af2d,0x27b049,
                [0x27af33,0x27af38,0x27af3d,0x27af40],0x27af46),
        };
        let (partial_pcs,partial_call,tail_pcs,tail_jle,bulk_pcs,bulk_call,residual_pcs,residual_jump)=match path {
            Path::A=>([0x27aaf4,0x27aaf8,0x27aafd,0x27ab02,0x27ab05],0x27ab0b,
                [0x27aac7,0x27aacc,0x27aacf,0x27aad2],(0x27aaca,0x27aad5),
                [0x27aad7,0x27aadc,0x27aadf],0x27aae2,[0x27aae7,0x27aaeb],0x27aaef),
            Path::B=>([0x27b049,0x27b04d,0x27b052,0x27b057,0x27b05a],0x27b060,
                [0x27b01c,0x27b021,0x27b024,0x27b027],(0x27b01f,0x27b02a),
                [0x27b02c,0x27b031,0x27b034],0x27b037,[0x27b03c,0x27b040],0x27b044),
        };
        let (final_pcs,final_jl,row_skip,final_rdi,final_cmp,final_jb,final_partial,final_args,final_call)=match path {
            Path::A=>([0x27ac10,0x27ac16,0x27ac18,0x27ac1b,0x27ac20,0x27ac28,0x27ac2d,0x27ac31],
                0x27ac36,0x27a8d5,0x27ac3c,0x27ac3f,0x27ac44,0x27a7c6,
                [0x27ac4a,0x27ac4f,0x27ac54,0x27ac57],0x27ac5d),
            Path::B=>([0x27b150,0x27b156,0x27b158,0x27b15b,0x27b160,0x27b168,0x27b16d,0x27b171],
                0x27b176,0x27ae4a,0x27b17c,0x27b17f,0x27b184,0x27ad53,
                [0x27b18a,0x27b18f,0x27b194,0x27b197],0x27b19d),
        };
        let mut entry=next_rva;

        'point: loop {
        if entry==final_partial {
            let (pcs,call)=match path {
                Path::A=>([0x27a7c6,0x27a7c9,0x27a7ce,0x27a7d3,0x27a7d6],0x27a7dc),
                Path::B=>([0x27ad53,0x27ad56,0x27ad5b,0x27ad60,0x27ad63],0x27ad69),
            };
            for (rva,op) in pcs.into_iter().zip([SarEsi8,LeaRcxWork40,LeaRdxWork24,R8dFromEbp,R9dFrom1]) {ins!(rva,op);}
            state.progress.phase=Phase::Complete;return Ok(call);
        }
        if entry==done.1 {
            for (rva,op) in final_pcs.into_iter().zip([EsiFromLeaRaxPlusFf,TestEax,CmovnsEsiEax,CmpEax100,
                R15FromQwordWorkC8,R14FromQwordWork38,EdxFromDwordWork28,RcxFromQwordWork30]) {ins!(rva,op);}

            if branch!(final_jl,SignedLess,row_skip,final_rdi) {state.progress.phase=Phase::Complete;return Ok(row_skip);}
            ins!(final_rdi,RdiFromSignedEbp);ins!(final_cmp,CmpEaxFf00);
            if branch!(final_jb,UnsignedBelow,final_partial,final_args[0]) {entry=final_partial;continue 'point;}
            for (rva,op) in final_args.into_iter().zip([LeaRcxWork40,LeaRdxWork24,R8dFromEbp,R9dFrom1]) {ins!(rva,op);}
            state.progress.phase=Phase::Complete;return Ok(final_call);
        }
        if entry==partial {
            for (rva,op) in partial_pcs.into_iter().zip([SarR13d8,LeaRcxWork40,LeaRdxWork24,R8dFromR15d,R9dFrom1]) {ins!(rva,op);}
            state.progress.phase=Phase::Complete;return Ok(partial_call);
        }
        if entry==low_target||entry==tail_pcs[1]||entry==residual_pcs[0] {
            let positive=if entry==tail_pcs[1] {true}else if entry==residual_pcs[0] {false}else{
                ins!(tail_pcs[0],TestR14d);
                !branch!(tail_jle.0,SignedLessEqual,residual_pcs[0],tail_pcs[1])
            };
            if positive {
                for (rva,op) in tail_pcs[1..].iter().copied().zip([IncR15d,R8dFromEbp,SubR8dR15d]) {ins!(rva,op);}
                if !branch!(tail_jle.1,SignedLessEqual,residual_pcs[0],bulk_pcs[0]) {
                    for (rva,op) in bulk_pcs.into_iter().zip([LeaRcxWork40,EdxFromR15d,R9dFromR14d]) {ins!(rva,op);}

                    state.progress.phase=Phase::Complete;return Ok(bulk_call);
                }
            }
            ins!(residual_pcs[0],MovzxEaxDil);ins!(residual_pcs[1],ImulEaxR14d);
            state.progress.phase=Phase::Jump;state.progress.attempted_rva=residual_jump;
            host.setup_jump(&mut state.shader.frame,residual_jump,same_tail[5]).map_err(Cause::Provider)?;
            for (rva,op) in same_tail[5..].iter().copied().zip([DecEbx,AddRsi8,EcxFromEdi,CmpEbx2]) {ins!(rva,op);}
            if branch!(done.0,UnsignedBelowEqual,done.1,head[0]) {entry=done.1;continue 'point;}
            entry=head[0];continue 'point;
        }
        loop {
            for (rva,op) in head.into_iter().zip([R14dFromDwordRsiMinus4,EdiFromDwordRsi,
                EbpFromLeaRdiPlusFf,TestEdi,CmovnsEbpEdi,SarEbp8,R15dFromLeaRcxPlusFf,
                TestEcx,CmovnsR15dEcx,SarR15d8,CmpEbpR15d]) {ins!(rva,op);}
            if !branch!(equal_rva,Equal,same_pixel,different) {break;}
            for (rva,op) in same_tail.into_iter().zip([EdxFromEdi,SubEdxEcx,ImulEdxR14d,
                AddEdxEax,EaxFromEdx,DecEbx,AddRsi8,EcxFromEdi,CmpEbx2]) {ins!(rva,op);}
            if branch!(done.0,UnsignedBelowEqual,done.1,head[0]) {
                entry=done.1;continue 'point;
            }
        }
        for (rva,op) in coverage.into_iter().zip([MovzxEcxCl,EdxFrom100,SubEdxEcx,
            ImulEdxR14d,R13dFromLeaRdxRax,AddR13dFf,EcxFromEdx,AddEcxEax,CmovnsR13dEcx,CmpEcx100]) {ins!(rva,op);}
        if branch!(low,SignedLess,low_target,r12) {entry=low_target;continue 'point;}
        ins!(r12,R12FromSignedR15d);ins!(threshold,CmpEcxFf00);
        if branch!(jb,UnsignedBelow,partial,sample_args[0]) {entry=partial;continue 'point;}
        for (rva,op) in sample_args.into_iter().zip([LeaRcxWork40,LeaRdxWork24,R8dFromR15d,R9dFrom1]) {ins!(rva,op);}
        state.progress.phase=Phase::Complete;return Ok(call);

        }
    })();
    match result {Ok(next_rva)=>Ok(Required {state,next_rva}),Err(cause)=>Err(Failure {state,cause})}
}
