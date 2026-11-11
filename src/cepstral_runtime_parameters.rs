use std::collections::{TryReserveError, VecDeque};
use crate::{cepstral, cepstral_parameters as parameters, cepstral_value_binding as values};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeBandFields {
    start: i32, end: i32, left_fade: i32, right_fade: i32,
}
impl RuntimeBandFields {

    pub fn from_explicit_runtime_prefix(prefix: [u8; 16]) -> Self {
        let word = |offset| i32::from_le_bytes(prefix[offset..offset + 4].try_into().expect("four runtime bytes"));
        Self::from_explicit_fields(word(0), word(4), word(8), word(12))
    }
    pub const fn from_explicit_fields(start: i32, end: i32, left_fade: i32, right_fade: i32) -> Self {
        Self { start, end, left_fade, right_fade }
    }
    pub fn start(&self) -> i32 { self.start }
    pub fn end(&self) -> i32 { self.end }
    pub fn left_fade(&self) -> i32 { self.left_fade }
    pub fn right_fade(&self) -> i32 { self.right_fade }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeField { Start, End, LeftFade, RightFade }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeBand {
    fields: RuntimeBandFields, range: cepstral::BandRange,
}
impl RuntimeBand {
    pub fn fields(&self) -> RuntimeBandFields { self.fields }
    pub fn range(&self) -> cepstral::BandRange { self.range }
    fn from_fields(fields: RuntimeBandFields) -> Result<Self, (RuntimeField, i32)> {
        for (field, value) in [(RuntimeField::Start, fields.start), (RuntimeField::End, fields.end),
            (RuntimeField::LeftFade, fields.left_fade), (RuntimeField::RightFade, fields.right_fade)]
        {
            if value < 0 { return Err((field, value)); }
        }
        Ok(Self { fields, range: cepstral::BandRange {
            start: fields.start as usize, end: fields.end as usize,
            left_fade: fields.left_fade as usize, right_fade: fields.right_fade as usize,
        } })
    }
}

pub struct BandInput<P> {
    pub fields: RuntimeBandFields,
    pub projection: P,
    pub pending: VecDeque<Vec<f32>>,
    pub split_allocation_words: [u32; 2],
}
pub struct Inputs<P> {

