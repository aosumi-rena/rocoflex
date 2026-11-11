use std::collections::TryReserveError;

#[derive(Clone, Copy, Debug)]
pub enum Field32 { OutputWidth50, Mode58, TransformWidth64, HistoryCount74 }
#[derive(Clone, Copy, Debug)]
pub enum Field64 { Factory48, Transform90, ScratchB0, HistoryBeginB8, HistoryEndC0, HistoryCapacityC8 }
#[derive(Clone, Copy, Debug)]
pub enum HeaderWord { InputBegin, InputEnd, OutputBegin, OutputEnd }
#[derive(Clone, Copy, Debug)]
pub enum Constant { Xmm7At4a7f0, Xmm8At4a7f8, Xmm6At4a801 }
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Field32(Field32), Field64(Field64), Header(HeaderWord), InputStatus,

    Read64 { address: u64 }, Read32 { address: u64 },
    SetScratchB0 { word: u64 }, SetHistoryEndC0 { word: u64 },
    SetOutputEnd { word: u64 }, SetOutputStatus { byte: u8 },
    PrepareOutput30bc0 { factory: u64, count: u64, width: u32 },
    ScratchPredicate33260 { argument: u32 },

    FactoryCheckoutV28 { factory: u64, width: u32, r9: u32, fifth_byte: u8 },
    ScratchMemberIsCheckoutTemporary,
    ReadCheckoutTemporary, ClearCheckoutTemporary,
    RetireV0 { row: u64, flag: u32 },
    ZeroFill8e20 { begin: u64, end: u64 },

    GrowHistory1fe50 { count: u64 },

    Helper49080 { mode: u32 },
    Helper49090 { mode: u32, width: u32 },
    Constant(Constant), Flag7b,
    CopyV48 { source: u64, destination: u64 },
    GetV20 { row: u64, index: u32 }, SetV28 { row: u64, index: u32, bits: u32 },
    WidthV18 { row: u64 },
    Helper5834c0 { input: u32 },
    TransformV10 { receiver: u64, input: u64, output: u64, flag: u8 },
    Mulss { destination: u32, source: u32 }, Addss { destination: u32, source: u32 },

    Helper3d7f0 { input: u32 },
}
#[derive(Clone, Copy, Debug)]
pub enum Reply { Unit, Word64(u64), Word32(u32), Byte(u8), Predicate(bool) }

pub fn helper5834c0_with_callback(
    input: u32,
    math: &mut crate::source_scalar::SourceVorbisCodecMath,
    callback: &mut impl crate::source_scalar::SourceMathErrorCallback,
) -> Result<Reply, LogValueFailure> {
    let before = math.exception_state();

    let result = math.log_f32_with_callback(f32::from_bits(input), callback);
    let after = math.exception_state();
    match result {
        Ok(value) => Ok(Reply::Word32(value.to_bits())),
        Err(cause) => Err(LogValueFailure { input, before, after, cause }),
    }
}

#[derive(Debug)]
pub struct LogValueFailure {
    pub input: u32,
    pub before: crate::source_scalar::ExplicitExceptionState,
    pub after: crate::source_scalar::ExplicitExceptionState,
    pub cause: crate::source_scalar::VorbisCodecMathError,
}

pub trait Services {
    type Error;
    type Output: ?Sized;

    fn perform(&mut self, pc: u32, operation: Operation) -> Result<Reply, Self::Error>;

    fn output(&self) -> &Self::Output;
}
#[derive(Clone, Copy, Debug)]
pub struct Attempt { pub pc: u32, pub operation: Operation, pub reply: Option<Reply> }
#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub next_pc: u32, pub effects: usize, pub row: u64,
    pub source_row: Option<u64>, pub target_row: Option<u64>,
    pub first_index: Option<u32>, pub second_index: Option<u32>,
    pub history_index: u64, pub tail_index: u32,
    pub empty_input: bool, pub output_status_stored: bool,
}
pub struct Trace { progress: Progress, attempts: Vec<Attempt>, pending: Option<Attempt> }
impl Trace {
    pub fn progress(&self) -> Progress { self.progress }
    pub fn attempts(&self) -> &[Attempt] { &self.attempts }
    pub fn pending(&self) -> Option<&Attempt> { self.pending.as_ref() }
}
#[derive(Debug)]
pub enum Cause<E> { EffectBudget, TraceAllocation(TryReserveError), Service(E), WrongReply(Reply) }
struct Owner<S> { services: S, trace: Trace, budget: usize }

