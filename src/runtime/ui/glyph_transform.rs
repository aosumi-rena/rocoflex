use super::canvas_primitives::{CompareFlags, F32Operation};

#[derive(Clone, Copy, Debug)]
pub struct Bindings {
    pub path_descriptor: u64,
    pub affine: u64,

    pub marker_addresses: [u64; 4],
}

pub trait Services {
    type Error;
    fn read_u32(&mut self, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn read_u64(&mut self, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn write_u32(&mut self, pc: u32, address: u64, bits: u32) -> Result<(), Self::Error>;

    fn clear_16(&mut self, pc: u32, address: u64) -> Result<(), Self::Error>;
    fn f32(&mut self, pc: u32, operation: F32Operation) -> Result<u32, Self::Error>;
    fn ucomiss(&mut self, pc: u32, left: u32, right: u32)
        -> Result<CompareFlags, Self::Error>;
}

#[derive(Clone, Copy, Debug)]
pub enum Request {
    Read32 { pc: u32, address: u64 },
    Read64 { pc: u32, address: u64 },
    Write32 { pc: u32, address: u64, bits: u32 },
    Clear16 { pc: u32, address: u64 },
    F32 { pc: u32, operation: F32Operation },
    Ucomiss { pc: u32, left: u32, right: u32 },
}

#[derive(Clone, Copy, Debug)]
pub enum Effect {
    Bits32(u32),
    Bits64(u64),
    Stored,
    Flags(CompareFlags),
}

#[derive(Clone, Copy, Debug)]
pub struct CompletedEffect { pub request: Request, pub effect: Effect }

#[derive(Clone, Copy, Debug)]
pub struct Branch {
    pub pc: u32,
    pub taken: bool,
    pub target: u32,
    pub flags: Option<CompareFlags>,
}

#[derive(Debug)]
pub struct Progress {
    pub pc: u32,
    pub bindings: Bindings,
    pub signed_count: Option<i32>,
    pub word_base: Option<u64>,
    pub cursor: Option<u64>,
    pub end: Option<u64>,
    pub first_flag: Option<bool>,
    pub selected_store: Option<u64>,
    pub xmm: [Option<u32>; 12],
    pub flags: Option<CompareFlags>,

    pub request: Option<Request>,
    pub effects: Vec<CompletedEffect>,
    pub branches: Vec<Branch>,
}

#[derive(Debug)]
pub enum Cause<E> {
    Service(E),

    MissingRegister(u8),
    MissingCursor,
}

pub struct Failure<'a, S: Services> {
    pub services: &'a mut S,
    pub progress: Progress,
    pub cause: Cause<S::Error>,
}

pub struct Completed<'a, S: Services> {
    services: &'a mut S,
    progress: Progress,
}

impl<'a, S: Services> Completed<'a, S> {
    pub fn progress(&self) -> &Progress { &self.progress }
    pub fn into_parts(self) -> (&'a mut S, Progress) { (self.services, self.progress) }
}

pub fn transform_with_services<'a, S: Services>(
    services: &'a mut S,
    bindings: Bindings,
) -> Result<Completed<'a, S>, Failure<'a, S>> {
    let mut run = Run {
        services,
        progress: Progress {
            pc: 0x1fa1fa, bindings, signed_count: None, word_base: None,
            cursor: None, end: None, first_flag: None, selected_store: None,
            xmm: [None; 12], flags: None, request: None,
            effects: Vec::new(), branches: Vec::new(),
        },
    };
    match run.execute() {
        Ok(()) => Ok(Completed { services: run.services, progress: run.progress }),
        Err(cause) => Err(Failure { services: run.services, progress: run.progress, cause }),
    }
}

struct Run<'a, S: Services> { services: &'a mut S, progress: Progress }

