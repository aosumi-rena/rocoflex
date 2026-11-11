use std::collections::{TryReserveError, VecDeque};
use crate::{cepstral, cepstral_parameters as parameters,
    cepstral_value_binding as values,
    conditional_gate::{Batch, BorrowedProjection, GateError, Projection},
    modm0_logical_sequence::RequiredReset,
    sequence_parameters as sequence, sequence_userdata_binding as sequences,
    sequence_value_binding::Linear, userdata};

type SequenceBound<'n, 'data, I, R> = sequence::Bound<'n, 'data, I,
    sequences::Activation, sequences::BankOwner<'n, 'data>, Linear, R>;

pub struct BandInput<I, R> {
    pub range: cepstral::BandRange,
    pub sequence: sequences::Parts<I, R>,
    pub pending: VecDeque<Vec<f32>>,
    pub split_allocation_words: [u32; 2],
}
pub struct Inputs<I, R> {
    pub layout: values::Layout,
    pub bands: VecDeque<BandInput<I, R>>,
    pub outputs: [cepstral::Output; 2],
    pub scratch_allocation_words: [u32; 2],
}

pub struct GenericFloatClear1BandInput<I, R> {
    pub range: cepstral::BandRange,
    pub sequence: sequences::Parts<I, R>,
    pub pending: VecDeque<Vec<f32>>,
}
pub struct GenericFloatClear1Inputs<I, R> {
    pub layout: values::Layout,
    pub bands: VecDeque<GenericFloatClear1BandInput<I, R>>,
    pub outputs: [cepstral::Output; 2],
    pub scratch_allocation_words: [u32; 2],
}

enum GenericFloatClear1FailureOwner<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    Packing { inputs: GenericFloatClear1Inputs<I, R>, cause: TryReserveError },
    Binding(Failure<'n, 'data, I, R>),
}

pub struct GenericFloatClear1Failure<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    userdata: &'n userdata::Decoded<'data>,
    owner: GenericFloatClear1FailureOwner<'n, 'data, I, R>,
}
impl<'n, 'data, I, R> GenericFloatClear1Failure<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn packing_inputs(&self) -> Option<&GenericFloatClear1Inputs<I, R>> {
        match &self.owner {
            GenericFloatClear1FailureOwner::Packing { inputs, .. } => Some(inputs),
            GenericFloatClear1FailureOwner::Binding(_) => None,
        }
    }
    pub fn packing_cause(&self) -> Option<&TryReserveError> {
        match &self.owner {
            GenericFloatClear1FailureOwner::Packing { cause, .. } => Some(cause),
            GenericFloatClear1FailureOwner::Binding(_) => None,
        }
    }
    pub fn binding_failure(&self) -> Option<&Failure<'n, 'data, I, R>> {
        match &self.owner {
            GenericFloatClear1FailureOwner::Packing { .. } => None,
            GenericFloatClear1FailureOwner::Binding(failure) => Some(failure),
        }
    }
}

pub struct BandProjection<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{ userdata: &'n userdata::Decoded<'data>, bound: SequenceBound<'n, 'data, I, R> }
impl<'n, 'data, I, R> BandProjection<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn bound(&self) -> &SequenceBound<'n, 'data, I, R> { &self.bound }
}
impl<'n, 'data, I: BorrowedProjection, R> Projection for BandProjection<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    fn reset(&mut self) -> Result<(), GateError> { self.bound.projection.reset() }
    fn forward(&mut self, input: &Batch) -> Result<Batch, GateError> { self.bound.projection.forward(input) }
}

pub struct Bound<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    userdata: &'n userdata::Decoded<'data>,
    bound: parameters::Bound<'n, 'data, BandProjection<'n, 'data, I, R>>,
}
impl<'n, 'data, I, R> Bound<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn bound(&self) -> &parameters::Bound<'n, 'data, BandProjection<'n, 'data, I, R>> { &self.bound }

    pub fn into_parts(self) -> (&'n userdata::Decoded<'data>,
        parameters::Bound<'n, 'data, BandProjection<'n, 'data, I, R>>)
    { (self.userdata, self.bound) }

    pub fn into_root_conditional<D, P, A>(self,
        upstream: crate::condition_ncwn_handoff::Completed<'n, 'data, D, P, A>)
        -> Result<crate::root_conditional_handoff::Run<'n, 'data, D, P, A, BandProjection<'n, 'data, I, R>>,
            crate::root_conditional_handoff::Failure<'n, 'data, D, P, A, BandProjection<'n, 'data, I, R>>>
    where I: BorrowedProjection
    {
        let Self { userdata: _, bound: mixer } = self;
        crate::root_conditional_handoff::begin(crate::root_conditional_handoff::Parts { upstream, mixer })
    }
}

pub enum PreparationCause { Domain(values::Domain), Capacity(TryReserveError) }

