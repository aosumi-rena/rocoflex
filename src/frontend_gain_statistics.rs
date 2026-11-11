use crate::{frontend_gated_energy_math as gated, frontend_gain_log_suffix_math as suffix};
use crate::source_scalar::SourceF64ValueMath;
pub use gated::Pass;

#[derive(Clone, Copy, Debug)]
pub struct SourceGainStatisticsRequest {

    pub threshold_and_fallback_bits: u32,
    pub multiply_bits: u64,
    pub add_bits: u64,
    pub first_extra_add_bits: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceGainStatisticsLogRequest { pub pass: Pass, pub pc: u32, pub input_bits: u32 }

pub trait SourceGainStatisticsLogServices {
    type Reply;
    type Error;
    type ProjectionError;
    fn log(&mut self, request: SourceGainStatisticsLogRequest) -> Result<Self::Reply, Self::Error>;
    fn reply_association(&self, reply: &Self::Reply) -> SourceGainStatisticsLogRequest;
    fn project_reply(&mut self, reply: &Self::Reply) -> Result<u32, Self::ProjectionError>;
}
pub struct SourceGainStatisticsLogAttempt<L: SourceGainStatisticsLogServices> {
    pub request: SourceGainStatisticsLogRequest,
    pub reported: Result<L::Reply, L::Error>,
    pub reported_association: Option<SourceGainStatisticsLogRequest>,
    pub projection: Option<Result<u32, L::ProjectionError>>,
}
pub struct SourceGainStatisticsGatedReceipt {
    request: gated::SourceGatedEnergyRequest,
    progress: gated::SourceGatedEnergyProgress,
    outcome: gated::SourceGatedEnergyOutcome,
    latest: Option<gated::SourceGatedEnergyAttempt>,
}
impl SourceGainStatisticsGatedReceipt {
    pub fn request(&self) -> gated::SourceGatedEnergyRequest { self.request }
    pub fn progress(&self) -> &gated::SourceGatedEnergyProgress { &self.progress }
    pub fn outcome(&self) -> gated::SourceGatedEnergyOutcome { self.outcome }

    pub fn latest_attempt(&self) -> Option<&gated::SourceGatedEnergyAttempt> { self.latest.as_ref() }
}
pub struct SourceGainStatisticsSuffixReceipt {
    request: suffix::Request, constants: suffix::Constants, ledger: suffix::Ledger,
    progress: suffix::Progress, result_bits: u32,
}
impl SourceGainStatisticsSuffixReceipt {
    pub fn request(&self) -> &suffix::Request { &self.request }
    pub fn constants(&self) -> &suffix::Constants { &self.constants }
    pub fn ledger(&self) -> &suffix::Ledger { &self.ledger }
    pub fn progress(&self) -> &suffix::Progress { &self.progress }
    pub fn result_bits(&self) -> u32 { self.result_bits }
}
pub enum SourceGainStatisticsResolution<L: SourceGainStatisticsLogServices> {
    EmptyFallback { bits: u32 },
    Mean { log: SourceGainStatisticsLogAttempt<L>, suffix: SourceGainStatisticsSuffixReceipt },
}
pub struct SourceGainStatisticsPassReceipt<L: SourceGainStatisticsLogServices> {
    gated: SourceGainStatisticsGatedReceipt, resolution: SourceGainStatisticsResolution<L>, result_bits: u32,
}
impl<L: SourceGainStatisticsLogServices> SourceGainStatisticsPassReceipt<L> {
    pub fn gated(&self) -> &SourceGainStatisticsGatedReceipt { &self.gated }
    pub fn resolution(&self) -> &SourceGainStatisticsResolution<L> { &self.resolution }

