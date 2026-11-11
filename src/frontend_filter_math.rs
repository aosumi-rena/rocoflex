use std::convert::Infallible;
use crate::source_scalar::{self as scalar, ExplicitExceptionState};
use scalar::shared_scalar_f32::{SourceScalarF32, ScalarF32Architecture, ScalarF32InstructionSet,
    ScalarF32Opcode, ScalarF32Operation, ScalarF32Mxcsr, ScalarF32Error, ScalarF32Outcome, XmmF32Outcome};
use scalar::logical_f32_outcome::{project_scalar_f32_outcome, project_xmm_f32_outcome,
    SourceScalarF32Projection, SourceXmmF32Projection};

#[derive(Clone, Copy, Debug)]
pub struct SourceFilterConfiguration {
    pub negative_a1_xor_mask: u32, pub architecture: ScalarF32Architecture,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceFilterBindingCause { Architecture, Controls, Scalar(ScalarF32Error) }
pub struct SourceFilterBindingFailure {
    configuration: SourceFilterConfiguration, entry: ExplicitExceptionState, cause: SourceFilterBindingCause,
}
impl SourceFilterBindingFailure {
    pub fn configuration(&self) -> SourceFilterConfiguration { self.configuration }
    pub fn entry(&self) -> ExplicitExceptionState { self.entry }
    pub fn cause(&self) -> SourceFilterBindingCause { self.cause }
}
#[derive(Clone, Copy, Debug)]
pub enum SourceFilterRole { First, Second }
#[derive(Clone, Copy, Debug)]
pub enum SourceFilterOperands { Scalar { left: u32, right: u32 }, Packed { left: [u32; 4], right: [u32; 4] } }
#[derive(Clone, Copy, Debug)]
pub struct SourceFilterAttempt {
    pub role: SourceFilterRole, pub index: usize, pub pc: u32, pub operands: SourceFilterOperands,
    pub entry: ExplicitExceptionState, pub known_advanced: ExplicitExceptionState,
    pub scalar_reported: Option<ScalarF32Outcome>, pub packed_reported: Option<XmmF32Outcome>,
    pub scalar_result: Option<u32>, pub packed_result: Option<[u32; 4]>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceFilterMathCause {
    Scalar(ScalarF32Error), LogicalState { mxcsr: u32 },
    ReportedScalar(SourceScalarF32Projection), ReportedPacked(SourceXmmF32Projection),
}
pub struct SourceFilterMathFailure { cause: SourceFilterMathCause, attempt: SourceFilterAttempt }
impl SourceFilterMathFailure {
    pub fn cause(&self) -> SourceFilterMathCause { self.cause }
    pub fn attempt(&self) -> &SourceFilterAttempt { &self.attempt }
}
pub struct SourceFilterMath {
    configuration: SourceFilterConfiguration, state: ExplicitExceptionState, scalar: SourceScalarF32,
    latest: Option<SourceFilterAttempt>, failure: Option<SourceFilterMathFailure>,
}
impl SourceFilterMath {
    pub fn from_explicit_parts(configuration: SourceFilterConfiguration, entry: ExplicitExceptionState)
        -> Result<Self, SourceFilterBindingFailure>
    {
        let bind = || {
            if configuration.architecture != ScalarF32Architecture::IntelSdm { return Err(SourceFilterBindingCause::Architecture); }
            if entry.mxcsr() & 0xffff_ffc0 != 0x1f80 { return Err(SourceFilterBindingCause::Controls); }
            let instructions = ScalarF32InstructionSet::from_explicit_opcodes(&[
                ScalarF32Opcode::Mulss, ScalarF32Opcode::Subss, ScalarF32Opcode::Addss, ScalarF32Opcode::MulpsXmm])
                .map_err(SourceFilterBindingCause::Scalar)?;
            SourceScalarF32::from_explicit_architecture(configuration.architecture, instructions)
                .map_err(SourceFilterBindingCause::Scalar)
        };
        match bind() {
            Ok(scalar) => Ok(Self { configuration, state: entry, scalar, latest: None, failure: None }),
            Err(cause) => Err(SourceFilterBindingFailure { configuration, entry, cause }),
        }
    }
    pub fn configuration(&self) -> SourceFilterConfiguration { self.configuration }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn latest_attempt(&self) -> Option<&SourceFilterAttempt> { self.latest.as_ref() }
    pub fn failure(&self) -> Option<&SourceFilterMathFailure> { self.failure.as_ref() }
    fn perform(&mut self, role: SourceFilterRole, index: usize, pc: u32, operands: SourceFilterOperands)
        -> Result<Reply, Stopped>
    {
        if self.failure.is_some() { return Err(Stopped); }
        let mut attempt = SourceFilterAttempt { role, index, pc, operands, entry: self.state, known_advanced: self.state,
            scalar_reported: None, packed_reported: None, scalar_result: None, packed_result: None };
        let result = (|| {
            let before = ScalarF32Mxcsr::from_explicit_word(self.state.mxcsr())
                .ok_or(SourceFilterMathCause::LogicalState { mxcsr: self.state.mxcsr() })?;
            match operands {
                SourceFilterOperands::Scalar { left, right } => {
                    let operation = match pc {
                        0x118d10 | 0x118d1a | 0x118d3b => ScalarF32Operation::Mulss { left, right },
                        0x118d1e => ScalarF32Operation::Subss { left, right },
                        0x118d22 | 0x118d48 | 0x118d4d | 0x118d52 => ScalarF32Operation::Addss { left, right },
                        _ => unreachable!(),
                    };
                    let reported = self.scalar.execute(operation, before).map_err(SourceFilterMathCause::Scalar)?;
                    attempt.scalar_reported = Some(reported);
                    let projection = project_scalar_f32_outcome(self.state, reported);
                    match projection {
                        SourceScalarF32Projection::Completed { bits, advanced } => {
                            self.state = advanced; attempt.scalar_result = Some(bits); Ok(Reply::Scalar(bits))
                        }
                        SourceScalarF32Projection::Trapped { advanced, .. } => {
                            self.state = advanced; Err(SourceFilterMathCause::ReportedScalar(projection))
                        }
                        SourceScalarF32Projection::ReportedStateRequired { known_advanced, .. } => {
                            self.state = known_advanced; Err(SourceFilterMathCause::ReportedScalar(projection))
                        }
                    }
                }
                SourceFilterOperands::Packed { left, right } => {
                    let reported = self.scalar.execute_mulps_xmm(left, right, before).map_err(SourceFilterMathCause::Scalar)?;
                    attempt.packed_reported = Some(reported);
                    let projection = project_xmm_f32_outcome(self.state, reported);
                    match projection {
                        SourceXmmF32Projection::Completed { bits, advanced } => {
                            self.state = advanced; attempt.packed_result = Some(bits); Ok(Reply::Packed(bits))
                        }
                        SourceXmmF32Projection::Trapped { advanced, .. } => {
                            self.state = advanced; Err(SourceFilterMathCause::ReportedPacked(projection))
                        }
                        SourceXmmF32Projection::ReportedStateRequired { known_advanced, .. } => {
                            self.state = known_advanced; Err(SourceFilterMathCause::ReportedPacked(projection))
                        }
                    }
                }
            }
        })();
        attempt.known_advanced = self.state;
        self.latest = Some(attempt);
        result.map_err(|cause| { self.failure = Some(SourceFilterMathFailure { cause, attempt }); Stopped })
    }
}

pub struct SourceFilterOutputs { pub first: Vec<f32>, pub second: Vec<f32> }
pub struct SourceFilterProgress {
    role: SourceFilterRole, index: usize, pc: u32, loop_entered: bool, negative_a1_bits: u32,
    previous_bits: u32, feedback_bits: Option<u32>, old_product_bits: Option<u32>,
    packed_products: Option<[u32; 4]>, b2_product_bits: Option<u32>, feedforward_bits: Option<u32>,
}
impl SourceFilterProgress {
    fn new(role: SourceFilterRole) -> Self {
        Self { role, index: 2, pc: 0x118cc5, loop_entered: false, negative_a1_bits: 0, previous_bits: 0,
            feedback_bits: None, old_product_bits: None, packed_products: None, b2_product_bits: None, feedforward_bits: None }
    }
    pub fn role(&self) -> SourceFilterRole { self.role }
    pub fn index(&self) -> usize { self.index }
    pub fn pc(&self) -> u32 { self.pc }

