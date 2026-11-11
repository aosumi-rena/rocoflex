use super::canvas_primitives::{CompareFlags, F32Operation};

pub const ENTRY: u32 = 0x1ff430;
pub const OTHER_FLAGS_REQUIRED: u32 = 0x1ff4ab;

enum Math { Div, Mul, Sub }

pub struct Bindings<B> { flags: B, output: B, source: B, destination: B }
impl<B> Bindings<B> {

    pub fn from_explicit_arguments(flags: B, output: B, source: B, destination: B) -> Self {
        Self { flags, output, source, destination }
    }
    pub fn flags(&self) -> &B { &self.flags }
    pub fn output(&self) -> &B { &self.output }
    pub fn source(&self) -> &B { &self.source }
    pub fn destination(&self) -> &B { &self.destination }
}
pub trait Services {
    type Binding;
    type Error;
    fn read_dword(&mut self, binding: &Self::Binding, offset: u32) -> Result<u32, Self::Error>;
    fn write_dword(&mut self, binding: &Self::Binding, offset: u32, value: u32) -> Result<(), Self::Error>;

    fn f32_operation(&mut self, pc: u32, operation: F32Operation) -> Result<u32, Self::Error>;
    fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<CompareFlags, Self::Error>;
}
pub struct Captures {
    float: [Option<u32>; 9], comparisons: [Option<CompareFlags>; 2],
    flags: Option<u32>, output: [Option<u32>; 6],
}
impl Captures {
    pub fn low_float_registers(&self) -> &[Option<u32>; 9] { &self.float }
    pub fn comparisons(&self) -> &[Option<CompareFlags>; 2] { &self.comparisons }
    pub fn flags(&self) -> Option<u32> { self.flags }
    pub fn output_stores(&self) -> &[Option<u32>; 6] { &self.output }
}
pub struct Prepared<S: Services> { services: S, bindings: Bindings<S::Binding>, pc: u32, captures: Captures }
pub enum Fault<E> { Service(E), MissingFloat(usize), MissingComparison(usize), MissingFlags, UnexpectedPc(u32) }
pub struct Stopped<S: Services> { pending: Prepared<S>, fault: Fault<S::Error> }
impl<S: Services> Stopped<S> {
    pub fn pending(&self) -> &Prepared<S> { &self.pending }
    pub fn fault(&self) -> &Fault<S::Error> { &self.fault }
}
pub struct OtherFlagsRequired<S: Services> { pending: Prepared<S> }
impl<S: Services> OtherFlagsRequired<S> { pub fn pending(&self) -> &Prepared<S> { &self.pending } }
pub struct Completed<S: Services> { pending: Prepared<S> }
impl<S: Services> Completed<S> {
    pub fn pending(&self) -> &Prepared<S> { &self.pending }

