use std::collections::VecDeque;
#[path = "cepstral_queue.rs"]
pub mod queue;
#[path = "cepstral_output_rows.rs"]
pub mod output_rows;
#[path = "cepstral_fanout.rs"]
pub mod fanout;
use crate::conditional_gate::{Batch, GateError, Projection};
pub use crate::cepstral_values::{SourceCepstralBandRange as BandRange, ValueError};

pub struct Band<P> {
    pub range: BandRange, pub projection: P, pub pending: VecDeque<Vec<f32>>,
    pub split: [Vec<f32>; 2],
}
pub struct Output { pub rows: Vec<Option<Vec<f32>>>, pub status: u8 }
pub struct Parts<P> {
    pub mode80: u32, pub band_count84: usize, pub input_width: usize,
    pub output_width: usize, pub bands: Vec<Band<P>>, pub outputs: [Output; 2],
    pub scratch: [Vec<f32>; 2],
}
pub struct State<P> { parts: Parts<P>, output_rows: Option<[output_rows::FactoryRows; 2]> }
pub struct BindingFailure<P> { pub reason: &'static str, pub parts: Parts<P> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactoryBindingCause {
    Parts(&'static str), MissingRow { which: usize, index: usize }, FactoryStopped { which: usize },
}
pub struct FactoryBindingFailure<P> {
    cause: FactoryBindingCause, parts: Parts<P>, factories: [crate::streaming_rows::ValueRows; 2],
}
impl<P> FactoryBindingFailure<P> {
    pub fn cause(&self) -> FactoryBindingCause { self.cause }
    pub fn parts(&self) -> &Parts<P> { &self.parts }
    pub fn factories(&self) -> &[crate::streaming_rows::ValueRows; 2] { &self.factories }
}
pub struct StartFailure<P> { pub reason: &'static str, pub state: State<P>, pub input: Batch }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Project(usize), ProjectInFlight(usize), Append(usize), Prepare(usize), PrepareInFlight(usize),
    Clear { row: usize, which: usize }, Split { row: usize, band: usize },
    Retire { row: usize, band: usize }, Blend { row: usize, band: usize, which: usize },
    Publish { row: usize, which: usize }, Complete, Failed,
}
pub enum Error {
    Environment(crate::matrix_f32::Error), Projection { band: usize, error: GateError },
    ProjectionWidth { band: usize }, Count, MissingFront { row: usize, band: usize },
    Blend { row: usize, band: usize, which: usize, error: ValueError },
    Allocation(std::collections::TryReserveError), Interrupted,
    OutputPreparation { which: usize, cause: output_rows::Error },
}
pub struct Failure { pub phase: Phase, pub cause: Error }
pub enum Poll<'a> { Pending(Phase), Complete, Failed(&'a Failure) }
pub struct RetiredRow { pub row: usize, pub band: usize, pub values: Vec<f32> }
pub struct Trace {
    pub input: Batch, pub projected_after_move: Vec<Option<Batch>>,
    pub fresh_counts: Vec<Option<usize>>, pub emitted_count: usize,
    pub retired: Vec<RetiredRow>, pub replaced_output_rows: [Vec<Option<Vec<f32>>>; 2],
}
pub struct Run<P> {
    state: State<P>, input: Batch, projected: Vec<Option<Batch>>,
    fresh_counts: Vec<Option<usize>>, minimum: usize, phase: Phase,
    retired: Vec<RetiredRow>, replaced: [Vec<Option<Vec<f32>>>; 2], failure: Option<Failure>,
}
pub struct Completed<P> { state: State<P>, trace: Trace, packets: [Batch; 2] }

fn width_ok(width: usize) -> bool { width > 0 && width <= i32::MAX as usize }
fn row_ok(row: &[f32], width: usize) -> bool {
    row.len() == width && row.iter().all(|v| v.is_finite())
}
impl<P> State<P> {

    pub fn from_explicit_parts(parts: Parts<P>) -> Result<Self, BindingFailure<P>> {
        if let Some(reason) = parts_reason(&parts, false) { return Err(BindingFailure { reason, parts }); }
        Ok(Self { parts, output_rows: None })
    }

