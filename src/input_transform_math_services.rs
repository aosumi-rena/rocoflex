use crate::input_transform_4a640::{self as control, Constant, Reply};
use crate::input_transform_value_services::Providers;
use crate::source_scalar::{ExplicitExceptionState, SourceMathErrorCallback, SourceVorbisCodecMath};
use crate::source_scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture,
    ScalarF32InstructionSet, ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr,
    ScalarF32Outcome, ScalarF32Error};
use crate::source_scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};

pub trait RequiredInputTransformContext {
    type Error;
    fn scratch_predicate(&mut self, argument: u32, scratch: Option<&[f32]>) -> Result<u8, Self::Error>;
    fn first_index(&mut self, mode: u32) -> Result<u32, Self::Error>;
    fn second_index(&mut self, mode: u32, width: u32) -> Result<u32, Self::Error>;
    fn constant(&mut self, constant: Constant) -> Result<u32, Self::Error>;
    fn transform_in_place(&mut self, receiver: u64, row: &mut [f32], flag: u8) -> Result<(), Self::Error>;
    fn update_history(&mut self, header: u64, history: &mut [u32], input: u32) -> Result<(), Self::Error>;
    fn copy_unequal(&mut self, source: &[f32], destination: &mut Vec<f32>) -> Result<(), Self::Error>;
    fn stack_fill_byte(&mut self) -> Result<u8, Self::Error>;
}