    pub layout: values::Layout,
    pub bands: Vec<BandInput<P>>,
    pub outputs: [cepstral::Output; 2],
    pub scratch_allocation_words: [u32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapacitySite { RuntimeBands, CanonicalBands }
pub enum PreparationCause {
    Domain(values::Domain),
    NegativeField { band: usize, field: RuntimeField, value: i32 },
    Capacity { site: CapacitySite, cause: TryReserveError },
}
enum FailureOwner<'n, 'data, P> {
    Preparation { parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P>,
        bands: Vec<RuntimeBand>, cause: PreparationCause },
    Initialization { bands: Vec<RuntimeBand>, failure: values::Failure<'n, 'data, P> },
}

pub struct Failure<'n, 'data, P> { owner: FailureOwner<'n, 'data, P> }
impl<'n, 'data, P> Failure<'n, 'data, P> {
    fn preparation(parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P>,
        bands: Vec<RuntimeBand>, cause: PreparationCause) -> Self
    { Self { owner: FailureOwner::Preparation { parameters, inputs, bands, cause } } }
    pub fn preparation_parameters(&self) -> Option<&parameters::Decoded<'n, 'data>> {
        match &self.owner { FailureOwner::Preparation { parameters, .. } => Some(parameters), _ => None }
    }
    pub fn preparation_inputs(&self) -> Option<&Inputs<P>> {
        match &self.owner { FailureOwner::Preparation { inputs, .. } => Some(inputs), _ => None }
    }
    pub fn preparation_cause(&self) -> Option<&PreparationCause> {
        match &self.owner { FailureOwner::Preparation { cause, .. } => Some(cause), _ => None }
    }
    pub fn runtime_bands(&self) -> &[RuntimeBand] {
        match &self.owner {
            FailureOwner::Preparation { bands, .. } | FailureOwner::Initialization { bands, .. } => bands,
        }
    }
    pub fn initialization_failure(&self) -> Option<&values::Failure<'n, 'data, P>> {
        match &self.owner { FailureOwner::Initialization { failure, .. } => Some(failure), _ => None }
    }
}
pub struct Bound<'n, 'data, P> {
    initialized: parameters::Bound<'n, 'data, P>, bands: Vec<RuntimeBand>,
}

enum FactoryFailureOwner<'n, 'data, P> {
    Preparation { failure: Failure<'n, 'data, P>, factories: [crate::streaming_rows::ValueRows; 2] },
    Initialization { bands: Vec<RuntimeBand>, failure: values::FactoryFailure<'n, 'data, P> },
}

pub struct FactoryFailure<'n, 'data, P> { owner: FactoryFailureOwner<'n, 'data, P> }
impl<'n, 'data, P> FactoryFailure<'n, 'data, P> {
    pub fn preparation_failure(&self) -> Option<&Failure<'n, 'data, P>> {
        match &self.owner { FactoryFailureOwner::Preparation { failure, .. } => Some(failure), _ => None }
    }
    pub fn untouched_factories(&self) -> Option<&[crate::streaming_rows::ValueRows; 2]> {
        match &self.owner { FactoryFailureOwner::Preparation { factories, .. } => Some(factories), _ => None }
    }
    pub fn runtime_bands(&self) -> &[RuntimeBand] {
        match &self.owner {
            FactoryFailureOwner::Preparation { failure, .. } => failure.runtime_bands(),
            FactoryFailureOwner::Initialization { bands, .. } => bands,
        }
    }
    pub fn initialization_failure(&self) -> Option<&values::FactoryFailure<'n, 'data, P>> {
        match &self.owner { FactoryFailureOwner::Initialization { failure, .. } => Some(failure), _ => None }
    }
}

struct PreparedRuntime<'n, 'data, P> {
    parameters: parameters::Decoded<'n, 'data>, inputs: values::Inputs<P>, bands: Vec<RuntimeBand>,
}
impl<'n, 'data, P> Bound<'n, 'data, P> {
    pub fn initialized(&self) -> &parameters::Bound<'n, 'data, P> { &self.initialized }
    pub fn runtime_bands(&self) -> &[RuntimeBand] { &self.bands }
    pub fn into_parts(self) -> (parameters::Bound<'n, 'data, P>, Vec<RuntimeBand>) {
        (self.initialized, self.bands)
    }
}

pub fn initialize<'n, 'data, P>(parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P>)
    -> Result<Bound<'n, 'data, P>, Failure<'n, 'data, P>>
{
    let PreparedRuntime { parameters, inputs, bands } = prepare_request(parameters, inputs)?;
    match values::initialize(parameters, inputs) {
        Ok(initialized) => Ok(Bound { initialized, bands }),
        Err(failure) => Err(Failure { owner: FailureOwner::Initialization { bands, failure } }),
    }
}

pub fn initialize_with_output_factories<'n, 'data, P>(parameters: parameters::Decoded<'n, 'data>,
    inputs: Inputs<P>, factories: [crate::streaming_rows::ValueRows; 2])
    -> Result<Bound<'n, 'data, P>, FactoryFailure<'n, 'data, P>>
{
    let prepared = match prepare_request(parameters, inputs) {
        Ok(prepared) => prepared,
        Err(failure) => return Err(FactoryFailure { owner: FactoryFailureOwner::Preparation { failure, factories } }),
    };
    let PreparedRuntime { parameters, inputs, bands } = prepared;
    match values::initialize_with_output_factories(parameters, inputs, factories) {
        Ok(initialized) => Ok(Bound { initialized, bands }),
        Err(failure) => Err(FactoryFailure { owner: FactoryFailureOwner::Initialization { bands, failure } }),
    }
}

fn prepare_request<'n, 'data, P>(parameters: parameters::Decoded<'n, 'data>, inputs: Inputs<P>)
    -> Result<PreparedRuntime<'n, 'data, P>, Failure<'n, 'data, P>>
{
    let count = parameters.bands().len();
    if let Some(cause) = values::admit_domain(count, inputs.bands.len(), inputs.layout,
        std::iter::empty::<cepstral::BandRange>())
    {
        return Err(Failure::preparation(parameters, inputs, Vec::new(), PreparationCause::Domain(cause)));
    }
    let mut mapped = Vec::new();
    if let Err(cause) = mapped.try_reserve_exact(count) {
        return Err(Failure::preparation(parameters, inputs, mapped,
            PreparationCause::Capacity { site: CapacitySite::RuntimeBands, cause }));
    }
    for band in 0..count {
        match RuntimeBand::from_fields(inputs.bands[band].fields) {
            Ok(value) => mapped.push(value),
            Err((field, value)) => return Err(Failure::preparation(parameters, inputs, mapped,
                PreparationCause::NegativeField { band, field, value })),
        }
    }
    if let Some(cause) = values::admit_domain(count, inputs.bands.len(), inputs.layout,
        mapped.iter().map(|band| band.range))
    {
        return Err(Failure::preparation(parameters, inputs, mapped, PreparationCause::Domain(cause)));
    }
    let mut canonical = Vec::new();
    if let Err(cause) = canonical.try_reserve_exact(count) {
        return Err(Failure::preparation(parameters, inputs, mapped,
            PreparationCause::Capacity { site: CapacitySite::CanonicalBands, cause }));
    }
    let Inputs { layout, bands, outputs, scratch_allocation_words } = inputs;
    for (input, band) in bands.into_iter().zip(&mapped) {
        canonical.push(values::BandInput { range: band.range, projection: input.projection,
            pending: input.pending, split_allocation_words: input.split_allocation_words });
    }
    Ok(PreparedRuntime { parameters,
        inputs: values::Inputs { layout, bands: canonical, outputs, scratch_allocation_words }, bands: mapped })
}