    pub fn loop_entered(&self) -> bool { self.loop_entered }
    pub fn negative_a1_bits(&self) -> u32 { self.negative_a1_bits }
    pub fn previous_bits(&self) -> u32 { self.previous_bits }
    pub fn feedback_bits(&self) -> Option<u32> { self.feedback_bits }
    pub fn old_product_bits(&self) -> Option<u32> { self.old_product_bits }
    pub fn packed_products(&self) -> Option<[u32; 4]> { self.packed_products }
    pub fn b2_product_bits(&self) -> Option<u32> { self.b2_product_bits }
    pub fn feedforward_bits(&self) -> Option<u32> { self.feedforward_bits }
}
pub enum SourceFilterGainCause {
    Input(String), ExpandedCountRequired { sample_count: usize },
    OutputExtent { required: usize }, Math, Suffix(String),
}
pub struct SourceFilterGainFailure {
    input: Vec<f32>, target_db: f32, math: SourceFilterMath, outputs: SourceFilterOutputs,
    progress: Option<SourceFilterProgress>, energy: Option<Vec<f32>>, metric_log: Option<Vec<f32>>,
    cause: SourceFilterGainCause,
}
impl SourceFilterGainFailure {
    pub fn input(&self) -> &[f32] { &self.input }
    pub fn target_db(&self) -> f32 { self.target_db }
    pub fn math(&self) -> &SourceFilterMath { &self.math }
    pub fn first_filter(&self) -> &[f32] { &self.outputs.first }
    pub fn second_filter(&self) -> &[f32] { &self.outputs.second }
    pub fn progress(&self) -> Option<&SourceFilterProgress> { self.progress.as_ref() }
    pub fn energy(&self) -> Option<&[f32]> { self.energy.as_deref() }
    pub fn metric_log(&self) -> Option<&[f32]> { self.metric_log.as_deref() }
    pub fn cause(&self) -> &SourceFilterGainCause { &self.cause }
}
pub fn frontend_gain_table_with_source_filters(input: Vec<f32>, target_db: f32,
    math: SourceFilterMath, outputs: SourceFilterOutputs)
    -> Result<(super::FrontendGainStages, SourceFilterMath, Vec<f32>), SourceFilterGainFailure>
{
    let PreparedSourceFilters { input, target_db, math, outputs, progress } =
        prepare_source_filters(input, target_db, math, outputs)?;
    match super::gain_table_suffix(input.len(), target_db, outputs.first, outputs.second) {
        Ok(stages) => Ok((stages, math, input)),
        Err(failure) => Err(SourceFilterGainFailure { input, target_db, math,
            outputs: SourceFilterOutputs { first: failure.first_filter, second: failure.second_filter },
            progress: Some(progress), energy: Some(failure.energy), metric_log: failure.metric_log,
            cause: SourceFilterGainCause::Suffix(failure.error) }),
    }
}
pub(super) struct PreparedSourceFilters {
    pub(super) input: Vec<f32>, pub(super) target_db: f32, pub(super) math: SourceFilterMath,
    pub(super) outputs: SourceFilterOutputs, pub(super) progress: SourceFilterProgress,
}
#[derive(Clone, Copy)]
pub(super) enum FilterInputAdmission { Legacy, SourceFinite }
impl FilterInputAdmission {
    fn validate(self, input: &[f32]) -> crate::Result<()> { match self {
        Self::Legacy => super::validate_gain_input(input),
        Self::SourceFinite => {
            if input.len() > i32::MAX as usize || input.iter().any(|x| !x.is_finite()) {

                return Err("source gain input requires finite nonempty signed32 stream".into());
            }
            Ok(())
        }
    } }
}
pub(super) fn prepare_source_filters(input: Vec<f32>, target_db: f32,
    math: SourceFilterMath, outputs: SourceFilterOutputs)
    -> Result<PreparedSourceFilters, SourceFilterGainFailure>
{
    prepare_source_filters_with_admission(input, target_db, math, outputs, FilterInputAdmission::Legacy)
}
pub(super) fn prepare_source_filters_with_admission(input: Vec<f32>, target_db: f32,
    mut math: SourceFilterMath, mut outputs: SourceFilterOutputs, admission: FilterInputAdmission)
    -> Result<PreparedSourceFilters, SourceFilterGainFailure>
{
    if let Err(error) = admission.validate(&input) {
        return Err(SourceFilterGainFailure { input, target_db, math, outputs, progress: None,
            energy: None, metric_log: None, cause: SourceFilterGainCause::Input(error) });
    }

    if input.len() > (i32::MAX as usize) - 2 {
        let sample_count = input.len();
        return Err(SourceFilterGainFailure { input, target_db, math, outputs, progress: None,
            energy: None, metric_log: None, cause: SourceFilterGainCause::ExpandedCountRequired { sample_count } });
    }
    let required = input.len() + 2;
    if outputs.first.len() < required || outputs.second.len() < required {
        return Err(SourceFilterGainFailure { input, target_db, math, outputs, progress: None,
            energy: None, metric_log: None, cause: SourceFilterGainCause::OutputExtent { required } });
    }
    if matches!(admission, FilterInputAdmission::SourceFinite) {

        if math.failure.is_some() {
            return Err(SourceFilterGainFailure { input, target_db, math, outputs, progress: None,
                energy: None, metric_log: None, cause: SourceFilterGainCause::Math });
        }
        if input.len() < 3 {

            let mut progress = SourceFilterProgress::new(SourceFilterRole::Second);
            progress.index = input.len(); progress.pc = 0x118d67;
            return Ok(PreparedSourceFilters { input, target_db, math, outputs, progress });
        }
    }
    let [(b0, a0), (b1, a1)] = super::gain_filter_coefficients();
    let mask = math.configuration.negative_a1_xor_mask;
    let mut progress = SourceFilterProgress::new(SourceFilterRole::First);
    if filter_graph(&input, b0, a0, mask, &mut outputs.first, &mut math, &mut progress).is_err() {
        return Err(SourceFilterGainFailure { input, target_db, math, outputs, progress: Some(progress),
            energy: None, metric_log: None, cause: SourceFilterGainCause::Math });
    }
    progress = SourceFilterProgress::new(SourceFilterRole::Second);
    if filter_graph(&outputs.first[..input.len()], b1, a1, mask, &mut outputs.second, &mut math, &mut progress).is_err() {
        return Err(SourceFilterGainFailure { input, target_db, math, outputs, progress: Some(progress),
            energy: None, metric_log: None, cause: SourceFilterGainCause::Math });
    }
    Ok(PreparedSourceFilters { input, target_db, math, outputs, progress })
}

struct Stopped;
enum Reply { Scalar(u32), Packed([u32; 4]) }
trait FilterOps {
    type Error;
    fn scalar(&mut self, role: SourceFilterRole, index: usize, pc: u32, left: u32, right: u32) -> Result<u32, Self::Error>;
    fn packed(&mut self, role: SourceFilterRole, index: usize, left: [u32; 4], right: [u32; 4]) -> Result<[u32; 4], Self::Error>;
}
impl FilterOps for SourceFilterMath {
    type Error = Stopped;
    fn scalar(&mut self, role: SourceFilterRole, index: usize, pc: u32, left: u32, right: u32) -> Result<u32, Stopped> {
        match self.perform(role, index, pc, SourceFilterOperands::Scalar { left, right })? {
            Reply::Scalar(bits) => Ok(bits), Reply::Packed(_) => unreachable!(),
        }
    }
    fn packed(&mut self, role: SourceFilterRole, index: usize, left: [u32; 4], right: [u32; 4]) -> Result<[u32; 4], Stopped> {
        match self.perform(role, index, 0x118d32, SourceFilterOperands::Packed { left, right })? {
            Reply::Packed(bits) => Ok(bits), Reply::Scalar(_) => unreachable!(),
        }
    }
}
struct HostFilter;
impl FilterOps for HostFilter {
    type Error = Infallible;
    fn scalar(&mut self, _: SourceFilterRole, _: usize, pc: u32, left: u32, right: u32) -> Result<u32, Infallible> {
        let left = f32::from_bits(left); let right = f32::from_bits(right);
        Ok(match pc { 0x118d10 | 0x118d1a | 0x118d3b => left * right, 0x118d1e => left - right,
            0x118d22 | 0x118d48 | 0x118d4d | 0x118d52 => left + right, _ => unreachable!() }.to_bits())
    }
    fn packed(&mut self, _: SourceFilterRole, _: usize, left: [u32; 4], right: [u32; 4]) -> Result<[u32; 4], Infallible> {

        let b1 = f32::from_bits(left[1]) * f32::from_bits(right[1]);
        let b0 = f32::from_bits(left[0]) * f32::from_bits(right[0]);
        Ok([b0.to_bits(), b1.to_bits(), 0, 0])
    }
}
pub(super) fn host_biquad(src: &[f32], b: [f32; 3], a: [f32; 3]) -> Vec<f32> {
    let mut output = vec![0.0; src.len()];
    let mut progress = SourceFilterProgress::new(SourceFilterRole::First);
    match filter_graph(src, b, a, 0x8000_0000, &mut output, &mut HostFilter, &mut progress) {
        Ok(()) => output, Err(error) => match error {},
    }
}
fn filter_graph<O: FilterOps>(src: &[f32], b: [f32; 3], a: [f32; 3], sign_mask: u32,
    output: &mut [f32], ops: &mut O, progress: &mut SourceFilterProgress) -> Result<(), O::Error>
{
    if src.len() < 3 { return Ok(()); }
    progress.loop_entered = true;
    progress.negative_a1_bits = a[1].to_bits() ^ sign_mask;
    let mut previous_input = src[1].to_bits();
    for index in 2..src.len() {
        progress.index = index;
        progress.feedback_bits = None; progress.old_product_bits = None;
        progress.packed_products = None; progress.b2_product_bits = None; progress.feedforward_bits = None;
        macro_rules! scalar {
            ($pc:expr, $left:expr, $right:expr) => {{
                progress.pc = $pc;
                ops.scalar(progress.role, index, $pc, $left, $right)?
            }};
        }
        let recurrence = scalar!(0x118d10, progress.previous_bits, progress.negative_a1_bits);
        progress.feedback_bits = Some(recurrence);
        let old = scalar!(0x118d1a, output[index - 2].to_bits(), a[2].to_bits());
        progress.old_product_bits = Some(old);
        let recurrence = scalar!(0x118d1e, recurrence, old);
        progress.feedback_bits = Some(recurrence);
        let recurrence = scalar!(0x118d22, recurrence, output[index].to_bits());
        progress.feedback_bits = Some(recurrence);
        let current_input = src[index].to_bits();

        progress.pc = 0x118d32;
        let products = ops.packed(progress.role, index, [current_input, previous_input, 0, 0],
            [b[0].to_bits(), b[1].to_bits(), 0, 0])?;
        progress.packed_products = Some(products);
        let b2 = scalar!(0x118d3b, src[index - 2].to_bits(), b[2].to_bits());
        progress.b2_product_bits = Some(b2);
        let sum = scalar!(0x118d48, products[1], products[0]);
        progress.feedforward_bits = Some(sum);
        let sum = scalar!(0x118d4d, sum, b2);
        progress.feedforward_bits = Some(sum);
        let result = scalar!(0x118d52, recurrence, sum);
        output[index] = f32::from_bits(result);
        progress.previous_bits = result;
        previous_input = current_input;
    }
    progress.index = src.len(); progress.pc = 0x118d67;
    Ok(())
}