#[derive(Clone, Copy)]
enum Operand { Reg(u8), Affine(u8) }
#[derive(Clone, Copy)]
enum Step {
    Load(u32, u8, u8),
    AffineLoad(u32, u8, u8),
    Store(u32, u8, u8),
    Mul(u32, u8, Operand),
    Add(u32, u8, Operand),
}
use Operand::{Affine as A, Reg as R};
use Step::{Add, AffineLoad, Load, Mul, Store};

const MOVE: &[Step] = &[
    Load(0x1fa287,4,4), Load(0x1fa28d,6,8),
    AffineLoad(0x1fa293,7,4), Mul(0x1fa298,7,R(6)),
    AffineLoad(0x1fa29c,5,0), Mul(0x1fa2a0,5,R(4)),
    Add(0x1fa2a4,5,R(7)), Add(0x1fa2a8,5,A(8)), Store(0x1fa2ad,5,4),
    Mul(0x1fa2b3,6,A(16)), Mul(0x1fa2b8,4,A(12)),
    Add(0x1fa2bd,4,R(6)), Add(0x1fa2c1,4,A(20)), Store(0x1fa2c6,4,8),
];
const LINE: &[Step] = &[
    Load(0x1fa31f,4,4), Load(0x1fa325,6,8),
    AffineLoad(0x1fa32b,7,4), Mul(0x1fa330,7,R(6)),
    AffineLoad(0x1fa334,5,0), Mul(0x1fa338,5,R(4)),
    Add(0x1fa33c,5,R(7)), Add(0x1fa340,5,A(8)), Store(0x1fa345,5,4),
    Mul(0x1fa34b,6,A(16)), Mul(0x1fa350,4,A(12)),
    Add(0x1fa355,4,R(6)), Add(0x1fa359,4,A(20)), Store(0x1fa35e,4,8),
];
const QUADRATIC: &[Step] = &[
    Load(0x1fa3ae,4,12), Load(0x1fa3b4,5,4), Load(0x1fa3ba,6,8),
    AffineLoad(0x1fa3c0,8,4), Mul(0x1fa3c6,8,R(6)),
    AffineLoad(0x1fa3cb,7,0), Mul(0x1fa3cf,7,R(5)),
    Add(0x1fa3d3,7,R(8)), Add(0x1fa3d8,7,A(8)), Store(0x1fa3dd,7,4),
    Mul(0x1fa3e3,6,A(16)), Mul(0x1fa3e8,5,A(12)),
    Add(0x1fa3ed,5,R(6)), Add(0x1fa3f1,5,A(20)), Store(0x1fa3f6,5,8),
    Load(0x1fa3fc,8,16), AffineLoad(0x1fa402,9,4), Mul(0x1fa408,9,R(8)),
    AffineLoad(0x1fa40d,6,0), Mul(0x1fa411,6,R(4)),
    Add(0x1fa415,6,R(9)), Add(0x1fa41a,6,A(8)), Store(0x1fa41f,6,12),
    Mul(0x1fa425,8,A(16)), Mul(0x1fa42b,4,A(12)),
    Add(0x1fa430,4,R(8)), Add(0x1fa435,4,A(20)), Store(0x1fa43a,4,16),
];
const CUBIC: &[Step] = &[
    Load(0x1fa4be,5,12), Load(0x1fa4c4,4,20),
    Load(0x1fa4ca,6,4), Load(0x1fa4d0,7,8),
    AffineLoad(0x1fa4d6,8,4), Mul(0x1fa4dc,8,R(7)),
    AffineLoad(0x1fa4e1,9,0), Mul(0x1fa4e6,9,R(6)),
    Add(0x1fa4eb,9,R(8)), Add(0x1fa4f0,9,A(8)), Store(0x1fa4f6,9,4),
    Mul(0x1fa4fc,7,A(16)), Mul(0x1fa501,6,A(12)),
    Add(0x1fa506,6,R(7)), Add(0x1fa50a,6,A(20)), Store(0x1fa50f,6,8),
    Load(0x1fa515,7,16), AffineLoad(0x1fa51b,10,4), Mul(0x1fa521,10,R(7)),
    AffineLoad(0x1fa526,8,0), Mul(0x1fa52b,8,R(5)),
    Add(0x1fa530,8,R(10)), Add(0x1fa535,8,A(8)), Store(0x1fa53b,8,12),
    Mul(0x1fa541,7,A(16)), Mul(0x1fa546,5,A(12)),
    Add(0x1fa54b,5,R(7)), Add(0x1fa54f,5,A(20)), Store(0x1fa554,5,16),
    Load(0x1fa55a,10,24), AffineLoad(0x1fa560,11,4), Mul(0x1fa566,11,R(10)),
    AffineLoad(0x1fa56b,7,0), Mul(0x1fa56f,7,R(4)),
    Add(0x1fa573,7,R(11)), Add(0x1fa578,7,A(8)), Store(0x1fa57d,7,20),
    Mul(0x1fa583,10,A(16)), Mul(0x1fa589,4,A(12)),
    Add(0x1fa58e,4,R(10)), Add(0x1fa593,4,A(20)), Store(0x1fa598,4,24),
];

