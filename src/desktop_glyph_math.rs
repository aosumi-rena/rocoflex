use super::glyph_fit as graph;
use super::empty_background::literal::raster_numeric_binding as raster;
#[path = "desktop_glyph_raster_math.rs"]
mod raster_math;
use crate::source_scalar::{self as scalar, ExplicitExceptionState, ScalarF32Architecture, LogArithmeticError,
    SourceF64ValueMath, SourceF64ValueError, SourceF64ValueCandidateReceipt, SourceF64ValueCompletion,
    SourcePcmProbeMath, PcmProbeMathError, PcmProbeTruncationOutcome, PcmProbeWideningOutcome,
    SourceF64I32Truncation, SourceF32I64Truncation, SourceScalarTruncationError,
    SourceUcomisdFlags, SourceStrictUcomisdOutcome, SourceStrictUcomisdError,
    SourceRawSquareRootF64, SourceSqrtF64Error, SourceSqrtF64Completion, PowerError};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Error, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Outcome, ScalarF32Mxcsr,
    ScalarF32CompareOperation, ScalarF32ComparisonOutcome, XmmF32Outcome,
    ScalarF32SelectionOutcome, ScalarF32SelectionRequired};
pub(super) struct BindingFailure {
    pub architecture: ScalarF32Architecture, pub state: ExplicitExceptionState, pub cause: ScalarF32Error,
}
pub(super) enum Cause {
    Allocation(std::collections::TryReserveError), Shared(ScalarF32Error), Divide(LogArithmeticError),
    ReportedState { mxcsr: u32 }, Trap { mxcsr: u32, unmasked: u8 },
    ReportedScalarOutcome(ScalarF32Outcome),
    ReportedComparisonOutcome(ScalarF32ComparisonOutcome),
    Unsupported(graph::F32Operation), OutputIndex(u8),
    RasterLowering(raster::F32LoweringError),
    RasterF64Lowering(raster::F64LoweringError), Wide(SourceF64ValueError),
    RasterConversionLowering(raster::ConversionLoweringError),
    Truncation(SourceScalarTruncationError), PcmConversion(PcmProbeMathError),
    RasterGuardFastLowering(raster::GuardFastLoweringError),
    Guard(SourceStrictUcomisdError), GuardTrap(SourceStrictUcomisdOutcome), SquareRoot(SourceSqrtF64Error),
    RasterPackedMaxLowering(raster::PackedMaxLoweringError),
    PackedTrap(XmmF32Outcome), SelectionTrap(ScalarF32SelectionOutcome),
    SelectionRequired { reported: ScalarF32SelectionOutcome, required: ScalarF32SelectionRequired },
    ReportedPackedOutcome { reported: XmmF32Outcome, reason: PowerError },
    ReportedSelectionOutcome { reported: ScalarF32SelectionOutcome, reason: PowerError },
    RasterInternalReply { lowered: Option<Lowered>, reported: Reply },
}
#[derive(Clone, Copy)]
pub(super) enum Reply { Bits(u32), Bits64(u64), Integer32Bits(u32), Integer64Bits(u64),
    Flags(graph::CompareFlags), F64Flags(SourceUcomisdFlags), MasksXmm([u32; 4]), MaxBits(u32), Written }
#[derive(Clone, Copy)]
pub(super) enum Lowered { F32(raster::F32Lowering), F64(raster::F64Lowering),
    Conversion(raster::ConversionLowering), GuardFast(raster::GuardFastLowering),
    PackedMax(raster::PackedMaxLowering) }
#[derive(Clone, Copy)]
pub(super) enum ConversionCompletion {
    SignedI64ToF64(PcmProbeWideningOutcome),
    F64ToI32(SourceF64I32Truncation),
    F32ToI64(SourceF32I64Truncation),
    F64ToI64(PcmProbeTruncationOutcome),
}
#[derive(Clone, Copy)]
pub(super) enum TraceRequest {
    Glyph(graph::Request),
    Raster { request: raster::Request, instruction: raster::Admitted },
}
pub(super) struct Call {
    pub request: TraceRequest, pub entry: ExplicitExceptionState,
    pub lowered: Option<Lowered>,
    pub wide_candidate: Option<SourceF64ValueCandidateReceipt>,
    pub wide_completion: Option<SourceF64ValueCompletion>,
    pub conversion_completion: Option<ConversionCompletion>,
    pub guard_outcome: Option<SourceStrictUcomisdOutcome>,
    pub sqrt_completion: Option<SourceSqrtF64Completion>,
    pub packed_outcome: Option<XmmF32Outcome>,
    pub selection_outcome: Option<ScalarF32SelectionOutcome>,
    pub cmpeqss_outcome: Option<ScalarF32Outcome>,

    pub advanced: ExplicitExceptionState, pub result: Option<Reply>,
}
pub(super) struct Failure {
    pub request: TraceRequest, pub entry: ExplicitExceptionState, pub advanced: ExplicitExceptionState, pub cause: Cause,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Stopped { Failed, AlreadyFailed }
pub(super) struct Provider {
    architecture: ScalarF32Architecture, state: ExplicitExceptionState, shared: SourceScalarF32,
    calls: Vec<Call>, failure: Option<Failure>, words: [Option<u32>; 6],

