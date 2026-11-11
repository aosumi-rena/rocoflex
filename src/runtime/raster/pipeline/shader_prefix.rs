#[path = "shader_inverse.rs"] pub mod inverse;
#[path = "shader_setup.rs"] pub mod setup;
#[path = "shader_driver.rs"] pub mod driver;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register { R15, R14, R13, R12, Rsi, Rdi, Rbp, Rbx }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Push(Register), SubRspE8, SaveXmm6WorkD0,
    EbxFromR9d, RdiFromR8, R14FromRdx, R15FromRcx,
    EbpFromWork158, R8dFromRdx10, XorEsi, XorpsXmm6,
    TestEbp, SeteSil,

    Xmm6FromRip331041,
    MovzxEaxByteWork160, RcxFromWork150, EdxFromRdi10,
    ShlEsi7, AddEsiMinus80, IncEbx, CmpR8d1, SaveR15WorkC8,
    CmpR8d2, CmpEdx1, CmpEdx2, LeaRdxWork40, TestAl,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition { Equal, NotEqual }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Branch { pub rva: u32, pub condition: Condition, pub taken: u32, pub fallthrough: u32 }
pub trait Services {

    type Frame;
    type Error;

    fn validate_entry_27a570(&mut self, frame: &mut Self::Frame) -> Result<(), Self::Error>;

    fn instruction(&mut self, frame: &mut Self::Frame, rva: u32, instruction: Instruction)
        -> Result<(), Self::Error>;

    fn branch(&mut self, frame: &mut Self::Frame, branch: Branch) -> Result<bool, Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    SourceOtherAt27a728,
    Source2Destination1At27b56f,
    Source2DestinationOtherAt27c17e,
    Source1Destination1At27bba9,
    Source1DestinationOtherAt27c5e8,

    SharedFrameBodyAt27d14f,
    SharedFrameBodyAt27d74f,

    Actual211800CallAt27a60d,
    Actual211800CallAt27a6aa,
}
impl Next {
    pub fn rva(self) -> u32 { match self {
        Self::SourceOtherAt27a728 => 0x27a728,
        Self::Source2Destination1At27b56f => 0x27b56f,
        Self::Source2DestinationOtherAt27c17e => 0x27c17e,
        Self::Source1Destination1At27bba9 => 0x27bba9,
        Self::Source1DestinationOtherAt27c5e8 => 0x27c5e8,
        Self::SharedFrameBodyAt27d14f => 0x27d14f,
        Self::SharedFrameBodyAt27d74f => 0x27d74f,
        Self::Actual211800CallAt27a60d => 0x27a60d,
        Self::Actual211800CallAt27a6aa => 0x27a6aa,
    } }
}
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub attempted_rva: u32, pub entry_validated: bool, pub completed_instructions: usize,
    pub last_branch: Option<Branch>, pub last_branch_taken: Option<bool>, pub next: Option<Next>,
}
#[must_use = "SAME raw frame/backings must reach a required exact body; this is NOT a render/RET"]
pub struct Required<F> { pub frame: F, pub progress: Progress, pub next: Next }
#[must_use = "keep advanced stack/regs/maps/resources and original provider error; do not retry"]
pub struct Failure<F,E> { pub frame: F, pub progress: Progress, pub error: E }