pub struct Failure<S: Services> { owner: Owner<S>, cause: Cause<S::Error> }
impl<S: Services> Failure<S> {
    pub fn services(&self) -> &S { &self.owner.services }
    pub fn output(&self) -> &S::Output { self.owner.services.output() }
    pub fn trace(&self) -> &Trace { &self.owner.trace }
    pub fn cause(&self) -> &Cause<S::Error> { &self.cause }
}

pub struct Completed<S: Services> { owner: Owner<S> }
impl<S: Services> Completed<S> {
    pub fn services(&self) -> &S { &self.owner.services }
    pub fn output(&self) -> &S::Output { self.owner.services.output() }
    pub fn trace(&self) -> &Trace { &self.owner.trace }

    pub fn into_parts(self) -> (S, Trace) { (self.owner.services, self.owner.trace) }
}

pub fn run<S: Services>(services: S, budget: usize) -> Result<Completed<S>, Failure<S>> {
    let mut owner = Owner { services, budget, trace: Trace { progress: Progress {
        next_pc: 0x4a666, effects: 0, row: 0, source_row: None, target_row: None,
        first_index: None, second_index: None, history_index: 0, tail_index: 0,
        empty_input: false, output_status_stored: false }, attempts: Vec::new(), pending: None } };
    match owner.body() {
        Ok(()) => Ok(Completed { owner }),
        Err(cause) => Err(Failure { owner, cause }),
    }
}

impl<S: Services> Owner<S> {
    fn call(&mut self, pc: u32, operation: Operation) -> Result<Reply, Cause<S::Error>> {
        self.trace.progress.next_pc = pc;
        self.trace.pending = Some(Attempt { pc, operation, reply: None });
        if self.trace.progress.effects == self.budget { return Err(Cause::EffectBudget); }
        self.trace.attempts.try_reserve(1).map_err(Cause::TraceAllocation)?;
        let index = self.trace.attempts.len();
        self.trace.attempts.push(self.trace.pending.take().expect("current required operation"));
        self.trace.progress.effects += 1;
        let reply = self.services.perform(pc, operation).map_err(Cause::Service)?;

        self.trace.attempts[index].reply = Some(reply);
        Ok(reply)
    }
    fn unit(&mut self, pc: u32, op: Operation) -> Result<(), Cause<S::Error>> {
        match self.call(pc, op)? { Reply::Unit => Ok(()), other => Err(Cause::WrongReply(other)) }
    }
    fn word(&mut self, pc: u32, op: Operation) -> Result<u64, Cause<S::Error>> {
        match self.call(pc, op)? { Reply::Word64(word) => Ok(word), other => Err(Cause::WrongReply(other)) }
    }
    fn bits(&mut self, pc: u32, op: Operation) -> Result<u32, Cause<S::Error>> {
        match self.call(pc, op)? { Reply::Word32(bits) => Ok(bits), other => Err(Cause::WrongReply(other)) }
    }
    fn byte(&mut self, pc: u32, op: Operation) -> Result<u8, Cause<S::Error>> {
        match self.call(pc, op)? { Reply::Byte(byte) => Ok(byte), other => Err(Cause::WrongReply(other)) }
    }
    fn predicate(&mut self, pc: u32, op: Operation) -> Result<bool, Cause<S::Error>> {
        match self.call(pc, op)? { Reply::Predicate(value) => Ok(value), other => Err(Cause::WrongReply(other)) }
    }
    fn field(&mut self, pc: u32, field: Field64) -> Result<u64, Cause<S::Error>> { self.word(pc, Operation::Field64(field)) }
    fn dword(&mut self, pc: u32, field: Field32) -> Result<u32, Cause<S::Error>> { self.bits(pc, Operation::Field32(field)) }