    pub fn result_bits(&self) -> u32 { self.result_bits }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGainStatisticsStage { Gated, LogCall, LogAssociation, LogProjection, Suffix, Complete }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGainStatisticsCause { Gated, Log, Association, Projection, Suffix }
pub struct SourceGainStatisticsCurrent<L: SourceGainStatisticsLogServices> {
    pub stage: SourceGainStatisticsStage,
    pub gated: SourceGainStatisticsGatedReceipt,
    pub log: Option<SourceGainStatisticsLogAttempt<L>>,
}
enum PassFailure<L: SourceGainStatisticsLogServices> {
    Gated { failure: gated::SourceGatedEnergyFailure, suffix_math: SourceF64ValueMath, services: L },
    Log { energy: Vec<f32>, metric_log: Vec<f32>, gated_math: gated::SourceGatedEnergyMath,
        suffix_math: SourceF64ValueMath, services: L, current: SourceGainStatisticsCurrent<L>, cause: SourceGainStatisticsCause },
    Suffix { energy: Vec<f32>, metric_log: Vec<f32>, gated_math: gated::SourceGatedEnergyMath,
        services: L, current: SourceGainStatisticsCurrent<L>, failure: suffix::Failure },
}
pub struct SourceGainStatisticsFailure<L: SourceGainStatisticsLogServices> {
    request: SourceGainStatisticsRequest, pass: Pass,
    first: Option<SourceGainStatisticsPassReceipt<L>>, failure: PassFailure<L>,
}
impl<L: SourceGainStatisticsLogServices> SourceGainStatisticsFailure<L> {
    pub fn request(&self) -> SourceGainStatisticsRequest { self.request }
    pub fn pass(&self) -> Pass { self.pass }
    pub fn first(&self) -> Option<&SourceGainStatisticsPassReceipt<L>> { self.first.as_ref() }
    pub fn cause(&self) -> SourceGainStatisticsCause { match &self.failure {
        PassFailure::Gated { .. } => SourceGainStatisticsCause::Gated,
        PassFailure::Log { cause, .. } => *cause, PassFailure::Suffix { .. } => SourceGainStatisticsCause::Suffix } }
    pub fn stage(&self) -> SourceGainStatisticsStage { match &self.failure {
        PassFailure::Gated { .. } => SourceGainStatisticsStage::Gated,
        PassFailure::Log { current, .. } | PassFailure::Suffix { current, .. } => current.stage } }
    pub fn energy(&self) -> &[f32] { match &self.failure {
        PassFailure::Gated { failure, .. } => failure.energy(),
        PassFailure::Log { energy, .. } | PassFailure::Suffix { energy, .. } => energy } }
    pub fn metric_log(&self) -> &[f32] { match &self.failure {
        PassFailure::Gated { failure, .. } => failure.metric_log(),
        PassFailure::Log { metric_log, .. } | PassFailure::Suffix { metric_log, .. } => metric_log } }
    pub fn gated_math(&self) -> &gated::SourceGatedEnergyMath { match &self.failure {
        PassFailure::Gated { failure, .. } => failure.math(),
        PassFailure::Log { gated_math, .. } | PassFailure::Suffix { gated_math, .. } => gated_math } }
    pub fn suffix_math(&self) -> &SourceF64ValueMath { match &self.failure {
        PassFailure::Gated { suffix_math, .. } | PassFailure::Log { suffix_math, .. } => suffix_math,
        PassFailure::Suffix { failure, .. } => failure.math() } }
    pub fn log_services(&self) -> &L { match &self.failure {
        PassFailure::Gated { services, .. } | PassFailure::Log { services, .. } | PassFailure::Suffix { services, .. } => services } }
    pub fn gated_failure(&self) -> Option<&gated::SourceGatedEnergyFailure> { match &self.failure {
        PassFailure::Gated { failure, .. } => Some(failure), _ => None } }
    pub fn current(&self) -> Option<&SourceGainStatisticsCurrent<L>> { match &self.failure {
        PassFailure::Log { current, .. } | PassFailure::Suffix { current, .. } => Some(current), _ => None } }
    pub fn suffix_failure(&self) -> Option<&suffix::Failure> { match &self.failure {
        PassFailure::Suffix { failure, .. } => Some(failure), _ => None } }
}
pub struct SourceGainStatisticsParts<L: SourceGainStatisticsLogServices> {
    pub energy: Vec<f32>, pub metric_log: Vec<f32>, pub request: SourceGainStatisticsRequest,
    pub gated_math: gated::SourceGatedEnergyMath, pub suffix_math: SourceF64ValueMath, pub log_services: L,
    pub first: SourceGainStatisticsPassReceipt<L>, pub second: SourceGainStatisticsPassReceipt<L>, pub result_bits: u32,
}
pub struct SourceGainStatisticsCompleted<L: SourceGainStatisticsLogServices> { parts: SourceGainStatisticsParts<L> }
impl<L: SourceGainStatisticsLogServices> SourceGainStatisticsCompleted<L> {
    pub fn energy(&self) -> &[f32] { &self.parts.energy }
    pub fn metric_log(&self) -> &[f32] { &self.parts.metric_log }
    pub fn request(&self) -> SourceGainStatisticsRequest { self.parts.request }
    pub fn gated_math(&self) -> &gated::SourceGatedEnergyMath { &self.parts.gated_math }
    pub fn suffix_math(&self) -> &SourceF64ValueMath { &self.parts.suffix_math }
    pub fn log_services(&self) -> &L { &self.parts.log_services }
    pub fn first(&self) -> &SourceGainStatisticsPassReceipt<L> { &self.parts.first }
    pub fn second(&self) -> &SourceGainStatisticsPassReceipt<L> { &self.parts.second }
    pub fn result_bits(&self) -> u32 { self.parts.result_bits }

