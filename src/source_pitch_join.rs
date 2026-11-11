use std::collections::VecDeque;

use crate::{Result, normalization::NormalizationBridge, pitch_selected_head::PitchRecord};
use crate::ltv_coefficient_math::ScalarUpper;
#[path = "source_pitch_join_math.rs"]
mod source_math;
pub use source_math::{SourceJoinConfiguration, SourceJoinBindingCause, SourceJoinMathBindingFailure,
    SourceJoinMath, SourceJoinMathStopped, SourceJoinMathCause, SourceJoinSite,
    SourceJoinMathAttempt, SourceJoinMathLeafFailure};

trait JoinMath {
    type Error;
    fn pitch_divisor(&self) -> f32;
    fn pitch_multiplier(&self) -> f32;
    fn pitch_offset(&self) -> f32;
    fn energy_floor(&self) -> f32;
    fn exp(&mut self, site: SourceJoinSite, input: f32, energy: bool) -> std::result::Result<f32, Self::Error>;
    fn divide(&mut self, site: SourceJoinSite, left: f32, right: f32) -> std::result::Result<f32, Self::Error>;
    fn log(&mut self, site: SourceJoinSite, input: f32) -> std::result::Result<f32, Self::Error>;
    fn multiply(&mut self, site: SourceJoinSite, left: f32, right: f32) -> std::result::Result<f32, Self::Error>;
    fn add(&mut self, site: SourceJoinSite, left: f32, right: f32) -> std::result::Result<f32, Self::Error>;
    fn count(&mut self, value: i32) -> std::result::Result<f32, Self::Error>;
    fn maximum(&mut self, left: f32, right: f32) -> std::result::Result<f32, Self::Error>;
}
#[derive(Clone, Copy)]
enum EnergyAdmission { Legacy, SourceRecordBits }

#[derive(Clone, Copy, Debug)]
pub enum SourceJoinEnergyStore {
    InitialPositiveZero { bits: u32 },
    Add { lane: usize, bits: u32 },
    FinalLog { bits: u32 },
}
struct HostJoinMath;
impl JoinMath for HostJoinMath {
    type Error = std::convert::Infallible;
    fn pitch_divisor(&self) -> f32 { 440.0 }
    fn pitch_multiplier(&self) -> f32 { f32::from_bits(0x418a7fac) }
    fn pitch_offset(&self) -> f32 { 69.0 }
    fn energy_floor(&self) -> f32 { f32::from_bits(0x1e3ce508) }
    fn exp(&mut self, _: SourceJoinSite, input: f32, _: bool) -> std::result::Result<f32, Self::Error> { Ok(f64::from(input).exp() as f32) }
    fn divide(&mut self, _: SourceJoinSite, left: f32, right: f32) -> std::result::Result<f32, Self::Error> { Ok(left / right) }
    fn log(&mut self, _: SourceJoinSite, input: f32) -> std::result::Result<f32, Self::Error> { Ok(f64::from(input).ln() as f32) }
    fn multiply(&mut self, _: SourceJoinSite, left: f32, right: f32) -> std::result::Result<f32, Self::Error> { Ok(left * right) }
    fn add(&mut self, _: SourceJoinSite, left: f32, right: f32) -> std::result::Result<f32, Self::Error> { Ok(left + right) }
    fn count(&mut self, value: i32) -> std::result::Result<f32, Self::Error> { Ok(value as f32) }
    fn maximum(&mut self, left: f32, right: f32) -> std::result::Result<f32, Self::Error> { Ok(left.max(right)) }
}
pub enum SourceJoinCoreCause<E> { Source(String), Math(E) }
fn host_reason(cause: SourceJoinCoreCause<std::convert::Infallible>) -> String {
    match cause { SourceJoinCoreCause::Source(reason) => reason, SourceJoinCoreCause::Math(never) => match never {} }
}

