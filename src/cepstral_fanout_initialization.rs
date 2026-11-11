use std::collections::{TryReserveError, VecDeque};
use crate::{cepstral::{self, fanout}, streaming_rows::ValueRows,
    value_row_allocation as rows};

pub struct BandInput<P> {
    pub range: cepstral::BandRange,
    pub projection: P,
    pub pending: VecDeque<Vec<f32>>,
    pub split_allocation_words: Vec<u32>,
}
pub struct Inputs<P> {
    pub runtime_count: usize, pub input_width: usize, pub output_width: usize,
    pub bands: Vec<BandInput<P>>, pub outputs: Vec<cepstral::Output>,
    pub scratch_allocation_words: Vec<u32>, pub factories: Vec<ValueRows>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    Count, Dimensions, BandCount,
    Extent { expected: usize, outputs: usize, scratch_words: usize, factories: usize },
    BandRange { band: usize },
    SplitWords { band: usize, expected: usize, actual: usize },
    Pending { band: usize },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationSite {
    SplitBands, SplitSlots { band: usize }, SplitRow { band: usize, which: usize },
    ScratchSlots, ScratchRow { which: usize }, BandOwners,
}
pub enum InitializationCause {
    Domain(Domain), Capacity { site: AllocationSite, cause: TryReserveError },
    Row { site: AllocationSite, cause: rows::LogicalError },
}
struct Staging<P> {
    inputs: Inputs<P>, split: Vec<Vec<Vec<f32>>>, scratch: Vec<Vec<f32>>,
}
enum FailureOwner<P> {
    Initialization { staged: Staging<P>, cause: InitializationCause },
    Binding(fanout::BindingFailure<P>),
}

pub struct Failure<P> { owner: FailureOwner<P> }
impl<P> Failure<P> {
    pub fn inputs(&self) -> Option<&Inputs<P>> {
        match &self.owner { FailureOwner::Initialization { staged, .. } => Some(&staged.inputs), _ => None }
    }
    pub fn split(&self) -> Option<&[Vec<Vec<f32>>]> {
        match &self.owner { FailureOwner::Initialization { staged, .. } => Some(&staged.split), _ => None }
    }
    pub fn scratch(&self) -> Option<&[Vec<f32>]> {
        match &self.owner { FailureOwner::Initialization { staged, .. } => Some(&staged.scratch), _ => None }
    }
    pub fn initialization_cause(&self) -> Option<&InitializationCause> {
        match &self.owner { FailureOwner::Initialization { cause, .. } => Some(cause), _ => None }
    }
    pub fn binding_failure(&self) -> Option<&fanout::BindingFailure<P>> {
        match &self.owner { FailureOwner::Binding(failure) => Some(failure), _ => None }
    }
}
impl<P> Staging<P> {
    fn failed(self, cause: InitializationCause) -> Failure<P> {
        Failure { owner: FailureOwner::Initialization { staged: self, cause } }
    }
}
fn domain<P>(inputs: &Inputs<P>) -> Option<Domain> {
    let count = inputs.runtime_count;
    if count == 0 || count > i32::MAX as usize { return Some(Domain::Count); }
    let width_ok = |width| width > 0 && width <= i32::MAX as usize;
    if !width_ok(inputs.input_width) || !width_ok(inputs.output_width) { return Some(Domain::Dimensions); }
    if inputs.bands.is_empty() || inputs.bands.len() > i32::MAX as usize { return Some(Domain::BandCount); }
    if inputs.outputs.len() != count || inputs.scratch_allocation_words.len() != count || inputs.factories.len() != count {
        return Some(Domain::Extent { expected: count, outputs: inputs.outputs.len(),
            scratch_words: inputs.scratch_allocation_words.len(), factories: inputs.factories.len() });
    }
    for (band, input) in inputs.bands.iter().enumerate() {
        if !input.range.admit(inputs.output_width) { return Some(Domain::BandRange { band }); }
        let width = input.range.end - input.range.start;
        let Some(projected_width) = count.checked_mul(width).filter(|width| *width <= i32::MAX as usize) else {
            return Some(Domain::BandRange { band });
        };
        if input.split_allocation_words.len() != count {
            return Some(Domain::SplitWords { band, expected: count, actual: input.split_allocation_words.len() });
        }
        if input.pending.len() > i32::MAX as usize || input.pending.iter()
            .any(|row| row.len() != projected_width || row.iter().any(|value| !value.is_finite())) {
            return Some(Domain::Pending { band });
        }
    }
    None
}

pub fn initialize<P>(inputs: Inputs<P>) -> Result<fanout::State<P>, Failure<P>> {
    let mut staged = Staging { inputs, split: Vec::new(), scratch: Vec::new() };
    if let Some(cause) = domain(&staged.inputs) {
        return Err(staged.failed(InitializationCause::Domain(cause)));
    }
    let count = staged.inputs.runtime_count;
    let band_count = staged.inputs.bands.len();
    if let Err(cause) = staged.split.try_reserve_exact(band_count) {
        return Err(staged.failed(InitializationCause::Capacity { site: AllocationSite::SplitBands, cause }));
    }
    staged.split.resize_with(band_count, Vec::new);
    for band in 0..band_count {
        if let Err(cause) = staged.split[band].try_reserve_exact(count) {
            return Err(staged.failed(InitializationCause::Capacity { site: AllocationSite::SplitSlots { band }, cause }));
        }
        let input = &staged.inputs.bands[band];
        let width = input.range.end - input.range.start;
        for which in 0..count {
            match rows::allocate_logical_row(width, input.split_allocation_words[which]) {
                Ok(row) => staged.split[band].push(row),
                Err(cause) => return Err(staged.failed(InitializationCause::Row {
                    site: AllocationSite::SplitRow { band, which }, cause,
                })),
            }
        }
    }
    if let Err(cause) = staged.scratch.try_reserve_exact(count) {
        return Err(staged.failed(InitializationCause::Capacity { site: AllocationSite::ScratchSlots, cause }));
    }
    for which in 0..count {
        match rows::allocate_logical_row(staged.inputs.output_width, staged.inputs.scratch_allocation_words[which]) {
            Ok(row) => staged.scratch.push(row),
            Err(cause) => return Err(staged.failed(InitializationCause::Row {
                site: AllocationSite::ScratchRow { which }, cause,
            })),
        }
    }
    let mut bands = Vec::new();
    if let Err(cause) = bands.try_reserve_exact(band_count) {
        return Err(staged.failed(InitializationCause::Capacity { site: AllocationSite::BandOwners, cause }));
    }
    let Staging { inputs, split, scratch } = staged;
    let Inputs { runtime_count, input_width, output_width, bands: owners, outputs, factories, .. } = inputs;
    for (owner, split) in owners.into_iter().zip(split) {
        bands.push(fanout::Band { range: owner.range, projection: owner.projection, pending: owner.pending, split });
    }
    fanout::State::from_explicit_parts(fanout::Parts {
        runtime_count, band_count, input_width, output_width, bands, outputs, scratch,
    }, factories).map_err(|failure| Failure { owner: FailureOwner::Binding(failure) })
}