    fn body(&mut self) -> Result<(), Cause<S::Error>> {
        let begin = self.word(0x4a666, Operation::Header(HeaderWord::InputBegin))?;
        let end = self.word(0x4a66e, Operation::Header(HeaderWord::InputEnd))?;
        if begin == end { return self.empty(); }
        let width = self.dword(0x4a67e, Field32::OutputWidth50)?;
        let factory = self.field(0x4a689, Field64::Factory48)?;
        self.unit(0x4a690, Operation::PrepareOutput30bc0 { factory, count: end.wrapping_sub(begin) >> 3, width })?;
        if self.byte(0x4a6a1, Operation::ScratchPredicate33260 { argument: 0 })? != 0 {
            self.prepare_scratch()?;
        }
        let mode = self.dword(0x4a7b1, Field32::Mode58)?;
        let width = self.dword(0x4a7b4, Field32::TransformWidth64)?;
        let first = self.bits(0x4a7ba, Operation::Helper49080 { mode })?;
        self.trace.progress.first_index = Some(first);
        let second = self.bits(0x4a7c8, Operation::Helper49090 { mode, width })?;
        self.trace.progress.second_index = Some(second);
        let mut begin = self.word(0x4a7d5, Operation::Header(HeaderWord::InputBegin))?;
        let end = self.word(0x4a7d8, Operation::Header(HeaderWord::InputEnd))?;
        if end != begin {
            let c7 = self.bits(0x4a7f0, Operation::Constant(Constant::Xmm7At4a7f0))?;
            let c8 = self.bits(0x4a7f8, Operation::Constant(Constant::Xmm8At4a7f8))?;
            let c6 = self.bits(0x4a801, Operation::Constant(Constant::Xmm6At4a801))?;
            loop {
                let row = self.trace.progress.row;
                let source = self.word(0x4a83a, Operation::Read64 { address: begin.wrapping_add(row.wrapping_mul(8)) })?;
                self.trace.progress.source_row = Some(source);
                let output_begin = self.word(0x4a83e, Operation::Header(HeaderWord::OutputBegin))?;
                let target = self.word(0x4a842, Operation::Read64 { address: output_begin.wrapping_add(row.wrapping_mul(8)) })?;
                self.trace.progress.target_row = Some(target);
                let scratch = self.field(0x4a846, Field64::ScratchB0)?;
                self.unit(0x4a854, Operation::CopyV48 { source, destination: scratch })?;
                if self.dword(0x4a857, Field32::Mode58)? == 0 && self.byte(0x4a85d, Operation::Flag7b)? != 0 {
                    let scratch = self.field(0x4a863, Field64::ScratchB0)?;
                    self.unit(0x4a870, Operation::SetV28 { row: scratch, index: first, bits: c6 })?;
                }
                let input = self.bits(0x4a87d, Operation::GetV20 { row: source, index: second })?;
                let scratch = self.field(0x4a880, Field64::ScratchB0)?;
                let transformed = self.bits(0x4a887, Operation::Helper5834c0 { input })?;
                self.unit(0x4a898, Operation::SetV28 { row: scratch, index: second, bits: transformed })?;
                let receiver = self.field(0x4a89b, Field64::Transform90)?;
                let scratch = self.field(0x4a8a2, Field64::ScratchB0)?;
                self.unit(0x4a8b2, Operation::TransformV10 { receiver, input: scratch, output: scratch, flag: 1 })?;
                let history_count = self.dword(0x4a8b5, Field32::HistoryCount74)? as i32;
                let scratch = self.field(0x4a8b9, Field64::ScratchB0)?;
                if history_count < 2 {
                    self.unit(0x4a813, Operation::CopyV48 { source: scratch, destination: target })?;
                } else {
                    self.with_history(target, scratch, first, second, c7, c8)?;
                }
                self.trace.progress.row = row.wrapping_add(1);
                begin = self.word(0x4a81e, Operation::Header(HeaderWord::InputBegin))?;
                let end = self.word(0x4a821, Operation::Header(HeaderWord::InputEnd))?;

                let count = (end.wrapping_sub(begin) as i64 >> 3) as u64;
                if self.trace.progress.row >= count { break; }
            }
        }
        let status = self.byte(0x4a98e, Operation::InputStatus)?;
        self.unit(0x4a992, Operation::SetOutputStatus { byte: status })?;
        self.trace.progress.output_status_stored = true;
        self.trace.progress.next_pc = 0x4a9b9;
        Ok(())
    }

    fn empty(&mut self) -> Result<(), Cause<S::Error>> {
        self.trace.progress.empty_input = true;
        let mut at = self.word(0x4a740, Operation::Header(HeaderWord::OutputBegin))?;
        let end = self.word(0x4a744, Operation::Header(HeaderWord::OutputEnd))?;
        if at != end {
            loop {
                let row = self.word(0x4a759, Operation::Read64 { address: at })?;
                if row != 0 { self.unit(0x4a769, Operation::RetireV0 { row, flag: 1 })?; }
                at = at.wrapping_add(8);
                if at == end { break; }
            }
            at = self.word(0x4a76d, Operation::Header(HeaderWord::OutputBegin))?;
        }
        self.unit(0x4a771, Operation::SetOutputEnd { word: at })?;

        self.trace.progress.next_pc = 0x4a9b9;
        Ok(())
    }