    pub fn boundary_pc(&self) -> u32 { 0x552c29 }
    pub fn into_parts(self) -> SourceGainStatisticsParts<L> { self.parts }
}

pub fn run_gain_statistics<L: SourceGainStatisticsLogServices>(energy: Vec<f32>, metric_log: Vec<f32>,
    request: SourceGainStatisticsRequest, gated_math: gated::SourceGatedEnergyMath,
    suffix_math: SourceF64ValueMath, log_services: L)
    -> Result<SourceGainStatisticsCompleted<L>, SourceGainStatisticsFailure<L>>
{
    let first = match run_pass(Pass::First, request.threshold_and_fallback_bits, request,
        energy, metric_log, gated_math, suffix_math, log_services) {
        Ok(completed) => completed,
        Err(failure) => return Err(SourceGainStatisticsFailure { request, pass: Pass::First, first: None, failure }),
    };
    let PassCompleted { energy, metric_log, gated_math, suffix_math, services, receipt: first } = first;
    let second = match run_pass(Pass::Second, first.result_bits, request, energy, metric_log, gated_math, suffix_math, services) {
        Ok(completed) => completed,
        Err(failure) => return Err(SourceGainStatisticsFailure { request, pass: Pass::Second, first: Some(first), failure }),
    };
    let PassCompleted { energy, metric_log, gated_math, suffix_math, services, receipt: second } = second;
    let result_bits = second.result_bits;
    Ok(SourceGainStatisticsCompleted { parts: SourceGainStatisticsParts {
        energy, metric_log, request, gated_math, suffix_math, log_services: services, first, second, result_bits } })
}
struct PassCompleted<L: SourceGainStatisticsLogServices> {
    energy: Vec<f32>, metric_log: Vec<f32>, gated_math: gated::SourceGatedEnergyMath,
    suffix_math: SourceF64ValueMath, services: L, receipt: SourceGainStatisticsPassReceipt<L>,
}
fn run_pass<L: SourceGainStatisticsLogServices>(pass: Pass, threshold_bits: u32, request: SourceGainStatisticsRequest,
    energy: Vec<f32>, metric_log: Vec<f32>, gated_math: gated::SourceGatedEnergyMath,
    suffix_math: SourceF64ValueMath, mut services: L) -> Result<PassCompleted<L>, PassFailure<L>>
{
    let gated_request = gated::SourceGatedEnergyRequest { pass, threshold_bits, empty_fallback_bits: request.threshold_and_fallback_bits };
    let completed = match gated::reduce_gated_energy(energy, metric_log, gated_request, gated_math) {
        Ok(completed) => completed,
        Err(failure) => return Err(PassFailure::Gated { failure, suffix_math, services }),
    };
    let gated::SourceGatedEnergyParts { energy, metric_log, request: gated_request, math: gated_math, progress, outcome } = completed.into_parts();
    let latest = gated_math.latest_attempt().copied();
    let gated = SourceGainStatisticsGatedReceipt { request: gated_request, progress, outcome, latest };
    let mean_bits = match outcome {
        gated::SourceGatedEnergyOutcome::EmptyFallback { bits } => return Ok(PassCompleted {
            energy, metric_log, gated_math, suffix_math, services,
            receipt: SourceGainStatisticsPassReceipt { gated, resolution: SourceGainStatisticsResolution::EmptyFallback { bits }, result_bits: bits } }),
        gated::SourceGatedEnergyOutcome::MeanEnergy { bits, .. } => bits,
    };
    let log_request = SourceGainStatisticsLogRequest { pass, pc: match pass { Pass::First => 0x552a50, Pass::Second => 0x552b8a }, input_bits: mean_bits };
    let mut current: SourceGainStatisticsCurrent<L> = SourceGainStatisticsCurrent { stage: SourceGainStatisticsStage::LogCall, gated, log: None };

    let reported = services.log(log_request);
    current.log = Some(SourceGainStatisticsLogAttempt { request: log_request, reported, reported_association: None, projection: None });
    if matches!(&current.log.as_ref().unwrap().reported, Err(_)) {
        return Err(PassFailure::Log { energy, metric_log, gated_math, suffix_math, services, current, cause: SourceGainStatisticsCause::Log });
    }
    current.stage = SourceGainStatisticsStage::LogAssociation;
    let association = match &current.log.as_ref().unwrap().reported {
        Ok(reply) => services.reply_association(reply), Err(_) => unreachable!(),
    };
    current.log.as_mut().unwrap().reported_association = Some(association);
    if association != log_request {
        return Err(PassFailure::Log { energy, metric_log, gated_math, suffix_math, services, current, cause: SourceGainStatisticsCause::Association });
    }
    current.stage = SourceGainStatisticsStage::LogProjection;
    let projection = match &current.log.as_ref().unwrap().reported {
        Ok(reply) => services.project_reply(reply), Err(_) => unreachable!(),
    };
    current.log.as_mut().unwrap().projection = Some(projection);
    let log_return_bits = match &current.log.as_ref().unwrap().projection {
        Some(Ok(bits)) => *bits,
        Some(Err(_)) => return Err(PassFailure::Log { energy, metric_log, gated_math, suffix_math, services, current, cause: SourceGainStatisticsCause::Projection }),
        None => unreachable!(),
    };
    current.stage = SourceGainStatisticsStage::Suffix;
    let suffix_request = suffix::Request { pass: match pass { Pass::First => suffix::Pass::First, Pass::Second => suffix::Pass::Second }, log_return_bits };
    let constants = suffix::Constants { multiply_bits: request.multiply_bits, add_bits: request.add_bits, first_extra_add_bits: request.first_extra_add_bits };
    let completed = match suffix::apply_log_suffix(suffix_request, constants, suffix_math) {
        Ok(completed) => completed,
        Err(failure) => return Err(PassFailure::Suffix { energy, metric_log, gated_math, services, current, failure }),
    };

    let suffix::NormalParts { request, constants, math: suffix_math, ledger, progress, result_bits } = completed.into_parts();
    let suffix = SourceGainStatisticsSuffixReceipt { request, constants, ledger, progress, result_bits };
    let SourceGainStatisticsCurrent { gated, log, .. } = current;
    Ok(PassCompleted { energy, metric_log, gated_math, suffix_math, services, receipt: SourceGainStatisticsPassReceipt {
        gated, resolution: SourceGainStatisticsResolution::Mean { log: log.unwrap(), suffix }, result_bits } })
}
