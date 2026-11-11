use std::collections::{TryReserveError, VecDeque};
use super::{BandRange, Output, Phase, Error, Failure, Poll, RetiredRow,
    BandStorage, BandView, StorageView, WorkView, output_rows, row_ok, width_ok,
    poll_mixer, poll_result};
use crate::{conditional_gate::{Batch, Projection}, streaming_rows::ValueRows};

pub struct Band<P> {
    pub range: BandRange, pub projection: P, pub pending: VecDeque<Vec<f32>>,
    pub split: Vec<Vec<f32>>,
}
pub struct Parts<P> {
    pub runtime_count: usize, pub band_count: usize,
    pub input_width: usize, pub output_width: usize,
    pub bands: Vec<Band<P>>, pub outputs: Vec<Output>, pub scratch: Vec<Vec<f32>>,
}
pub enum Admission {
    Count, Dimensions, BandCount,
    Extent { expected: usize, outputs: usize, scratch: usize, factories: usize },
    Band { band: usize }, Output { which: usize },
    MissingRow { which: usize, index: usize }, FactoryStopped { which: usize },
    FactorySlots(TryReserveError),
}

pub struct BindingFailure<P> { cause: Admission, parts: Parts<P>, factories: Vec<ValueRows> }
impl<P> BindingFailure<P> {
    pub fn cause(&self) -> &Admission { &self.cause }
    pub fn parts(&self) -> &Parts<P> { &self.parts }
    pub fn factories(&self) -> &[ValueRows] { &self.factories }
}
pub struct State<P> { parts: Parts<P>, factories: Vec<output_rows::FactoryRows> }
pub struct StartFailure<P> { state: State<P>, input: Batch }
impl<P> StartFailure<P> {
    pub fn state(&self) -> &State<P> { &self.state }
    pub fn input(&self) -> &Batch { &self.input }
}
pub struct Trace {
    pub input: Batch, pub projected_after_move: Vec<Option<Batch>>,
    pub fresh_counts: Vec<Option<usize>>, pub emitted_count: usize,
    pub retired: Vec<RetiredRow>, pub replaced_output_rows: Vec<Vec<Option<Vec<f32>>>>,
}
pub struct Run<P> {
    state: State<P>, input: Batch, projected: Vec<Option<Batch>>,
    fresh_counts: Vec<Option<usize>>, minimum: usize, phase: Phase,
    retired: Vec<RetiredRow>, replaced: Vec<Vec<Option<Vec<f32>>>>, failure: Option<Failure>,
}
pub struct Completed<P> { state: State<P>, trace: Trace, packets: Vec<Batch> }

fn admission<P>(parts: &Parts<P>, factories: &[ValueRows]) -> Option<Admission> {
    let count = parts.runtime_count;
    if count == 0 || count > i32::MAX as usize { return Some(Admission::Count); }
    if !width_ok(parts.input_width) || !width_ok(parts.output_width) { return Some(Admission::Dimensions); }
    if parts.bands.is_empty() || parts.bands.len() > i32::MAX as usize || parts.band_count != parts.bands.len() {
        return Some(Admission::BandCount);
    }
    if parts.outputs.len() != count || parts.scratch.len() != count || factories.len() != count {
        return Some(Admission::Extent { expected: count, outputs: parts.outputs.len(),
            scratch: parts.scratch.len(), factories: factories.len() });
    }
    for (band, value) in parts.bands.iter().enumerate() {
        if !value.range.admit(parts.output_width) { return Some(Admission::Band { band }); }
        let width = value.range.end - value.range.start;

        let Some(projected_width) = count.checked_mul(width).filter(|n| *n <= i32::MAX as usize) else {
            return Some(Admission::Band { band });
        };
        if value.split.len() != count || value.split.iter().any(|row| row.len() != width)
            || value.pending.len() > i32::MAX as usize
            || value.pending.iter().any(|row| !row_ok(row, projected_width)) {
            return Some(Admission::Band { band });
        }
    }
    for (which, (output, scratch)) in parts.outputs.iter().zip(&parts.scratch).enumerate() {
        if output.rows.len() > i32::MAX as usize || scratch.len() != parts.output_width {
            return Some(Admission::Output { which });
        }

    }
    factories.iter().position(|factory| factory.progress().stopped.is_some())
        .map(|which| Admission::FactoryStopped { which })
}
impl<P> State<P> {