    pub fn from_explicit_factory_parts(parts: Parts<P>, factories: [crate::streaming_rows::ValueRows; 2])
        -> Result<Self, FactoryBindingFailure<P>>
    {
        let cause = if let Some(reason) = parts_reason(&parts, true) {
            Some(FactoryBindingCause::Parts(reason))
        } else {
            factories.iter().position(|factory| factory.progress().stopped.is_some())
                .map(|which| FactoryBindingCause::FactoryStopped { which })
        };
        if let Some(cause) = cause { return Err(FactoryBindingFailure { cause, parts, factories }); }
        Ok(Self { parts, output_rows: Some(factories.map(output_rows::FactoryRows::new)) })
    }
    pub fn output_factories(&self) -> Option<&[output_rows::FactoryRows; 2]> { self.output_rows.as_ref() }
    pub fn parts(&self) -> &Parts<P> { &self.parts }
    pub fn begin(self, input: Batch) -> Result<Run<P>, StartFailure<P>> {
        if input.rows.len() > i32::MAX as usize
            || input.rows.iter().any(|row| !row_ok(row, self.parts.input_width)) {
            return Err(StartFailure { reason: "cepstral input domain", state: self, input });
        }
        let count = self.parts.bands.len();
        Ok(Run { state: self, input, projected: (0..count).map(|_| None).collect(),
            fresh_counts: vec![None; count], minimum: i32::MAX as usize,
            phase: Phase::Project(0), retired: Vec::new(), replaced: [Vec::new(), Vec::new()], failure: None })
    }
}

fn parts_reason<P>(parts: &Parts<P>, factory_mode: bool) -> Option<&'static str> {
    if parts.mode80 != 2 || parts.band_count84 == 0
            || parts.band_count84 != parts.bands.len() || parts.bands.len() > i32::MAX as usize
            || !width_ok(parts.input_width) || !width_ok(parts.output_width) {
            Some("cepstral mode2 dimensions/count are not admitted")
        } else if parts.bands.iter().any(|band| {
            if !band.range.admit(parts.output_width) { return true; }
            let width = band.range.end - band.range.start;
            width > i32::MAX as usize / 2
                || band.split.iter().any(|row| row.len() != width)
                || band.pending.len() > i32::MAX as usize
                || band.pending.iter().any(|row| !row_ok(row, 2 * width))
        }) { Some("cepstral band range/workspace/history is not admitted")
        } else if parts.scratch.iter().any(|row| row.len() != parts.output_width)
            || parts.outputs.iter().any(|out| out.rows.len() > i32::MAX as usize
                || (!factory_mode && out.rows.iter().flatten().any(|row| row.len() != parts.output_width))) {
            Some("cepstral output/scratch width is not admitted")
    } else { None }
}
impl<P: Projection> Run<P> {
    pub fn state(&self) -> &State<P> { &self.state }
    pub fn input(&self) -> &Batch { &self.input }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn fresh_counts(&self) -> &[Option<usize>] { &self.fresh_counts }
    pub fn projected(&self) -> &[Option<Batch>] { &self.projected }
    pub fn retired(&self) -> &[RetiredRow] { &self.retired }
    pub fn failure(&self) -> Option<&Failure> { self.failure.as_ref() }

    pub fn poll(&mut self, budget: usize) -> Poll<'_> {
        let State { parts, output_rows } = &mut self.state;
        poll_mixer(StorageView { bands: &mut parts.bands, outputs: &mut parts.outputs,
            scratch: &mut parts.scratch, factories: output_rows.as_mut().map(|rows| rows.as_mut_slice()),
            output_width: parts.output_width },
            WorkView { input: &self.input, projected: &mut self.projected, fresh_counts: &mut self.fresh_counts,
                minimum: &mut self.minimum, phase: &mut self.phase, retired: &mut self.retired,
                replaced: &mut self.replaced, failure: &mut self.failure }, budget);
        poll_result(&self.failure, self.phase)
    }
    pub fn into_completed(mut self) -> Result<Completed<P>, Self> {
        if self.failure.is_some() || self.phase != Phase::Complete { return Err(self); }
        let packets = std::array::from_fn(|which| Batch {
            rows: self.state.parts.outputs[which].rows.drain(..).map(|row| row.expect("both output copies completed")).collect(),
            status: self.state.parts.outputs[which].status,
        });
        let trace = Trace { input: self.input, projected_after_move: self.projected,
            fresh_counts: self.fresh_counts, emitted_count: self.minimum,
            retired: self.retired, replaced_output_rows: self.replaced };
        Ok(Completed { state: self.state, trace, packets })
    }
}

