use crate::{Result, pitch_selected_head::PitchRecord};
use crate::ltv_transform_log::{SourcePositiveNormalLog2, PositiveNormalLog2Error};
use crate::source_scalar::{self as scalar, ExplicitExceptionState,
    SourceExpDispatchReceipt, SourceExpDispatchError, SourceExpDispatchOutcome, SourceExpEntry,
    shared_scalar_f32::ScalarF32Architecture};

fn insert(sorted: &mut Vec<f32>, value: f32) {
    let mut low = 0;
    let mut high = sorted.len();
    while low < high {
        if value == sorted[low] {
            low += 1;
            break;
        }
        let middle = (low + high) / 2;
        if middle == low {
            low += usize::from(sorted[low] <= value);
            break;
        }
        if sorted[middle] > value {
            high = middle;
        } else {
            low = middle;
        }
    }
    sorted.insert(low, value);
}

pub fn pitch_statistics(records: &[PitchRecord], energies: &[f32]) -> Result<Option<[f32; 5]>> {
    let result = collect_statistics(records, energies,
        &mut HostExpLog(&mut |ratio: f64| Ok::<_, std::convert::Infallible>(ratio.log2())));
    result.map_err(|failure| match failure.cause {
        PitchStatisticsCause::CountMismatch { .. } => "pitch statistics record/energy count mismatch".into(),
        PitchStatisticsCause::AcceptedHz => "accepted pitch outside finite positive-Hz range".into(),
        PitchStatisticsCause::Exponential(never) | PitchStatisticsCause::Logarithm(never) => match never {},
        PitchStatisticsCause::NonfiniteMidi { .. } => "nonfinite generated pitch statistic".into(),
        PitchStatisticsCause::CountLimit { .. } => "too many pitch statistics records".into(),
    })
}

#[derive(Clone, Copy)]
pub struct PitchStatisticsAttempt {
    pub ordinal: usize, pub record: PitchRecord, pub energy: f32,
    pub hz: Option<f32>, pub ratio: Option<f64>,
}

pub enum PitchStatisticsCause<E> {
    CountMismatch { records: usize, energies: usize }, AcceptedHz,
    Exponential(E), Logarithm(E), NonfiniteMidi { result_bits: u32 }, CountLimit { accepted: usize },
}
struct StatisticsFailure<E> {
    cause: PitchStatisticsCause<E>, attempt: Option<PitchStatisticsAttempt>, sorted: Vec<f32>,
}

pub struct SourcePitchStatistics {
    statistics: Option<[f32; 5]>, logarithm: SourcePositiveNormalLog2,
}
impl SourcePitchStatistics {
    pub fn statistics(&self) -> Option<[f32; 5]> { self.statistics }
    pub fn logarithm(&self) -> &SourcePositiveNormalLog2 { &self.logarithm }
    pub fn into_parts(self) -> (Option<[f32; 5]>, SourcePositiveNormalLog2) {
        (self.statistics, self.logarithm)
    }
}

pub struct SourcePitchStatisticsFailure {
    logarithm: SourcePositiveNormalLog2, cause: PitchStatisticsCause<PositiveNormalLog2Error>,
    attempt: Option<PitchStatisticsAttempt>, sorted: Vec<f32>,
}
impl SourcePitchStatisticsFailure {
    pub fn logarithm(&self) -> &SourcePositiveNormalLog2 { &self.logarithm }
    pub fn cause(&self) -> &PitchStatisticsCause<PositiveNormalLog2Error> { &self.cause }
    pub fn attempt(&self) -> Option<PitchStatisticsAttempt> { self.attempt }
    pub fn sorted_prefix(&self) -> &[f32] { &self.sorted }
}

pub fn pitch_statistics_with_source_log2(records: &[PitchRecord], energies: &[f32],
    mut logarithm: SourcePositiveNormalLog2)
    -> std::result::Result<SourcePitchStatistics, SourcePitchStatisticsFailure>
{
    let result = collect_statistics(records, energies, &mut HostExpLog(&mut |ratio| logarithm.evaluate(ratio)));
    match result {
        Ok(statistics) => Ok(SourcePitchStatistics { statistics, logarithm }),
        Err(StatisticsFailure { cause, attempt, sorted }) =>
            Err(SourcePitchStatisticsFailure { logarithm, cause, attempt, sorted }),
    }
}