pub struct BandRuntime {
    range: cepstral::BandRange, pending: VecDeque<Vec<f32>>, split_allocation_words: [u32; 2],
}
impl BandRuntime {
    pub fn range(&self) -> cepstral::BandRange { self.range }
    pub fn pending(&self) -> &VecDeque<Vec<f32>> { &self.pending }
    pub fn split_allocation_words(&self) -> [u32; 2] { self.split_allocation_words }
}
pub struct Stopped<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    userdata: &'n userdata::Decoded<'data>, parameters: parameters::Decoded<'n, 'data>,
    index: usize, initialized: Vec<values::BandInput<BandProjection<'n, 'data, I, R>>>,
    current: sequences::Failure<'n, 'data, I, R>, runtime: BandRuntime, remaining: Inputs<I, R>,
}
impl<'n, 'data, I, R> Stopped<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    pub fn userdata(&self) -> &'n userdata::Decoded<'data> { self.userdata }
    pub fn parameters(&self) -> &parameters::Decoded<'n, 'data> { &self.parameters }
    pub fn index(&self) -> usize { self.index }
    pub fn initialized(&self) -> &[values::BandInput<BandProjection<'n, 'data, I, R>>] { &self.initialized }
    pub fn current(&self) -> &sequences::Failure<'n, 'data, I, R> { &self.current }
    pub fn current_runtime(&self) -> &BandRuntime { &self.runtime }
    pub fn remaining(&self) -> &Inputs<I, R> { &self.remaining }
}
pub enum Failure<'n, 'data, I, R>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    Decode { userdata: &'n userdata::Decoded<'data>, cause: userdata::Error, inputs: Inputs<I, R> },
    Preparation { userdata: &'n userdata::Decoded<'data>, parameters: parameters::Decoded<'n, 'data>,
        cause: PreparationCause, inputs: Inputs<I, R> },
    Band(Stopped<'n, 'data, I, R>),
    Initialization { userdata: &'n userdata::Decoded<'data>,
        failure: values::Failure<'n, 'data, BandProjection<'n, 'data, I, R>> },
}

pub fn bind_generic_float_clear1_userdata<'n, 'data, I: BorrowedProjection, R>(
    userdata: &'n userdata::Decoded<'data>, inputs: GenericFloatClear1Inputs<I, R>)
    -> Result<Bound<'n, 'data, I, R>, GenericFloatClear1Failure<'n, 'data, I, R>>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    let mut canonical = VecDeque::new();
    if let Err(cause) = canonical.try_reserve(inputs.bands.len()) {
        return Err(GenericFloatClear1Failure { userdata,
            owner: GenericFloatClear1FailureOwner::Packing { inputs, cause } });
    }
    let GenericFloatClear1Inputs { layout, bands: mut requests,
        outputs, scratch_allocation_words } = inputs;
    while let Some(GenericFloatClear1BandInput { range, sequence, pending }) = requests.pop_front() {
        canonical.push_back(BandInput { range, sequence, pending, split_allocation_words: [0; 2] });
    }
    bind_userdata(userdata, Inputs { layout, bands: canonical, outputs, scratch_allocation_words })
        .map_err(|failure| GenericFloatClear1Failure { userdata,
            owner: GenericFloatClear1FailureOwner::Binding(failure) })
}

pub fn bind_userdata<'n, 'data, I: BorrowedProjection, R>(
    userdata: &'n userdata::Decoded<'data>, mut inputs: Inputs<I, R>)
    -> Result<Bound<'n, 'data, I, R>, Failure<'n, 'data, I, R>>
where R: RequiredReset<sequences::Child<'n, 'data, I>, sequences::Child<'n, 'data, I>>
{
    let parameters = match userdata.decode_cepstral_parameters() {
        Ok(parameters) => parameters,
        Err(cause) => return Err(Failure::Decode { userdata, cause, inputs }),
    };
    let count = parameters.bands().len();
    if let Some(cause) = values::admit_domain(count, inputs.bands.len(), inputs.layout,
        inputs.bands.iter().map(|band| band.range))
    {
        return Err(Failure::Preparation { userdata, parameters, cause: PreparationCause::Domain(cause), inputs });
    }
    let mut initialized = Vec::new();
    if let Err(cause) = initialized.try_reserve_exact(count) {
        return Err(Failure::Preparation { userdata, parameters, cause: PreparationCause::Capacity(cause), inputs });
    }
    while let Some(input) = inputs.bands.pop_front() {
        let index = initialized.len();
        let BandInput { range, sequence, pending, split_allocation_words } = input;
        let runtime = BandRuntime { range, pending, split_allocation_words };
        match sequences::bind_node(parameters.bands()[index].projection(), sequence) {
            Ok(bound) => initialized.push(values::BandInput { range: runtime.range,
                projection: BandProjection { userdata, bound }, pending: runtime.pending,
                split_allocation_words: runtime.split_allocation_words }),
            Err(current) => return Err(Failure::Band(Stopped { userdata, parameters, index,
                initialized, current, runtime, remaining: inputs })),
        }
    }
    let Inputs { layout, bands: _, outputs, scratch_allocation_words } = inputs;
    match values::initialize(parameters, values::Inputs { layout, bands: initialized, outputs, scratch_allocation_words }) {
        Ok(bound) => Ok(Bound { userdata, bound }),
        Err(failure) => Err(Failure::Initialization { userdata, failure }),
    }
}