struct BandView<'a, P> {
    range: BandRange, projection: &'a mut P, pending: &'a mut VecDeque<Vec<f32>>,
    split: &'a mut [Vec<f32>],
}
trait BandStorage {
    type Projection;
    fn view(&mut self) -> BandView<'_, Self::Projection>;
}
impl<P> BandStorage for Band<P> {
    type Projection = P;
    fn view(&mut self) -> BandView<'_, P> {
        BandView { range: self.range, projection: &mut self.projection,
            pending: &mut self.pending, split: &mut self.split }
    }
}
struct StorageView<'s, B> {
    bands: &'s mut [B], outputs: &'s mut [Output], scratch: &'s mut [Vec<f32>],
    factories: Option<&'s mut [output_rows::FactoryRows]>, output_width: usize,
}
struct WorkView<'w> {
    input: &'w Batch, projected: &'w mut [Option<Batch>], fresh_counts: &'w mut [Option<usize>],
    minimum: &'w mut usize, phase: &'w mut Phase, retired: &'w mut Vec<RetiredRow>,
    replaced: &'w mut [Vec<Option<Vec<f32>>>], failure: &'w mut Option<Failure>,
}
fn poll_result(failure: &Option<Failure>, phase: Phase) -> Poll<'_> {
    if let Some(failure) = failure { Poll::Failed(failure) }
    else if phase == Phase::Complete { Poll::Complete }
    else { Poll::Pending(phase) }
}

fn poll_mixer<B: BandStorage>(mut storage: StorageView<'_, B>, mut work: WorkView<'_>, budget: usize)
where B::Projection: Projection
{
    if work.failure.is_none() {
        for _ in 0..budget {
            if let Err(cause) = advance_mixer(&mut storage, &mut work) {
                *work.failure = Some(Failure { phase: *work.phase, cause });
                *work.phase = Phase::Failed;
                break;
            }
            if matches!(*work.phase, Phase::Complete | Phase::Failed) { break; }
        }
    }
}