    pub fn from_explicit_parts(parts: Parts<P>, factories: Vec<ValueRows>) -> Result<Self, BindingFailure<P>> {
        if let Some(cause) = admission(&parts, &factories) { return Err(BindingFailure { cause, parts, factories }); }
        let mut owned = Vec::new();
        if let Err(cause) = owned.try_reserve_exact(factories.len()) {
            return Err(BindingFailure { cause: Admission::FactorySlots(cause), parts, factories });
        }
        for factory in factories { owned.push(output_rows::FactoryRows::new(factory)); }
        Ok(Self { parts, factories: owned })
    }
    pub fn parts(&self) -> &Parts<P> { &self.parts }
    pub fn output_factories(&self) -> &[output_rows::FactoryRows] { &self.factories }

    pub fn begin(self, input: Batch) -> Result<Run<P>, StartFailure<P>> {
        if input.rows.len() > i32::MAX as usize || input.rows.iter().any(|row| !row_ok(row, self.parts.input_width)) {
            return Err(StartFailure { state: self, input });
        }
        let bands = self.parts.bands.len();
        let count = self.parts.runtime_count;
        Ok(Run { state: self, input, projected: (0..bands).map(|_| None).collect(),
            fresh_counts: vec![None; bands], minimum: i32::MAX as usize, phase: Phase::Project(0),
            retired: Vec::new(), replaced: (0..count).map(|_| Vec::new()).collect(), failure: None })
    }
}
impl<P> BandStorage for Band<P> {
    type Projection = P;
    fn view(&mut self) -> BandView<'_, P> {
        BandView { range: self.range, projection: &mut self.projection,
            pending: &mut self.pending, split: &mut self.split }
    }
}
impl<P: Projection> Run<P> {
    pub fn state(&self) -> &State<P> { &self.state }
    pub fn input(&self) -> &Batch { &self.input }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }
    pub fn projected(&self) -> &[Option<Batch>] { &self.projected }
    pub fn fresh_counts(&self) -> &[Option<usize>] { &self.fresh_counts }
    pub fn retired(&self) -> &[RetiredRow] { &self.retired }
    pub fn poll(&mut self, budget: usize) -> Poll<'_> {
        let State { parts, factories } = &mut self.state;
        poll_mixer(StorageView { bands: &mut parts.bands, outputs: &mut parts.outputs,
            scratch: &mut parts.scratch, factories: Some(factories.as_mut_slice()), output_width: parts.output_width },
            WorkView { input: &self.input, projected: &mut self.projected, fresh_counts: &mut self.fresh_counts,
                minimum: &mut self.minimum, phase: &mut self.phase, retired: &mut self.retired,
                replaced: &mut self.replaced, failure: &mut self.failure }, budget);
        poll_result(&self.failure, self.phase)
    }

    pub fn into_completed(mut self) -> Result<Completed<P>, Self> {
        if self.failure.is_some() || self.phase != Phase::Complete { return Err(self); }
        let packets = self.state.parts.outputs.iter_mut().map(|output| Batch {
            rows: output.rows.drain(..).map(|row| row.expect("all output copies completed")).collect(),
            status: output.status,
        }).collect();
        let trace = Trace { input: self.input, projected_after_move: self.projected, fresh_counts: self.fresh_counts,
            emitted_count: self.minimum, retired: self.retired, replaced_output_rows: self.replaced };
        Ok(Completed { state: self.state, trace, packets })
    }
}
impl<P> Completed<P> {
    pub fn state(&self) -> &State<P> { &self.state }
    pub fn trace(&self) -> &Trace { &self.trace }
    pub fn packets(&self) -> &[Batch] { &self.packets }
    pub fn into_parts(self) -> (State<P>, Trace, Vec<Batch>) { (self.state, self.trace, self.packets) }
}