#[derive(Clone, Copy)]
struct Bound { value: u8, temp: u8, lower: u8, upper: u8, pc: [u32; 8], skip: u32 }

impl<'a, S: Services> Run<'a, S> {
    fn begin(&mut self, pc: u32, request: Request) {
        self.progress.pc = pc;
        self.progress.request = Some(request);
    }
    fn done(&mut self, effect: Effect) {
        if let Some(request) = self.progress.request {
            self.progress.effects.push(CompletedEffect { request, effect });
        }
    }
    fn read32(&mut self, pc: u32, address: u64) -> Result<u32, Cause<S::Error>> {
        self.begin(pc, Request::Read32 { pc, address });
        let bits = self.services.read_u32(pc, address).map_err(Cause::Service)?;
        self.done(Effect::Bits32(bits));
        Ok(bits)
    }
    fn read64(&mut self, pc: u32, address: u64) -> Result<u64, Cause<S::Error>> {
        self.begin(pc, Request::Read64 { pc, address });
        let bits = self.services.read_u64(pc, address).map_err(Cause::Service)?;
        self.done(Effect::Bits64(bits));
        Ok(bits)
    }
    fn write(&mut self, pc: u32, address: u64, bits: u32) -> Result<(), Cause<S::Error>> {
        self.begin(pc, Request::Write32 { pc, address, bits });
        self.services.write_u32(pc, address, bits).map_err(Cause::Service)?;
        self.done(Effect::Stored);
        Ok(())
    }
    fn reg(&self, reg: u8) -> Result<u32, Cause<S::Error>> {
        self.progress.xmm[reg as usize].ok_or(Cause::MissingRegister(reg))
    }
    fn cursor(&self) -> Result<u64, Cause<S::Error>> {
        self.progress.cursor.ok_or(Cause::MissingCursor)
    }
    fn path(&self, offset: u8) -> u64 {
        self.progress.bindings.path_descriptor.wrapping_add(offset as u64)
    }
    fn affine_load(&mut self, pc: u32, reg: u8, offset: u8) -> Result<(), Cause<S::Error>> {
        let address = self.progress.bindings.affine.wrapping_add(offset as u64);
        let bits = self.read32(pc, address)?;
        self.progress.xmm[reg as usize] = Some(bits);
        Ok(())
    }
    fn compare(&mut self, pc: u32, left: u32, right: u32)
        -> Result<CompareFlags, Cause<S::Error>> {
        self.begin(pc, Request::Ucomiss { pc, left, right });
        let flags = self.services.ucomiss(pc, left, right).map_err(Cause::Service)?;
        self.progress.flags = Some(flags);
        self.done(Effect::Flags(flags));
        Ok(flags)
    }
    fn branch(&mut self, pc: u32, taken: bool, target: u32, fp: bool) {
        self.progress.pc = pc;
        self.progress.branches.push(Branch {
            pc, taken, target, flags: if fp { self.progress.flags } else { None },
        });
    }
    fn select(&mut self, pc: u32, offset: u8) {
        self.progress.pc = pc;
        self.progress.selected_store = Some(self.path(offset));
    }
    fn advance(&mut self, pc: u32, bytes: u64, jump: Option<u32>) -> Result<(), Cause<S::Error>> {
        self.progress.pc = pc;
        self.progress.cursor = Some(self.cursor()?.wrapping_add(bytes));
        if let Some(jump_pc) = jump { self.branch(jump_pc, true, 0x1fa26a, false); }
        Ok(())
    }
    fn plan(&mut self, steps: &[Step]) -> Result<(), Cause<S::Error>> {
        for &step in steps {
            match step {
                AffineLoad(pc, reg, offset) => self.affine_load(pc,reg,offset)?,
                Load(pc, reg, offset) => {
                    let address = self.cursor()?.wrapping_add(offset as u64);
                    let bits = self.read32(pc, address)?;
                    self.progress.xmm[reg as usize] = Some(bits);
                }
                Store(pc, reg, offset) => {
                    let bits = self.reg(reg)?;
                    self.write(pc, self.cursor()?.wrapping_add(offset as u64), bits)?;
                }
                Mul(pc, dst, source) | Add(pc, dst, source) => {
                    let left = self.reg(dst)?;
                    let right = match source {
                        R(reg) => self.reg(reg)?,
                        A(offset) => {
                            let address = self.progress.bindings.affine.wrapping_add(offset as u64);
                            self.read32(pc, address)?
                        }
                    };
                    let operation = match step {
                        Mul(..) => F32Operation::Mul { destination: left, source: right },
                        _ => F32Operation::Add { destination: left, source: right },
                    };
                    self.begin(pc, Request::F32 { pc, operation });
                    let bits = self.services.f32(pc, operation).map_err(Cause::Service)?;
                    self.progress.xmm[dst as usize] = Some(bits);
                    self.done(Effect::Bits32(bits));
                }
            }
        }
        Ok(())
    }
    fn bound(&mut self, b: Bound) -> Result<(), Cause<S::Error>> {
        let [load, cmp, select, ja, upper_cmp, upper_select, jbe, store] = b.pc;
        let bits = self.read32(load, self.path(b.lower))?;
        self.progress.xmm[b.temp as usize] = Some(bits);
        let value = self.reg(b.value)?;
        let flags = self.compare(cmp, bits, value)?;
        self.select(select, b.lower);
        let above = !flags.carry && !flags.zero;
        self.branch(ja, above, store, true);
        if above { return self.write(store, self.path(b.lower), value); }
        let upper = self.read32(upper_cmp, self.path(b.upper))?;
        let flags = self.compare(upper_cmp, value, upper)?;
        self.select(upper_select, b.upper);
        let skip = flags.carry || flags.zero;
        self.branch(jbe, skip, b.skip, true);
        if !skip { self.write(store, self.path(b.upper), value)?; }
        Ok(())
    }
    fn marker(&mut self, reg: u8, cmp: u32, jne: u32, jp: u32, target: u32)
        -> Result<bool, Cause<S::Error>> {
        let flags = self.compare(cmp, self.reg(4)?, self.reg(reg)?)?;
        self.branch(jne, !flags.zero, target, true);
        if !flags.zero { return Ok(false); }
        self.branch(jp, flags.parity, target, true);
        Ok(!flags.parity)
    }
    fn single_y(&mut self, move_branch: bool) -> Result<(), Cause<S::Error>> {
        let (load,cmp,ja) = if move_branch { (0x1fa2ef,0x1fa2f4,0x1fa2f7) }
            else { (0x1fa37d,0x1fa382,0x1fa388) };
        let bits = self.read32(load, self.path(24))?;
        self.progress.xmm[5] = Some(bits);
        let value = self.reg(4)?;
        let flags = self.compare(cmp, bits, value)?;
        if !move_branch { self.select(0x1fa385,24); }
        let above = !flags.carry && !flags.zero;
        self.branch(ja, above, if move_branch { 0x1fa25f } else { 0x1fa262 }, true);
        if above {
            if move_branch { self.select(0x1fa25f,24); }
            self.write(0x1fa262,self.path(24),value)?;
        } else {
            if move_branch { self.branch(0x1fa2fd,true,0x1fa38e,false); }
            let upper = self.read32(0x1fa38e,self.path(28))?;
            let flags = self.compare(0x1fa38e,value,upper)?;
            self.select(0x1fa391,28);
            let above = !flags.carry && !flags.zero;
            self.branch(0x1fa394,above,0x1fa262,true);
            if above { self.write(0x1fa262,self.path(28),value)?; }
            else { self.branch(0x1fa39a,true,0x1fa266,false); }
        }
        self.advance(0x1fa266,12,None)
    }
    fn execute(&mut self) -> Result<(), Cause<S::Error>> {
        let address = self.path(16);
        self.begin(0x1fa1fa,Request::Clear16 { pc: 0x1fa1fa, address });
        self.services.clear_16(0x1fa1fa,address).map_err(Cause::Service)?;
        self.done(Effect::Stored);
        let count = self.read32(0x1fa1fe,self.path(12))? as i32;
        self.progress.signed_count = Some(count);
        self.branch(0x1fa205,count <= 0,0x1fa64c,false);
        if count <= 0 { self.progress.pc = 0x1fa64c; return Ok(()); }
        let base = self.read64(0x1fa20f,self.path(0))?;
        self.progress.word_base = Some(base);
        self.progress.cursor = Some(base);
        let end = base.wrapping_add((count as u64).wrapping_mul(4));
        self.progress.end = Some(end);
        self.progress.pc = 0x1fa222;
        self.progress.first_flag = Some(true);
        for (reg,pc) in [(0,0x1fa225),(1,0x1fa22d),(2,0x1fa235),(3,0x1fa23d)] {
            let bits = self.read32(pc,self.progress.bindings.marker_addresses[reg])?;
            self.progress.xmm[reg] = Some(bits);
        }
        self.branch(0x1fa245,true,0x1fa273,false);
        loop {
            let bits = self.read32(0x1fa273,self.cursor()?)?;
            self.progress.xmm[4] = Some(bits);
            if self.marker(0,0x1fa278,0x1fa27b,0x1fa281,0x1fa310)? {
                self.plan(MOVE)?;
                let first = self.progress.first_flag == Some(true);
                self.branch(0x1fa2d0,first,0x1fa250,false);
                if first {
                    let x = self.reg(5)?;
                    let y = self.reg(4)?;
                    self.write(0x1fa250,self.path(20),x)?;
                    self.write(0x1fa255,self.path(16),x)?;
                    self.write(0x1fa259,self.path(28),y)?;
                    self.progress.pc = 0x1fa25d;
                    self.progress.first_flag = Some(false);
                    self.select(0x1fa25f,24);
                    self.write(0x1fa262,self.path(24),y)?;
                    self.advance(0x1fa266,12,None)?;
                } else {
                    self.bound(Bound { value:5,temp:6,lower:16,upper:20,
                        pc:[0x1fa2d6,0x1fa2da,0x1fa2dd,0x1fa2e0,0x1fa2e2,0x1fa2e6,0x1fa2e9,0x1fa2eb],skip:0x1fa2ef })?;
                    self.single_y(true)?;
                }
            } else if self.marker(1,0x1fa310,0x1fa313,0x1fa319,0x1fa39f)? {
                self.plan(LINE)?;
                self.bound(Bound { value:5,temp:6,lower:16,upper:20,
                    pc:[0x1fa364,0x1fa368,0x1fa36b,0x1fa36e,0x1fa370,0x1fa374,0x1fa377,0x1fa379],skip:0x1fa37d })?;
                self.single_y(false)?;
            } else if self.marker(2,0x1fa39f,0x1fa3a2,0x1fa3a8,0x1fa4af)? {
                self.plan(QUADRATIC)?;
                for b in QUADRATIC_BOUNDS { self.bound(*b)?; }
                self.advance(0x1fa4a6,20,Some(0x1fa4aa))?;
            } else if self.marker(3,0x1fa4af,0x1fa4b2,0x1fa4b8,0x1fa643)? {
                self.plan(CUBIC)?;
                for b in CUBIC_BOUNDS { self.bound(*b)?; }
                self.advance(0x1fa63a,28,Some(0x1fa63e))?;
            } else {
                self.advance(0x1fa643,4,Some(0x1fa647))?;
            }
            let stop = self.cursor()? >= end;
            self.branch(0x1fa26d,stop,0x1fa64c,false);
            if stop { self.progress.pc = 0x1fa64c; return Ok(()); }
        }
    }
}