    pub fn into_completed_parts(self) -> (S, Bindings<S::Binding>, Captures) {
        (self.pending.services, self.pending.bindings, self.pending.captures)
    }
}
pub enum Poll<S: Services> {
    Pending(Prepared<S>), Failed(Stopped<S>), OtherFlags(OtherFlagsRequired<S>), Complete(Completed<S>),
}
impl<S: Services> Prepared<S> {
    pub fn from_explicit_call(services: S, bindings: Bindings<S::Binding>) -> Self {
        Self { services, bindings, pc: 0x1ff446, captures: Captures {
            float: [None; 9], comparisons: [None; 2], flags: None, output: [None; 6],
        } }
    }
    pub fn services(&self) -> &S { &self.services }
    pub fn bindings(&self) -> &Bindings<S::Binding> { &self.bindings }
    pub fn next_pc(&self) -> u32 { self.pc }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        for _ in 0..budget {
            if self.pc == 0x1ff5a3 { return Poll::Complete(Completed { pending: self }); }
            if self.pc == OTHER_FLAGS_REQUIRED { return Poll::OtherFlags(OtherFlagsRequired { pending: self }); }
            if let Err(fault) = self.step() { return Poll::Failed(Stopped { pending: self, fault }); }
        }
        if self.pc == 0x1ff5a3 { Poll::Complete(Completed { pending: self }) }
        else if self.pc == OTHER_FLAGS_REQUIRED { Poll::OtherFlags(OtherFlagsRequired { pending: self }) }
        else { Poll::Pending(self) }
    }
    fn f(&self, i: usize) -> Result<u32, Fault<S::Error>> { self.captures.float[i].ok_or(Fault::MissingFloat(i)) }
    fn read_source(&mut self, f: usize, offset: u32, next: u32) -> Result<(), Fault<S::Error>> {
        self.captures.float[f] = Some(self.services.read_dword(&self.bindings.source, offset).map_err(Fault::Service)?);
        self.pc = next; Ok(())
    }
    fn read_destination(&mut self, f: usize, offset: u32, next: u32) -> Result<(), Fault<S::Error>> {
        self.captures.float[f] = Some(self.services.read_dword(&self.bindings.destination, offset).map_err(Fault::Service)?);
        self.pc = next; Ok(())
    }
    fn math(&mut self, to: usize, from: usize, next: u32, kind: Math) -> Result<(), Fault<S::Error>> {
        let destination = self.f(to)?; let source = self.f(from)?;
        let operation = match kind {
            Math::Div => F32Operation::Div { destination, source },
            Math::Mul => F32Operation::Mul { destination, source },
            Math::Sub => F32Operation::Sub { destination, source },
        };
        self.captures.float[to] = Some(self.services.f32_operation(self.pc, operation).map_err(Fault::Service)?);
        self.pc = next; Ok(())
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.pc {
            0x1ff446 => self.read_source(6, 8, 0x1ff44c)?,
            0x1ff44c => { self.captures.float[4] = Some(0); self.pc = 0x1ff44f; }
            0x1ff44f => {
                let left = self.f(4)?; let right = self.f(6)?;
                self.captures.comparisons[0] = Some(self.services.ucomiss(self.pc, left, right).map_err(Fault::Service)?);
                self.pc = 0x1ff452;
            }

            0x1ff452 => { self.captures.float[0] = Some(0x3f800000); self.pc = 0x1ff45a; }
            0x1ff45a => {
                let flags = self.captures.comparisons[0].ok_or(Fault::MissingComparison(0))?;
                self.pc = if !flags.carry { 0x1ff503 } else { 0x1ff460 };
            }
            0x1ff460 => self.read_source(3, 12, 0x1ff466)?,
            0x1ff466 => { self.captures.float[1] = Some(0); self.pc = 0x1ff469; }
            0x1ff469 => {
                let left = self.f(1)?; let right = self.f(3)?;
                self.captures.comparisons[1] = Some(self.services.ucomiss(self.pc, left, right).map_err(Fault::Service)?);
                self.pc = 0x1ff46c;
            }
            0x1ff46c => { self.captures.float[7] = Some(0); self.pc = 0x1ff46f; }
            0x1ff46f => { self.captures.float[5] = Some(self.f(0)?); self.pc = 0x1ff472; }
            0x1ff472 => { self.captures.float[2] = Some(0); self.pc = 0x1ff475; }
            0x1ff475 => {
                let flags = self.captures.comparisons[1].ok_or(Fault::MissingComparison(1))?;
                self.pc = if !flags.carry { 0x1ff586 } else { 0x1ff47b };
            }
            0x1ff47b => self.read_destination(1, 0, 0x1ff480)?,
            0x1ff480 => self.read_destination(2, 4, 0x1ff486)?,
            0x1ff486 => self.read_destination(7, 8, 0x1ff48c)?,
            0x1ff48c => { self.captures.float[0] = Some(self.f(7)?); self.pc = 0x1ff48f; }
            0x1ff48f => self.math(0, 6, 0x1ff493, Math::Div)?,
            0x1ff493 => self.read_destination(4, 12, 0x1ff499)?,
            0x1ff499 => { self.captures.float[5] = Some(self.f(4)?); self.pc = 0x1ff49c; }
            0x1ff49c => self.math(5, 3, 0x1ff4a0, Math::Div)?,
            0x1ff4a0 => {
                self.captures.flags = Some(self.services.read_dword(&self.bindings.flags, 0).map_err(Fault::Service)?);
                self.pc = 0x1ff4a2;
            }
            0x1ff4a2 => {
                self.pc = if self.captures.flags.ok_or(Fault::MissingFlags)? & 0x40 != 0 { 0x1ff55d } else { OTHER_FLAGS_REQUIRED };
            }
            0x1ff503 => { self.captures.float[1] = Some(0); self.pc = 0x1ff506; }
            0x1ff506 => { self.captures.float[7] = Some(0); self.pc = 0x1ff509; }
            0x1ff509 => { self.captures.float[5] = Some(self.f(0)?); self.pc = 0x1ff50c; }
            0x1ff50c => { self.captures.float[2] = Some(0); self.pc = 0x1ff586; }
            0x1ff55d => { self.captures.float[7] = Some(0); self.pc = 0x1ff560; }
            0x1ff560 => { self.captures.float[4] = Some(self.f(0)?); self.pc = 0x1ff563; }
            0x1ff563 => self.math(4, 7, 0x1ff567, Math::Mul)?,
            0x1ff567 => self.math(7, 5, 0x1ff56b, Math::Mul)?,
            0x1ff56b => self.read_source(3, 0, 0x1ff570)?,
            0x1ff570 => self.math(3, 0, 0x1ff574, Math::Mul)?,
            0x1ff574 => self.math(1, 3, 0x1ff578, Math::Sub)?,
            0x1ff578 => self.read_source(3, 4, 0x1ff57e)?,
            0x1ff57e => self.math(3, 5, 0x1ff582, Math::Mul)?,
            0x1ff582 => self.math(2, 3, 0x1ff586, Math::Sub)?,
            pc @ (0x1ff586 | 0x1ff58a | 0x1ff58f | 0x1ff594 | 0x1ff599 | 0x1ff59e) => {
                let (index, register, next) = match pc {
                    0x1ff586 => (0, 0, 0x1ff58a), 0x1ff58a => (1, 4, 0x1ff58f),
                    0x1ff58f => (2, 1, 0x1ff594), 0x1ff594 => (3, 7, 0x1ff599),
                    0x1ff599 => (4, 5, 0x1ff59e), _ => (5, 2, 0x1ff5a3),
                };
                let bits = self.f(register)?;
                self.services.write_dword(&self.bindings.output, index as u32 * 4, bits).map_err(Fault::Service)?;
                self.captures.output[index] = Some(bits); self.pc = next;
            }
            pc => return Err(Fault::UnexpectedPc(pc)),
        }
        Ok(())
    }
}