pub struct SourceJoinWork {
    inverse: Vec<f32>, paired72: Vec<f32>, normalized72: Vec<f32>, records: Vec<PitchRecord>, energies: Vec<f32>,
    paired_ordinal: Option<usize>, record: Option<PitchRecord>, energy_row: Option<usize>,
    energy_input: Option<[f32; 72]>, energy_lane: Option<usize>,
    hz: Option<f32>, ratio: Option<f32>, logarithm: Option<f32>, scaled: Option<f32>,
    sum: Option<f32>, mean: Option<f32>, guarded: Option<f32>,
    current_energy_slot: Option<usize>, completed_energy_count: usize,
    last_energy_store: Option<SourceJoinEnergyStore>, pending_energy_exp_bits: Option<u32>,
}
impl SourceJoinWork {
    fn empty() -> Self {
        Self { inverse: Vec::new(), paired72: Vec::new(), normalized72: Vec::new(), records: Vec::new(), energies: Vec::new(),
            paired_ordinal: None, record: None, energy_row: None, energy_input: None, energy_lane: None,
            hz: None, ratio: None, logarithm: None, scaled: None, sum: None, mean: None, guarded: None,
            current_energy_slot: None, completed_energy_count: 0, last_energy_store: None, pending_energy_exp_bits: None }
    }
    pub fn inverse(&self) -> &[f32] { &self.inverse }
    pub fn paired72(&self) -> &[f32] { &self.paired72 }
    pub fn normalized72(&self) -> &[f32] { &self.normalized72 }
    pub fn records(&self) -> &[PitchRecord] { &self.records }

    pub fn energies(&self) -> &[f32] { &self.energies }

    pub fn current_energy_slot(&self) -> Option<usize> { self.current_energy_slot }
    pub fn completed_energy_count(&self) -> usize { self.completed_energy_count }
    pub fn last_energy_store(&self) -> Option<SourceJoinEnergyStore> { self.last_energy_store }

    pub fn pending_energy_exp_bits(&self) -> Option<u32> { self.pending_energy_exp_bits }

    pub fn energy_record(&self) -> Option<&PitchRecord> {
        self.energy_row.and_then(|row| self.records.get(row))
    }
    pub fn paired_ordinal(&self) -> Option<usize> { self.paired_ordinal }
    pub fn record(&self) -> Option<PitchRecord> { self.record }
    pub fn energy_row(&self) -> Option<usize> { self.energy_row }
    pub fn energy_input(&self) -> Option<&[f32; 72]> { self.energy_input.as_ref() }
    pub fn energy_lane(&self) -> Option<usize> { self.energy_lane }
    pub fn hz(&self) -> Option<f32> { self.hz }
    pub fn ratio(&self) -> Option<f32> { self.ratio }
    pub fn logarithm(&self) -> Option<f32> { self.logarithm }
    pub fn scaled(&self) -> Option<f32> { self.scaled }
    pub fn sum(&self) -> Option<f32> { self.sum }
    pub fn mean(&self) -> Option<f32> { self.mean }
    pub fn guarded(&self) -> Option<f32> { self.guarded }
}
struct CoreFailure<E> { cause: SourceJoinCoreCause<E>, work: SourceJoinWork }
pub struct SourceJoinedRows<U: ScalarUpper> { rows: JoinedRows, join: SourcePitchJoin, math: SourceJoinMath<U> }
impl<U: ScalarUpper> SourceJoinedRows<U> {
    pub fn rows(&self) -> &JoinedRows { &self.rows }
    pub fn join(&self) -> &SourcePitchJoin { &self.join }
    pub fn math(&self) -> &SourceJoinMath<U> { &self.math }
    pub fn into_parts(self) -> (JoinedRows, SourcePitchJoin, SourceJoinMath<U>) { (self.rows, self.join, self.math) }
}

pub struct SourceJoinMathFailure<U: ScalarUpper> { math: SourceJoinMath<U>, core: CoreFailure<SourceJoinMathStopped> }
impl<U: ScalarUpper> SourceJoinMathFailure<U> {
    pub fn math(&self) -> &SourceJoinMath<U> { &self.math }
    pub fn cause(&self) -> &SourceJoinCoreCause<SourceJoinMathStopped> { &self.core.cause }
    pub fn work(&self) -> &SourceJoinWork { &self.core.work }
}
pub struct SourceJoinFailure<U: ScalarUpper> { join: SourcePitchJoin, failure: SourceJoinMathFailure<U> }
impl<U: ScalarUpper> SourceJoinFailure<U> {
    pub fn join(&self) -> &SourcePitchJoin { &self.join }
    pub fn failure(&self) -> &SourceJoinMathFailure<U> { &self.failure }
}