const QUADRATIC_BOUNDS: &[Bound] = &[
    Bound { value:7,temp:8,lower:16,upper:20,pc:[0x1fa440,0x1fa445,0x1fa449,0x1fa44c,0x1fa44e,0x1fa452,0x1fa455,0x1fa457],skip:0x1fa45b },
    Bound { value:5,temp:7,lower:24,upper:28,pc:[0x1fa45b,0x1fa460,0x1fa463,0x1fa466,0x1fa468,0x1fa46b,0x1fa46e,0x1fa470],skip:0x1fa474 },
    Bound { value:6,temp:5,lower:16,upper:20,pc:[0x1fa474,0x1fa478,0x1fa47b,0x1fa47e,0x1fa480,0x1fa484,0x1fa487,0x1fa489],skip:0x1fa48d },
    Bound { value:4,temp:5,lower:24,upper:28,pc:[0x1fa48d,0x1fa492,0x1fa495,0x1fa498,0x1fa49a,0x1fa49d,0x1fa4a0,0x1fa4a2],skip:0x1fa4a6 },
];
const CUBIC_BOUNDS: &[Bound] = &[
    Bound { value:9,temp:10,lower:16,upper:20,pc:[0x1fa59e,0x1fa5a3,0x1fa5a7,0x1fa5aa,0x1fa5ac,0x1fa5b0,0x1fa5b3,0x1fa5b5],skip:0x1fa5ba },
    Bound { value:6,temp:9,lower:24,upper:28,pc:[0x1fa5ba,0x1fa5bf,0x1fa5c3,0x1fa5c6,0x1fa5c8,0x1fa5cb,0x1fa5ce,0x1fa5d0],skip:0x1fa5d4 },
    Bound { value:8,temp:6,lower:16,upper:20,pc:[0x1fa5d4,0x1fa5d8,0x1fa5dc,0x1fa5df,0x1fa5e1,0x1fa5e5,0x1fa5e8,0x1fa5ea],skip:0x1fa5ef },
    Bound { value:5,temp:6,lower:24,upper:28,pc:[0x1fa5ef,0x1fa5f4,0x1fa5f7,0x1fa5fa,0x1fa5fc,0x1fa5ff,0x1fa602,0x1fa604],skip:0x1fa608 },
    Bound { value:7,temp:5,lower:16,upper:20,pc:[0x1fa608,0x1fa60c,0x1fa60f,0x1fa612,0x1fa614,0x1fa618,0x1fa61b,0x1fa61d],skip:0x1fa621 },
    Bound { value:4,temp:5,lower:24,upper:28,pc:[0x1fa621,0x1fa626,0x1fa629,0x1fa62c,0x1fa62e,0x1fa631,0x1fa634,0x1fa636],skip:0x1fa63a },
];
