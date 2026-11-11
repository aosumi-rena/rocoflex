use crate::source_scalar::{ExplicitExceptionState, SourceF64ValueMath, SourceF64ValueError,
    SourceF64ValueOperation, SourceF64ValueOperands, SourceF64ValueResult,
    SourceF64ValueCandidateReceipt, SourceF64ValueCompletion, SourceF64ValueFailure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass { First, Second }
#[derive(Clone, Copy, Debug)]
pub struct Request { pub pass: Pass, pub log_return_bits: u32 }
#[derive(Clone, Copy, Debug)]
pub struct Constants {
    pub multiply_bits: u64,
    pub add_bits: u64,

    pub first_extra_add_bits: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct RowRequest { pub log_return_bits: u32 }
#[derive(Clone, Copy, Debug)]
pub struct RowConstants {
    pub xmm10_multiply_bits: u64,
    pub xmm11_add_bits: u64,
}
#[derive(Clone, Copy, Debug)]
pub enum MethodResult {
    F64(Result<u64, SourceF64ValueError>),
    F32(Result<u32, SourceF64ValueError>),
}
#[derive(Clone, Copy, Debug)]
pub struct ProviderMetadata {
    pub exception_state: ExplicitExceptionState,

    pub last_candidate: Option<SourceF64ValueCandidateReceipt>,

    pub last_completion: Option<SourceF64ValueCompletion>,
    pub first_failure: Option<SourceF64ValueFailure>,
}
fn metadata(math: &SourceF64ValueMath) -> ProviderMetadata {
    ProviderMetadata { exception_state: math.exception_state(), last_candidate: math.last_candidate(),
        last_completion: math.last_completion(), first_failure: math.failure().copied() }
}
#[derive(Clone, Copy, Debug)]
pub struct Attempt {
    pub pc: u32, pub operation: SourceF64ValueOperation, pub operands: SourceF64ValueOperands,
    pub entry: ExplicitExceptionState, pub before: ProviderMetadata,

    pub reported: Option<MethodResult>, pub after: Option<ProviderMetadata>,

    pub current_completion: Option<SourceF64ValueCompletion>,

    pub error_known_advanced: Option<ExplicitExceptionState>,
}
pub type Ledger = [Option<Attempt>; 5];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationFailure { MissingCompletion, Operation, Operands, Entry, Advanced, Result, FailedProvider }
#[derive(Clone, Copy, Debug)]
pub enum Cause { Canonical(SourceF64ValueError), Association(AssociationFailure) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage { Widen, Multiply, Add, ExtraAdd, Narrow, Complete }
pub struct Progress {
    stage: Stage, next_pc: u32, completed_operations: usize,
    widened_bits: Option<u64>, multiplied_bits: Option<u64>, added_bits: Option<u64>,
    extra_added_bits: Option<u64>, narrowed_bits: Option<u32>,
}
impl Progress {
    fn new(profile: Profile) -> Self {
        Self { stage: Stage::Widen, next_pc: profile.pcs().widen, completed_operations: 0,
            widened_bits: None, multiplied_bits: None, added_bits: None, extra_added_bits: None, narrowed_bits: None }
    }
    pub fn stage(&self) -> Stage { self.stage }
    pub fn next_pc(&self) -> u32 { self.next_pc }
    pub fn completed_operations(&self) -> usize { self.completed_operations }
    pub fn widened_bits(&self) -> Option<u64> { self.widened_bits }
    pub fn multiplied_bits(&self) -> Option<u64> { self.multiplied_bits }
    pub fn added_bits(&self) -> Option<u64> { self.added_bits }
    pub fn extra_added_bits(&self) -> Option<u64> { self.extra_added_bits }
    pub fn narrowed_bits(&self) -> Option<u32> { self.narrowed_bits }
}
#[derive(Clone, Copy)]
enum Profile { Statistics(Pass), Row }
struct Inputs {
    log_return_bits: u32, multiply_bits: u64, add_bits: u64, extra_add_bits: Option<u64>,
}
struct Work<R, C> {
    request: R, constants: C, math: SourceF64ValueMath, ledger: Ledger, progress: Progress,
    profile: Profile, inputs: Inputs,
}

pub struct Failure<R = Request, C = Constants> { work: Work<R, C>, cause: Cause }
impl<R, C> Failure<R, C> {
    pub fn request(&self) -> R where R: Copy { self.work.request }
    pub fn constants(&self) -> C where C: Copy { self.work.constants }
    pub fn math(&self) -> &SourceF64ValueMath { &self.work.math }
    pub fn ledger(&self) -> &Ledger { &self.work.ledger }
    pub fn progress(&self) -> &Progress { &self.work.progress }
    pub fn cause(&self) -> Cause { self.cause }
}
pub struct Completed<R = Request, C = Constants> { work: Work<R, C>, result_bits: u32 }
pub struct NormalParts<R = Request, C = Constants> {
    pub request: R, pub constants: C, pub math: SourceF64ValueMath,
    pub ledger: Ledger, pub progress: Progress, pub result_bits: u32,
}
impl<R, C> Completed<R, C> {
    pub fn request(&self) -> R where R: Copy { self.work.request }
    pub fn constants(&self) -> C where C: Copy { self.work.constants }
    pub fn math(&self) -> &SourceF64ValueMath { &self.work.math }
    pub fn ledger(&self) -> &Ledger { &self.work.ledger }
    pub fn progress(&self) -> &Progress { &self.work.progress }
    pub fn result_bits(&self) -> u32 { self.result_bits }
    pub fn into_parts(self) -> NormalParts<R, C> {
        NormalParts { request: self.work.request, constants: self.work.constants, math: self.work.math,
            ledger: self.work.ledger, progress: self.work.progress, result_bits: self.result_bits }
    }
}

pub fn apply_log_suffix(request: Request, constants: Constants, math: SourceF64ValueMath) -> Result<Completed, Failure> {
    let profile = Profile::Statistics(request.pass);
    let inputs = Inputs { log_return_bits: request.log_return_bits, multiply_bits: constants.multiply_bits,
        add_bits: constants.add_bits, extra_add_bits: match request.pass {
            Pass::First => Some(constants.first_extra_add_bits), Pass::Second => None,
        } };
    run(Work { request, constants, math, ledger: [None; 5], progress: Progress::new(profile), profile, inputs })
}

pub fn apply_row_log_suffix(request: RowRequest, constants: RowConstants, math: SourceF64ValueMath)
    -> Result<Completed<RowRequest, RowConstants>, Failure<RowRequest, RowConstants>>
{
    let profile = Profile::Row;
    let inputs = Inputs { log_return_bits: request.log_return_bits,
        multiply_bits: constants.xmm10_multiply_bits, add_bits: constants.xmm11_add_bits, extra_add_bits: None };
    run(Work { request, constants, math, ledger: [None; 5], progress: Progress::new(profile), profile, inputs })
}
fn run<R, C>(mut work: Work<R, C>) -> Result<Completed<R, C>, Failure<R, C>> {
    match work.body() {
        Ok(result_bits) => Ok(Completed { work, result_bits }),
        Err(cause) => Err(Failure { work, cause }),
    }
}

#[derive(Clone, Copy)]
enum Value { F64(u64), F32(u32) }
impl<R, C> Work<R, C> {
    fn call(&mut self, slot: usize, pc: u32, operation: SourceF64ValueOperation, operands: SourceF64ValueOperands)
        -> Result<Value, Cause>
    {
        let before = metadata(&self.math);
        let mut attempt = Attempt { pc, operation, operands, entry: before.exception_state, before,
            reported: None, after: None, current_completion: None, error_known_advanced: None };
        self.ledger[slot] = Some(attempt);

        let reported = match (operation, operands) {
            (SourceF64ValueOperation::WidenF32, SourceF64ValueOperands::F32 { source }) =>
                MethodResult::F64(self.math.widen_f32_bits(source)),
            (SourceF64ValueOperation::Multiply, SourceF64ValueOperands::Binary { left, right }) =>
                MethodResult::F64(self.math.multiply_bits(left, right)),
            (SourceF64ValueOperation::Add, SourceF64ValueOperands::Binary { left, right }) =>
                MethodResult::F64(self.math.add_bits(left, right)),
            (SourceF64ValueOperation::NarrowF64, SourceF64ValueOperands::F64 { source }) =>
                MethodResult::F32(self.math.narrow_f64_bits(source)),
            _ => unreachable!(),
        };
        attempt.reported = Some(reported);
        self.ledger[slot] = Some(attempt);
        let after = metadata(&self.math);
        attempt.after = Some(after);
        self.ledger[slot] = Some(attempt);
        let value = match reported {
            MethodResult::F64(Ok(bits)) => Value::F64(bits),
            MethodResult::F32(Ok(bits)) => Value::F32(bits),
            MethodResult::F64(Err(error)) | MethodResult::F32(Err(error)) => {
                attempt.error_known_advanced = Some(error.known_advanced_state(attempt.entry));

                self.ledger[slot] = Some(attempt);
                return Err(Cause::Canonical(error));
            }
        };
        let expected_result = match value { Value::F64(bits) => SourceF64ValueResult::F64Bits(bits),
            Value::F32(bits) => SourceF64ValueResult::F32Bits(bits) };
        let completion = after.last_completion.ok_or(Cause::Association(AssociationFailure::MissingCompletion))?;
        let mismatch = if completion.operation != operation { Some(AssociationFailure::Operation) }
            else if completion.operands != operands { Some(AssociationFailure::Operands) }
            else if completion.entry != attempt.entry { Some(AssociationFailure::Entry) }
            else if completion.advanced != after.exception_state { Some(AssociationFailure::Advanced) }
            else if completion.result != expected_result { Some(AssociationFailure::Result) }
            else if after.first_failure.is_some() { Some(AssociationFailure::FailedProvider) }
            else { None };
        if let Some(reason) = mismatch { return Err(Cause::Association(reason)); }
        attempt.current_completion = Some(completion);
        self.ledger[slot] = Some(attempt);
        self.progress.completed_operations += 1;
        Ok(value)
    }
    fn f64(&mut self, slot: usize, pc: u32, operation: SourceF64ValueOperation, operands: SourceF64ValueOperands)
        -> Result<u64, Cause>
    {
        match self.call(slot, pc, operation, operands)? { Value::F64(bits) => Ok(bits), _ => unreachable!() }
    }
    fn body(&mut self) -> Result<u32, Cause> {
        let pcs = self.profile.pcs();
        let widened = self.f64(0, pcs.widen, SourceF64ValueOperation::WidenF32,
            SourceF64ValueOperands::F32 { source: self.inputs.log_return_bits })?;
        self.progress.widened_bits = Some(widened);
        self.progress.stage = Stage::Multiply;
        self.progress.next_pc = pcs.multiply;
        let multiplied = self.f64(1, pcs.multiply, SourceF64ValueOperation::Multiply,
            SourceF64ValueOperands::Binary { left: widened, right: self.inputs.multiply_bits })?;
        self.progress.multiplied_bits = Some(multiplied);
        self.progress.stage = Stage::Add;
        self.progress.next_pc = pcs.add;
        let added = self.f64(2, pcs.add, SourceF64ValueOperation::Add,
            SourceF64ValueOperands::Binary { left: multiplied, right: self.inputs.add_bits })?;
        self.progress.added_bits = Some(added);
        let before_narrow = match (pcs.extra_add, self.inputs.extra_add_bits) {
            (Some(pc), Some(right)) => {
                self.progress.stage = Stage::ExtraAdd;
                self.progress.next_pc = pc;
                let extra_added = self.f64(3, pc, SourceF64ValueOperation::Add,
                    SourceF64ValueOperands::Binary { left: added, right })?;
                self.progress.extra_added_bits = Some(extra_added);
                extra_added
            }
            (None, None) => added,
            _ => unreachable!(),
        };
        self.progress.stage = Stage::Narrow;
        self.progress.next_pc = pcs.narrow;
        let narrowed = match self.call(4, pcs.narrow, SourceF64ValueOperation::NarrowF64,
            SourceF64ValueOperands::F64 { source: before_narrow })? {
                Value::F32(bits) => bits, _ => unreachable!(),
            };
        self.progress.narrowed_bits = Some(narrowed);
        self.progress.stage = Stage::Complete;
        self.progress.next_pc = pcs.after;
        Ok(narrowed)
    }
}
struct Pcs { widen: u32, multiply: u32, add: u32, extra_add: Option<u32>, narrow: u32, after: u32 }
impl Profile {
    fn pcs(self) -> Pcs {
        match self {
            Profile::Statistics(pass) => pass.pcs(),
            Profile::Row => Pcs { widen: 0x552991, multiply: 0x552995, add: 0x55299a,
                extra_add: None, narrow: 0x55299f, after: 0x5529a3 },
        }
    }
}
impl Pass {
    fn pcs(self) -> Pcs {
        match self {
            Pass::First => Pcs { widen: 0x552a55, multiply: 0x552a59, add: 0x552a61,
                extra_add: Some(0x552a69), narrow: 0x552a71, after: 0x552a75 },
            Pass::Second => Pcs { widen: 0x552b8f, multiply: 0x552b93, add: 0x552b9b,
                extra_add: None, narrow: 0x552ba3, after: 0x552ba7 },
        }
    }
}