pub struct SourcePitchJoin {
    normalization: NormalizationBridge,
    source: VecDeque<[f32; 72]>,
    pitch: VecDeque<PitchRecord>,
    halted: bool,
}

pub struct JoinedRows {

    pub paired72: Vec<f32>,
    pub normalized72: Vec<f32>,
    pub records: Vec<PitchRecord>,
    pub energies: Vec<f32>,
}

pub fn pitch_lane(record: PitchRecord) -> Result<f32> {
    pitch_lane_using(record, &mut HostJoinMath, &mut SourceJoinWork::empty()).map_err(host_reason)
}
fn pitch_lane_using<M: JoinMath>(record: PitchRecord, math: &mut M, work: &mut SourceJoinWork)
    -> std::result::Result<f32, SourceJoinCoreCause<M::Error>>
{
    work.record = Some(record); work.hz = None; work.ratio = None; work.logarithm = None; work.scaled = None;
    if !record.value.is_finite() {
        return Err(SourceJoinCoreCause::Source("nonfinite pitch record".into()));
    }
    let hz = if record.valid {
        math.exp(SourceJoinSite::PitchExp, record.value, false).map_err(SourceJoinCoreCause::Math)?
    } else {
        record.value
    };
    work.hz = Some(hz);
    if !hz.is_finite() || hz <= 0.0 {
        return Err(SourceJoinCoreCause::Source("pitch record requires finite positive frequency".into()));
    }
    let divisor = math.pitch_divisor();
    let ratio = math.divide(SourceJoinSite::PitchDivide, hz, divisor).map_err(SourceJoinCoreCause::Math)?;
    work.ratio = Some(ratio);
    let logged = math.log(SourceJoinSite::PitchLog, ratio).map_err(SourceJoinCoreCause::Math)?;
    work.logarithm = Some(logged);
    let multiplier = math.pitch_multiplier();
    let scaled = math.multiply(SourceJoinSite::PitchMultiply, logged, multiplier).map_err(SourceJoinCoreCause::Math)?;
    work.scaled = Some(scaled);
    let offset = math.pitch_offset();
    let lane = math.add(SourceJoinSite::PitchOffset, scaled, offset).map_err(SourceJoinCoreCause::Math)?;
    if !lane.is_finite() {
        return Err(SourceJoinCoreCause::Source("pitch conversion outside finite domain".into()));
    }
    Ok(lane)
}

