use super::finite_power::{ExplicitExceptionState, PowerError};
use super::logical_f32_outcome::{project_scalar_f32_outcome,
    project_selection_f32_outcome, SourceScalarF32Projection, SourceSelectionF32Projection};
use super::selected_log::{raw_divss_with_explicit_state, LogArithmeticError, SourceLogWithState};
use super::shared_scalar_f32::{ScalarF32Error, ScalarF32Mxcsr, ScalarF32Operation,
    ScalarF32Outcome, ScalarF32SelectionOutcome, ScalarF32SelectionRequired, SourceScalarF32};

pub const SOURCE_RANGE_RECEIPTS: usize = 12;
const EPSILON_BITS: u32 = 0x3727_c5ac;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeDirection { Forward3d310, Inverse3d3b0 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeClamp { pub lower_bits: u32, pub upper_bits: u32 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeInput {
    pub direction: SourceRangeDirection,
    pub index_edx: u32,
    pub input_bits: u32,
    pub out_min_bits: u32,
    pub out_max_bits: u32,
    pub forward_clamp: Option<SourceRangeClamp>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeGetter { Low, High }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeGetterRequest {
    pub getter: SourceRangeGetter,
    pub call_rva: u32,
    pub index_edx: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeGetterAttempt {
    pub request: SourceRangeGetterRequest,
    pub receipt_index: usize,
    pub entry: ExplicitExceptionState,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeGetterReturn {
    pub request: SourceRangeGetterRequest,
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
}

pub struct SourceRangeGetterFailure<E> {
    pub request: SourceRangeGetterRequest,
    pub known_advanced: ExplicitExceptionState,
    pub cause: E,
}
pub trait SourceRangeGetters {
    type Error;
    fn getter(&mut self, request: SourceRangeGetterRequest, entry: ExplicitExceptionState)
        -> Result<SourceRangeGetterReturn, SourceRangeGetterFailure<Self::Error>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeStage {
    GetLow, GetHigh, InputDelta, InputSpan, AddEpsilon, Divide,
    ClampMaximum, ClampCompare, ClampSelect, OutputSpan, OutputProduct, OutputAdd,
    InverseProduct, InverseDivide, InverseAdd, Complete, Failed,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceRangeOperation {
    Getter(SourceRangeGetterRequest),
    Scalar(ScalarF32Operation),
    Divide { left: u32, right: u32 },
    Maximum { left: u32, right: u32 },
    LessMask { left: u32, right: u32 },
    RawSelect { mask: u32, maximum: u32, upper: u32, original_normalized: u32 },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceRangeReported {
    Getter(SourceRangeGetterReturn),
    GetterFailed { request: SourceRangeGetterRequest, known_advanced: ExplicitExceptionState },
    Scalar(Result<ScalarF32Outcome, ScalarF32Error>),
    Selection(Result<ScalarF32SelectionOutcome, ScalarF32Error>),

    Divide(Result<SourceLogWithState, LogArithmeticError>),
}
#[derive(Clone, Copy, Debug)]
pub struct SourceRangeReceipt {
    pub stage: SourceRangeStage,
    pub rva: u32,
    pub operation: SourceRangeOperation,
    pub entry: ExplicitExceptionState,
    pub reported: Option<SourceRangeReported>,
    pub result_bits: Option<u32>,
    pub known_advanced: ExplicitExceptionState,
}
pub enum SourceRangeCause<E> {
    Getter(SourceRangeGetterFailure<E>),
    GetterRequestMismatch { expected: SourceRangeGetterRequest, actual: SourceRangeGetterReturn },
    CurrentStateRequired { mxcsr: u32 },
    Scalar(ScalarF32Error),
    ScalarTrap { reported_mxcsr: u32, unmasked: u8 },
    ScalarState(PowerError),
    SelectionRequired(ScalarF32SelectionRequired),
    Divide(LogArithmeticError),
}
pub struct SourceRangeFailure<E> {
    pub receipt: SourceRangeReceipt,
    pub cause: SourceRangeCause<E>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeStopped;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeAdvance {
    Advanced { completed: SourceRangeStage, next: SourceRangeStage },
    Complete { bits: u32 },
}
#[derive(Clone, Copy, Debug)]
pub struct SourceRangeObservation {
    pub input: SourceRangeInput,
    pub bits: u32,
    pub exception_state: ExplicitExceptionState,
    pub receipts: [Option<SourceRangeReceipt>; SOURCE_RANGE_RECEIPTS],
    pub receipt_count: usize,
}

pub struct SourceRangeCompletion<S> {
    services: S,
    backend: SourceScalarF32,
    observation: SourceRangeObservation,
}
impl<S> SourceRangeCompletion<S> {
    pub fn observation(&self) -> &SourceRangeObservation { &self.observation }
    pub fn services(&self) -> &S { &self.services }
    pub fn into_parts(self) -> (S, SourceScalarF32, SourceRangeObservation) {
        (self.services, self.backend, self.observation)
    }
}

pub struct SourceRangeScalar<S: SourceRangeGetters> {
    services: S,
    backend: SourceScalarF32,
    input: SourceRangeInput,
    state: ExplicitExceptionState,
    stage: SourceRangeStage,
    receipts: [Option<SourceRangeReceipt>; SOURCE_RANGE_RECEIPTS],
    receipt_count: usize,
    pending_getter: Option<SourceRangeGetterAttempt>,
    failure: Option<SourceRangeFailure<S::Error>>,
    low: Option<u32>, high: Option<u32>, delta: Option<u32>, span: Option<u32>,
    denominator: Option<u32>, normalized: Option<u32>, maximum: Option<u32>,
    mask: Option<u32>, selected: Option<u32>, product: Option<u32>, result: Option<u32>,
}
impl<S: SourceRangeGetters> SourceRangeScalar<S> {

    pub fn from_explicit_parts(input: SourceRangeInput, backend: SourceScalarF32,
        state: ExplicitExceptionState, services: S) -> Self {
        Self { services, backend, input, state, stage: SourceRangeStage::GetLow,
            receipts: [None; SOURCE_RANGE_RECEIPTS], receipt_count: 0,
            pending_getter: None, failure: None,
            low: None, high: None, delta: None, span: None, denominator: None,
            normalized: None, maximum: None, mask: None, selected: None, product: None, result: None }
    }
    pub fn stage(&self) -> SourceRangeStage { self.stage }
    pub fn input(&self) -> SourceRangeInput { self.input }

    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn services(&self) -> &S { &self.services }

    pub fn pending_getter(&self) -> Option<SourceRangeGetterAttempt> { self.pending_getter }
    pub fn receipts(&self) -> &[Option<SourceRangeReceipt>] { &self.receipts[..self.receipt_count] }
    pub fn failure(&self) -> Option<&SourceRangeFailure<S::Error>> { self.failure.as_ref() }
    pub fn completed_bits(&self) -> Option<u32> {
        if self.stage == SourceRangeStage::Complete { self.result } else { None }
    }
    pub fn into_completed(self) -> Result<SourceRangeCompletion<S>, Self> {
        if self.stage != SourceRangeStage::Complete || self.failure.is_some()
            || self.pending_getter.is_some() { return Err(self); }
        let observation = SourceRangeObservation { input: self.input,
            bits: self.result.expect("completed literal final ADDSS"), exception_state: self.state,
            receipts: self.receipts, receipt_count: self.receipt_count };
        Ok(SourceRangeCompletion { services: self.services, backend: self.backend, observation })
    }
    fn begin(&mut self, rva: u32, operation: SourceRangeOperation) -> usize {

        let index = self.receipt_count;
        self.receipts[index] = Some(SourceRangeReceipt { stage: self.stage, rva, operation,
            entry: self.state, reported: None, result_bits: None, known_advanced: self.state });
        self.receipt_count += 1;
        index
    }
    fn report(&mut self, index: usize, reported: SourceRangeReported) {
        self.receipts[index].as_mut().expect("reserved fixed source receipt").reported = Some(reported);
    }
    fn commit(&mut self, index: usize, bits: u32, advanced: ExplicitExceptionState) -> u32 {
        self.state = advanced;
        let receipt = self.receipts[index].as_mut().expect("actual source receipt");
        receipt.result_bits = Some(bits); receipt.known_advanced = advanced;
        bits
    }
    fn stop(&mut self, index: usize, advanced: ExplicitExceptionState, cause: SourceRangeCause<S::Error>) {
        self.state = advanced;
        let receipt = self.receipts[index].as_mut().expect("actual stopped receipt");
        receipt.known_advanced = advanced;
        self.failure = Some(SourceRangeFailure { receipt: *receipt, cause });
        self.stage = SourceRangeStage::Failed;
    }
    fn before(&mut self, index: usize) -> Option<ScalarF32Mxcsr> {
        match ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr()) {
            Some(before) => Some(before),
            None => { self.stop(index, self.state,
                SourceRangeCause::CurrentStateRequired { mxcsr: self.state.mxcsr() }); None }
        }
    }
    fn scalar_result(&mut self, index: usize, reported: Result<ScalarF32Outcome, ScalarF32Error>) -> Option<u32> {
        self.report(index, SourceRangeReported::Scalar(reported));
        let outcome = match reported {
            Ok(outcome) => outcome,
            Err(error) => { self.stop(index, self.state, SourceRangeCause::Scalar(error)); return None; }
        };
        match project_scalar_f32_outcome(self.state, outcome) {
            SourceScalarF32Projection::Completed { bits, advanced } => Some(self.commit(index, bits, advanced)),
            SourceScalarF32Projection::Trapped { reported_mxcsr, unmasked, advanced } => {
                self.stop(index, advanced, SourceRangeCause::ScalarTrap { reported_mxcsr, unmasked }); None
            }
            SourceScalarF32Projection::ReportedStateRequired { reason, known_advanced, .. } => {
                self.stop(index, known_advanced, SourceRangeCause::ScalarState(reason)); None
            }
        }
    }
    fn scalar(&mut self, rva: u32, operation: ScalarF32Operation) -> Option<u32> {
        let index = self.begin(rva, SourceRangeOperation::Scalar(operation));
        let before = self.before(index)?;
        let reported = self.backend.execute(operation, before);
        self.scalar_result(index, reported)
    }
    fn getter(&mut self, getter: SourceRangeGetter) -> Option<u32> {
        let call_rva = match (self.input.direction, getter) {
            (SourceRangeDirection::Forward3d310, SourceRangeGetter::Low) => 0x3d333,
            (SourceRangeDirection::Forward3d310, SourceRangeGetter::High) => 0x3d342,
            (SourceRangeDirection::Inverse3d3b0, SourceRangeGetter::Low) => 0x3d3cf,
            (SourceRangeDirection::Inverse3d3b0, SourceRangeGetter::High) => 0x3d3de,
        };
        let request = SourceRangeGetterRequest { getter, call_rva, index_edx: self.input.index_edx };
        let index = self.begin(call_rva, SourceRangeOperation::Getter(request));
        self.pending_getter = Some(SourceRangeGetterAttempt { request, receipt_index: index, entry: self.state });
        let reported = self.services.getter(request, self.state);
        match reported {
            Ok(actual) => {
                self.report(index, SourceRangeReported::Getter(actual));
                self.pending_getter = None;

                if actual.request != request {
                    self.stop(index, actual.exception_state, SourceRangeCause::GetterRequestMismatch { expected: request, actual });
                    None
                } else { Some(self.commit(index, actual.bits, actual.exception_state)) }
            }
            Err(actual) => {
                self.report(index, SourceRangeReported::GetterFailed { request: actual.request, known_advanced: actual.known_advanced });
                self.pending_getter = None;
                self.stop(index, actual.known_advanced, SourceRangeCause::Getter(actual));
                None
            }
        }
    }
    fn divide(&mut self, rva: u32, left: u32, right: u32) -> Option<u32> {
        let index = self.begin(rva, SourceRangeOperation::Divide { left, right });
        let reported = raw_divss_with_explicit_state(left, right, self.state, self.backend.architecture());
        self.report(index, SourceRangeReported::Divide(reported));
        match reported {
            Ok(actual) => Some(self.commit(index, actual.bits, actual.exception_state)),
            Err(error) => {
                let advanced = match error {
                    LogArithmeticError::Dispatch(_) => self.state,
                    LogArithmeticError::ArithmeticRequired { advanced, .. }
                    | LogArithmeticError::ScalarServiceRequired { advanced, .. }
                    | LogArithmeticError::UnexpectedScalarState { advanced, .. }
                    | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => advanced,
                };
                self.stop(index, advanced, SourceRangeCause::Divide(error)); None
            }
        }
    }
    fn maximum(&mut self, left: u32, right: u32) -> Option<u32> {
        let index = self.begin(0x3d365, SourceRangeOperation::Maximum { left, right });
        let before = self.before(index)?;
        let reported = self.backend.execute_maxss(left, right, before);
        self.report(index, SourceRangeReported::Selection(reported));
        let outcome = match reported {
            Ok(outcome) => outcome,
            Err(error) => { self.stop(index, self.state, SourceRangeCause::Scalar(error)); return None; }
        };
        match project_selection_f32_outcome(self.state, outcome) {
            SourceSelectionF32Projection::Completed { bits, advanced } => Some(self.commit(index, bits, advanced)),
            SourceSelectionF32Projection::Trapped { reported_mxcsr, unmasked, advanced } => {
                self.stop(index, advanced, SourceRangeCause::ScalarTrap { reported_mxcsr, unmasked }); None
            }
            SourceSelectionF32Projection::Required { required, advanced } => {
                self.stop(index, advanced, SourceRangeCause::SelectionRequired(required)); None
            }
            SourceSelectionF32Projection::ReportedStateRequired { reason, known_advanced, .. } => {
                self.stop(index, known_advanced, SourceRangeCause::ScalarState(reason)); None
            }
        }
    }
    fn less_mask(&mut self, left: u32, right: u32) -> Option<u32> {
        let index = self.begin(0x3d374, SourceRangeOperation::LessMask { left, right });
        let before = self.before(index)?;
        let reported = self.backend.execute_cmpltss(left, right, before);
        self.scalar_result(index, reported)
    }
    pub fn poll(&mut self) -> Result<SourceRangeAdvance, SourceRangeStopped> {
        use ScalarF32Operation as O;
        use SourceRangeDirection as D;
        use SourceRangeStage as P;

        if self.pending_getter.is_some() || self.failure.is_some() || self.stage == P::Failed {
            return Err(SourceRangeStopped);
        }
        if self.stage == P::Complete {
            return Ok(SourceRangeAdvance::Complete { bits: self.result.expect("completed final ADDSS") });
        }
        let completed = self.stage;
        let forward = self.input.direction == D::Forward3d310;
        match self.stage {
            P::GetLow => if let Some(bits) = self.getter(SourceRangeGetter::Low) { self.low = Some(bits); self.stage = P::GetHigh; },
            P::GetHigh => if let Some(bits) = self.getter(SourceRangeGetter::High) { self.high = Some(bits); self.stage = P::InputDelta; },
            P::InputDelta => {
                let right = if forward { self.low.expect("actual low getter") } else { self.input.out_min_bits };
                if let Some(bits) = self.scalar(if forward { 0x3d345 } else { 0x3d3eb },
                    O::Subss { left: self.input.input_bits, right }) { self.delta = Some(bits); self.stage = P::InputSpan; }
            }
            P::InputSpan => {
                if let Some(bits) = self.scalar(if forward { 0x3d349 } else { 0x3d3ef },
                    O::Subss { left: self.high.expect("actual high getter"), right: self.low.expect("actual low getter") }) {
                    self.span = Some(bits); self.stage = if forward { P::AddEpsilon } else { P::InverseProduct };
                }
            }
            P::AddEpsilon => if let Some(bits) = self.scalar(0x3d34d,
                O::Addss { left: self.span.expect("completed input span"), right: EPSILON_BITS }) {
                self.denominator = Some(bits); self.stage = P::Divide;
            },
            P::Divide => if let Some(bits) = self.divide(0x3d355, self.delta.expect("completed input delta"), self.denominator.expect("completed epsilon ADDSS")) {
                self.normalized = Some(bits); self.selected = Some(bits);
                self.stage = if self.input.forward_clamp.is_some() { P::ClampMaximum } else { P::OutputSpan };
            },
            P::ClampMaximum => {
                let clamp = self.input.forward_clamp.expect("actual forward clamp branch");
                if let Some(bits) = self.maximum(clamp.lower_bits, self.normalized.expect("original normalized value")) {
                    self.maximum = Some(bits); self.stage = P::ClampCompare;
                }
            }
            P::ClampCompare => {
                let clamp = self.input.forward_clamp.expect("actual forward clamp branch");
                if let Some(bits) = self.less_mask(clamp.upper_bits, self.normalized.expect("ORIGINAL normalized value")) {
                    self.mask = Some(bits); self.stage = P::ClampSelect;
                }
            }
            P::ClampSelect => {
                let upper = self.input.forward_clamp.expect("actual clamp branch").upper_bits;
                let mask = self.mask.expect("completed signaling CMPLTSS");
                let maximum = self.maximum.expect("completed MAXSS");
                let index = self.begin(0x3d379, SourceRangeOperation::RawSelect { mask, maximum, upper,
                    original_normalized: self.normalized.expect("original normalized value") });

                let bits = (!mask & maximum) | (mask & upper);
                self.selected = Some(self.commit(index, bits, self.state)); self.stage = P::OutputSpan;
            }
            P::OutputSpan => if let Some(bits) = self.scalar(if forward { 0x3d392 } else { 0x3d3f7 },
                O::Subss { left: self.input.out_max_bits, right: self.input.out_min_bits }) {
                self.denominator = Some(bits); self.stage = if forward { P::OutputProduct } else { P::InverseDivide };
            },
            P::OutputProduct => if let Some(bits) = self.scalar(0x3d396,
                O::Mulss { left: self.denominator.expect("per-invocation output span"), right: self.selected.expect("normalized/clamped value") }) {
                self.product = Some(bits); self.stage = P::OutputAdd;
            },
            P::OutputAdd => if let Some(bits) = self.scalar(0x3d39a,
                O::Addss { left: self.product.expect("completed output product"), right: self.input.out_min_bits }) {
                self.result = Some(bits); self.stage = P::Complete;
            },
            P::InverseProduct => if let Some(bits) = self.scalar(0x3d3f3,
                O::Mulss { left: self.span.expect("completed input span"), right: self.delta.expect("completed inverse delta") }) {
                self.product = Some(bits); self.stage = P::OutputSpan;
            },
            P::InverseDivide => if let Some(bits) = self.divide(0x3d3fb,
                self.product.expect("completed inverse product"), self.denominator.expect("per-invocation denominator SUBSS")) {
                self.normalized = Some(bits); self.stage = P::InverseAdd;
            },
            P::InverseAdd => if let Some(bits) = self.scalar(0x3d3ff,
                O::Addss { left: self.normalized.expect("completed inverse DIVSS"), right: self.low.expect("actual low getter") }) {
                self.result = Some(bits); self.stage = P::Complete;
            },
            P::Complete | P::Failed => unreachable!("terminal stages handled before source effects"),
        }
        if self.failure.is_some() { Err(SourceRangeStopped) }
        else if self.stage == P::Complete { Ok(SourceRangeAdvance::Complete { bits: self.result.expect("completed final ADDSS") }) }
        else { Ok(SourceRangeAdvance::Advanced { completed, next: self.stage }) }
    }
}
