use std::collections::TryReserveError;
use crate::{frontend_gain_statistics as statistics, frontend_gated_energy_math as gated,
    frontend_gain_log_suffix_math as suffix, source_scalar::SourceF64ValueMath};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceGainRowLogRequest { pub row_dword: u32, pub pc: u32, pub input_energy_bits: u32 }

pub trait SourceGainRowLogServices {
    type Reply;
    type Error;
    type ProjectionError;
    fn row_log(&mut self, request: SourceGainRowLogRequest)
        -> Result<<Self as SourceGainRowLogServices>::Reply, <Self as SourceGainRowLogServices>::Error>;
    fn row_reply_association(&self, reply: &<Self as SourceGainRowLogServices>::Reply) -> SourceGainRowLogRequest;
    fn project_row_reply(&mut self, reply: &<Self as SourceGainRowLogServices>::Reply)
        -> Result<u32, <Self as SourceGainRowLogServices>::ProjectionError>;
}
pub struct SourceGainRowLogAttempt<L: SourceGainRowLogServices> {
    pub request: SourceGainRowLogRequest,
    pub reported: Result<<L as SourceGainRowLogServices>::Reply, <L as SourceGainRowLogServices>::Error>,
    pub reported_association: Option<SourceGainRowLogRequest>,
    pub projection: Option<Result<u32, <L as SourceGainRowLogServices>::ProjectionError>>,
}
pub struct SourceGainLogRowSuffixReceipt {
    request: suffix::RowRequest, constants: suffix::RowConstants,
    ledger: suffix::Ledger, progress: suffix::Progress, result_bits: u32,
}
impl SourceGainLogRowSuffixReceipt {
    pub fn request(&self) -> suffix::RowRequest { self.request }
    pub fn constants(&self) -> suffix::RowConstants { self.constants }
    pub fn ledger(&self) -> &suffix::Ledger { &self.ledger }
    pub fn progress(&self) -> &suffix::Progress { &self.progress }
    pub fn result_bits(&self) -> u32 { self.result_bits }
}
pub struct SourceGainLogRowReceipt<L: SourceGainRowLogServices> {
    log: SourceGainRowLogAttempt<L>, suffix: SourceGainLogRowSuffixReceipt,
}
impl<L: SourceGainRowLogServices> SourceGainLogRowReceipt<L> {
    pub fn row_dword(&self) -> u32 { self.log.request.row_dword }
    pub fn log(&self) -> &SourceGainRowLogAttempt<L> { &self.log }
    pub fn suffix(&self) -> &SourceGainLogRowSuffixReceipt { &self.suffix }
    pub fn into_parts(self) -> (SourceGainRowLogAttempt<L>, SourceGainLogRowSuffixReceipt) { (self.log, self.suffix) }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGainLogRowsStage { Admission, HistoryReserve, RowLog, Association, Projection, RowSuffix, MetricStore, Statistics, Complete }
pub enum SourceGainLogRowsCause {