pub fn dispatch<S: Services>(host: &mut S, mut frame: S::Frame)
    -> Result<Required<S::Frame>, Failure<S::Frame,S::Error>>
{
    use Instruction::*;
    let mut p = Progress { attempted_rva: 0x27a570, entry_validated: false,
        completed_instructions: 0, last_branch: None, last_branch_taken: None, next: None };
    let result = (|| -> Result<Next,S::Error> {
        host.validate_entry_27a570(&mut frame)?; p.entry_validated = true;
        macro_rules! ins { ($rva:expr,$instruction:expr) => {{
            p.attempted_rva=$rva;
            host.instruction(&mut frame,$rva,$instruction)?;
            p.completed_instructions+=1;
        }}; }
        macro_rules! branch { ($rva:expr,$condition:ident,$taken:expr,$fallthrough:expr) => {{
            p.attempted_rva=$rva;
            let branch=Branch { rva:$rva,condition:Condition::$condition,taken:$taken,fallthrough:$fallthrough };
            p.last_branch=Some(branch);p.last_branch_taken=None;
            let taken=host.branch(&mut frame,branch)?; p.last_branch_taken=Some(taken); taken
        }}; }
        for (rva,register) in [(0x27a570,Register::R15),(0x27a572,Register::R14),
            (0x27a574,Register::R13),(0x27a576,Register::R12),(0x27a578,Register::Rsi),
            (0x27a579,Register::Rdi),(0x27a57a,Register::Rbp),(0x27a57b,Register::Rbx)] {
            ins!(rva,Push(register));
        }
        ins!(0x27a57c,SubRspE8); ins!(0x27a583,SaveXmm6WorkD0);
        ins!(0x27a58b,EbxFromR9d); ins!(0x27a58e,RdiFromR8);
        ins!(0x27a591,R14FromRdx); ins!(0x27a594,R15FromRcx);
        ins!(0x27a597,EbpFromWork158); ins!(0x27a59e,R8dFromRdx10);
        ins!(0x27a5a2,XorEsi); ins!(0x27a5a4,XorpsXmm6);
        ins!(0x27a5a7,TestEbp); ins!(0x27a5a9,SeteSil);

        if !branch!(0x27a5ad,Equal,0x27a5b7,0x27a5af) { ins!(0x27a5af,Xmm6FromRip331041); }
        ins!(0x27a5b7,MovzxEaxByteWork160); ins!(0x27a5bf,RcxFromWork150);
        ins!(0x27a5c7,EdxFromRdi10); ins!(0x27a5ca,ShlEsi7);
        ins!(0x27a5cd,AddEsiMinus80); ins!(0x27a5d0,IncEbx);
        ins!(0x27a5d2,CmpR8d1); ins!(0x27a5d6,SaveR15WorkC8);

        if branch!(0x27a5de,Equal,0x27a68b,0x27a5e4) {
            ins!(0x27a68b,CmpEdx1);
            if branch!(0x27a68e,Equal,0x27bba9,0x27a694) { return Ok(Next::Source1Destination1At27bba9); }
            ins!(0x27a694,CmpEdx2);
            if branch!(0x27a697,NotEqual,0x27c5e8,0x27a69d) { return Ok(Next::Source1DestinationOtherAt27c5e8); }
            ins!(0x27a69d,LeaRdxWork40); ins!(0x27a6a2,TestAl);
            if branch!(0x27a6a4,Equal,0x27d74f,0x27a6aa) { return Ok(Next::SharedFrameBodyAt27d74f); }
            return Ok(Next::Actual211800CallAt27a6aa);
        }
        ins!(0x27a5e4,CmpR8d2);
        if branch!(0x27a5e8,NotEqual,0x27a728,0x27a5ee) { return Ok(Next::SourceOtherAt27a728); }
        ins!(0x27a5ee,CmpEdx1);
        if branch!(0x27a5f1,Equal,0x27b56f,0x27a5f7) { return Ok(Next::Source2Destination1At27b56f); }
        ins!(0x27a5f7,CmpEdx2);
        if branch!(0x27a5fa,NotEqual,0x27c17e,0x27a600) { return Ok(Next::Source2DestinationOtherAt27c17e); }
        ins!(0x27a600,LeaRdxWork40); ins!(0x27a605,TestAl);
        if branch!(0x27a607,Equal,0x27d14f,0x27a60d) { return Ok(Next::SharedFrameBodyAt27d14f); }
        Ok(Next::Actual211800CallAt27a60d)
    })();
    match result {
        Ok(next) => { p.next=Some(next); Ok(Required { frame,progress:p,next }) },
        Err(error) => Err(Failure { frame,progress:p,error }),
    }
}