fn advance_mixer<B: BandStorage>(storage: &mut StorageView<'_, B>, work: &mut WorkView<'_>) -> Result<(), Error>
where B::Projection: Projection
{
        let output_count = storage.outputs.len();
        match *work.phase {
            Phase::Project(band) => {
                crate::matrix_f32::check_environment().map_err(Error::Environment)?;
                *work.phase = Phase::ProjectInFlight(band);
                let current = storage.bands[band].view();
                let output = current.projection.forward(work.input)
                    .map_err(|error| Error::Projection { band, error })?;
                work.projected[band] = Some(output);
                let output = work.projected[band].as_ref().expect("actual projection retained");
                let width = output_count * (current.range.end - current.range.start);
                if output.rows.len() > i32::MAX as usize { return Err(Error::Count); }
                if output.rows.iter().any(|row| !row_ok(row, width)) { return Err(Error::ProjectionWidth { band }); }
                work.fresh_counts[band] = Some(output.rows.len());

                *work.minimum = (*work.minimum).min(output.rows.len());
                *work.phase = Phase::Append(band);
            }
            Phase::Append(band) => {
                let output = work.projected[band].as_mut().expect("completed projection");
                let current = storage.bands[band].view();
                let queue = current.pending;
                if queue.len().checked_add(output.rows.len()).is_none_or(|n| n > i32::MAX as usize) { return Err(Error::Count); }
                queue.try_reserve(output.rows.len()).map_err(Error::Allocation)?;
                queue.extend(output.rows.drain(..));
                *work.phase = if band + 1 < storage.bands.len() { Phase::Project(band + 1) } else { Phase::Prepare(0) };
            }
            Phase::Prepare(which) => {
                if let Some(factories) = storage.factories.as_mut() {
                    *work.phase = Phase::PrepareInFlight(which);
                    factories[which].prepare(&mut storage.outputs[which].rows,
                        *work.minimum, storage.output_width)
                        .map_err(|cause| Error::OutputPreparation { which, cause })?;
                } else {
                    let out = &mut storage.outputs[which];
                    if out.rows.len() > *work.minimum {
                        work.replaced[which].try_reserve(out.rows.len() - *work.minimum).map_err(Error::Allocation)?;
                        work.replaced[which].extend(out.rows.drain(*work.minimum..));
                    }
                    out.rows.try_reserve((*work.minimum).saturating_sub(out.rows.len())).map_err(Error::Allocation)?;

                    out.rows.resize_with(*work.minimum, || None);
                }

                storage.outputs[which].status = work.input.status;
                *work.phase = if which + 1 < output_count { Phase::Prepare(which + 1) }
                    else if *work.minimum == 0 { Phase::Complete } else { Phase::Clear { row: 0, which: 0 } };
            }
            Phase::Clear { row, which } => {

                storage.scratch[which].fill(0.0);
                *work.phase = if which + 1 < output_count { Phase::Clear { row, which: which + 1 } } else { Phase::Split { row, band: 0 } };
            }
            Phase::Split { row, band } => {
                let current = storage.bands[band].view();
                let front = current.pending.front().ok_or(Error::MissingFront { row, band })?;
                let width = current.range.end - current.range.start;
                if front.len() != width * output_count { return Err(Error::ProjectionWidth { band }); }

                for (which, target) in current.split.iter_mut().enumerate() {
                    target.copy_from_slice(&front[which * width..(which + 1) * width]);
                }
                *work.phase = Phase::Retire { row, band };
            }
            Phase::Retire { row, band } => {
                work.retired.try_reserve(1).map_err(Error::Allocation)?;
                let values = storage.bands[band].view().pending.pop_front().ok_or(Error::MissingFront { row, band })?;

                work.retired.push(RetiredRow { row, band, values });
                *work.phase = Phase::Blend { row, band, which: 0 };
            }
            Phase::Blend { row, band, which } => {
                crate::matrix_f32::check_environment().map_err(Error::Environment)?;
                let current = storage.bands[band].view();
                current.range.blend(&current.split[which], &mut storage.scratch[which])
                    .map_err(|error| Error::Blend { row, band, which, error })?;
                *work.phase = if which + 1 < output_count { Phase::Blend { row, band, which: which + 1 } }
                    else if band + 1 < storage.bands.len() { Phase::Split { row, band: band + 1 } }
                    else { Phase::Publish { row, which: 0 } };
            }
            Phase::Publish { row, which } => {
                let values = &storage.scratch[which];
                let target = &mut storage.outputs[which].rows[row];
                if let Some(target_values) = target.as_mut() {
                    target_values.copy_from_slice(values);
                } else {
                    if storage.factories.is_some() {
                        return Err(Error::OutputPreparation { which,
                            cause: output_rows::Error::MissingRow { index: row } });
                    }
                    let mut copied = Vec::new();
                    copied.try_reserve_exact(values.len()).map_err(Error::Allocation)?;
                    copied.extend_from_slice(values);
                    *target = Some(copied);
                }
                *work.phase = if which + 1 < output_count { Phase::Publish { row, which: which + 1 } }
                    else if row + 1 < *work.minimum { Phase::Clear { row: row + 1, which: 0 } } else { Phase::Complete };
            }
            Phase::ProjectInFlight(_) | Phase::PrepareInFlight(_) => return Err(Error::Interrupted),
            Phase::Complete | Phase::Failed => {}
        }
        Ok(())
}
impl<P> Completed<P> {
    pub fn state(&self) -> &State<P> { &self.state }
    pub fn trace(&self) -> &Trace { &self.trace }
    pub fn packets(&self) -> &[Batch; 2] { &self.packets }
    pub fn into_parts(self) -> (State<P>, Trace, [Batch; 2]) { (self.state, self.trace, self.packets) }
}