pub fn normalized_energy(row: &[f32]) -> Result<f32> {
    normalized_energy_using(row, &mut HostJoinMath, &mut SourceJoinWork::empty()).map_err(host_reason)
}
fn normalized_energy_using<M: JoinMath>(row: &[f32], math: &mut M, work: &mut SourceJoinWork)
    -> std::result::Result<f32, SourceJoinCoreCause<M::Error>>
{
    normalized_energy_using_admission(row, math, work, EnergyAdmission::Legacy)
}
fn normalized_energy_using_admission<M: JoinMath>(row: &[f32], math: &mut M,
    work: &mut SourceJoinWork, admission: EnergyAdmission)
    -> std::result::Result<f32, SourceJoinCoreCause<M::Error>>
{
    work.energy_lane = None; work.sum = None; work.mean = None; work.guarded = None; work.logarithm = None;
    work.current_energy_slot = None; work.last_energy_store = None; work.pending_energy_exp_bits = None;
    if row.len() != 72 || (matches!(admission, EnergyAdmission::Legacy) && row.iter().any(|v| !v.is_finite())) {
        return Err(SourceJoinCoreCause::Source("energy requires a finite 72-lane normalized row".into()));
    }
    let mut sum = 0.0_f32;
    work.sum = Some(sum);
    let slot = match admission {
        EnergyAdmission::Legacy => None,
        EnergyAdmission::SourceRecordBits => {

            work.energies.reserve(1);
            let slot = work.energies.len();
            work.energies.push(sum);
            work.current_energy_slot = Some(slot);
            work.last_energy_store = Some(SourceJoinEnergyStore::InitialPositiveZero { bits: sum.to_bits() });
            Some(slot)
        }
    };
    for (lane, &v) in row[..row.len()-1].iter().enumerate() {
        work.energy_lane = Some(lane);
        work.pending_energy_exp_bits = None;
        let value = math.exp(SourceJoinSite::EnergyExp, v, true).map_err(SourceJoinCoreCause::Math)?;
        work.pending_energy_exp_bits = Some(value.to_bits());
        sum = math.add(SourceJoinSite::EnergySum, value, sum).map_err(SourceJoinCoreCause::Math)?;
        work.sum = Some(sum);
        if let Some(slot) = slot {
            work.energies[slot] = sum;
            work.last_energy_store = Some(SourceJoinEnergyStore::Add { lane, bits: sum.to_bits() });
        }
        work.pending_energy_exp_bits = None;
    }

    let divisor = math.count((row.len()-1) as i32).map_err(SourceJoinCoreCause::Math)?;
    let mean = math.divide(SourceJoinSite::EnergyDivide, sum, divisor).map_err(SourceJoinCoreCause::Math)?;
    work.mean = Some(mean);
    let floor = math.energy_floor();
    let guarded = math.maximum(mean, floor).map_err(SourceJoinCoreCause::Math)?;
    work.guarded = Some(guarded);
    let energy = math.log(SourceJoinSite::EnergyLog, guarded).map_err(SourceJoinCoreCause::Math)?;
    work.logarithm = Some(energy);
    if matches!(admission, EnergyAdmission::Legacy) && !energy.is_finite() {
        return Err(SourceJoinCoreCause::Source("energy outside finite domain".into()));
    }
    if let Some(slot) = slot {
        work.energies[slot] = energy;
        work.last_energy_store = Some(SourceJoinEnergyStore::FinalLog { bits: energy.to_bits() });
        work.completed_energy_count += 1;
    }
    Ok(energy)
}