#[derive(Clone, Copy)]
pub enum SourcePitchMathBindingError { Architecture, Controls }
pub struct SourcePitchMathBindingFailure {
    cause: SourcePitchMathBindingError, exp_source_word: u32,
    architecture: ScalarF32Architecture, logical_exp_entry: ExplicitExceptionState,
    logarithm: SourcePositiveNormalLog2,
}
impl SourcePitchMathBindingFailure {
    pub fn cause(&self) -> SourcePitchMathBindingError { self.cause }
    pub fn exp_source_word(&self) -> u32 { self.exp_source_word }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn logical_exp_entry(&self) -> ExplicitExceptionState { self.logical_exp_entry }
    pub fn logarithm(&self) -> &SourcePositiveNormalLog2 { &self.logarithm }
}
pub enum SourcePitchExpError {
    Dispatch(SourceExpDispatchError), RangeRequired(SourceExpEntry), CallCountLimit,
}
pub enum SourcePitchMathError {
    Exponential(SourcePitchExpError), Logarithm(PositiveNormalLog2Error),
}

pub struct SourcePitchMath {
    exp_source_word: u32, architecture: ScalarF32Architecture,
    logical_exp_state: ExplicitExceptionState, logarithm: SourcePositiveNormalLog2,
    latest_exp_receipt: Option<SourceExpDispatchReceipt>,
    exp_call_count: usize, completed_exp_count: usize,
}
impl SourcePitchMath {
    pub fn from_explicit_parts(exp_source_word: u32, architecture: ScalarF32Architecture,
        logical_exp_entry: ExplicitExceptionState, logarithm: SourcePositiveNormalLog2)
        -> std::result::Result<Self, SourcePitchMathBindingFailure>
    {
        let cause = if architecture != ScalarF32Architecture::IntelSdm {
            Some(SourcePitchMathBindingError::Architecture)
        } else if logical_exp_entry.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            Some(SourcePitchMathBindingError::Controls)
        } else { None };
        if let Some(cause) = cause {
            return Err(SourcePitchMathBindingFailure { cause, exp_source_word,
                architecture, logical_exp_entry, logarithm });
        }
        Ok(Self { exp_source_word, architecture, logical_exp_state: logical_exp_entry,
            logarithm, latest_exp_receipt: None, exp_call_count: 0, completed_exp_count: 0 })
    }
    pub fn exp_source_word(&self) -> u32 { self.exp_source_word }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn logical_exp_state(&self) -> ExplicitExceptionState { self.logical_exp_state }
    pub fn logarithm(&self) -> &SourcePositiveNormalLog2 { &self.logarithm }
    pub fn latest_exp_receipt(&self) -> Option<&SourceExpDispatchReceipt> { self.latest_exp_receipt.as_ref() }
    pub fn exp_call_count(&self) -> usize { self.exp_call_count }
    pub fn completed_exp_count(&self) -> usize { self.completed_exp_count }
    fn exponential_once(&mut self, input: f32) -> std::result::Result<f32, SourcePitchExpError> {
        let next = self.exp_call_count.checked_add(1).ok_or(SourcePitchExpError::CallCountLimit)?;

        let receipt = scalar::selected_exp_with_explicit_upper_lane_receipt(
            self.exp_source_word, self.architecture, input.to_bits(), 0, self.logical_exp_state);
        let outcome = receipt.outcome.clone();
        self.exp_call_count = next;
        self.logical_exp_state = receipt.known_advanced;
        self.latest_exp_receipt = Some(receipt);
        match outcome {
            Ok(SourceExpDispatchOutcome::Complete(result)) => {
                self.completed_exp_count += 1;
                Ok(f32::from_bits(result.bits))
            }
            Ok(SourceExpDispatchOutcome::RangeRequired(required)) => Err(SourcePitchExpError::RangeRequired(required)),
            Err(error) => Err(SourcePitchExpError::Dispatch(error)),
        }
    }
}
pub struct SourceMathPitchStatistics {
    statistics: Option<[f32; 5]>, math: SourcePitchMath,
}
impl SourceMathPitchStatistics {
    pub fn statistics(&self) -> Option<[f32; 5]> { self.statistics }
    pub fn math(&self) -> &SourcePitchMath { &self.math }
    pub fn into_parts(self) -> (Option<[f32; 5]>, SourcePitchMath) { (self.statistics, self.math) }
}

