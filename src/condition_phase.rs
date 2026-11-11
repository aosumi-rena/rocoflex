use crate::{condition::PhaseServices, source_scalar::{ExplicitExceptionState,
    SourceF64ValueCompletion, SourceF64ValueError, SourceF64ValueMath}};

pub trait Trig {
    type Error;
    fn sin_586960(&mut self, math: &mut SourceF64ValueMath, angle_bits: u64) -> Result<u64, Self::Error>;
    fn cos_57ddb0(&mut self, math: &mut SourceF64ValueMath, angle_bits: u64) -> Result<u64, Self::Error>;
}

pub struct SourceWordTrig { word: u32 }
impl SourceWordTrig {
    pub fn from_explicit_source_word(word: u32) -> Self { Self { word } }
    pub fn explicit_source_word(&self) -> u32 { self.word }
}
impl Trig for SourceWordTrig {
    type Error = SourceF64ValueError;
    fn sin_586960(&mut self, math: &mut SourceF64ValueMath, angle_bits: u64) -> Result<u64, Self::Error> {
        math.sine_586960_bits(angle_bits, self.word)
    }
    fn cos_57ddb0(&mut self, math: &mut SourceF64ValueMath, angle_bits: u64) -> Result<u64, Self::Error> {
        math.cosine_57ddb0_bits(angle_bits, self.word)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Phase(u32), Period(u16), Frequency(u32), Multiply(u64, u64), Divide(u64, u64),
    Sin(u64), Cos(u64), Narrow(u64),
}
pub struct Call {
    pub operation: Operation, pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState,
    pub result: Option<u64>, pub scalar_completion: Option<SourceF64ValueCompletion>,
}
pub enum Failure<E> { Scalar(SourceF64ValueError), Trig(E) }
#[derive(Clone, Copy, Debug)]
pub struct Stopped;
pub struct Provider<T: Trig> {
    math: SourceF64ValueMath, trig: T, calls: Vec<Call>, failure: Option<Failure<T::Error>>, inflight: bool,
}
pub struct BindingFailure<T> { pub math: SourceF64ValueMath, pub trig: T }

impl<T: Trig> Provider<T> {
    pub fn from_explicit_math(math: SourceF64ValueMath, trig: T) -> Result<Self, BindingFailure<T>> {
        if math.failure().is_some() { return Err(BindingFailure { math, trig }); }
        Ok(Self { math, trig, calls: Vec::new(), failure: None, inflight: false })
    }
    pub fn math(&self) -> &SourceF64ValueMath { &self.math }
    pub fn trig(&self) -> &T { &self.trig }
    pub fn calls(&self) -> &[Call] { &self.calls }
    pub fn failure(&self) -> Option<&Failure<T::Error>> { self.failure.as_ref() }
    pub fn inflight(&self) -> bool { self.inflight }

    fn scalar(&mut self, operation: Operation,
        execute: impl FnOnce(&mut SourceF64ValueMath) -> Result<u64, SourceF64ValueError>)
        -> Result<u64, Stopped>
    {
        if self.failure.is_some() || self.inflight { return Err(Stopped); }
        let index = self.calls.len(); let entry = self.math.exception_state();
        self.calls.push(Call { operation, entry, advanced: entry, result: None, scalar_completion: None });
        self.inflight = true;
        let result = execute(&mut self.math);
        self.inflight = false;
        self.calls[index].advanced = self.math.exception_state();
        match result {
            Ok(bits) => {
                self.calls[index].result = Some(bits);
                self.calls[index].scalar_completion = self.math.last_completion(); Ok(bits)
            }
            Err(error) => { self.failure = Some(Failure::Scalar(error)); Err(Stopped) }
        }
    }
    fn trig_call(&mut self, angle: u64, sine: bool) -> Result<u64, Stopped> {
        if self.failure.is_some() || self.inflight { return Err(Stopped); }
        let index = self.calls.len(); let entry = self.math.exception_state();
        self.calls.push(Call { operation: if sine { Operation::Sin(angle) } else { Operation::Cos(angle) },
            entry, advanced: entry, result: None, scalar_completion: None });
        self.inflight = true;
        let result = if sine { self.trig.sin_586960(&mut self.math, angle) }
            else { self.trig.cos_57ddb0(&mut self.math, angle) };
        self.inflight = false;
        self.calls[index].advanced = self.math.exception_state();
        match result {
            Ok(bits) => {

                if let Some(failed) = self.math.failure().copied() {
                    self.failure = Some(Failure::Scalar(SourceF64ValueError::AlreadyFailed(failed)));
                    return Err(Stopped);
                }
                self.calls[index].result = Some(bits); Ok(bits)
            }
            Err(error) => { self.failure = Some(Failure::Trig(error)); Err(Stopped) }
        }
    }
}
impl<T: Trig> PhaseServices for Provider<T> {
    type Error = Stopped;
    fn phase_u32_to_f64(&mut self, value: u32) -> Result<f64, Stopped> {
        self.scalar(Operation::Phase(value), |math| math.unsigned_u32_to_f64_bits(value)).map(f64::from_bits)
    }
    fn period_u16_to_f64(&mut self, value: u16) -> Result<f64, Stopped> {
        self.scalar(Operation::Period(value), |math| math.signed_i32_to_f64_bits(i32::from(value))).map(f64::from_bits)
    }
    fn frequency_f32_to_f64(&mut self, value: f32) -> Result<f64, Stopped> {
        self.scalar(Operation::Frequency(value.to_bits()), |math| math.widen_f32_bits(value.to_bits())).map(f64::from_bits)
    }
    fn multiply_f64(&mut self, left: f64, right: f64) -> Result<f64, Stopped> {
        self.scalar(Operation::Multiply(left.to_bits(), right.to_bits()),
            |math| math.multiply_bits(left.to_bits(), right.to_bits())).map(f64::from_bits)
    }
    fn divide_f64(&mut self, left: f64, right: f64) -> Result<f64, Stopped> {
        self.scalar(Operation::Divide(left.to_bits(), right.to_bits()),
            |math| math.divide_bits(left.to_bits(), right.to_bits())).map(f64::from_bits)
    }
    fn sin_586960(&mut self, angle: f64) -> Result<f64, Stopped> {
        self.trig_call(angle.to_bits(), true).map(f64::from_bits)
    }
    fn cos_57ddb0(&mut self, angle: f64) -> Result<f64, Stopped> {
        self.trig_call(angle.to_bits(), false).map(f64::from_bits)
    }
    fn narrow_f64_to_f32(&mut self, value: f64) -> Result<f32, Stopped> {
        self.scalar(Operation::Narrow(value.to_bits()), |math| math.narrow_f64_bits(value.to_bits()).map(u64::from))
            .map(|bits| f32::from_bits(bits as u32))
    }
}
