use super::finite_power::ExplicitExceptionState;
use super::range_scalar::{SourceRangeClamp, SourceRangeCompletion, SourceRangeDirection,
    SourceRangeGetterFailure, SourceRangeGetterRequest, SourceRangeGetterReturn,
    SourceRangeGetters, SourceRangeInput, SourceRangeObservation, SourceRangeScalar};
use super::shared_scalar_f32::SourceScalarF32;
use std::collections::TryReserveError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeRowsCall { OutputCount, InputGet, ScalarSlot8, OutputSet }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeRowsRequest {
    pub kind: SourceRangeRowsCall,
    pub call_rva: u32,
    pub loop_index: u32,

    pub argument_edx: Option<u32>,
    pub original_clamp_word: u32,
    pub value_bits: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceInverseRangeRowsCall { OutputCount, InputGet, ScalarSlot18, OutputSet }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceInverseRangeRowsRequest {
    pub kind: SourceInverseRangeRowsCall,
    pub call_rva: u32,
    pub loop_index: u32,
    pub argument_edx: Option<u32>,
    pub value_bits: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeRowsValue { CountEax(u32), Bits(u32), Stored }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeRowsReturn<Q = SourceRangeRowsRequest> {
    pub request: Q,
    pub value: SourceRangeRowsValue,
}
pub struct SourceRangeRowsServiceFailure<E, Q = SourceRangeRowsRequest> {
    pub request: Q,
    pub cause: E,
}

pub trait RequiredRangeRowServices {
    type Error;

    fn call(&mut self, request: SourceRangeRowsRequest, scalar: Option<&SourceRangeObservation>)
        -> Result<SourceRangeRowsReturn, SourceRangeRowsServiceFailure<Self::Error>>;
}

pub trait RequiredInverseRangeRowServices {
    type Error;

    fn call(&mut self, request: SourceInverseRangeRowsRequest, scalar: Option<&SourceRangeObservation>)
        -> Result<SourceRangeRowsReturn<SourceInverseRangeRowsRequest>,
            SourceRangeRowsServiceFailure<Self::Error, SourceInverseRangeRowsRequest>>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeRowsStage { InitialCount, InputGet, ScalarRequired, OutputSet, FreshCount, Complete, Failed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeRowsAttempt<Q = SourceRangeRowsRequest> { pub request: Q, pub receipt_index: usize }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeRowsReceipt<Q = SourceRangeRowsRequest> {
    pub request: Q,
    pub actual: Option<SourceRangeRowsReturn<Q>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeRowsProgress<Q = SourceRangeRowsRequest> {
    pub stage: SourceRangeRowsStage,
    pub index: u32,
    pub last_count_eax: Option<u32>,
    pub current_input_bits: Option<u32>,
    pub current_result_bits: Option<u32>,
    pub pending: Option<SourceRangeRowsAttempt<Q>>,
    pub receipt_count: usize,
    pub scalar_completions: usize,
}
pub enum SourceRangeRowsCause<E, Q = SourceRangeRowsRequest> {
    Service(SourceRangeRowsServiceFailure<E, Q>),
    RequestMismatch(SourceRangeRowsReturn<Q>),
    ReturnShape(SourceRangeRowsReturn<Q>),
    DiagnosticBudget { limit: usize },
    ReceiptCapacity(TryReserveError),
    ScalarHistoryCapacity(TryReserveError),
}
pub struct SourceRangeRowsFailure<E, Q = SourceRangeRowsRequest> {
    pub stage: SourceRangeRowsStage,
    pub request: Q,
    pub receipt_index: Option<usize>,
    pub cause: SourceRangeRowsCause<E, Q>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRangeRowsStopped;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeRowsAdvance<Q = SourceRangeRowsRequest> {
    Advanced(SourceRangeRowsProgress<Q>),
    RequiredScalar(Q),
    Complete,
}

pub struct SourceRangeRowsForwardBinding {
    pub backend: SourceScalarF32,
    pub entry: ExplicitExceptionState,
    pub out_min_bits: u32,
    pub out_max_bits: u32,
    pub clamp: Option<SourceRangeClamp>,
}

pub struct SourceRangeRowsInverseBinding {
    pub backend: SourceScalarF32,
    pub entry: ExplicitExceptionState,
    pub out_min_bits: u32,
    pub out_max_bits: u32,
}
pub struct SourceRangeRowsScalarRecord<Q = SourceRangeRowsRequest> {
    backend: SourceScalarF32,
    observation: SourceRangeObservation,
    actual: SourceRangeRowsReturn<Q>,
}
impl<Q: Copy> SourceRangeRowsScalarRecord<Q> {
    pub fn observation(&self) -> &SourceRangeObservation { &self.observation }
    pub fn actual_return(&self) -> SourceRangeRowsReturn<Q> { self.actual }
    pub fn backend(&self) -> &SourceScalarF32 { &self.backend }
}
pub struct SourceRangeRowsObservation {
    pub original_clamp_word: u32,
    pub index: u32,
    pub last_count_eax: Option<u32>,
    pub receipts: Vec<SourceRangeRowsReceipt>,
    pub scalar_completions: Vec<SourceRangeRowsScalarRecord>,
}
pub struct SourceRangeRowsCompletion<S> { services: S, observation: SourceRangeRowsObservation }
impl<S> SourceRangeRowsCompletion<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn observation(&self) -> &SourceRangeRowsObservation { &self.observation }
    pub fn into_parts(self) -> (S, SourceRangeRowsObservation) { (self.services, self.observation) }
}

trait RowRoute<S> {
    type Request: Copy + Eq;
    type Error;
    fn request(&self, stage: SourceRangeRowsStage, index: u32, input: Option<u32>, result: Option<u32>)
        -> Option<Self::Request>;
    fn call(&self, services: &mut S, request: Self::Request, scalar: Option<&SourceRangeObservation>)
        -> Result<SourceRangeRowsReturn<Self::Request>, SourceRangeRowsServiceFailure<Self::Error, Self::Request>>;
}
struct ForwardRoute { clamp_word: u32 }
struct InverseRoute;
impl<S: RequiredRangeRowServices> RowRoute<S> for ForwardRoute {
    type Request = SourceRangeRowsRequest;
    type Error = S::Error;
    fn request(&self, stage: SourceRangeRowsStage, index: u32, input: Option<u32>, result: Option<u32>)
        -> Option<SourceRangeRowsRequest> {
        use SourceRangeRowsCall as C;
        use SourceRangeRowsStage as P;
        let (kind, call_rva, argument_edx, value_bits) = match stage {
            P::InitialCount => (C::OutputCount, 0x3d04c, None, None),
            P::InputGet => (C::InputGet, 0x3d068, Some(index), None),
            P::ScalarRequired => (C::ScalarSlot8, 0x3d079, Some(index), input),
            P::OutputSet => (C::OutputSet, 0x3d087, Some(index), result),
            P::FreshCount => (C::OutputCount, 0x3d092, None, None),
            P::Complete | P::Failed => return None,
        };
        Some(SourceRangeRowsRequest { kind, call_rva, loop_index: index,
            argument_edx, original_clamp_word: self.clamp_word, value_bits })
    }
    fn call(&self, services: &mut S, request: SourceRangeRowsRequest, scalar: Option<&SourceRangeObservation>)
        -> Result<SourceRangeRowsReturn, SourceRangeRowsServiceFailure<S::Error>> {
        services.call(request, scalar)
    }
}
impl<S: RequiredInverseRangeRowServices> RowRoute<S> for InverseRoute {
    type Request = SourceInverseRangeRowsRequest;
    type Error = S::Error;
    fn request(&self, stage: SourceRangeRowsStage, index: u32, input: Option<u32>, result: Option<u32>)
        -> Option<SourceInverseRangeRowsRequest> {
        use SourceInverseRangeRowsCall as C;
        use SourceRangeRowsStage as P;
        let (kind, call_rva, argument_edx, value_bits) = match stage {
            P::InitialCount => (C::OutputCount, 0x3d0c7, None, None),
            P::InputGet => (C::InputGet, 0x3d0d8, Some(index), None),
            P::ScalarRequired => (C::ScalarSlot18, 0x3d0e6, Some(index), input),
            P::OutputSet => (C::OutputSet, 0x3d0f4, Some(index), result),
            P::FreshCount => (C::OutputCount, 0x3d0ff, None, None),
            P::Complete | P::Failed => return None,
        };
        Some(SourceInverseRangeRowsRequest { kind, call_rva, loop_index: index, argument_edx, value_bits })
    }
    fn call(&self, services: &mut S, request: SourceInverseRangeRowsRequest, scalar: Option<&SourceRangeObservation>)
        -> Result<SourceRangeRowsReturn<SourceInverseRangeRowsRequest>,
            SourceRangeRowsServiceFailure<S::Error, SourceInverseRangeRowsRequest>> {
        services.call(request, scalar)
    }
}
struct RowsCoreCompletion<S, R: RowRoute<S>> {
    services: S, route: R, index: u32, last_count: Option<u32>,
    receipts: Vec<SourceRangeRowsReceipt<R::Request>>,
    scalar_completions: Vec<SourceRangeRowsScalarRecord<R::Request>>,
}
struct RowsCore<S, R: RowRoute<S>> {
    services: S,
    route: R,
    diagnostic_budget: usize,
    stage: SourceRangeRowsStage,
    index: u32,
    last_count: Option<u32>,
    input: Option<u32>,
    result: Option<u32>,
    pending: Option<SourceRangeRowsAttempt<R::Request>>,
    receipts: Vec<SourceRangeRowsReceipt<R::Request>>,
    scalar_completions: Vec<SourceRangeRowsScalarRecord<R::Request>>,
    failure: Option<SourceRangeRowsFailure<R::Error, R::Request>>,
}
impl<S, R: RowRoute<S>> RowsCore<S, R> {
    fn new(services: S, route: R, diagnostic_budget: usize) -> Self {
        Self { services, route, diagnostic_budget,
            stage: SourceRangeRowsStage::InitialCount, index: 0, last_count: None,
            input: None, result: None, pending: None, receipts: Vec::new(),
            scalar_completions: Vec::new(), failure: None }
    }
    fn progress(&self) -> SourceRangeRowsProgress<R::Request> {
        SourceRangeRowsProgress { stage: self.stage, index: self.index, last_count_eax: self.last_count,
            current_input_bits: self.input, current_result_bits: self.result, pending: self.pending,
            receipt_count: self.receipts.len(), scalar_completions: self.scalar_completions.len() }
    }
    fn into_completed(self) -> Result<RowsCoreCompletion<S, R>, Self> {
        if self.stage != SourceRangeRowsStage::Complete || self.pending.is_some() || self.failure.is_some() {
            return Err(self);
        }
        Ok(RowsCoreCompletion { services: self.services, route: self.route, index: self.index,
            last_count: self.last_count, receipts: self.receipts, scalar_completions: self.scalar_completions })
    }
    fn request(&self) -> Option<R::Request> {
        self.route.request(self.stage, self.index, self.input, self.result)
    }
    fn reserve_receipt(&mut self) -> Result<(), SourceRangeRowsCause<R::Error, R::Request>> {
        if self.receipts.len() >= self.diagnostic_budget {
            return Err(SourceRangeRowsCause::DiagnosticBudget { limit: self.diagnostic_budget });
        }
        self.receipts.try_reserve(1).map_err(SourceRangeRowsCause::ReceiptCapacity)
    }
    fn arm(&mut self, request: R::Request) -> usize {
        let receipt_index = self.receipts.len();
        self.receipts.push(SourceRangeRowsReceipt { request, actual: None });
        self.pending = Some(SourceRangeRowsAttempt { request, receipt_index });
        receipt_index
    }
    fn stop(&mut self, request: R::Request, receipt_index: Option<usize>,
        cause: SourceRangeRowsCause<R::Error, R::Request>) {
        self.failure = Some(SourceRangeRowsFailure { stage: self.stage, request, receipt_index, cause });
        self.stage = SourceRangeRowsStage::Failed;
    }
    fn poll(&mut self) -> Result<SourceRangeRowsAdvance<R::Request>, SourceRangeRowsStopped> {
        use SourceRangeRowsStage as P;
        if self.pending.is_some() || self.failure.is_some() || self.stage == P::Failed {
            return Err(SourceRangeRowsStopped);
        }
        if self.stage == P::Complete { return Ok(SourceRangeRowsAdvance::Complete); }
        let request = self.request().expect("nonterminal literal row source request");
        if self.stage == P::ScalarRequired { return Ok(SourceRangeRowsAdvance::RequiredScalar(request)); }
        if let Err(cause) = self.reserve_receipt() {
            self.stop(request, None, cause); return Err(SourceRangeRowsStopped);
        }
        let slot = self.arm(request);

        let scalar = if self.stage == P::OutputSet {
            Some(&self.scalar_completions.last().expect("same completed scalar before SET").observation)
        } else { None };
        let reported = self.route.call(&mut self.services, request, scalar);
        let actual = match reported {
            Ok(actual) => { self.receipts[slot].actual = Some(actual); self.pending = None; actual }
            Err(actual) => {
                self.pending = None;
                self.stop(request, Some(slot), SourceRangeRowsCause::Service(actual));
                return Err(SourceRangeRowsStopped);
            }
        };
        if actual.request != request {
            self.stop(request, Some(slot), SourceRangeRowsCause::RequestMismatch(actual));
            return Err(SourceRangeRowsStopped);
        }
        match (self.stage, actual.value) {
            (P::InitialCount, SourceRangeRowsValue::CountEax(count)) => {
                self.last_count = Some(count);
                self.stage = if (count as i32) <= 0 { P::Complete } else { P::InputGet };
            }
            (P::InputGet, SourceRangeRowsValue::Bits(bits)) => { self.input = Some(bits); self.stage = P::ScalarRequired; }
            (P::OutputSet, SourceRangeRowsValue::Stored) => {
                self.index = self.index.wrapping_add(1); self.stage = P::FreshCount;
            }
            (P::FreshCount, SourceRangeRowsValue::CountEax(count)) => {
                self.last_count = Some(count);
                if (self.index as i32) < (count as i32) {
                    self.input = None; self.result = None; self.stage = P::InputGet;
                } else { self.stage = P::Complete; }
            }
            _ => { self.stop(request, Some(slot), SourceRangeRowsCause::ReturnShape(actual)); return Err(SourceRangeRowsStopped); }
        }
        if self.stage == P::Complete { Ok(SourceRangeRowsAdvance::Complete) }
        else { Ok(SourceRangeRowsAdvance::Advanced(self.progress())) }
    }
    fn scalar_ready(&self) -> bool {
        self.stage == SourceRangeRowsStage::ScalarRequired && self.pending.is_none() && self.failure.is_none()
    }
    fn reserve_scalar(&mut self, request: R::Request) -> Result<(), SourceRangeRowsScalarAdmissionReason> {
        if let Err(cause) = self.reserve_receipt() {
            self.stop(request, None, cause);
            return Err(SourceRangeRowsScalarAdmissionReason::DiagnosticStopped);
        }
        if let Err(cause) = self.scalar_completions.try_reserve(1) {
            self.stop(request, None, SourceRangeRowsCause::ScalarHistoryCapacity(cause));
            return Err(SourceRangeRowsScalarAdmissionReason::DiagnosticStopped);
        }
        Ok(())
    }
    fn scalar_pending_matches(&self, expected: R::Request) -> bool {
        self.stage == SourceRangeRowsStage::ScalarRequired
            && self.pending.map(|a| a.request) == Some(expected)
    }
    fn accept_scalar(&mut self, backend: SourceScalarF32, observation: SourceRangeObservation,
        actual: SourceRangeRowsReturn<R::Request>) {
        let attempt = self.pending.expect("same armed scalar request");
        self.receipts[attempt.receipt_index].actual = Some(actual);

        self.scalar_completions.push(SourceRangeRowsScalarRecord { backend, observation, actual });
        self.pending = None;
        self.result = Some(observation.bits);
        self.stage = SourceRangeRowsStage::OutputSet;
    }
}

pub struct SourceRangeRows<S: RequiredRangeRowServices> { core: RowsCore<S, ForwardRoute> }
impl<S: RequiredRangeRowServices> SourceRangeRows<S> {

    pub fn from_explicit_parts(services: S, original_clamp_word: u32, diagnostic_budget: usize) -> Self {
        Self { core: RowsCore::new(services, ForwardRoute { clamp_word: original_clamp_word }, diagnostic_budget) }
    }
    pub fn progress(&self) -> SourceRangeRowsProgress { self.core.progress() }
    pub fn services(&self) -> &S { &self.core.services }
    pub fn receipts(&self) -> &[SourceRangeRowsReceipt] { &self.core.receipts }
    pub fn scalar_completions(&self) -> &[SourceRangeRowsScalarRecord] { &self.core.scalar_completions }
    pub fn failure(&self) -> Option<&SourceRangeRowsFailure<S::Error>> { self.core.failure.as_ref() }
    pub fn poll(&mut self) -> Result<SourceRangeRowsAdvance, SourceRangeRowsStopped> { self.core.poll() }
    pub fn into_completed(self) -> Result<SourceRangeRowsCompletion<S>, Self> {
        match self.core.into_completed() {
            Ok(completed) => Ok(SourceRangeRowsCompletion { services: completed.services,
                observation: SourceRangeRowsObservation { original_clamp_word: completed.route.clamp_word,
                    index: completed.index, last_count_eax: completed.last_count,
                    receipts: completed.receipts, scalar_completions: completed.scalar_completions } }),
            Err(core) => Err(Self { core }),
        }
    }
}
pub struct SourceInverseRangeRowsObservation {
    pub index: u32,
    pub last_count_eax: Option<u32>,
    pub receipts: Vec<SourceRangeRowsReceipt<SourceInverseRangeRowsRequest>>,
    pub scalar_completions: Vec<SourceRangeRowsScalarRecord<SourceInverseRangeRowsRequest>>,
}
pub struct SourceInverseRangeRowsCompletion<S> { services: S, observation: SourceInverseRangeRowsObservation }
impl<S> SourceInverseRangeRowsCompletion<S> {
    pub fn services(&self) -> &S { &self.services }
    pub fn observation(&self) -> &SourceInverseRangeRowsObservation { &self.observation }
    pub fn into_parts(self) -> (S, SourceInverseRangeRowsObservation) { (self.services, self.observation) }
}

pub struct SourceInverseRangeRows<S: RequiredInverseRangeRowServices> { core: RowsCore<S, InverseRoute> }
impl<S: RequiredInverseRangeRowServices> SourceInverseRangeRows<S> {
    pub fn from_explicit_parts(services: S, diagnostic_budget: usize) -> Self {
        Self { core: RowsCore::new(services, InverseRoute, diagnostic_budget) }
    }
    pub fn progress(&self) -> SourceRangeRowsProgress<SourceInverseRangeRowsRequest> { self.core.progress() }
    pub fn services(&self) -> &S { &self.core.services }
    pub fn receipts(&self) -> &[SourceRangeRowsReceipt<SourceInverseRangeRowsRequest>] { &self.core.receipts }
    pub fn scalar_completions(&self) -> &[SourceRangeRowsScalarRecord<SourceInverseRangeRowsRequest>] {
        &self.core.scalar_completions
    }
    pub fn failure(&self) -> Option<&SourceRangeRowsFailure<S::Error, SourceInverseRangeRowsRequest>> {
        self.core.failure.as_ref()
    }
    pub fn poll(&mut self) -> Result<SourceRangeRowsAdvance<SourceInverseRangeRowsRequest>, SourceRangeRowsStopped> {
        self.core.poll()
    }
    pub fn into_completed(self) -> Result<SourceInverseRangeRowsCompletion<S>, Self> {
        match self.core.into_completed() {
            Ok(completed) => Ok(SourceInverseRangeRowsCompletion { services: completed.services,
                observation: SourceInverseRangeRowsObservation { index: completed.index,
                    last_count_eax: completed.last_count, receipts: completed.receipts,
                    scalar_completions: completed.scalar_completions } }),
            Err(core) => Err(Self { core }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeRowsScalarAdmissionReason { WrongBoundary, ClampBranchMismatch, DiagnosticStopped }

pub struct SourceRangeRowsScalarAdmissionRefusal<S: RequiredRangeRowServices> {
    owner: SourceRangeRows<S>, binding: SourceRangeRowsForwardBinding,
    reason: SourceRangeRowsScalarAdmissionReason,
}
impl<S: RequiredRangeRowServices> SourceRangeRowsScalarAdmissionRefusal<S> {
    pub fn owner(&self) -> &SourceRangeRows<S> { &self.owner }
    pub fn binding(&self) -> &SourceRangeRowsForwardBinding { &self.binding }
    pub fn reason(&self) -> SourceRangeRowsScalarAdmissionReason { self.reason }
}

pub struct SourceRangeRowsScalarServices<S: RequiredRangeRowServices> {
    parent: SourceRangeRows<S>, expected: SourceRangeRowsRequest,
}
impl<S: RequiredRangeRowServices> SourceRangeRowsScalarServices<S> {
    pub fn row_progress(&self) -> SourceRangeRowsProgress { self.parent.progress() }
    pub fn row_services(&self) -> &S { &self.parent.core.services }
    pub fn expected_call(&self) -> SourceRangeRowsRequest { self.expected }
}
impl<S: RequiredRangeRowServices + SourceRangeGetters> SourceRangeGetters for SourceRangeRowsScalarServices<S> {
    type Error = <S as SourceRangeGetters>::Error;
    fn getter(&mut self, request: SourceRangeGetterRequest, entry: ExplicitExceptionState)
        -> Result<SourceRangeGetterReturn, SourceRangeGetterFailure<Self::Error>> {
        self.parent.core.services.getter(request, entry)
    }
}
impl<S: RequiredRangeRowServices + SourceRangeGetters> SourceRangeRows<S> {

    pub fn into_explicit_forward_3d310(mut self, binding: SourceRangeRowsForwardBinding)
        -> Result<SourceRangeScalar<SourceRangeRowsScalarServices<S>>, SourceRangeRowsScalarAdmissionRefusal<S>> {
        use SourceRangeRowsScalarAdmissionReason as R;
        if !self.core.scalar_ready() {
            return Err(SourceRangeRowsScalarAdmissionRefusal { owner: self, binding, reason: R::WrongBoundary });
        }
        if binding.clamp.is_some() != (self.core.route.clamp_word as u8 != 0) {
            return Err(SourceRangeRowsScalarAdmissionRefusal { owner: self, binding, reason: R::ClampBranchMismatch });
        }
        let request = self.core.request().expect("actual required scalar slot8");
        if let Err(reason) = self.core.reserve_scalar(request) {
            return Err(SourceRangeRowsScalarAdmissionRefusal { owner: self, binding, reason });
        }
        let input = SourceRangeInput { direction: SourceRangeDirection::Forward3d310,
            index_edx: self.core.index, input_bits: self.core.input.expect("completed actual input GET"),
            out_min_bits: binding.out_min_bits, out_max_bits: binding.out_max_bits, forward_clamp: binding.clamp };
        self.core.arm(request);
        Ok(SourceRangeScalar::from_explicit_parts(input, binding.backend, binding.entry,
            SourceRangeRowsScalarServices { parent: self, expected: request }))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRangeRowsScalarReturnReason { WrongActualReturn, ChildInputMismatch, ParentBoundaryMismatch }
pub struct SourceRangeRowsScalarReturnStop<S: RequiredRangeRowServices + SourceRangeGetters> {
    child: SourceRangeCompletion<SourceRangeRowsScalarServices<S>>,
    actual: SourceRangeRowsReturn, reason: SourceRangeRowsScalarReturnReason,
}
impl<S: RequiredRangeRowServices + SourceRangeGetters> SourceRangeRowsScalarReturnStop<S> {
    pub fn child(&self) -> &SourceRangeCompletion<SourceRangeRowsScalarServices<S>> { &self.child }
    pub fn actual_return(&self) -> SourceRangeRowsReturn { self.actual }
    pub fn reason(&self) -> SourceRangeRowsScalarReturnReason { self.reason }
}
impl<S: RequiredRangeRowServices + SourceRangeGetters> SourceRangeCompletion<SourceRangeRowsScalarServices<S>> {

    pub fn into_actual_range_rows(self, actual: SourceRangeRowsReturn)
        -> Result<SourceRangeRows<S>, SourceRangeRowsScalarReturnStop<S>> {
        use SourceRangeRowsScalarReturnReason as R;
        let expected = self.services().expected;
        let observation = self.observation();
        let reason = if actual.request != expected || actual.value != SourceRangeRowsValue::Bits(observation.bits) {
            Some(R::WrongActualReturn)
        } else if observation.input.direction != SourceRangeDirection::Forward3d310
            || Some(observation.input.input_bits) != expected.value_bits
            || Some(observation.input.index_edx) != expected.argument_edx
            || observation.input.forward_clamp.is_some() != (expected.original_clamp_word as u8 != 0) {
            Some(R::ChildInputMismatch)
        } else if !self.services().parent.core.scalar_pending_matches(expected) {
            Some(R::ParentBoundaryMismatch)
        } else { None };
        if let Some(reason) = reason { return Err(SourceRangeRowsScalarReturnStop { child: self, actual, reason }); }
        let (services, backend, observation) = self.into_parts();
        let SourceRangeRowsScalarServices { mut parent, expected: _ } = services;
        parent.core.accept_scalar(backend, observation, actual);
        Ok(parent)
    }
}

pub struct SourceInverseRangeRowsScalarAdmissionRefusal<S: RequiredInverseRangeRowServices> {
    owner: SourceInverseRangeRows<S>, binding: SourceRangeRowsInverseBinding,
    reason: SourceRangeRowsScalarAdmissionReason,
}
impl<S: RequiredInverseRangeRowServices> SourceInverseRangeRowsScalarAdmissionRefusal<S> {
    pub fn owner(&self) -> &SourceInverseRangeRows<S> { &self.owner }
    pub fn binding(&self) -> &SourceRangeRowsInverseBinding { &self.binding }
    pub fn reason(&self) -> SourceRangeRowsScalarAdmissionReason { self.reason }
}

pub struct SourceInverseRangeRowsScalarServices<S: RequiredInverseRangeRowServices> {
    parent: SourceInverseRangeRows<S>, expected: SourceInverseRangeRowsRequest,
}
impl<S: RequiredInverseRangeRowServices> SourceInverseRangeRowsScalarServices<S> {
    pub fn row_progress(&self) -> SourceRangeRowsProgress<SourceInverseRangeRowsRequest> { self.parent.progress() }
    pub fn row_services(&self) -> &S { &self.parent.core.services }
    pub fn expected_call(&self) -> SourceInverseRangeRowsRequest { self.expected }
}
impl<S: RequiredInverseRangeRowServices + SourceRangeGetters> SourceRangeGetters for SourceInverseRangeRowsScalarServices<S> {
    type Error = <S as SourceRangeGetters>::Error;
    fn getter(&mut self, request: SourceRangeGetterRequest, entry: ExplicitExceptionState)
        -> Result<SourceRangeGetterReturn, SourceRangeGetterFailure<Self::Error>> {
        self.parent.core.services.getter(request, entry)
    }
}
impl<S: RequiredInverseRangeRowServices + SourceRangeGetters> SourceInverseRangeRows<S> {

    pub fn into_explicit_inverse_3d3b0(mut self, binding: SourceRangeRowsInverseBinding)
        -> Result<SourceRangeScalar<SourceInverseRangeRowsScalarServices<S>>,
            SourceInverseRangeRowsScalarAdmissionRefusal<S>> {
        if !self.core.scalar_ready() {
            return Err(SourceInverseRangeRowsScalarAdmissionRefusal { owner: self, binding,
                reason: SourceRangeRowsScalarAdmissionReason::WrongBoundary });
        }
        let request = self.core.request().expect("actual required inverse scalar slot18");
        if let Err(reason) = self.core.reserve_scalar(request) {
            return Err(SourceInverseRangeRowsScalarAdmissionRefusal { owner: self, binding, reason });
        }
        let input = SourceRangeInput { direction: SourceRangeDirection::Inverse3d3b0,
            index_edx: self.core.index, input_bits: self.core.input.expect("completed actual input GET"),
            out_min_bits: binding.out_min_bits, out_max_bits: binding.out_max_bits, forward_clamp: None };
        self.core.arm(request);
        Ok(SourceRangeScalar::from_explicit_parts(input, binding.backend, binding.entry,
            SourceInverseRangeRowsScalarServices { parent: self, expected: request }))
    }
}
pub struct SourceInverseRangeRowsScalarReturnStop<S: RequiredInverseRangeRowServices + SourceRangeGetters> {
    child: SourceRangeCompletion<SourceInverseRangeRowsScalarServices<S>>,
    actual: SourceRangeRowsReturn<SourceInverseRangeRowsRequest>, reason: SourceRangeRowsScalarReturnReason,
}
impl<S: RequiredInverseRangeRowServices + SourceRangeGetters> SourceInverseRangeRowsScalarReturnStop<S> {
    pub fn child(&self) -> &SourceRangeCompletion<SourceInverseRangeRowsScalarServices<S>> { &self.child }
    pub fn actual_return(&self) -> SourceRangeRowsReturn<SourceInverseRangeRowsRequest> { self.actual }
    pub fn reason(&self) -> SourceRangeRowsScalarReturnReason { self.reason }
}
impl<S: RequiredInverseRangeRowServices + SourceRangeGetters> SourceRangeCompletion<SourceInverseRangeRowsScalarServices<S>> {

    pub fn into_actual_inverse_range_rows(self, actual: SourceRangeRowsReturn<SourceInverseRangeRowsRequest>)
        -> Result<SourceInverseRangeRows<S>, SourceInverseRangeRowsScalarReturnStop<S>> {
        use SourceRangeRowsScalarReturnReason as R;
        let expected = self.services().expected;
        let observation = self.observation();
        let reason = if actual.request != expected || actual.value != SourceRangeRowsValue::Bits(observation.bits) {
            Some(R::WrongActualReturn)
        } else if observation.input.direction != SourceRangeDirection::Inverse3d3b0
            || Some(observation.input.input_bits) != expected.value_bits
            || Some(observation.input.index_edx) != expected.argument_edx
            || observation.input.forward_clamp.is_some() {
            Some(R::ChildInputMismatch)
        } else if !self.services().parent.core.scalar_pending_matches(expected) {
            Some(R::ParentBoundaryMismatch)
        } else { None };
        if let Some(reason) = reason { return Err(SourceInverseRangeRowsScalarReturnStop { child: self, actual, reason }); }
        let (services, backend, observation) = self.into_parts();
        let SourceInverseRangeRowsScalarServices { mut parent, expected: _ } = services;
        parent.core.accept_scalar(backend, observation, actual);
        Ok(parent)
    }
}