pub struct SourceMathPitchStatisticsFailure {
    math: SourcePitchMath, cause: PitchStatisticsCause<SourcePitchMathError>,
    attempt: Option<PitchStatisticsAttempt>, sorted: Vec<f32>,
}
impl SourceMathPitchStatisticsFailure {
    pub fn math(&self) -> &SourcePitchMath { &self.math }
    pub fn cause(&self) -> &PitchStatisticsCause<SourcePitchMathError> { &self.cause }
    pub fn attempt(&self) -> Option<PitchStatisticsAttempt> { self.attempt }
    pub fn sorted_prefix(&self) -> &[f32] { &self.sorted }
}
pub fn pitch_statistics_with_source_math(records: &[PitchRecord], energies: &[f32], mut math: SourcePitchMath)
    -> std::result::Result<SourceMathPitchStatistics, SourceMathPitchStatisticsFailure>
{
    match collect_statistics(records, energies, &mut math) {
        Ok(statistics) => Ok(SourceMathPitchStatistics { statistics, math }),
        Err(StatisticsFailure { cause, attempt, sorted }) =>
            Err(SourceMathPitchStatisticsFailure { math, cause, attempt, sorted }),
    }
}

trait StatisticsMath {
    type Error;
    fn exponential(&mut self, input: f32) -> std::result::Result<f32, Self::Error>;
    fn logarithm(&mut self, input: f64) -> std::result::Result<f64, Self::Error>;
}
struct HostExpLog<'a, F>(&'a mut F);
impl<E, F: FnMut(f64) -> std::result::Result<f64, E>> StatisticsMath for HostExpLog<'_, F> {
    type Error = E;
    fn exponential(&mut self, input: f32) -> std::result::Result<f32, E> { Ok(input.exp()) }
    fn logarithm(&mut self, input: f64) -> std::result::Result<f64, E> { (self.0)(input) }
}
impl StatisticsMath for SourcePitchMath {
    type Error = SourcePitchMathError;
    fn exponential(&mut self, input: f32) -> std::result::Result<f32, Self::Error> {
        self.exponential_once(input).map_err(SourcePitchMathError::Exponential)
    }
    fn logarithm(&mut self, input: f64) -> std::result::Result<f64, Self::Error> {
        self.logarithm.evaluate(input).map_err(SourcePitchMathError::Logarithm)
    }
}

fn collect_statistics<M: StatisticsMath>(records: &[PitchRecord], energies: &[f32], math: &mut M)
    -> std::result::Result<Option<[f32; 5]>, StatisticsFailure<M::Error>>
{
    if records.len() != energies.len() {
        return Err(StatisticsFailure { cause: PitchStatisticsCause::CountMismatch {
            records: records.len(), energies: energies.len() }, attempt: None, sorted: Vec::new() });
    }
    let mut sorted = Vec::new();
    for (ordinal, (record, &energy)) in records.iter().zip(energies).enumerate() {
        if !(f64::from(energy) > 0.1_f64) || !record.voiced {
            continue;
        }
        let mut attempt = PitchStatisticsAttempt { ordinal, record: *record, energy, hz: None, ratio: None };
        let hz = if record.valid {
            match math.exponential(record.value) {
                Ok(hz) => hz,
                Err(cause) => return Err(StatisticsFailure { cause: PitchStatisticsCause::Exponential(cause),
                    attempt: Some(attempt), sorted }),
            }
        } else {
            record.value
        };
        attempt.hz = Some(hz);
        if !hz.is_finite() || hz <= 0.0 {
            return Err(StatisticsFailure { cause: PitchStatisticsCause::AcceptedHz,
                attempt: Some(attempt), sorted });
        }
        let ratio = f64::from(hz) / 440.0_f64;
        attempt.ratio = Some(ratio);
        let log = match math.logarithm(ratio) {
            Ok(log) => log,
            Err(cause) => return Err(StatisticsFailure { cause: PitchStatisticsCause::Logarithm(cause),
                attempt: Some(attempt), sorted }),
        };
        let scaled = log * 12.0_f64;
        let midi = (scaled + 69.0_f64) as f32;
        if !midi.is_finite() {
            return Err(StatisticsFailure { cause: PitchStatisticsCause::NonfiniteMidi { result_bits: midi.to_bits() },
                attempt: Some(attempt), sorted });
        }
        insert(&mut sorted, midi);
    }
    if sorted.is_empty() {
        return Ok(None);
    }
    let count = match i32::try_from(sorted.len()) {
        Ok(count) => count,
        Err(_) => return Err(StatisticsFailure { cause: PitchStatisticsCause::CountLimit {
            accepted: sorted.len() }, attempt: None, sorted }),
    };
    let indices = [
        count / 10,
        count / 4,
        count / 2,
        count.wrapping_mul(75) / 100,
        count.wrapping_mul(90) / 100,
    ];
    Ok(Some(indices.map(|index| {
        if (count as u32) > (index as u32) {
            sorted[index as usize]
        } else {
            0.0
        }
    })))
}