#[derive(Clone, Copy, Debug)]
pub enum SourceInputTransformOperation {
    ScratchPredicate { argument: u32, scratch_len: Option<usize> },
    FirstIndex { mode: u32 }, SecondIndex { mode: u32, width: u32 },
    Constant(Constant), Logarithm { input: u32 },
    Multiply { destination: u32, source: u32 }, Add { destination: u32, source: u32 },
    Transform { receiver: u64, row_len: usize, flag: u8 },
    History { header: u64, history_len: usize, input: u32 },
    CopyUnequal { source_len: usize, destination_len: usize }, StackFillByte,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceInputTransformContextReply { Byte(u8), Word(u32), Unit }

pub enum SourceInputTransformReport<E> {
    Context(Result<SourceInputTransformContextReply, E>),
    Scalar(Result<ScalarF32Outcome, ScalarF32Error>),
    Log(Result<Reply, control::LogValueFailure>),
}
pub struct SourceInputTransformAttempt<E> {
    operation: SourceInputTransformOperation,
    scalar_entry: Option<ExplicitExceptionState>, scalar_advanced: Option<ExplicitExceptionState>,
    log_entry: Option<ExplicitExceptionState>, log_advanced: Option<ExplicitExceptionState>,
    reported: Option<SourceInputTransformReport<E>>,
    scalar_projection: Option<SourceScalarF32Projection>, result_bits: Option<u32>,
}
impl<E> SourceInputTransformAttempt<E> {
    pub fn operation(&self) -> SourceInputTransformOperation { self.operation }
    pub fn scalar_entry(&self) -> Option<ExplicitExceptionState> { self.scalar_entry }
    pub fn scalar_advanced(&self) -> Option<ExplicitExceptionState> { self.scalar_advanced }
    pub fn log_entry(&self) -> Option<ExplicitExceptionState> { self.log_entry }
    pub fn log_advanced(&self) -> Option<ExplicitExceptionState> { self.log_advanced }
    pub fn reported(&self) -> Option<&SourceInputTransformReport<E>> { self.reported.as_ref() }
    pub fn scalar_projection(&self) -> Option<SourceScalarF32Projection> { self.scalar_projection }
    pub fn result_bits(&self) -> Option<u32> { self.result_bits }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceInputTransformCause {
    Context, ScalarService, ScalarState { mxcsr: u32 }, ScalarProjection,
    Log, UnexpectedLogReply,
}
pub struct SourceInputTransformFailure<E> { cause: SourceInputTransformCause, attempt: SourceInputTransformAttempt<E> }
impl<E> SourceInputTransformFailure<E> {
    pub fn cause(&self) -> SourceInputTransformCause { self.cause }
    pub fn attempt(&self) -> &SourceInputTransformAttempt<E> { &self.attempt }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceInputTransformStop { FirstFailure, InFlight }
#[derive(Clone, Copy, Debug)]
pub enum SourceInputTransformBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceInputTransformBindingFailure<C, B> {
    context: C, architecture: ScalarF32Architecture, scalar_entry: ExplicitExceptionState,
    log_math: SourceVorbisCodecMath, callback: B, cause: SourceInputTransformBindingCause,
}
impl<C, B> SourceInputTransformBindingFailure<C, B> {
    pub fn context(&self) -> &C { &self.context }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn scalar_entry(&self) -> ExplicitExceptionState { self.scalar_entry }
    pub fn log_math(&self) -> &SourceVorbisCodecMath { &self.log_math }
    pub fn callback(&self) -> &B { &self.callback }
    pub fn cause(&self) -> SourceInputTransformBindingCause { self.cause }
}

pub struct SourceInputTransformMath<C: RequiredInputTransformContext, B: SourceMathErrorCallback> {
    context: C, architecture: ScalarF32Architecture,
    scalar_entry: ExplicitExceptionState, scalar_state: ExplicitExceptionState, scalar: SourceScalarF32,
    log_math: SourceVorbisCodecMath, callback: B, pending: bool,
    latest: Option<SourceInputTransformAttempt<C::Error>>, failure: Option<SourceInputTransformFailure<C::Error>>,
}
impl<C: RequiredInputTransformContext, B: SourceMathErrorCallback> SourceInputTransformMath<C, B> {
    pub fn from_explicit_parts(context: C, architecture: ScalarF32Architecture, scalar_entry: ExplicitExceptionState,
        log_math: SourceVorbisCodecMath, callback: B) -> Result<Self, SourceInputTransformBindingFailure<C, B>>
    {
        let bind = || {
            if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceInputTransformBindingCause::Architecture); }
            if scalar_entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceInputTransformBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Mulss, ScalarF32Opcode::Addss])
                .map_err(SourceInputTransformBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(architecture, instructions).map_err(SourceInputTransformBindingCause::Scalar)
        };
        match bind() {
            Ok(scalar) => Ok(Self { context, architecture, scalar_entry, scalar_state: scalar_entry, scalar,
                log_math, callback, pending: false, latest: None, failure: None }),
            Err(cause) => Err(SourceInputTransformBindingFailure { context, architecture, scalar_entry, log_math, callback, cause }),
        }
    }
    pub fn context(&self) -> &C { &self.context }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn scalar_entry(&self) -> ExplicitExceptionState { self.scalar_entry }
    pub fn scalar_state(&self) -> ExplicitExceptionState { self.scalar_state }
    pub fn log_math(&self) -> &SourceVorbisCodecMath { &self.log_math }
    pub fn callback(&self) -> &B { &self.callback }
    pub fn pending(&self) -> bool { self.pending }
    pub fn failure(&self) -> Option<&SourceInputTransformFailure<C::Error>> { self.failure.as_ref() }
    pub fn latest_attempt(&self) -> Option<&SourceInputTransformAttempt<C::Error>> {
        match &self.failure { Some(failure) => Some(&failure.attempt), None => self.latest.as_ref() }
    }
    fn begin(&mut self, operation: SourceInputTransformOperation) -> Result<(), SourceInputTransformStop> {
        if self.failure.is_some() { return Err(SourceInputTransformStop::FirstFailure); }
        if self.pending { return Err(SourceInputTransformStop::InFlight); }
        self.pending = true;
        self.latest = Some(SourceInputTransformAttempt { operation, scalar_entry: None, scalar_advanced: None,
            log_entry: None, log_advanced: None, reported: None, scalar_projection: None, result_bits: None });
        Ok(())
    }
    fn stop(&mut self, cause: SourceInputTransformCause) -> SourceInputTransformStop {

        let attempt = self.latest.take().expect("armed actual provider attempt");
        self.failure = Some(SourceInputTransformFailure { cause, attempt });
        self.pending = false;
        SourceInputTransformStop::FirstFailure
    }
    fn context_call(&mut self, operation: SourceInputTransformOperation,
        call: impl FnOnce(&mut C) -> Result<SourceInputTransformContextReply, C::Error>)
        -> Result<SourceInputTransformContextReply, SourceInputTransformStop>
    {
        self.begin(operation)?;
        let reported = call(&mut self.context);
        self.latest.as_mut().expect("armed context").reported = Some(SourceInputTransformReport::Context(reported));
        let reply = match self.latest.as_ref().expect("saved context").reported.as_ref().expect("actual result") {
            SourceInputTransformReport::Context(Ok(reply)) => *reply,
            SourceInputTransformReport::Context(Err(_)) => return Err(self.stop(SourceInputTransformCause::Context)),
            _ => unreachable!(),
        };
        self.pending = false;
        Ok(reply)
    }
    fn binary(&mut self, operation: SourceInputTransformOperation, instruction: ScalarF32Operation)
        -> Result<u32, SourceInputTransformStop>
    {
        self.begin(operation)?;
        let entry = self.scalar_state;
        self.latest.as_mut().expect("armed scalar").scalar_entry = Some(entry);
        let before = match ScalarF32Mxcsr::from_explicit_word(entry.mxcsr()) {
            Some(before) => before,
            None => return Err(self.stop(SourceInputTransformCause::ScalarState { mxcsr: entry.mxcsr() })),
        };
        let reported = self.scalar.execute(instruction, before);
        self.latest.as_mut().expect("armed scalar").reported = Some(SourceInputTransformReport::Scalar(reported));
        let reported = match self.latest.as_ref().expect("saved scalar").reported.as_ref().expect("actual result") {
            SourceInputTransformReport::Scalar(Ok(reported)) => *reported,
            SourceInputTransformReport::Scalar(Err(_)) => return Err(self.stop(SourceInputTransformCause::ScalarService)),
            _ => unreachable!(),
        };
        let projection = project_scalar_f32_outcome(entry, reported);
        self.latest.as_mut().expect("saved scalar").scalar_projection = Some(projection);
        let (bits, advanced) = match projection {
            SourceScalarF32Projection::Completed { bits, advanced } => (Some(bits), advanced),
            SourceScalarF32Projection::Trapped { advanced, .. } => (None, advanced),
            SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => (None, known_advanced),
        };
        self.scalar_state = advanced;
        self.latest.as_mut().expect("saved scalar").scalar_advanced = Some(advanced);
        match bits {
            Some(bits) => {
                self.latest.as_mut().expect("saved scalar").result_bits = Some(bits);
                self.pending = false;
                Ok(bits)
            }
            None => Err(self.stop(SourceInputTransformCause::ScalarProjection)),
        }
    }
}

