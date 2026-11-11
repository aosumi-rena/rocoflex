use std::{collections::TryReserveError, convert::Infallible};
use super::remap_math::{SourceInterpolationRemapMath, SourceInterpolationRemapStopped, SourceInterpolationRemapTag};
use crate::source_scalar::SourceUcomissFlags;

const INPUT_ERROR: &str = "requires finite monotone cumulative values and ordered unit queries";

pub fn inverse_cumulative_source_order(
    cumulative: &[f32],
    sampled_positions: &[f32],
    queries: &[f32],
) -> Result<Vec<f32>, &'static str> {
    match remap_core(cumulative, sampled_positions, queries, HostRemap) {
        Ok(completed) => Ok(completed.output),
        Err(failure) => match failure.cause {
            CoreCause::Input => Err(INPUT_ERROR),
            CoreCause::Choice(never) => match never {},
        },
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceRemapStage { Ready, QueryStorage, Wanted, Scan, RequiredControl, Interpolation, Store, Complete }
#[derive(Clone, Copy, Debug)]
pub struct SourceRemapScanFlags {
    pub query: usize,
    pub raw_index: i64,
    pub flags: SourceUcomissFlags,
}

pub struct SourceRemapProgress {
    stage: SourceRemapStage, query: Option<usize>, raw_index: i64, last: i64,
    wanted_bits: Option<u32>, scan_index: Option<i64>, last_completed_scan: Option<SourceRemapScanFlags>,
}
impl SourceRemapProgress {
    pub fn stage(&self) -> SourceRemapStage { self.stage }
    pub fn query(&self) -> Option<usize> { self.query }
    pub fn raw_index(&self) -> i64 { self.raw_index }
    pub fn last(&self) -> i64 { self.last }
    pub fn wanted_bits(&self) -> Option<u32> { self.wanted_bits }

    pub fn scan_index(&self) -> Option<i64> { self.scan_index }

    pub fn last_completed_scan(&self) -> Option<SourceRemapScanFlags> { self.last_completed_scan }
}
pub enum SourceRemapCause {
    Input(&'static str), AlreadyStopped, Storage(TryReserveError), Math(SourceInterpolationRemapStopped),

    RequiredControl { query: usize, raw_index: i64, last: i64, wanted_bits: u32 },
}
pub struct SourceRemapParts {
    pub cumulative: Vec<f32>, pub sampled_positions: Vec<f32>, pub queries: Vec<f32>,
    pub output: Vec<f32>, pub math: SourceInterpolationRemapMath, pub progress: SourceRemapProgress,
}
pub struct SourceRemapCompleted { parts: SourceRemapParts }
impl SourceRemapCompleted {
    pub fn cumulative(&self) -> &[f32] { &self.parts.cumulative }
    pub fn sampled_positions(&self) -> &[f32] { &self.parts.sampled_positions }
    pub fn queries(&self) -> &[f32] { &self.parts.queries }
    pub fn output(&self) -> &[f32] { &self.parts.output }
    pub fn math(&self) -> &SourceInterpolationRemapMath { &self.parts.math }
    pub fn progress(&self) -> &SourceRemapProgress { &self.parts.progress }
    pub fn into_parts(self) -> SourceRemapParts { self.parts }
}
struct RemapWork { output: Option<Vec<f32>>, progress: Option<SourceRemapProgress> }

pub struct SourceRemapFailure {
    cumulative: Vec<f32>, sampled_positions: Vec<f32>, queries: Vec<f32>,
    math: SourceInterpolationRemapMath, work: RemapWork, cause: SourceRemapCause,
}
impl SourceRemapFailure {
    pub fn cumulative(&self) -> &[f32] { &self.cumulative }
    pub fn sampled_positions(&self) -> &[f32] { &self.sampled_positions }
    pub fn queries(&self) -> &[f32] { &self.queries }
    pub fn output(&self) -> Option<&[f32]> { self.work.output.as_deref() }
    pub fn math(&self) -> &SourceInterpolationRemapMath { &self.math }
    pub fn progress(&self) -> Option<&SourceRemapProgress> { self.work.progress.as_ref() }
    pub fn cause(&self) -> &SourceRemapCause { &self.cause }
}

pub fn inverse_cumulative_with_source_math(cumulative: Vec<f32>, sampled_positions: Vec<f32>,
    queries: Vec<f32>, math: SourceInterpolationRemapMath) -> Result<SourceRemapCompleted, SourceRemapFailure>
{
    match remap_core(&cumulative, &sampled_positions, &queries, SourceRemap { math }) {
        Ok(completed) => Ok(SourceRemapCompleted { parts: SourceRemapParts {
            cumulative, sampled_positions, queries, output: completed.output,
            math: completed.choice.math, progress: completed.progress.unwrap() } }),
        Err(failure) => {
            let cause = match failure.cause {
                CoreCause::Input => SourceRemapCause::Input(INPUT_ERROR),
                CoreCause::Choice(cause) => cause,
            };
            Err(SourceRemapFailure { cumulative, sampled_positions, queries,
                math: failure.choice.math, work: failure.work, cause })
        }
    }
}

struct ScanDecision { advance: bool, flags: Option<SourceUcomissFlags> }
trait RemapChoice {
    type Error;
    fn values_admitted(&self, cumulative: &[f32], positions: &[f32], queries: &[f32]) -> bool;
    fn ready(&self) -> Result<(), Self::Error>;
    fn output(&self, query_count: usize) -> Vec<f32>;
    fn progress(&self, last: usize) -> Option<SourceRemapProgress>;
    fn reserve(&mut self, output: &mut Vec<f32>) -> Result<(), Self::Error>;
    fn wanted(&mut self, query: usize, query_bits: u32, total_bits: u32) -> Result<u32, Self::Error>;
    fn can_compare(&self, index: usize, last: usize) -> bool;
    fn compare(&mut self, tag: SourceInterpolationRemapTag, wanted: u32, next: u32) -> Result<ScanDecision, Self::Error>;
    fn control(&self, work: &mut RemapWork, query: usize, index: usize, last: usize, wanted: u32)
        -> Result<(), Self::Error>;
    fn interpolate(&mut self, tag: SourceInterpolationRemapTag, wanted: u32, current: u32, next: u32,
        position: u32, next_position: u32) -> Result<u32, Self::Error>;
}
struct HostRemap;
impl RemapChoice for HostRemap {
    type Error = Infallible;
    fn values_admitted(&self, cumulative: &[f32], positions: &[f32], queries: &[f32]) -> bool {
        !(cumulative.iter().chain(positions).chain(queries).any(|v| !v.is_finite())
            || cumulative[0] != 0.0
            || cumulative.iter().any(|v| *v < 0.0)
            || cumulative.windows(2).any(|w| w[0] > w[1])
            || positions.iter().chain(queries).any(|v| *v < 0.0 || *v > 1.0)
            || queries.windows(2).any(|w| w[0] > w[1]))
    }
    fn ready(&self) -> Result<(), Infallible> { Ok(()) }
    fn output(&self, query_count: usize) -> Vec<f32> { Vec::with_capacity(query_count) }
    fn progress(&self, _: usize) -> Option<SourceRemapProgress> { None }
    fn reserve(&mut self, _: &mut Vec<f32>) -> Result<(), Infallible> { Ok(()) }
    fn wanted(&mut self, _: usize, query: u32, total: u32) -> Result<u32, Infallible> {
        Ok((f32::from_bits(query) * f32::from_bits(total)).to_bits())
    }
    fn can_compare(&self, index: usize, last: usize) -> bool { index + 1 < last }
    fn compare(&mut self, _: SourceInterpolationRemapTag, wanted: u32, next: u32) -> Result<ScanDecision, Infallible> {
        Ok(ScanDecision { advance: f32::from_bits(wanted) > f32::from_bits(next), flags: None })
    }
    fn control(&self, _: &mut RemapWork, _: usize, _: usize, _: usize, _: u32) -> Result<(), Infallible> { Ok(()) }
    fn interpolate(&mut self, _: SourceInterpolationRemapTag, wanted: u32, current: u32, next: u32,
        position: u32, next_position: u32) -> Result<u32, Infallible>
    {
        let current = f32::from_bits(current);
        let shifted = f32::from_bits(wanted) - current;
        let divided = shifted / f32::from_bits(next);
        let fraction = divided - current;
        let fraction = if fraction.is_nan() || fraction <= 0.0 { 0.0 } else { fraction };
        let fraction = if fraction >= 1.0 { 1.0 } else { fraction };
        let position = f32::from_bits(position);
        let delta = f32::from_bits(next_position) - position;
        let scaled = delta * fraction;
        Ok((position + scaled).to_bits())
    }
}
struct SourceRemap { math: SourceInterpolationRemapMath }
impl RemapChoice for SourceRemap {
    type Error = SourceRemapCause;
    fn values_admitted(&self, cumulative: &[f32], _: &[f32], _: &[f32]) -> bool { cumulative.len() <= i32::MAX as usize }
    fn ready(&self) -> Result<(), SourceRemapCause> {
        if self.math.failure().is_some() || self.math.is_active() { Err(SourceRemapCause::AlreadyStopped) }
        else { Ok(()) }
    }
    fn output(&self, _: usize) -> Vec<f32> { Vec::new() }
    fn progress(&self, last: usize) -> Option<SourceRemapProgress> {
        Some(SourceRemapProgress { stage: SourceRemapStage::Ready, query: None, raw_index: 0,
            last: last as i64, wanted_bits: None, scan_index: None, last_completed_scan: None })
    }
    fn reserve(&mut self, output: &mut Vec<f32>) -> Result<(), SourceRemapCause> {
        output.try_reserve(1).map_err(SourceRemapCause::Storage)
    }
    fn wanted(&mut self, query: usize, query_bits: u32, total_bits: u32) -> Result<u32, SourceRemapCause> {
        self.math.wanted(query, query_bits, total_bits).map_err(SourceRemapCause::Math)
    }
    fn can_compare(&self, index: usize, last: usize) -> bool { index < last }
    fn compare(&mut self, tag: SourceInterpolationRemapTag, wanted: u32, next: u32) -> Result<ScanDecision, SourceRemapCause> {
        let flags = self.math.compare(tag, wanted, next).map_err(SourceRemapCause::Math)?;
        Ok(ScanDecision { advance: !flags.carry && !flags.zero, flags: Some(flags) })
    }
    fn control(&self, work: &mut RemapWork, query: usize, index: usize, last: usize, wanted: u32)
        -> Result<(), SourceRemapCause>
    {

        if index >= last {
            work.progress.as_mut().unwrap().stage = SourceRemapStage::RequiredControl;
            return Err(SourceRemapCause::RequiredControl { query, raw_index: index as i64,
                last: last as i64, wanted_bits: wanted });
        }
        Ok(())
    }
    fn interpolate(&mut self, tag: SourceInterpolationRemapTag, wanted: u32, current: u32, next: u32,
        position: u32, next_position: u32) -> Result<u32, SourceRemapCause>
    {
        self.math.interpolate(tag, wanted, current, next, position, next_position).map_err(SourceRemapCause::Math)
    }
}

enum CoreCause<E> { Input, Choice(E) }
struct CoreFailure<C: RemapChoice> { choice: C, work: RemapWork, cause: CoreCause<C::Error> }
struct CoreCompleted<C: RemapChoice> { choice: C, output: Vec<f32>, progress: Option<SourceRemapProgress> }
fn remap_core<C: RemapChoice>(cumulative: &[f32], positions: &[f32], queries: &[f32], mut choice: C)
    -> Result<CoreCompleted<C>, CoreFailure<C>>
{
    let mut work = RemapWork { output: None, progress: None };
    let result = (|| {
        if cumulative.len() < 2 || cumulative.len() != positions.len() || !choice.values_admitted(cumulative, positions, queries) {
            return Err(CoreCause::Input);
        }
        choice.ready().map_err(CoreCause::Choice)?;
        let total = cumulative[cumulative.len() - 1];
        let last = cumulative.len() - 1;
        let mut index = 0;
        work.output = Some(choice.output(queries.len()));
        work.progress = choice.progress(last);
        for (query_ordinal, &query) in queries.iter().enumerate() {
            if let Some(progress) = work.progress.as_mut() {
                progress.stage = SourceRemapStage::QueryStorage;
                progress.query = Some(query_ordinal);
                progress.raw_index = index as i64;
                progress.wanted_bits = None;
                progress.scan_index = None;
            }
            choice.reserve(work.output.as_mut().unwrap()).map_err(CoreCause::Choice)?;
            if let Some(progress) = work.progress.as_mut() { progress.stage = SourceRemapStage::Wanted; }
            let wanted = choice.wanted(query_ordinal, query.to_bits(), total.to_bits()).map_err(CoreCause::Choice)?;
            if let Some(progress) = work.progress.as_mut() { progress.wanted_bits = Some(wanted); }
            while choice.can_compare(index, last) {
                if let Some(progress) = work.progress.as_mut() {
                    progress.stage = SourceRemapStage::Scan;
                    progress.scan_index = Some(index as i64);
                }
                let decision = choice.compare(SourceInterpolationRemapTag { query: query_ordinal, index: index as i64 },
                    wanted, cumulative[index + 1].to_bits()).map_err(CoreCause::Choice)?;
                if let (Some(progress), Some(flags)) = (work.progress.as_mut(), decision.flags) {
                    progress.last_completed_scan = Some(SourceRemapScanFlags { query: query_ordinal, raw_index: index as i64, flags });
                }
                if !decision.advance { break; }
                index += 1;
                if let Some(progress) = work.progress.as_mut() { progress.raw_index = index as i64; }
            }
            choice.control(&mut work, query_ordinal, index, last, wanted).map_err(CoreCause::Choice)?;
            if let Some(progress) = work.progress.as_mut() { progress.stage = SourceRemapStage::Interpolation; }
            let word = choice.interpolate(SourceInterpolationRemapTag { query: query_ordinal, index: index as i64 },
                wanted, cumulative[index].to_bits(), cumulative[index + 1].to_bits(),
                positions[index].to_bits(), positions[index + 1].to_bits()).map_err(CoreCause::Choice)?;
            if let Some(progress) = work.progress.as_mut() { progress.stage = SourceRemapStage::Store; }
            work.output.as_mut().unwrap().push(f32::from_bits(word));
        }
        if let Some(progress) = work.progress.as_mut() { progress.stage = SourceRemapStage::Complete; }
        Ok(work.output.take().unwrap())
    })();
    match result {
        Ok(output) => Ok(CoreCompleted { choice, output, progress: work.progress }),
        Err(cause) => Err(CoreFailure { choice, work, cause }),
    }
}