    Extent { energy: usize, metric: usize }, Storage(TryReserveError),
    Log, Association, Projection, Suffix, Statistics,
}
pub struct SourceGainLogRowCurrent<L: SourceGainRowLogServices> {
    pub stage: SourceGainLogRowsStage,
    pub request: SourceGainRowLogRequest,
    pub log: Option<SourceGainRowLogAttempt<L>>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceGainLogRowCause { Log, Association, Projection, Suffix }
enum RowFailureOwner<L: SourceGainRowLogServices> {
    Ready { math: SourceF64ValueMath, services: L, current: SourceGainLogRowCurrent<L> },
    Suffix { failure: suffix::Failure<suffix::RowRequest, suffix::RowConstants>, services: L, current: SourceGainLogRowCurrent<L> },
}

pub struct SourceGainLogRowFailure<L: SourceGainRowLogServices> {
    cause: SourceGainLogRowCause, constants: suffix::RowConstants, owner: RowFailureOwner<L>,
}
impl<L: SourceGainRowLogServices> SourceGainLogRowFailure<L> {
    pub fn cause(&self) -> SourceGainLogRowCause { self.cause }
    pub fn request(&self) -> SourceGainRowLogRequest { self.current().request }
    pub fn constants(&self) -> suffix::RowConstants { self.constants }
    pub fn stage(&self) -> SourceGainLogRowsStage { self.current().stage }
    pub fn math(&self) -> &SourceF64ValueMath { match &self.owner {
        RowFailureOwner::Ready { math, .. } => math, RowFailureOwner::Suffix { failure, .. } => failure.math(),
    } }
    pub fn services(&self) -> &L { match &self.owner {
        RowFailureOwner::Ready { services, .. } | RowFailureOwner::Suffix { services, .. } => services,
    } }
    pub fn current(&self) -> &SourceGainLogRowCurrent<L> { match &self.owner {
        RowFailureOwner::Ready { current, .. } | RowFailureOwner::Suffix { current, .. } => current,
    } }
    pub fn suffix_failure(&self) -> Option<&suffix::Failure<suffix::RowRequest, suffix::RowConstants>> { match &self.owner {
        RowFailureOwner::Suffix { failure, .. } => Some(failure), _ => None,
    } }
}
pub(crate) struct SourceGainLogRowParts<L: SourceGainRowLogServices> {
    pub(crate) math: SourceF64ValueMath, pub(crate) services: L, pub(crate) receipt: SourceGainLogRowReceipt<L>,
}
pub(crate) struct SourceGainLogRowCompleted<L: SourceGainRowLogServices> { parts: SourceGainLogRowParts<L> }
impl<L: SourceGainRowLogServices> SourceGainLogRowCompleted<L> {
    pub(crate) fn into_parts(self) -> SourceGainLogRowParts<L> { self.parts }
}

pub(crate) fn run_gain_log_row<L: SourceGainRowLogServices>(row_request: SourceGainRowLogRequest,
    constants: suffix::RowConstants, metric_slot: &mut f32, math: SourceF64ValueMath, mut services: L)
    -> Result<SourceGainLogRowCompleted<L>, SourceGainLogRowFailure<L>>
{
    let mut current: SourceGainLogRowCurrent<L> = SourceGainLogRowCurrent { stage: SourceGainLogRowsStage::RowLog, request: row_request, log: None };
    let reported = <L as SourceGainRowLogServices>::row_log(&mut services, row_request);
    current.log = Some(SourceGainRowLogAttempt { request: row_request, reported, reported_association: None, projection: None });
    if matches!(&current.log.as_ref().unwrap().reported, Err(_)) {
        return Err(SourceGainLogRowFailure { cause: SourceGainLogRowCause::Log, constants, owner: RowFailureOwner::Ready { math, services, current } });
    }
    current.stage = SourceGainLogRowsStage::Association;
    let association = match &current.log.as_ref().unwrap().reported {
        Ok(reply) => <L as SourceGainRowLogServices>::row_reply_association(&services, reply), Err(_) => unreachable!(),
    };
    current.log.as_mut().unwrap().reported_association = Some(association);
    if association != row_request {
        return Err(SourceGainLogRowFailure { cause: SourceGainLogRowCause::Association, constants, owner: RowFailureOwner::Ready { math, services, current } });
    }
    current.stage = SourceGainLogRowsStage::Projection;
    let projection = match &current.log.as_ref().unwrap().reported {
        Ok(reply) => <L as SourceGainRowLogServices>::project_row_reply(&mut services, reply), Err(_) => unreachable!(),
    };
    current.log.as_mut().unwrap().projection = Some(projection);
    let log_return_bits = match &current.log.as_ref().unwrap().projection {
        Some(Ok(bits)) => *bits,
        Some(Err(_)) => return Err(SourceGainLogRowFailure { cause: SourceGainLogRowCause::Projection, constants,
            owner: RowFailureOwner::Ready { math, services, current } }),
        None => unreachable!(),
    };
    current.stage = SourceGainLogRowsStage::RowSuffix;
    let completed = match suffix::apply_row_log_suffix(suffix::RowRequest { log_return_bits }, constants, math) {
        Ok(completed) => completed,
        Err(failure) => return Err(SourceGainLogRowFailure { cause: SourceGainLogRowCause::Suffix, constants,
            owner: RowFailureOwner::Suffix { failure, services, current } }),
    };
    let suffix::NormalParts { request, constants, math, ledger, progress, result_bits } = completed.into_parts();
    let receipt = SourceGainLogRowSuffixReceipt { request, constants, ledger, progress, result_bits };
    current.stage = SourceGainLogRowsStage::MetricStore;
    *metric_slot = f32::from_bits(result_bits);
    Ok(SourceGainLogRowCompleted { parts: SourceGainLogRowParts { math, services,
        receipt: SourceGainLogRowReceipt { log: current.log.unwrap(), suffix: receipt } } })
}
enum FailureOwner<L: SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    Rows { energy: Vec<f32>, metric: Vec<f32>, gated_math: gated::SourceGatedEnergyMath,
        suffix_math: SourceF64ValueMath, services: L, current: Option<SourceGainLogRowCurrent<L>> },
    Suffix { energy: Vec<f32>, metric: Vec<f32>, gated_math: gated::SourceGatedEnergyMath,
        services: L, current: SourceGainLogRowCurrent<L>, failure: suffix::Failure<suffix::RowRequest, suffix::RowConstants> },
    Statistics { failure: statistics::SourceGainStatisticsFailure<L> },
}
pub struct SourceGainLogRowsFailure<L: SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    request: statistics::SourceGainStatisticsRequest, row: usize,
    history: Vec<SourceGainLogRowReceipt<L>>, stage: SourceGainLogRowsStage, cause: SourceGainLogRowsCause,
    owner: FailureOwner<L>,
}
impl<L: SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> SourceGainLogRowsFailure<L> {
    pub fn request(&self) -> statistics::SourceGainStatisticsRequest { self.request }