pub mod values {
    use super::{Bindings, CompareFlags, F32Operation, Prepared, Services};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Region { Flags, Source, Destination, Output }
    pub trait Fp {
        type Error;
        fn operation(&mut self, pc: u32, operation: F32Operation) -> Result<u32, Self::Error>;
        fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<CompareFlags, Self::Error>;
    }
    impl<F: Fp + ?Sized> Fp for &mut F {
        type Error = F::Error;
        fn operation(&mut self, pc: u32, operation: F32Operation) -> Result<u32, Self::Error> {
            (**self).operation(pc, operation)
        }
        fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<CompareFlags, Self::Error> {
            (**self).ucomiss(pc, left, right)
        }
    }
    pub enum Error<E> {
        Arithmetic(E), Range { region: Region, offset: u32 },
        UnwrittenOutput(usize), ReadOnly { region: Region, offset: u32 },
    }
    pub struct Frame<F: Fp> {
        fp: F, flags: u32, source: [u32; 4], destination: [u32; 4], output: [Option<u32>; 6],
    }
    impl<F: Fp> Frame<F> {
        pub fn fp(&self) -> &F { &self.fp }
        pub fn flags(&self) -> u32 { self.flags }
        pub fn source(&self) -> &[u32; 4] { &self.source }
        pub fn destination(&self) -> &[u32; 4] { &self.destination }
        pub fn output_stores(&self) -> &[Option<u32>; 6] { &self.output }
        pub fn into_owned_parts(self) -> (F, u32, [u32; 4], [u32; 4], [Option<u32>; 6]) {
            (self.fp, self.flags, self.source, self.destination, self.output)
        }
    }
    pub fn prepare<F: Fp>(fp: F, flags: u32, source: [u32; 4], destination: [u32; 4]) -> Prepared<Frame<F>> {
        Prepared::from_explicit_call(Frame { fp, flags, source, destination, output: [None; 6] },
            Bindings::from_explicit_arguments(Region::Flags, Region::Output, Region::Source, Region::Destination))
    }
    impl<F: Fp> Services for Frame<F> {
        type Binding = Region;
        type Error = Error<F::Error>;
        fn read_dword(&mut self, region: &Region, offset: u32) -> Result<u32, Self::Error> {
            if offset & 3 != 0 { return Err(Error::Range { region: *region, offset }); }
            let i = (offset / 4) as usize;
            match *region {
                Region::Flags if offset == 0 => Ok(self.flags),
                Region::Source => self.source.get(i).copied().ok_or(Error::Range { region: *region, offset }),
                Region::Destination => self.destination.get(i).copied().ok_or(Error::Range { region: *region, offset }),
                Region::Output => self.output.get(i).ok_or(Error::Range { region: *region, offset })?
                    .ok_or(Error::UnwrittenOutput(i)),
                _ => Err(Error::Range { region: *region, offset }),
            }
        }
        fn write_dword(&mut self, region: &Region, offset: u32, bits: u32) -> Result<(), Self::Error> {
            if *region != Region::Output { return Err(Error::ReadOnly { region: *region, offset }); }
            if offset & 3 != 0 { return Err(Error::Range { region: *region, offset }); }
            let slot = self.output.get_mut((offset / 4) as usize).ok_or(Error::Range { region: *region, offset })?;
            *slot = Some(bits); Ok(())
        }
        fn f32_operation(&mut self, pc: u32, operation: F32Operation) -> Result<u32, Self::Error> {
            self.fp.operation(pc, operation).map_err(Error::Arithmetic)
        }
        fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<CompareFlags, Self::Error> {
            self.fp.ucomiss(pc, left, right).map_err(Error::Arithmetic)
        }
    }
}
