use super::cache_blocking::{self, Blocking, CacheSelection, CpuidRecord};
use super::geometry::Statistics;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Source(cache_blocking::Error), Environment, Count, NonFiniteInput, NonFiniteOutput,
}

#[derive(Debug)]
struct Inner {
    records: Box<[CpuidRecord]>,
    selected: CacheSelection,
    declared_mxcsr: u32,
}

#[derive(Clone, Debug)]
pub struct GeometryContext(Arc<Inner>);
impl GeometryContext {

    pub fn from_owned_records(
        records: Vec<CpuidRecord>, declared_mxcsr: u32, separate_operations: bool,
    ) -> Result<Self, Error> {
        if declared_mxcsr & 0xffc0 != 0x1f80 || !separate_operations { return Err(Error::Environment); }
        let selected = cache_blocking::select_cache(&records).map_err(Error::Source)?;
        Ok(Self(Arc::new(Inner { records: records.into_boxed_slice(), selected, declared_mxcsr })))
    }
    pub fn records(&self) -> &[CpuidRecord] { &self.0.records }
    pub fn selected(&self) -> CacheSelection { self.0.selected }
    pub fn declared_mxcsr(&self) -> u32 { self.0.declared_mxcsr }
    pub fn same_snapshot(&self, other: &Self) -> bool { Arc::ptr_eq(&self.0, &other.0) }
}

fn input<const D: usize>(rows: &[[f32; D]], minimum: usize) -> Result<(), Error> {

    if rows.len() < minimum || rows.len() > 2049 { return Err(Error::Count); }
    if rows.iter().flatten().any(|v| !v.is_finite()) { return Err(Error::NonFiniteInput); }
    Ok(())
}

pub fn statistics_with_context(
    rows: &[[f32; 4]], context: &GeometryContext,
) -> Result<(Statistics, Blocking), Error> {
    input(rows, 2)?;
    let (stats, plan) = super::products::global_statistics(rows, context.0.selected.bytes).map_err(Error::Source)?;
    if stats.offset.iter().chain(&stats.scale).chain(&stats.covariance).any(|v| !v.is_finite()) {
        return Err(Error::NonFiniteOutput);
    }
    Ok((stats, plan))
}

pub fn parent_summary_with_context(
    child_base: &[[f32; 2]], context: &GeometryContext,
) -> Result<(([f32; 2], [f32; 4]), Blocking), Error> {
    input(child_base, 1)?;
    let (summary, plan) = super::products::parent_summary(child_base, context.0.selected.bytes).map_err(Error::Source)?;
    if summary.0.iter().chain(&summary.1).any(|v| !v.is_finite()) { return Err(Error::NonFiniteOutput); }
    Ok((summary, plan))
}

