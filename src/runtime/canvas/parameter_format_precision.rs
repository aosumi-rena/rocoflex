use super::{BranchSelected, Next, Retained};
use super::super::super::super::{Binary, Comparison};
#[path = "parameter_float_classification.rs"]
pub mod classification;
#[path = "parameter_formatter_object.rs"]
pub mod object;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F64Binary { Multiply, Add }
pub trait Services: super::Services {
    fn read_interval90(&mut self, parameter: &Self::Parameter) -> Result<u32, Self::Error>;
    fn write_temp_m20(&mut self, frame: &mut Self::Frame, bits: u32) -> Result<(), Self::Error>;

    fn read_temp_word(&mut self, frame: &mut Self::Frame, byte_offset: u8) -> Result<u16, Self::Error>;
    fn read_temp_byte(&mut self, frame: &mut Self::Frame, byte_offset: u8) -> Result<u8, Self::Error>;
    fn binary_f32(&mut self, pc: u32, operation: Binary, left: u32, right: u32) -> Result<u32, Self::Error>;
    fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<Comparison, Self::Error>;

    fn floor580388(&mut self, frame: &mut Self::Frame, bits: u32) -> Result<u32, Self::Error>;
    fn widen_f32(&mut self, pc: u32, bits: u32) -> Result<u64, Self::Error>;
    fn binary_f64(&mut self, pc: u32, operation: F64Binary, left: u64, right: u64) -> Result<u64, Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct ClassRead { pub offset: u8, pub width: u8, pub value: u16 }

struct TempRegion;
struct TempMemory<'a, S: Services> {
    services: &'a mut S, frame: &'a mut S::Frame,
    reads: &'a mut [Option<ClassRead>; 3], next: usize,
}
impl<S: Services> classification::Memory for TempMemory<'_, S> {
    type Pointer = TempRegion;
    type Error = S::Error;
    fn read_word(&mut self, _: &TempRegion, offset: u8) -> Result<u16, Self::Error> {
        let value = self.services.read_temp_word(self.frame, offset)?;

        self.reads[self.next] = Some(ClassRead { offset, width: 2, value }); self.next += 1;
        Ok(value)
    }
    fn read_byte(&mut self, _: &TempRegion, offset: u8) -> Result<u8, Self::Error> {
        let value = self.services.read_temp_byte(self.frame, offset)?;
        self.reads[self.next] = Some(ClassRead { offset, width: 1, value: u16::from(value) }); self.next += 1;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct IntegerSelection {
    pub raw_word: u32, pub negated: u32, pub absolute_word: u32,
    pub quantities: [Option<u32>; 7], pub tests: [Option<u32>; 7], pub digits: u32,
}

fn select_integer(raw: u32) -> IntegerSelection {
    let negated = raw.wrapping_neg();
    let absolute = if negated & 0x80000000 != 0 { raw } else { negated };
    let mut out = IntegerSelection { raw_word: raw, negated, absolute_word: absolute,
        quantities: [None; 7], tests: [None; 7], digits: 7 };
    for index in 0..7 {
        let quantity = match index {
            0 => absolute,
            1 => ((u64::from(absolute).wrapping_mul(0xcccccccd)) >> 35) as u32,
            2 => ((u64::from(absolute).wrapping_mul(0x51eb851f)) >> 37) as u32,
            3 => ((u64::from(absolute).wrapping_mul(0x10624dd3)) >> 38) as u32,
            4 => ((u64::from(absolute).wrapping_mul(0xd1b71759)) >> 45) as u32,
            5 => ((u64::from(absolute >> 5).wrapping_mul(0x0a7c5ac5)) >> 39) as u32,
            _ => ((u64::from(absolute).wrapping_mul(0x431bde83)) >> 50) as u32,
        };
        let test = quantity.wrapping_mul(0xcccccccd).rotate_right(1);
        out.quantities[index] = Some(quantity); out.tests[index] = Some(test);
        if index < 6 && test > 0x19999999 { out.digits = 7 - index as u32; return out; }
        if index == 6 { out.digits = u32::from(test >= 0x1999999a); }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    ReadInitial, StoreInitial, ClassInitial, StoreInitialZero, ClassInitialZero,
    IntervalZeroTest, AbsIntervalTest, IntervalSmallMultiply, IntervalSmallTest,
    ReadFloorInput, Floor, Subtract, StoreFraction, ClassFraction,
    StoreFractionZero, ClassFractionZero, FractionZeroTest, AbsFractionTest,
    FractionSmallMultiply, FractionSmallTest, ReadRoundInput, Widen, Multiply64,
    AddBias64, SelectInteger, FormatterObjectRequired,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Captures {
    pub initial_interval: Option<u32>, pub interval: Option<u32>,
    pub classifications: [Option<i16>; 4], pub class_reads: [[Option<ClassRead>; 3]; 4],
    pub comparisons: [Option<Comparison>; 4], pub floor_bits: Option<u32>,
    pub fraction: Option<u32>, pub fraction_abs: Option<u32>, pub small_products: [Option<u32>; 2],
    pub round_input: Option<u32>, pub widened: Option<u64>, pub scaled: Option<u64>,
    pub biased: Option<u64>, pub integer: Option<IntegerSelection>, pub digits: Option<u32>,
}
pub enum Fault<E> { Service(E), Classification(classification::Failure<E>), MissingState, AlreadyStopped }
pub struct Prepared<S: Services> {
    retained: Retained<S>, captures: Captures, phase: Phase,
    attempted: Option<Phase>, stopped: bool,
}
pub struct Rejected<S: Services> { pub selected: BranchSelected<S> }
pub struct Stopped<S: Services> { pub pending: Prepared<S>, pub fault: Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Prepared<S>), Ready(PrecisionSelected<S>), Failed(Stopped<S>) }
pub struct PrecisionSelected<S: Services> { returned: FormatterObjectRequired<S> }
pub struct FormatterObjectRequired<S: Services> { pub retained: Retained<S>, pub captures: Captures }
pub struct Parts<S: Services> {
    retained: Retained<S>, pub captures: Captures, pub phase: Phase,
    pub attempted: Option<Phase>, pub stopped: bool,
}
impl<S: Services> Parts<S> {
    pub fn retained(&self) -> &Retained<S> { &self.retained }
}
impl<S: Services> PrecisionSelected<S> {
    pub fn digits(&self) -> u32 { self.returned.captures.digits.unwrap() }
    pub fn into_formatter_object_required(self) -> FormatterObjectRequired<S> { self.returned }
}
impl<S: Services> Prepared<S> {
    pub fn from_selected_formatting(selected: BranchSelected<S>) -> Result<Self, Rejected<S>> {
        if selected.next() != Next::Formatting2b3307 { return Err(Rejected { selected }); }
        Ok(Self { retained: selected.into_required_continuation(), captures: Captures::default(),
            phase: Phase::ReadInitial, attempted: None, stopped: false })
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn captures(&self) -> &Captures { &self.captures }
    pub fn into_parts(self) -> Parts<S> {
        Parts { retained: self.retained, captures: self.captures, phase: self.phase,
            attempted: self.attempted, stopped: self.stopped }
    }
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        if self.stopped { return Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        let mut spent = 0usize;
        while self.phase != Phase::FormatterObjectRequired && spent < budget {
            self.attempted = Some(self.phase); spent += 1;
            if let Err(fault) = self.step() {
                self.stopped = true;
                return Poll::Failed(Stopped { pending: self, fault });
            }
            self.attempted = None;
        }
        if self.phase != Phase::FormatterObjectRequired { return Poll::Pending(self); }
        if self.captures.digits.is_none() {
            self.stopped = true;
            return Poll::Failed(Stopped { pending: self, fault: Fault::MissingState });
        }
        Poll::Ready(PrecisionSelected { returned: FormatterObjectRequired { retained: self.retained, captures: self.captures } })
    }
    fn finish(&mut self, digits: u32) { self.captures.digits = Some(digits); self.phase = Phase::FormatterObjectRequired; }
    fn classify(&mut self, index: usize) -> Result<i16, Fault<S::Error>> {
        let p = &mut self.retained.parent;
        let mut memory = TempMemory { services: &mut p.services, frame: &mut p.frame,
            reads: &mut self.captures.class_reads[index], next: 0 };
        let ax = classification::classify_with_memory(&mut memory, &TempRegion).map_err(Fault::Classification)?;
        self.captures.classifications[index] = Some(ax); Ok(ax)
    }
    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        const MIN_NORMAL: u32 = 0x00800000;
        const EPSILON: u32 = 0x34000000;
        match self.phase {
            Phase::ReadInitial => {
                let p = &mut self.retained.parent;
                let bits = p.services.read_interval90(&p.parameter).map_err(Fault::Service)?;
                self.captures.initial_interval = Some(bits); self.captures.interval = Some(bits);
                self.phase = Phase::StoreInitial;
            }
            Phase::StoreInitial => {
                let bits = self.captures.interval.ok_or(Fault::MissingState)?;
                let p = &mut self.retained.parent;
                p.services.write_temp_m20(&mut p.frame, bits).map_err(Fault::Service)?;
                self.phase = Phase::ClassInitial;
            }
            Phase::ClassInitial => self.phase = if self.classify(0)? > 0 { Phase::IntervalZeroTest } else { Phase::StoreInitialZero },
            Phase::StoreInitialZero | Phase::StoreFractionZero => {
                let fraction = self.phase == Phase::StoreFractionZero;
                let p = &mut self.retained.parent;
                p.services.write_temp_m20(&mut p.frame, 0).map_err(Fault::Service)?;
                self.phase = if fraction { Phase::ClassFractionZero } else { Phase::ClassInitialZero };
            }
            Phase::ClassInitialZero => self.phase = if self.classify(1)? <= 0 { Phase::AbsIntervalTest } else { Phase::IntervalZeroTest },
            Phase::IntervalZeroTest => {
                let value = self.captures.interval.ok_or(Fault::MissingState)?;
                let cmp = self.retained.parent.services.ucomiss(0x2b3347, value, 0).map_err(Fault::Service)?;
                self.captures.comparisons[0] = Some(cmp);
                if !cmp.unordered && cmp.equal { self.finish(7); } else { self.phase = Phase::ReadFloorInput; }
            }
            Phase::AbsIntervalTest => {
                let value = self.captures.interval.ok_or(Fault::MissingState)? & 0x7fffffff;
                self.captures.interval = Some(value);
                let cmp = self.retained.parent.services.ucomiss(0x2b34ee, MIN_NORMAL, value).map_err(Fault::Service)?;
                self.captures.comparisons[0] = Some(cmp);
                if !cmp.unordered && !cmp.less { self.finish(7); } else { self.phase = Phase::IntervalSmallMultiply; }
            }
            Phase::IntervalSmallMultiply => {
                let value = self.captures.interval.ok_or(Fault::MissingState)?;
                self.captures.small_products[0] = Some(self.retained.parent.services.binary_f32(0x2b34fb, Binary::Multiply, value, EPSILON).map_err(Fault::Service)?);
                self.phase = Phase::IntervalSmallTest;
            }
            Phase::IntervalSmallTest => {
                let product = self.captures.small_products[0].ok_or(Fault::MissingState)?;
                let value = self.captures.interval.ok_or(Fault::MissingState)?;
                let cmp = self.retained.parent.services.ucomiss(0x2b3503, product, value).map_err(Fault::Service)?;
                self.captures.comparisons[1] = Some(cmp);
                if cmp.unordered || cmp.less { self.phase = Phase::ReadFloorInput; } else { self.finish(7); }
            }
            Phase::ReadFloorInput => {
                let p = &mut self.retained.parent;
                self.captures.interval = Some(p.services.read_interval90(&p.parameter).map_err(Fault::Service)?);
                self.phase = Phase::Floor;
            }
            Phase::Floor => {
                let value = self.captures.interval.ok_or(Fault::MissingState)?;
                let p = &mut self.retained.parent;
                self.captures.floor_bits = Some(p.services.floor580388(&mut p.frame, value).map_err(Fault::Service)?);
                self.phase = Phase::Subtract;
            }
            Phase::Subtract => {
                let input = self.captures.interval.ok_or(Fault::MissingState)?;
                let floor = self.captures.floor_bits.ok_or(Fault::MissingState)?;
                let bits = self.retained.parent.services.binary_f32(0x2b3363, Binary::Subtract, input, floor).map_err(Fault::Service)?;
                self.captures.fraction = Some(bits); self.captures.fraction_abs = Some(bits & 0x7fffffff);
                self.phase = Phase::StoreFraction;
            }
            Phase::StoreFraction => {
                let bits = self.captures.fraction_abs.ok_or(Fault::MissingState)?;
                let p = &mut self.retained.parent;
                p.services.write_temp_m20(&mut p.frame, bits).map_err(Fault::Service)?;
                self.phase = Phase::ClassFraction;
            }
            Phase::ClassFraction => self.phase = if self.classify(2)? > 0 { Phase::FractionZeroTest } else { Phase::StoreFractionZero },
            Phase::ClassFractionZero => self.phase = if self.classify(3)? <= 0 { Phase::AbsFractionTest } else { Phase::FractionZeroTest },
            Phase::FractionZeroTest => {
                let value = self.captures.fraction.ok_or(Fault::MissingState)?;
                let cmp = self.retained.parent.services.ucomiss(0x2b33a2, value, 0).map_err(Fault::Service)?;
                self.captures.comparisons[2] = Some(cmp);
                if !cmp.unordered && cmp.equal { self.finish(0); } else { self.phase = Phase::ReadRoundInput; }
            }
            Phase::AbsFractionTest => {
                let value = self.captures.fraction_abs.ok_or(Fault::MissingState)?;
                let cmp = self.retained.parent.services.ucomiss(0x2b3518, MIN_NORMAL, value).map_err(Fault::Service)?;
                self.captures.comparisons[2] = Some(cmp);
                if !cmp.unordered && !cmp.less { self.finish(0); } else { self.phase = Phase::FractionSmallMultiply; }
            }
            Phase::FractionSmallMultiply => {
                let value = self.captures.fraction_abs.ok_or(Fault::MissingState)?;
                self.captures.small_products[1] = Some(self.retained.parent.services.binary_f32(0x2b3525, Binary::Multiply, EPSILON, value).map_err(Fault::Service)?);
                self.phase = Phase::FractionSmallTest;
            }
            Phase::FractionSmallTest => {
                let product = self.captures.small_products[1].ok_or(Fault::MissingState)?;
                let value = self.captures.fraction_abs.ok_or(Fault::MissingState)?;
                let cmp = self.retained.parent.services.ucomiss(0x2b3529, product, value).map_err(Fault::Service)?;
                self.captures.comparisons[3] = Some(cmp);
                if cmp.unordered || cmp.less { self.phase = Phase::ReadRoundInput; } else { self.finish(0); }
            }
            Phase::ReadRoundInput => {
                let p = &mut self.retained.parent;
                self.captures.round_input = Some(p.services.read_interval90(&p.parameter).map_err(Fault::Service)?);
                self.phase = Phase::Widen;
            }
            Phase::Widen => {
                self.captures.widened = Some(self.retained.parent.services.widen_f32(0x2b33b6, self.captures.round_input.ok_or(Fault::MissingState)?).map_err(Fault::Service)?);
                self.phase = Phase::Multiply64;
            }
            Phase::Multiply64 => {
                self.captures.scaled = Some(self.retained.parent.services.binary_f64(0x2b33ba, F64Binary::Multiply,
                    self.captures.widened.ok_or(Fault::MissingState)?, 0x416312d000000000).map_err(Fault::Service)?);
                self.phase = Phase::AddBias64;
            }
            Phase::AddBias64 => {
                self.captures.biased = Some(self.retained.parent.services.binary_f64(0x2b33c2, F64Binary::Add,
                    self.captures.scaled.ok_or(Fault::MissingState)?, 0x4338000000000000).map_err(Fault::Service)?);
                self.phase = Phase::SelectInteger;
            }
            Phase::SelectInteger => {
                let selection = select_integer(self.captures.biased.ok_or(Fault::MissingState)? as u32);
                self.captures.integer = Some(selection); self.finish(selection.digits);
            }
            Phase::FormatterObjectRequired => {}
        }
        Ok(())
    }
}