impl<C: RequiredInputTransformContext, B: SourceMathErrorCallback> Providers for SourceInputTransformMath<C, B> {
    type Error = SourceInputTransformStop;
    fn scratch_predicate(&mut self, argument: u32, scratch: Option<&[f32]>) -> Result<u8, Self::Error> {
        let operation = SourceInputTransformOperation::ScratchPredicate { argument, scratch_len: scratch.map(|row| row.len()) };
        match self.context_call(operation, |context| context.scratch_predicate(argument, scratch).map(SourceInputTransformContextReply::Byte))? {
            SourceInputTransformContextReply::Byte(value) => Ok(value), _ => unreachable!(),
        }
    }
    fn first_index(&mut self, mode: u32) -> Result<u32, Self::Error> {
        match self.context_call(SourceInputTransformOperation::FirstIndex { mode },
            |context| context.first_index(mode).map(SourceInputTransformContextReply::Word))? {
            SourceInputTransformContextReply::Word(value) => Ok(value), _ => unreachable!(),
        }
    }
    fn second_index(&mut self, mode: u32, width: u32) -> Result<u32, Self::Error> {
        match self.context_call(SourceInputTransformOperation::SecondIndex { mode, width },
            |context| context.second_index(mode, width).map(SourceInputTransformContextReply::Word))? {
            SourceInputTransformContextReply::Word(value) => Ok(value), _ => unreachable!(),
        }
    }
    fn constant(&mut self, constant: Constant) -> Result<u32, Self::Error> {
        match self.context_call(SourceInputTransformOperation::Constant(constant),
            |context| context.constant(constant).map(SourceInputTransformContextReply::Word))? {
            SourceInputTransformContextReply::Word(value) => Ok(value), _ => unreachable!(),
        }
    }
    fn logarithm(&mut self, input: u32) -> Result<u32, Self::Error> {
        self.begin(SourceInputTransformOperation::Logarithm { input })?;
        self.latest.as_mut().expect("armed log").log_entry = Some(self.log_math.exception_state());
        let reported = control::helper5834c0_with_callback(input, &mut self.log_math, &mut self.callback);
        self.latest.as_mut().expect("armed log").reported = Some(SourceInputTransformReport::Log(reported));
        self.latest.as_mut().expect("saved log").log_advanced = Some(self.log_math.exception_state());
        let bits = match self.latest.as_ref().expect("saved log").reported.as_ref().expect("actual result") {
            SourceInputTransformReport::Log(Ok(Reply::Word32(bits))) => *bits,
            SourceInputTransformReport::Log(Ok(_)) => return Err(self.stop(SourceInputTransformCause::UnexpectedLogReply)),
            SourceInputTransformReport::Log(Err(_)) => return Err(self.stop(SourceInputTransformCause::Log)),
            _ => unreachable!(),
        };
        self.latest.as_mut().expect("saved log").result_bits = Some(bits);
        self.pending = false;
        Ok(bits)
    }
    fn multiply(&mut self, destination: u32, source: u32) -> Result<u32, Self::Error> {
        self.binary(SourceInputTransformOperation::Multiply { destination, source },
            ScalarF32Operation::Mulss { left: destination, right: source })
    }
    fn add(&mut self, destination: u32, source: u32) -> Result<u32, Self::Error> {
        self.binary(SourceInputTransformOperation::Add { destination, source },
            ScalarF32Operation::Addss { left: destination, right: source })
    }
    fn transform_in_place(&mut self, receiver: u64, row: &mut [f32], flag: u8) -> Result<(), Self::Error> {
        let operation = SourceInputTransformOperation::Transform { receiver, row_len: row.len(), flag };
        match self.context_call(operation, |context| context.transform_in_place(receiver, row, flag).map(|()| SourceInputTransformContextReply::Unit))? {
            SourceInputTransformContextReply::Unit => Ok(()), _ => unreachable!(),
        }
    }
    fn update_history(&mut self, header: u64, history: &mut [u32], input: u32) -> Result<(), Self::Error> {
        let operation = SourceInputTransformOperation::History { header, history_len: history.len(), input };
        match self.context_call(operation, |context| context.update_history(header, history, input).map(|()| SourceInputTransformContextReply::Unit))? {
            SourceInputTransformContextReply::Unit => Ok(()), _ => unreachable!(),
        }
    }
    fn copy_unequal(&mut self, source: &[f32], destination: &mut Vec<f32>) -> Result<(), Self::Error> {
        let operation = SourceInputTransformOperation::CopyUnequal { source_len: source.len(), destination_len: destination.len() };
        match self.context_call(operation, |context| context.copy_unequal(source, destination).map(|()| SourceInputTransformContextReply::Unit))? {
            SourceInputTransformContextReply::Unit => Ok(()), _ => unreachable!(),
        }
    }
    fn stack_fill_byte(&mut self) -> Result<u8, Self::Error> {
        match self.context_call(SourceInputTransformOperation::StackFillByte,
            |context| context.stack_fill_byte().map(SourceInputTransformContextReply::Byte))? {
            SourceInputTransformContextReply::Byte(value) => Ok(value), _ => unreachable!(),
        }
    }
}
