pub use super::canvas_primitives::{CompareFlags, F32Operation};

const ONE: u32 = 0x3f80_0000;
const HALF: u32 = 0x3f00_0000;
const NEGATIVE_HALF: u32 = 0xbf00_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    pub bounds: [u32; 4],
    pub target: [u32; 4],
    pub preserve: u8,
    pub justification: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Arithmetic { pc: u32, operation: F32Operation },
    Ucomiss { pc: u32, left: u32, right: u32 },
    Cmpltss { pc: u32, left: u32, right: u32 },
    WriteWord { pc: u32, index: u8, word: u32 },
}

pub trait Services {
    type Error;
    fn arithmetic(&mut self, pc: u32, operation: F32Operation) -> Result<u32, Self::Error>;
    fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<CompareFlags, Self::Error>;

    fn cmpltss(&mut self, pc: u32, left: u32, right: u32) -> Result<u32, Self::Error>;

    fn write_word(&mut self, pc: u32, index: u8, word: u32) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logic { And, AndNot, Or }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchInput {
    Preserve { byte: u8 },
    CarryClear { flags_pc: u32, flags: CompareFlags },
    Above { flags_pc: u32, flags: CompareFlags },
    TestNonzero { test_pc: u32, byte: u8, mask: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    InputWord { register: u8, word: u32 },
    Constant { register: u8, rva: u32, word: u32 },
    Copy { destination: u8, source: u8, word: u32 },
    Zero { register: u8 },
    Arithmetic { destination: u8, operation: F32Operation },
    Ucomiss { left: u32, right: u32 },
    Cmpltss { destination: u8, left: u32, right: u32 },
    Logic { destination: u8, kind: Logic, left: u32, right: u32 },
    PreserveTest { byte: u8 },
    JustificationLoad { word: u32 },
    Test { byte: u8, mask: u8 },
    Branch { input: BranchInput, target: u32 },
    Jump { target: u32 },
    WriteWord { index: u8, word: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value { Word(u32), Flags(CompareFlags), Decision(bool), Unit }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step { pub pc: u32, pub operation: Operation, pub result: Value }

#[derive(Debug)]
pub struct Progress {
    pub input: Input,

    pub registers: [Option<u32>; 14],
    pub words: [Option<u32>; 6],
    pub steps: Vec<Step>,
    pub attempted: Option<Request>,
    pub loaded_justification: Option<u32>,
    pub comparison_754: Option<CompareFlags>,
}

#[derive(Debug)]
pub enum Cause<E> {
    Service(E),
    MissingRegister { pc: u32, register: u8 },
    MissingJustification { pc: u32 },
}

#[derive(Debug)]
pub struct Failure<E> {
    pub cause: Cause<E>,
    pub attempted: Option<Request>,
    pub progress: Progress,
}

#[derive(Debug)]
pub struct Completed { pub input: Input, pub words: [u32; 6], pub progress: Progress }

#[derive(Clone, Copy)]
enum Binary { Add, Sub, Mul, Div }

struct Graph<'a, S: Services> { services: &'a mut S, progress: Progress }

impl<S: Services> Graph<'_, S> {
    fn step(&mut self, pc: u32, operation: Operation, result: Value) {
        self.progress.steps.push(Step { pc, operation, result });
    }
    fn word(&self, register: u8, pc: u32) -> Result<u32, Cause<S::Error>> {
        self.progress.registers[register as usize]
            .ok_or(Cause::MissingRegister { pc, register })
    }
    fn set(&mut self, pc: u32, register: u8, word: u32, operation: Operation) {
        self.progress.registers[register as usize] = Some(word);
        self.step(pc, operation, Value::Word(word));
    }
    fn input(&mut self, pc: u32, register: u8, word: u32) {
        self.set(pc, register, word, Operation::InputWord { register, word });
    }
    fn constant(&mut self, pc: u32, register: u8, rva: u32, word: u32) {
        self.set(pc, register, word, Operation::Constant { register, rva, word });
    }
    fn zero(&mut self, pc: u32, register: u8) {
        self.set(pc, register, 0, Operation::Zero { register });
    }
    fn copy(&mut self, pc: u32, destination: u8, source: u8) -> Result<(), Cause<S::Error>> {
        let word = self.word(source, pc)?;
        self.set(pc, destination, word, Operation::Copy { destination, source, word });
        Ok(())
    }
    fn binary(&mut self, pc: u32, destination: u8, source: u32, kind: Binary)
        -> Result<(), Cause<S::Error>>
    {
        let left = self.word(destination, pc)?;
        let operation = match kind {
            Binary::Add => F32Operation::Add { destination: left, source },
            Binary::Sub => F32Operation::Sub { destination: left, source },
            Binary::Mul => F32Operation::Mul { destination: left, source },
            Binary::Div => F32Operation::Div { destination: left, source },
        };
        self.progress.attempted = Some(Request::Arithmetic { pc, operation });
        let word = self.services.arithmetic(pc, operation).map_err(Cause::Service)?;
        self.progress.attempted = None;
        self.set(pc, destination, word, Operation::Arithmetic { destination, operation });
        Ok(())
    }
    fn rr(&mut self, pc: u32, destination: u8, source: u8, kind: Binary)
        -> Result<(), Cause<S::Error>>
    {
        let word = self.word(source, pc)?;
        self.binary(pc, destination, word, kind)
    }
    fn compare(&mut self, pc: u32, left: u8, right: u8)
        -> Result<CompareFlags, Cause<S::Error>>
    {
        let left = self.word(left, pc)?;
        let right = self.word(right, pc)?;
        self.progress.attempted = Some(Request::Ucomiss { pc, left, right });
        let flags = self.services.ucomiss(pc, left, right).map_err(Cause::Service)?;
        self.progress.attempted = None;
        self.step(pc, Operation::Ucomiss { left, right }, Value::Flags(flags));
        if pc == 0x1fa754 { self.progress.comparison_754 = Some(flags); }
        Ok(flags)
    }
    fn mask(&mut self, pc: u32, destination: u8, source: u8) -> Result<(), Cause<S::Error>> {
        let left = self.word(destination, pc)?;
        let right = self.word(source, pc)?;
        self.progress.attempted = Some(Request::Cmpltss { pc, left, right });
        let word = self.services.cmpltss(pc, left, right).map_err(Cause::Service)?;
        self.progress.attempted = None;
        self.set(pc, destination, word, Operation::Cmpltss { destination, left, right });
        Ok(())
    }
    fn logic(&mut self, pc: u32, destination: u8, source: u8, kind: Logic)
        -> Result<(), Cause<S::Error>>
    {
        let left = self.word(destination, pc)?;
        let right = self.word(source, pc)?;
        let word = match kind { Logic::And => left & right, Logic::AndNot => !left & right,
            Logic::Or => left | right };
        self.set(pc, destination, word, Operation::Logic { destination, kind, left, right });
        Ok(())
    }
    fn branch(&mut self, pc: u32, target: u32, input: BranchInput, taken: bool) -> bool {
        self.step(pc, Operation::Branch { input, target }, Value::Decision(taken));
        taken
    }
    fn jump(&mut self, pc: u32, target: u32) {
        self.step(pc, Operation::Jump { target }, Value::Unit);
    }
    fn test(&mut self, test_pc: u32, jump_pc: u32, mask: u8, target: u32)
        -> Result<bool, Cause<S::Error>>
    {
        let byte = self.progress.loaded_justification
            .ok_or(Cause::MissingJustification { pc: test_pc })? as u8;
        let taken = byte & mask != 0;
        self.step(test_pc, Operation::Test { byte, mask }, Value::Decision(taken));
        Ok(self.branch(jump_pc, target, BranchInput::TestNonzero { test_pc, byte, mask }, taken))
    }
    fn guard_output(&mut self) -> Result<(), Cause<S::Error>> {
        self.zero(0x1fa7d4, 12);
        self.copy(0x1fa7d8, 9, 0)?;
        self.zero(0x1fa7dc, 13);
        self.jump(0x1fa7e0, 0x1fa88d);
        Ok(())
    }
    fn independent(&mut self) -> Result<(), Cause<S::Error>> {
        use Binary::*;
        self.rr(0x1fa795, 1, 5, Div)?;
        self.rr(0x1fa799, 4, 8, Div)?;
        self.zero(0x1fa79e, 12);
        self.copy(0x1fa7a2, 10, 4)?;
        self.rr(0x1fa7a6, 10, 12, Mul)?;
        self.rr(0x1fa7ab, 12, 1, Mul)?;
        self.rr(0x1fa7b0, 6, 4, Mul)?;
        self.rr(0x1fa7b4, 2, 6, Sub)?;
        self.rr(0x1fa7b8, 7, 1, Mul)?;
        self.rr(0x1fa7bc, 3, 7, Sub)?;
        self.copy(0x1fa7c0, 0, 4)?;
        self.copy(0x1fa7c3, 11, 2)?;
        self.copy(0x1fa7c7, 9, 1)?;
        self.copy(0x1fa7cb, 13, 3)?;
        self.jump(0x1fa7cf, 0x1fa88d);
        Ok(())
    }
    fn preserving(&mut self) -> Result<(), Cause<S::Error>> {
        use Binary::*;
        self.zero(0x1fa6f5, 11);
        let flags = self.compare(0x1fa6f9, 11, 4)?;
        self.constant(0x1fa6fd, 0, 0x5dd17c, ONE);
        self.zero(0x1fa705, 10);
        if self.branch(0x1fa709, 0x1fa7d4,
            BranchInput::CarryClear { flags_pc: 0x1fa6f9, flags }, !flags.carry)
        { return self.guard_output(); }
        let flags = self.compare(0x1fa70f, 10, 1)?;
        if self.branch(0x1fa713, 0x1fa7d4,
            BranchInput::CarryClear { flags_pc: 0x1fa70f, flags }, !flags.carry)
        { return self.guard_output(); }
        let flags = self.compare(0x1fa719, 10, 8)?;
        if self.branch(0x1fa71d, 0x1fa7d4,
            BranchInput::CarryClear { flags_pc: 0x1fa719, flags }, !flags.carry)
        { return self.guard_output(); }
        self.zero(0x1fa723, 9);
        let flags = self.compare(0x1fa727, 9, 5)?;
        self.zero(0x1fa72b, 12);
        self.copy(0x1fa72f, 9, 0)?;
        self.zero(0x1fa733, 13);
        if self.branch(0x1fa737, 0x1fa88d,
            BranchInput::CarryClear { flags_pc: 0x1fa727, flags }, !flags.carry)
        { return Ok(()); }
        let word = self.progress.input.justification;
        self.progress.loaded_justification = Some(word);
        self.step(0x1fa73d, Operation::JustificationLoad { word }, Value::Word(word));
        self.copy(0x1fa744, 10, 5)?;
        self.rr(0x1fa748, 10, 8, Div)?;
        self.copy(0x1fa74d, 0, 1)?;
        self.rr(0x1fa750, 0, 4, Div)?;
        let flags_754 = self.compare(0x1fa754, 10, 0)?;
        self.copy(0x1fa758, 9, 1)?;
        self.rr(0x1fa75c, 9, 10, Div)?;
        self.mask(0x1fa761, 0, 10)?;
        self.logic(0x1fa767, 9, 0, Logic::And)?;
        self.logic(0x1fa76b, 0, 4, Logic::AndNot)?;
        self.logic(0x1fa76e, 0, 9, Logic::Or)?;
        self.copy(0x1fa772, 9, 1)?;

        if !self.branch(0x1fa776, 0x1fa781,
            BranchInput::Above { flags_pc: 0x1fa754, flags: flags_754 },
            !flags_754.carry && !flags_754.zero)
        {
            self.rr(0x1fa778, 10, 4, Mul)?;
            self.copy(0x1fa77d, 9, 10)?;
        }
        if self.test(0x1fa781, 0x1fa784, 1, 0x1fa7e5)? {
            self.constant(0x1fa7e5, 4, 0x5de098, HALF);
            self.rr(0x1fa7ed, 4, 0, Mul)?;
            self.jump(0x1fa7f1, 0x1fa806);
        } else if self.test(0x1fa786, 0x1fa789, 2, 0x1fa7f3)? {
            self.constant(0x1fa7f3, 10, 0x5def70, NEGATIVE_HALF);
            self.rr(0x1fa7fc, 10, 0, Mul)?;
            self.rr(0x1fa801, 4, 10, Add)?;
        } else {
            self.binary(0x1fa78b, 4, HALF, Mul)?;
            self.jump(0x1fa793, 0x1fa806);
        }
        self.rr(0x1fa806, 4, 2, Add)?;
        if self.test(0x1fa80a, 0x1fa80d, 8, 0x1fa81e)? {
            self.constant(0x1fa81e, 1, 0x5de098, HALF);
            self.rr(0x1fa826, 1, 9, Mul)?;
            self.jump(0x1fa82b, 0x1fa83e);
        } else if self.test(0x1fa80f, 0x1fa812, 0x10, 0x1fa82d)? {
            self.constant(0x1fa82d, 2, 0x5def70, NEGATIVE_HALF);
            self.rr(0x1fa835, 2, 9, Mul)?;
            self.rr(0x1fa83a, 1, 2, Add)?;
        } else {
            self.binary(0x1fa814, 1, HALF, Mul)?;
            self.jump(0x1fa81c, 0x1fa83e);
        }
        self.rr(0x1fa83e, 1, 3, Add)?;
        self.constant(0x1fa842, 11, 0x5def70, NEGATIVE_HALF);
        self.rr(0x1fa84b, 9, 5, Div)?;
        self.rr(0x1fa850, 5, 11, Mul)?;
        self.rr(0x1fa855, 5, 7, Sub)?;
        self.rr(0x1fa859, 11, 8, Mul)?;
        self.rr(0x1fa85e, 11, 6, Sub)?;
        self.rr(0x1fa863, 0, 8, Div)?;
        self.copy(0x1fa868, 10, 0)?;
        self.rr(0x1fa86c, 10, 12, Mul)?;
        self.rr(0x1fa871, 11, 0, Mul)?;
        self.rr(0x1fa876, 11, 4, Add)?;
        self.rr(0x1fa87b, 12, 9, Mul)?;
        self.rr(0x1fa880, 5, 9, Mul)?;
        self.rr(0x1fa885, 5, 1, Add)?;
        self.copy(0x1fa889, 13, 5)?;
        Ok(())
    }
    fn run(&mut self) -> Result<[u32; 6], Cause<S::Error>> {
        let input = self.progress.input;

        self.progress.registers[2] = Some(input.target[0]);
        self.progress.registers[3] = Some(input.target[1]);
        self.input(0x1fa6b7, 1, input.target[3]);
        self.input(0x1fa6c0, 4, input.target[2]);
        self.input(0x1fa6c9, 6, input.bounds[0]);
        self.input(0x1fa6ce, 8, input.bounds[1]);
        self.input(0x1fa6d4, 7, input.bounds[2]);
        self.rr(0x1fa6d9, 8, 6, Binary::Sub)?;
        self.input(0x1fa6de, 5, input.bounds[3]);
        self.rr(0x1fa6e3, 5, 7, Binary::Sub)?;
        self.step(0x1fa6e7, Operation::PreserveTest { byte: input.preserve }, Value::Decision(input.preserve == 0));
        if self.branch(0x1fa6ef, 0x1fa795, BranchInput::Preserve { byte: input.preserve }, input.preserve == 0) {
            self.independent()?;
        } else {
            self.preserving()?;
        }
        let mut words = [0; 6];
        for (index, (pc, register)) in [(0x1fa88d, 0), (0x1fa891, 10), (0x1fa897, 11),
            (0x1fa89d, 12), (0x1fa8a3, 9), (0x1fa8a9, 13)].into_iter().enumerate()
        {
            let word = self.word(register, pc)?;
            let index = index as u8;
            self.progress.attempted = Some(Request::WriteWord { pc, index, word });
            self.services.write_word(pc, index, word).map_err(Cause::Service)?;
            self.progress.attempted = None;
            words[index as usize] = word;
            self.progress.words[index as usize] = Some(word);
            self.step(pc, Operation::WriteWord { index, word }, Value::Unit);
        }
        Ok(words)
    }
}

pub fn prepare<S: Services>(input: Input, services: &mut S) -> Result<Completed, Failure<S::Error>> {
    let mut graph = Graph { services, progress: Progress {
        input, registers: [None; 14], words: [None; 6], steps: Vec::new(),
        attempted: None, loaded_justification: None, comparison_754: None,
    } };
    match graph.run() {
        Ok(words) => Ok(Completed { input, words, progress: graph.progress }),
        Err(cause) => Err(Failure {
            cause, attempted: graph.progress.attempted, progress: graph.progress,
        }),
    }
}