    fn prepare_scratch(&mut self) -> Result<(), Cause<S::Error>> {
        let factory = self.field(0x4a6ae, Field64::Factory48)?;
        let width = self.dword(0x4a6b2, Field32::TransformWidth64)?;
        self.unit(0x4a6c9, Operation::FactoryCheckoutV28 { factory, width, r9: 0, fifth_byte: 0 })?;
        if !self.predicate(0x4a6cc, Operation::ScratchMemberIsCheckoutTemporary)? {
            let new = self.word(0x4a6d1, Operation::ReadCheckoutTemporary)?;
            self.unit(0x4a6d6, Operation::ClearCheckoutTemporary)?;
            let old = self.field(0x4a6df, Field64::ScratchB0)?;
            self.unit(0x4a6e2, Operation::SetScratchB0 { word: new })?;
            if old != 0 { self.unit(0x4a6f2, Operation::RetireV0 { row: old, flag: 1 })?; }
            else { return self.prepare_history(); }
        }
        let temporary = self.word(0x4a6f4, Operation::ReadCheckoutTemporary)?;
        if temporary != 0 { self.unit(0x4a706, Operation::RetireV0 { row: temporary, flag: 1 })?; }
        self.prepare_history()
    }
    fn prepare_history(&mut self) -> Result<(), Cause<S::Error>> {
        let count = self.dword(0x4a70f, Field32::HistoryCount74)? as i32 as i64 as u64;
        let begin = self.field(0x4a713, Field64::HistoryBeginB8)?;
        let end = self.field(0x4a71a, Field64::HistoryEndC0)?;
        let old_count = (end.wrapping_sub(begin) as i64 >> 2) as u64;
        if count < old_count {
            self.unit(0x4a737, Operation::SetHistoryEndC0 { word: begin.wrapping_add(count.wrapping_mul(4)) })?;
        } else if count > old_count {
            let capacity_end = self.field(0x4a77c, Field64::HistoryCapacityC8)?;
            let capacity = (capacity_end.wrapping_sub(begin) as i64 >> 2) as u64;
            if capacity < count { self.unit(0x4a794, Operation::GrowHistory1fe50 { count })?; }
            else {
                let new_end = end.wrapping_add(count.wrapping_sub(old_count).wrapping_mul(4));
                self.unit(0x4a7a5, Operation::ZeroFill8e20 { begin: end, end: new_end })?;
                self.unit(0x4a7aa, Operation::SetHistoryEndC0 { word: new_end })?;
            }
        }
        Ok(())
    }
    fn with_history(&mut self, target: u64, scratch: u64, first: u32, second: u32,
        c7: u32, c8: u32) -> Result<(), Cause<S::Error>> {
        let value = self.bits(0x4a8cc, Operation::GetV20 { row: scratch, index: second })?;
        let product = self.bits(0x4a8cf, Operation::Mulss { destination: value, source: c7 })?;
        let value = self.bits(0x4a8d3, Operation::Addss { destination: product, source: c7 })?;
        self.unit(0x4a8df, Operation::Helper3d7f0 { input: value })?;
        let count = self.dword(0x4a8e4, Field32::HistoryCount74)?;
        let scratch = self.field(0x4a8e7, Field64::ScratchB0)?;
        let value = self.bits(0x4a8f5, Operation::GetV20 { row: scratch, index: first })?;
        self.unit(0x4a904, Operation::SetV28 { row: target, index: 0, bits: value })?;
        let current_count = self.dword(0x4a907, Field32::HistoryCount74)? as i32;
        self.trace.progress.history_index = 0;
        if current_count > 0 {
            loop {
                let i = self.trace.progress.history_index;
                let begin = self.field(0x4a910, Field64::HistoryBeginB8)?;
                let bits = self.bits(0x4a917, Operation::Read32 { address: begin.wrapping_add(i.wrapping_mul(4)) })?;
                let doubled = self.bits(0x4a91c, Operation::Addss { destination: bits, source: bits })?;
                let value = self.bits(0x4a920, Operation::Addss { destination: doubled, source: c8 })?;
                let next = i.wrapping_add(1);
                self.trace.progress.history_index = next;
                self.unit(0x4a931, Operation::SetV28 { row: target, index: next as u32, bits: value })?;
                let limit = self.dword(0x4a934, Field32::HistoryCount74)? as i32 as i64;
                if next as i64 >= limit { break; }
            }
        }
        let scratch = self.field(0x4a93d, Field64::ScratchB0)?;
        let width = self.bits(0x4a943, Operation::WidthV18 { row: scratch })? as i32;
        if width >= 3 {
            let mut index = 2u32;
            loop {
                self.trace.progress.tail_index = index;
                let scratch = self.field(0x4a960, Field64::ScratchB0)?;
                let value = self.bits(0x4a968, Operation::GetV20 { row: scratch, index })?;
                let destination = count.wrapping_add(index).wrapping_sub(1);
                self.unit(0x4a97a, Operation::SetV28 { row: target, index: destination, bits: value })?;
                index = index.wrapping_add(1);
                if index == width as u32 { break; }
            }
        }
        Ok(())
    }
}