impl SourcePitchJoin {

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        Ok(Self::new(NormalizationBridge::from_dnni(model)?))
    }

    pub fn new(normalization: NormalizationBridge) -> Self {
        Self {
            normalization,
            source: VecDeque::new(),
            pitch: VecDeque::new(),
            halted: false,
        }
    }

    pub fn append(&mut self, network72: &[f32], records: &[PitchRecord]) -> Result<JoinedRows> {
        self.append_using(network72, records, &mut HostJoinMath).map_err(|failure| host_reason(failure.cause))
    }

    pub fn append_with_source_math<U: ScalarUpper>(mut self, network72: &[f32], records: &[PitchRecord], math: SourceJoinMath<U>)
        -> std::result::Result<SourceJoinedRows<U>, SourceJoinFailure<U>>
    {
        match self.append_with_owned_source_math(network72, records, math) {
            Ok((rows, math)) => Ok(SourceJoinedRows { rows, join: self, math }),
            Err(failure) => Err(SourceJoinFailure { join: self, failure }),
        }
    }

    pub fn append_with_source_record_math<U: ScalarUpper>(mut self, network72: &[f32],
        records: &[PitchRecord], math: SourceJoinMath<U>)
        -> std::result::Result<SourceJoinedRows<U>, SourceJoinFailure<U>>
    {
        match self.append_with_owned_source_record_math(network72, records, math) {
            Ok((rows, math)) => Ok(SourceJoinedRows { rows, join: self, math }),
            Err(failure) => Err(SourceJoinFailure { join: self, failure }),
        }
    }

    pub(crate) fn append_with_owned_source_math<U: ScalarUpper>(&mut self, network72: &[f32], records: &[PitchRecord],
        math: SourceJoinMath<U>) -> std::result::Result<(JoinedRows, SourceJoinMath<U>), SourceJoinMathFailure<U>>
    {
        self.append_with_owned_source_math_admission(network72, records, math, EnergyAdmission::Legacy)
    }

    pub(crate) fn append_with_owned_source_record_math<U: ScalarUpper>(&mut self, network72: &[f32],
        records: &[PitchRecord], math: SourceJoinMath<U>)
        -> std::result::Result<(JoinedRows, SourceJoinMath<U>), SourceJoinMathFailure<U>>
    {
        self.append_with_owned_source_math_admission(network72, records, math, EnergyAdmission::SourceRecordBits)
    }
    fn append_with_owned_source_math_admission<U: ScalarUpper>(&mut self, network72: &[f32], records: &[PitchRecord],
        mut math: SourceJoinMath<U>, admission: EnergyAdmission)
        -> std::result::Result<(JoinedRows, SourceJoinMath<U>), SourceJoinMathFailure<U>>
    {
        match self.append_using_admission(network72, records, &mut math, admission) {
            Ok(rows) => Ok((rows, math)),
            Err(core) => Err(SourceJoinMathFailure { math, core }),
        }
    }

    fn append_using<M: JoinMath>(&mut self, network72: &[f32], records: &[PitchRecord], math: &mut M)
        -> std::result::Result<JoinedRows, CoreFailure<M::Error>>
    {
        self.append_using_admission(network72, records, math, EnergyAdmission::Legacy)
    }
    fn append_using_admission<M: JoinMath>(&mut self, network72: &[f32], records: &[PitchRecord],
        math: &mut M, admission: EnergyAdmission) -> std::result::Result<JoinedRows, CoreFailure<M::Error>>
    {
        let mut work = SourceJoinWork::empty();
        if self.halted {
            return Err(CoreFailure { cause: SourceJoinCoreCause::Source("Source/pitch join is halted with advanced queued rows".into()), work });
        }
        work.inverse = match self.normalization.inverse_source188(network72) {
            Ok(value) => value,
            Err(reason) => return Err(CoreFailure { cause: SourceJoinCoreCause::Source(reason), work }),
        };
        if work.inverse.iter().any(|v| !v.is_finite()) {
            return Err(CoreFailure { cause: SourceJoinCoreCause::Source("inverse-normalized features outside finite domain".into()), work });
        }
        self.source.extend(work.inverse.chunks_exact(72).map(|row| {
            let mut value = [0.0; 72];
            value.copy_from_slice(row);
            value
        }));

        self.pitch.extend(records.iter().copied());

        self.halted = true;
        let count = self.source.len().min(self.pitch.len());
        work.paired72 = Vec::with_capacity(count * 72);
        work.records = Vec::with_capacity(count);
        for (ordinal, (source, &record)) in self.source.iter().zip(&self.pitch).take(count).enumerate() {
            work.paired_ordinal = Some(ordinal);
            let lane = match pitch_lane_using(record, math, &mut work) {
                Ok(value) => value,
                Err(cause) => return Err(CoreFailure { cause, work }),
            };
            let mut row = *source;
            row[71] = lane;
            work.paired72.extend_from_slice(&row);
            work.records.push(record);
        }
        work.normalized72 = match self.normalization.vec90_from_vec3b0(&work.paired72, true) {
            Ok(value) => value,
            Err(reason) => return Err(CoreFailure { cause: SourceJoinCoreCause::Source(reason), work }),
        };
        for row in 0..work.normalized72.len()/72 {
            let mut input = [0.0; 72];
            input.copy_from_slice(&work.normalized72[row*72..row*72+72]);
            work.energy_row = Some(row); work.energy_input = Some(input);
            let energy = match normalized_energy_using_admission(&input, math, &mut work, admission) {
                Ok(value) => value,
                Err(cause) => return Err(CoreFailure { cause, work }),
            };
            if matches!(admission, EnergyAdmission::Legacy) {
                work.energies.push(energy);
                work.last_energy_store = Some(SourceJoinEnergyStore::FinalLog { bits: energy.to_bits() });
                work.completed_energy_count += 1;
            }
        }
        drop(self.source.drain(..count));
        drop(self.pitch.drain(..count));
        self.halted = false;
        Ok(JoinedRows {
            paired72: work.paired72,
            normalized72: work.normalized72,
            records: work.records,
            energies: work.energies,
        })
    }

    pub fn pending_counts(&self) -> (usize, usize) {
        (self.source.len(), self.pitch.len())
    }

    pub fn is_halted(&self) -> bool { self.halted }

    pub fn clear_queues(&mut self) {
        self.source.clear();
        self.pitch.clear();
    }
}

