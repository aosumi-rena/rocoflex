use std::collections::{TryReserveError, VecDeque};
use crate::{cepstral, cepstral_parameters as parameters, value_row_allocation as rows};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout { pub input_width: usize, pub output_width: usize }

pub struct BandInput<P> {
    pub range: cepstral::BandRange,
    pub projection: P,
    pub pending: VecDeque<Vec<f32>>,
    pub split_allocation_words: [u32; 2],
}
pub struct Inputs<P> {
    pub layout: Layout,
    pub bands: Vec<BandInput<P>>,
    pub outputs: [cepstral::Output; 2],
    pub scratch_allocation_words: [u32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    BandCount { expected: usize, actual: usize }, InputWidth, OutputWidth,
    BandRange { band: usize },
}

pub(crate) fn admit_domain(expected_count: usize, actual_count: usize, layout: Layout,
    ranges: impl IntoIterator<Item = cepstral::BandRange>) -> Option<Domain>
{
    if actual_count != expected_count {
        Some(Domain::BandCount { expected: expected_count, actual: actual_count })
    } else if layout.input_width == 0 || layout.input_width > i32::MAX as usize {
        Some(Domain::InputWidth)
    } else if layout.output_width == 0 || layout.output_width > i32::MAX as usize {
        Some(Domain::OutputWidth)
    } else {
        ranges.into_iter().position(|range| !range.admit(layout.output_width)
            || range.end - range.start > i32::MAX as usize / 2)
            .map(|band| Domain::BandRange { band })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationSite { WorkspaceSlots, Split { band: usize, which: usize }, Scratch(usize), BandOwners }
pub enum AllocationError { Capacity(TryReserveError), Row(rows::LogicalError) }

pub struct AllocationFailure<'n, 'data, P> {
    parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P>,
    split: Vec<[Option<Vec<f32>>; 2]>, scratch: [Option<Vec<f32>>; 2],
    site: AllocationSite, cause: AllocationError,
}
impl<'n, 'data, P> AllocationFailure<'n, 'data, P> {
    pub fn parameters(&self) -> &parameters::Decoded<'n, 'data> { &self.parameters }
    pub fn inputs(&self) -> &Inputs<P> { &self.inputs }
    pub fn split(&self) -> &[[Option<Vec<f32>>; 2]] { &self.split }
    pub fn scratch(&self) -> &[Option<Vec<f32>>; 2] { &self.scratch }
    pub fn site(&self) -> AllocationSite { self.site }
    pub fn cause(&self) -> &AllocationError { &self.cause }
}
pub enum Failure<'n, 'data, P> {
    Domain { cause: Domain, parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P> },
    Allocation(AllocationFailure<'n, 'data, P>),
    Binding(parameters::BindingFailure<'n, 'data, P>),
}
enum FactoryFailureOwner<'n, 'data, P> {
    Initialization { failure: Failure<'n, 'data, P>, factories: [crate::streaming_rows::ValueRows; 2] },
    Binding(parameters::FactoryBindingFailure<'n, 'data, P>),
}

pub struct FactoryFailure<'n, 'data, P> { owner: FactoryFailureOwner<'n, 'data, P> }
impl<'n, 'data, P> FactoryFailure<'n, 'data, P> {
    pub fn initialization_failure(&self) -> Option<&Failure<'n, 'data, P>> {
        match &self.owner { FactoryFailureOwner::Initialization { failure, .. } => Some(failure), _ => None }
    }
    pub fn untouched_factories(&self) -> Option<&[crate::streaming_rows::ValueRows; 2]> {
        match &self.owner { FactoryFailureOwner::Initialization { factories, .. } => Some(factories), _ => None }
    }
    pub fn binding_failure(&self) -> Option<&parameters::FactoryBindingFailure<'n, 'data, P>> {
        match &self.owner { FactoryFailureOwner::Binding(failure) => Some(failure), _ => None }
    }
}
struct Staging<'n, 'data, P> {
    parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P>,
    split: Vec<[Option<Vec<f32>>; 2]>, scratch: [Option<Vec<f32>>; 2],
}
impl<'n, 'data, P> Staging<'n, 'data, P> {
    fn failed(self, site: AllocationSite, cause: AllocationError) -> Failure<'n, 'data, P> {
        let Self { parameters, inputs, split, scratch } = self;
        Failure::Allocation(AllocationFailure { parameters, inputs, split, scratch, site, cause })
    }
}

pub fn initialize<'n, 'data, P>(parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P>)
    -> Result<parameters::Bound<'n, 'data, P>, Failure<'n, 'data, P>>
{
    let (parameters, parts) = build_parts(parameters, inputs)?;
    parameters.bind_initialized(parts).map_err(Failure::Binding)
}

pub fn initialize_with_output_factories<'n, 'data, P>(parameters: parameters::Decoded<'n, 'data>,
    inputs: Inputs<P>, factories: [crate::streaming_rows::ValueRows; 2])
    -> Result<parameters::Bound<'n, 'data, P>, FactoryFailure<'n, 'data, P>>
{
    let (parameters, parts) = match build_parts(parameters, inputs) {
        Ok(parts) => parts,
        Err(failure) => return Err(FactoryFailure {
            owner: FactoryFailureOwner::Initialization { failure, factories },
        }),
    };
    parameters.bind_factory_initialized(parts, factories)
        .map_err(|failure| FactoryFailure { owner: FactoryFailureOwner::Binding(failure) })
}

fn build_parts<'n, 'data, P>(parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P>)
    -> Result<(parameters::Decoded<'n, 'data>, cepstral::Parts<P>), Failure<'n, 'data, P>>
{
    let count = parameters.bands().len();
    let cause = admit_domain(count, inputs.bands.len(), inputs.layout,
        inputs.bands.iter().map(|band| band.range));
    if let Some(cause) = cause { return Err(Failure::Domain { cause, parameters, inputs }); }

    let mut staged = Staging { parameters, inputs, split: Vec::new(), scratch: [None, None] };
    if let Err(cause) = staged.split.try_reserve_exact(count) {
        return Err(staged.failed(AllocationSite::WorkspaceSlots, AllocationError::Capacity(cause)));
    }
    staged.split.resize_with(count, || [None, None]);
    for band in 0..count {
        let input = &staged.inputs.bands[band];
        let width = input.range.end - input.range.start;
        for which in 0..2 {
            match rows::allocate_logical_row(width, input.split_allocation_words[which]) {
                Ok(row) => staged.split[band][which] = Some(row),
                Err(cause) => {
                    return Err(staged.failed(AllocationSite::Split { band, which }, AllocationError::Row(cause)));
                }
            }
        }
    }
    for which in 0..2 {
        match rows::allocate_logical_row(staged.inputs.layout.output_width,
            staged.inputs.scratch_allocation_words[which])
        {
            Ok(row) => staged.scratch[which] = Some(row),
            Err(cause) => {
                return Err(staged.failed(AllocationSite::Scratch(which), AllocationError::Row(cause)));
            }
        }
    }
    let mut bands = Vec::new();
    if let Err(cause) = bands.try_reserve_exact(count) {
        return Err(staged.failed(AllocationSite::BandOwners, AllocationError::Capacity(cause)));
    }
    let Staging { parameters, inputs, split, scratch } = staged;
    let Inputs { layout, bands: owners, outputs, .. } = inputs;
    for (owner, [first, second]) in owners.into_iter().zip(split) {

        bands.push(cepstral::Band { range: owner.range, projection: owner.projection,
            pending: owner.pending, split: [first.expect("allocated first split"), second.expect("allocated second split")] });
    }
    let [first, second] = scratch;
    let mode80 = parameters.mode();
    Ok((parameters, cepstral::Parts {
        mode80, band_count84: count, input_width: layout.input_width,
        output_width: layout.output_width, bands, outputs,
        scratch: [first.expect("allocated first scratch"), second.expect("allocated second scratch")],
    }))
}