    failed_wide: Option<SourceF64ValueMath>,
    failed_conversion: Option<SourcePcmProbeMath>,
    failed_sqrt: Option<SourceRawSquareRootF64>,
}
impl Provider {
    pub fn from_explicit_parts(architecture: ScalarF32Architecture, state: ExplicitExceptionState) -> Result<Self, BindingFailure> {
        let shared = ScalarF32InstructionSet::from_explicit_opcodes(&[ScalarF32Opcode::Addss,
            ScalarF32Opcode::Subss, ScalarF32Opcode::Mulss, ScalarF32Opcode::Ucomiss,
            ScalarF32Opcode::Cmpltss, ScalarF32Opcode::Cmpless,
            ScalarF32Opcode::Maxss, ScalarF32Opcode::CmpeqpsXmm, ScalarF32Opcode::Cmpeqss])
            .and_then(|instructions| SourceScalarF32::from_explicit_architecture(architecture, instructions))
            .map_err(|cause| BindingFailure { architecture, state, cause })?;
        Ok(Self { architecture, state, shared, calls: Vec::new(), failure: None,
            words: [None; 6], failed_wide: None, failed_conversion: None, failed_sqrt: None })
    }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn calls(&self) -> &[Call] { &self.calls }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }
    pub fn failed_wide(&self) -> Option<&SourceF64ValueMath> { self.failed_wide.as_ref() }
    pub fn failed_conversion(&self) -> Option<&SourcePcmProbeMath> { self.failed_conversion.as_ref() }
    pub fn failed_sqrt(&self) -> Option<&SourceRawSquareRootF64> { self.failed_sqrt.as_ref() }
    pub fn words(&self) -> &[Option<u32>; 6] { &self.words }
    fn before(&self) -> Result<ScalarF32Mxcsr, Cause> {
        ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr()).ok_or(Cause::ReportedState { mxcsr: self.state.mxcsr() })
    }
    fn scalar_result(&mut self, outcome: ScalarF32Outcome) -> Result<Reply, Cause> {
        use scalar::logical_f32_outcome::{project_scalar_f32_outcome, SourceScalarF32Projection};
        match project_scalar_f32_outcome(self.state, outcome) {
            SourceScalarF32Projection::Completed { bits, advanced } => { self.state = advanced; Ok(Reply::Bits(bits)) }
            SourceScalarF32Projection::Trapped { reported_mxcsr: mxcsr, unmasked, advanced } => {
                self.state = advanced; Err(Cause::Trap { mxcsr, unmasked })
            }
            SourceScalarF32Projection::ReportedStateRequired { reported, known_advanced, .. } => {
                self.state = known_advanced; Err(Cause::ReportedScalarOutcome(reported))
            }
        }
    }

    fn execute_low(&mut self, index: usize, operation: raster::F32LoweredOperation) -> Result<Reply, Cause> {
        use raster::F32LoweredOperation as Low;
        let operation = match operation {
            Low::Addss { left, right } => ScalarF32Operation::Addss { left, right },
            Low::Subss { left, right } => ScalarF32Operation::Subss { left, right },
            Low::Mulss { left, right } => ScalarF32Operation::Mulss { left, right },
            Low::Divss { left, right } => {
                return match scalar::raw_divss_with_explicit_state(left, right, self.state, self.architecture) {
                    Ok(result) => { self.state = result.exception_state; Ok(Reply::Bits(result.bits)) }
                    Err(error) => {
                        self.state = match &error {
                            LogArithmeticError::ArithmeticRequired { advanced, .. }
                            | LogArithmeticError::ScalarServiceRequired { advanced, .. }
                            | LogArithmeticError::UnexpectedScalarState { advanced, .. }
                            | LogArithmeticError::UnexpectedScalarTrap { advanced, .. } => *advanced,
                            LogArithmeticError::Dispatch(_) => self.state,
                        };
                        Err(Cause::Divide(error))
                    }
                };
            }
            Low::Cmpless { left, right } => {
                let before = self.before()?;
                let outcome = self.shared.execute_cmpless(left, right, before).map_err(Cause::Shared)?;
                return self.scalar_result(outcome);
            }
            Low::Cmpeqss { left, right } => {
                let before = self.before()?;
                let outcome = self.shared.execute_cmpeqss(left, right, before).map_err(Cause::Shared)?;
                self.calls[index].cmpeqss_outcome = Some(outcome);
                return self.scalar_result(outcome);
            }
            Low::Ucomiss { left, right } => {
                let before = self.before()?;
                let outcome = self.shared.execute_comparison(ScalarF32CompareOperation::Ucomiss { left, right }, before).map_err(Cause::Shared)?;
                use scalar::logical_f32_outcome::{project_comparison_f32_outcome, SourceComparisonF32Projection};
                return match project_comparison_f32_outcome(self.state, outcome) {
                    SourceComparisonF32Projection::Completed { flags, advanced } => {
                        self.state = advanced;
                        Ok(Reply::Flags(graph::CompareFlags { carry: flags.carry, zero: flags.zero, parity: flags.parity }))
                    }
                    SourceComparisonF32Projection::Trapped { reported_mxcsr: mxcsr, unmasked, advanced } => {
                        self.state = advanced; Err(Cause::Trap { mxcsr, unmasked })
                    }
                    SourceComparisonF32Projection::ReportedStateRequired { reported, known_advanced, .. } => {
                        self.state = known_advanced; Err(Cause::ReportedComparisonOutcome(reported))
                    }
                };
            }
        };
        let before = self.before()?;
        let outcome = self.shared.execute(operation, before).map_err(Cause::Shared)?;
        self.scalar_result(outcome)
    }
    fn execute(&mut self, index: usize, request: TraceRequest) -> Result<Reply, Cause> {
        use graph::F32Operation;
        use raster::F32LoweredOperation as Low;
        match request {
            TraceRequest::Glyph(request) => match request {
                graph::Request::Arithmetic { operation, .. } => {
                    let operation = match operation {
                        F32Operation::Add { destination: left, source: right } => Low::Addss { left, right },
                        F32Operation::Sub { destination: left, source: right } => Low::Subss { left, right },
                        F32Operation::Mul { destination: left, source: right } => Low::Mulss { left, right },
                        F32Operation::Div { destination: left, source: right } => Low::Divss { left, right },
                        other => return Err(Cause::Unsupported(other)),
                    };
                    self.execute_low(index, operation)
                }
                graph::Request::Ucomiss { left, right, .. } => self.execute_low(index, Low::Ucomiss { left, right }),
                graph::Request::Cmpltss { left, right, .. } => {
                    let before = self.before()?;
                    let outcome = self.shared.execute_cmpltss(left, right, before).map_err(Cause::Shared)?;
                    self.scalar_result(outcome)
                }
                graph::Request::WriteWord { index, word, .. } => {
                    *self.words.get_mut(index as usize).ok_or(Cause::OutputIndex(index))? = Some(word);
                    Ok(Reply::Written)
                }
            },
            TraceRequest::Raster { request, instruction } => {
                self.execute_raster(index, request, instruction)
            }
        }
    }
    fn call<T>(&mut self, request: TraceRequest,
        project: impl FnOnce(Option<Lowered>, Reply) -> Result<T, Cause>) -> Result<T, Stopped> {
        if self.failure.is_some() { return Err(Stopped::AlreadyFailed); }
        let entry = self.state;
        if let Err(error) = self.calls.try_reserve(1) {
            self.failure = Some(Failure { request, entry, advanced: entry, cause: Cause::Allocation(error) });
            return Err(Stopped::Failed);
        }
        let index = self.calls.len();
        self.calls.push(Call { request, entry, lowered: None, wide_candidate: None,
            wide_completion: None, conversion_completion: None, guard_outcome: None,
            sqrt_completion: None, packed_outcome: None, selection_outcome: None, cmpeqss_outcome: None,
            advanced: entry, result: None });
        let result = self.execute(index, request);
        self.calls[index].advanced = self.state;
        let result = match result {
            Ok(reply) => {
                self.calls[index].result = Some(reply);
                project(self.calls[index].lowered, reply)
            }
            Err(cause) => Err(cause),
        };
        match result {
            Ok(value) => Ok(value),
            Err(cause) => { self.failure = Some(Failure { request, entry, advanced: self.state, cause }); Err(Stopped::Failed) }
        }
    }
}
impl graph::Services for Provider {
    type Error = Stopped;
    fn arithmetic(&mut self, pc: u32, operation: graph::F32Operation) -> Result<u32, Stopped> {
        self.call(TraceRequest::Glyph(graph::Request::Arithmetic { pc, operation }), |_, reply| {
            match reply { Reply::Bits(bits) => Ok(bits), _ => unreachable!("private arithmetic result") }
        })
    }
    fn ucomiss(&mut self, pc: u32, left: u32, right: u32) -> Result<graph::CompareFlags, Stopped> {
        self.call(TraceRequest::Glyph(graph::Request::Ucomiss { pc, left, right }), |_, reply| {
            match reply { Reply::Flags(flags) => Ok(flags), _ => unreachable!("private flags result") }
        })
    }
    fn cmpltss(&mut self, pc: u32, left: u32, right: u32) -> Result<u32, Stopped> {
        self.call(TraceRequest::Glyph(graph::Request::Cmpltss { pc, left, right }), |_, reply| {
            match reply { Reply::Bits(bits) => Ok(bits), _ => unreachable!("private mask result") }
        })
    }
    fn write_word(&mut self, pc: u32, index: u8, word: u32) -> Result<(), Stopped> {
        self.call(TraceRequest::Glyph(graph::Request::WriteWord { pc, index, word }), |_, reply| {
            match reply { Reply::Written => Ok(()), _ => unreachable!("private store result") }
        })
    }
}
