#[path = "parameter_formatter_parse_entry.rs"]
pub mod native_entry;
#[path = "parameter_formatter_parse_return.rs"]
pub mod native_return;
pub trait Services {
    type Storage;
    type Error;
    fn read_word(&mut self, storage: &mut Self::Storage, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn write_word(&mut self, storage: &mut Self::Storage, pc: u32, address: u64, word: u64) -> Result<(), Self::Error>;

    fn call1f9100(&mut self, storage: &mut Self::Storage, request: ParseRequest,
        returned: &mut Option<ParseReturned>) -> Result<(), Self::Error>;
    fn xorps_xmm6(&mut self, storage: &mut Self::Storage, pc: u32) -> Result<(), Self::Error>;

    fn cvtsd2ss_xmm6(&mut self, storage: &mut Self::Storage, pc: u32, source: u64,
        returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
    fn call561c40(&mut self, storage: &mut Self::Storage, request: CookieRequest,
        returned: &mut Option<CookieReturned>) -> Result<(), Self::Error>;

    fn movaps_xmm0_xmm6(&mut self, storage: &mut Self::Storage, pc: u32, source: [u32; 4],
        returned: &mut Option<[u32; 4]>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct ActualBody { pub pc: u32, pub work_rsp: u64, pub input_rdx: u64 }
pub struct Input<S: Services> { pub services: S, pub storage: S::Storage, pub actual: ActualBody }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseRequest { pub pc: u32, pub callee: u32, pub rcx: u64, pub work_rsp: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseReturned { pub resume_pc: u32, pub work_rsp: u64, pub xmm0_f64_bits: u64, pub raw_rax: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CookieRequest { pub pc: u32, pub callee: u32, pub rcx: u64, pub work_rsp: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CookieReturned { pub resume_pc: u32, pub work_rsp: u64, pub raw_rax: u64, pub xmm6: [u32; 4] }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { ReadInputWord, StoreBorrowedWord, Parse, ClearXmm6, Narrow, ReadCookie, Cookie, MoveResult, NativeRestoreRequired }
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub native_entry: Option<native_entry::History>,
    pub input_word: Option<u64>, pub borrowed_store_returned: bool,
    pub parse_request: Option<ParseRequest>, pub parse_returned: Option<ParseReturned>,
    pub xmm6_clear_returned: bool, pub narrowed_xmm6: Option<[u32; 4]>, pub cookie_word: Option<u64>,
    pub cookie_request: Option<CookieRequest>, pub cookie_returned: Option<CookieReturned>, pub xmm0: Option<[u32; 4]>,
}
pub enum Fault<E> { Service(E), MissingState, ParseReturnBinding, NarrowUpperLanes, CookieReturnBinding, MoveBinding, AlreadyStopped }
pub struct Prepared<S: Services> { input: Input<S>, captures: Captures, phase: Phase, attempted: Option<Phase>, stopped: bool }
pub struct Rejected<S: Services> { pub input: Input<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(NativeRestoreRequired<S>), Failed(Stopped<S>) }
pub struct Parts<S: Services> { input: Input<S>, pub captures: Captures, pub phase: Phase, pub attempted: Option<Phase>, pub stopped: bool }
impl<S: Services> Parts<S> {
    pub fn input(&self) -> &Input<S> { &self.input }
}
pub struct NativeRestoreRequired<S: Services> { parts: Parts<S> }
impl<S: Services> NativeRestoreRequired<S> {
    pub fn required_next_pc(&self) -> u32 { 0x2b6f31 }

    pub fn actual_xmm0(&self) -> [u32; 4] { self.parts.captures.xmm0.unwrap() }
    pub fn into_parts(self) -> Parts<S> { self.parts }
}
impl<S: Services> Prepared<S> {

    pub fn from_actual_body(input: Input<S>) -> Result<Self, Rejected<S>> {
        if input.actual.pc != 0x2b6f08 { return Err(Rejected { input }); }
        Ok(Self { input, captures: Captures::default(), phase: Phase::ReadInputWord, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> { Parts { input: self.input, captures: self.captures,
        phase: self.phase, attempted: self.attempted, stopped: self.stopped } }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::NativeRestoreRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() { self.stopped = true; return Poll::Failed(Stopped { pending: self, fault }); }
            self.attempted = None;
        }
        if self.phase == Phase::NativeRestoreRequired { Poll::Ready(NativeRestoreRequired { parts: self.into_parts() }) }
        else { Poll::Pending(self) }
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        let s = &mut self.input.services; let o = &mut self.input.storage; let a = self.input.actual; let c = &mut self.captures;
        match self.phase {
            Phase::ReadInputWord => { c.input_word = Some(s.read_word(o, 0x2b6f08, a.input_rdx).map_err(Fault::Service)?); self.phase = Phase::StoreBorrowedWord; }
            Phase::StoreBorrowedWord => {
                s.write_word(o, 0x2b6f0b, a.work_rsp.wrapping_add(0x20), c.input_word.ok_or(Fault::MissingState)?).map_err(Fault::Service)?;
                c.borrowed_store_returned = true; self.phase = Phase::Parse;
            }
            Phase::Parse => {
                let request = ParseRequest { pc: 0x2b6f15, callee: 0x1f9100, rcx: a.work_rsp.wrapping_add(0x20), work_rsp: a.work_rsp };
                c.parse_request = Some(request); s.call1f9100(o, request, &mut c.parse_returned).map_err(Fault::Service)?;
                let actual = c.parse_returned.ok_or(Fault::MissingState)?;
                if actual.resume_pc != 0x2b6f1a || actual.work_rsp != a.work_rsp { return Err(Fault::ParseReturnBinding); }
                self.phase = Phase::ClearXmm6;
            }
            Phase::ClearXmm6 => { s.xorps_xmm6(o, 0x2b6f1a).map_err(Fault::Service)?; c.xmm6_clear_returned = true; self.phase = Phase::Narrow; }
            Phase::Narrow => {
                s.cvtsd2ss_xmm6(o, 0x2b6f1d, c.parse_returned.ok_or(Fault::MissingState)?.xmm0_f64_bits,
                    &mut c.narrowed_xmm6).map_err(Fault::Service)?;
                let actual = c.narrowed_xmm6.ok_or(Fault::MissingState)?;
                if actual[1..] != [0, 0, 0] { return Err(Fault::NarrowUpperLanes); }
                self.phase = Phase::ReadCookie;
            }
            Phase::ReadCookie => { c.cookie_word = Some(s.read_word(o, 0x2b6f21, a.work_rsp.wrapping_add(0x28)).map_err(Fault::Service)?); self.phase = Phase::Cookie; }
            Phase::Cookie => {
                let request = CookieRequest { pc: 0x2b6f29, callee: 0x561c40,
                    rcx: c.cookie_word.ok_or(Fault::MissingState)? ^ a.work_rsp, work_rsp: a.work_rsp };
                c.cookie_request = Some(request); s.call561c40(o, request, &mut c.cookie_returned).map_err(Fault::Service)?;
                let actual = c.cookie_returned.ok_or(Fault::MissingState)?;
                if actual.resume_pc != 0x2b6f2e || actual.work_rsp != a.work_rsp { return Err(Fault::CookieReturnBinding); }
                self.phase = Phase::MoveResult;
            }
            Phase::MoveResult => {
                let source = c.cookie_returned.ok_or(Fault::MissingState)?.xmm6;
                s.movaps_xmm0_xmm6(o, 0x2b6f2e, source, &mut c.xmm0).map_err(Fault::Service)?;
                if c.xmm0.ok_or(Fault::MissingState)? != source { return Err(Fault::MoveBinding); }
                self.phase = Phase::NativeRestoreRequired;
            }
            Phase::NativeRestoreRequired => {}
        }
        Ok(())
    }
}