    pub fn row(&self) -> usize { self.row }
    pub fn stage(&self) -> SourceGainLogRowsStage { self.stage }
    pub fn cause(&self) -> &SourceGainLogRowsCause { &self.cause }
    pub fn rows(&self) -> &[SourceGainLogRowReceipt<L>] { &self.history }
    pub fn energy(&self) -> &[f32] { match &self.owner {
        FailureOwner::Rows { energy, .. } | FailureOwner::Suffix { energy, .. } => energy,
        FailureOwner::Statistics { failure } => failure.energy() } }
    pub fn metric(&self) -> &[f32] { match &self.owner {
        FailureOwner::Rows { metric, .. } | FailureOwner::Suffix { metric, .. } => metric,
        FailureOwner::Statistics { failure } => failure.metric_log() } }
    pub fn gated_math(&self) -> &gated::SourceGatedEnergyMath { match &self.owner {
        FailureOwner::Rows { gated_math, .. } | FailureOwner::Suffix { gated_math, .. } => gated_math,
        FailureOwner::Statistics { failure } => failure.gated_math() } }
    pub fn suffix_math(&self) -> &SourceF64ValueMath { match &self.owner {
        FailureOwner::Rows { suffix_math, .. } => suffix_math,
        FailureOwner::Suffix { failure, .. } => failure.math(),
        FailureOwner::Statistics { failure } => failure.suffix_math() } }
    pub fn log_services(&self) -> &L { match &self.owner {
        FailureOwner::Rows { services, .. } | FailureOwner::Suffix { services, .. } => services,
        FailureOwner::Statistics { failure } => failure.log_services() } }
    pub fn current(&self) -> Option<&SourceGainLogRowCurrent<L>> { match &self.owner {
        FailureOwner::Rows { current, .. } => current.as_ref(),
        FailureOwner::Suffix { current, .. } => Some(current), FailureOwner::Statistics { .. } => None } }
    pub fn suffix_failure(&self) -> Option<&suffix::Failure<suffix::RowRequest, suffix::RowConstants>> { match &self.owner {
        FailureOwner::Suffix { failure, .. } => Some(failure), _ => None } }
    pub fn statistics_failure(&self) -> Option<&statistics::SourceGainStatisticsFailure<L>> { match &self.owner {
        FailureOwner::Statistics { failure } => Some(failure), _ => None } }
}
pub struct SourceGainLogRowsParts<L: SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {

    pub statistics: statistics::SourceGainStatisticsParts<L>,
    pub rows: Vec<SourceGainLogRowReceipt<L>>,
}
pub struct SourceGainLogRowsCompleted<L: SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> {
    parts: SourceGainLogRowsParts<L>,
}
impl<L: SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices> SourceGainLogRowsCompleted<L> {
    pub fn statistics(&self) -> &statistics::SourceGainStatisticsParts<L> { &self.parts.statistics }
    pub fn rows(&self) -> &[SourceGainLogRowReceipt<L>] { &self.parts.rows }
    pub fn result_bits(&self) -> u32 { self.parts.statistics.result_bits }
    pub fn boundary_pc(&self) -> u32 { 0x552c29 }
    pub fn into_parts(self) -> SourceGainLogRowsParts<L> { self.parts }
}

pub fn run_gain_log_rows<L: SourceGainRowLogServices + statistics::SourceGainStatisticsLogServices>(
    energy: Vec<f32>, mut metric: Vec<f32>, request: statistics::SourceGainStatisticsRequest,
    gated_math: gated::SourceGatedEnergyMath, mut suffix_math: SourceF64ValueMath, mut services: L)
    -> Result<SourceGainLogRowsCompleted<L>, SourceGainLogRowsFailure<L>>
{
    let mut history = Vec::new();
    if energy.is_empty() || energy.len() > i32::MAX as usize || metric.len() != energy.len() {
        let cause = SourceGainLogRowsCause::Extent { energy: energy.len(), metric: metric.len() };
        return Err(SourceGainLogRowsFailure { request, row: 0, history, stage: SourceGainLogRowsStage::Admission, cause,
            owner: FailureOwner::Rows { energy, metric, gated_math, suffix_math, services, current: None } });
    }
    for row in 0..energy.len() {
        if let Err(error) = history.try_reserve(1) {
            return Err(SourceGainLogRowsFailure { request, row, history, stage: SourceGainLogRowsStage::HistoryReserve,
                cause: SourceGainLogRowsCause::Storage(error),
                owner: FailureOwner::Rows { energy, metric, gated_math, suffix_math, services, current: None } });
        }

        let row_request = SourceGainRowLogRequest { row_dword: row as u32, pc: 0x55298c, input_energy_bits: energy[row].to_bits() };
        let completed = match run_gain_log_row(row_request, suffix::RowConstants {
            xmm10_multiply_bits: request.multiply_bits, xmm11_add_bits: request.add_bits }, &mut metric[row], suffix_math, services) {
            Ok(completed) => completed,
            Err(failure) => {

                let stage = failure.stage();
                let cause = match failure.cause { SourceGainLogRowCause::Log => SourceGainLogRowsCause::Log,
                    SourceGainLogRowCause::Association => SourceGainLogRowsCause::Association,
                    SourceGainLogRowCause::Projection => SourceGainLogRowsCause::Projection,
                    SourceGainLogRowCause::Suffix => SourceGainLogRowsCause::Suffix };
                let owner = match failure.owner {
                    RowFailureOwner::Ready { math: suffix_math, services, current } =>
                        FailureOwner::Rows { energy, metric, gated_math, suffix_math, services, current: Some(current) },
                    RowFailureOwner::Suffix { failure, services, current } =>
                        FailureOwner::Suffix { energy, metric, gated_math, services, current, failure },
                };
                return Err(SourceGainLogRowsFailure { request, row, history, stage, cause, owner });
            }
        };
        let completed = completed.into_parts();
        suffix_math = completed.math; services = completed.services;

        history.push(completed.receipt);
    }
    let row = energy.len();
    match statistics::run_gain_statistics(energy, metric, request, gated_math, suffix_math, services) {
        Ok(completed) => Ok(SourceGainLogRowsCompleted { parts: SourceGainLogRowsParts { statistics: completed.into_parts(), rows: history } }),
        Err(failure) => Err(SourceGainLogRowsFailure { request, row, history, stage: SourceGainLogRowsStage::Statistics,
            cause: SourceGainLogRowsCause::Statistics, owner: FailureOwner::Statistics { failure } }),
    }
}
