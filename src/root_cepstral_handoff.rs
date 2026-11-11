use super::{Carried, Refused, call};
use crate::{cepstral::queue, cepstral_queue_model as queues,
    conditional_gate::Batch, ltv_coefficient_math::{self as coefficient, ScalarUpper},
    ltv_filter::{self, PairedKernel, SpanObserver}, ltv_filter_model as filter,
    ltv_output_values::ValueOutputs, ltv_render::{ConfiguredFft, SourceLtvFft},
    ltv_stream::Kernel, ltv_synthesis::SourceCoefficientMath,
    ltv_transform_initialization::{SourceConfiguredBound, SourceInitializationHistory},
    ncwn_value_binding::Network, paired_features, paired_input, paired_post,
    paired_post_join as joined, ltv_second_refill as second_refill,
    root_conditional_handoff::{self, Owners}};
use crate::paired_post::retirement::second_append as appended;

pub fn begin_queues<'n, 'data, D, P, A, C>(
    upstream: root_conditional_handoff::Completed<'n, 'data, D, P, A, C>, state: queue::State)
    -> Result<Carried<queues::Run<D, P, A, Network, Network, C>, Owners<'n, 'data>>,
        Refused<queues::StartFailure<D, P, A, Network, Network, C>, Owners<'n, 'data>>>
{
    let (held, completed) = upstream.into_parts();
    begin_output_queues(completed, state, held)
}

pub fn begin_queues_holding<'n, 'data, D, P, A, C, H>(
    upstream: root_conditional_handoff::Completed<'n, 'data, D, P, A, C>, state: queue::State, held: H)
    -> Result<Carried<queues::Run<D, P, A, Network, Network, C>, (Owners<'n, 'data>, H)>,
        Refused<queues::StartFailure<D, P, A, Network, Network, C>, (Owners<'n, 'data>, H)>>
{
    let (owners, completed) = upstream.into_parts();
    begin_output_queues(completed, state, (owners, held))
}
fn begin_output_queues<D, P, A, C, H>(
    completed: crate::cepstral_model::Completed<D, P, A, Network, Network, C>,
    state: queue::State, held: H)
    -> Result<Carried<queues::Run<D, P, A, Network, Network, C>, H>,
        Refused<queues::StartFailure<D, P, A, Network, Network, C>, H>>
{
    match completed.into_output_queues(state) {
        Ok(child) => Ok(Carried { child, held }),
        Err(start) => Err(Refused { start, held }),
    }
}
impl<D, P, A, C, H>
    Carried<queues::Run<D, P, A, Network, Network, C>, H>
{
    pub fn poll(&mut self, budget: usize) -> queue::Poll<'_> { self.child.poll(budget) }
    pub fn into_completed(self)
        -> Result<Carried<queues::Completed<D, P, A, Network, Network, C>, H>, Self>
    {
        self.finish(queues::Run::into_completed)
    }
}
impl<D, P, A, C, H>
    Carried<queues::Completed<D, P, A, Network, Network, C>, H>
{

    pub fn begin_refill(self, selection: queue::refill::Selection, output: Batch, raw_status: u8)
        -> Result<Carried<queues::Refilling<D, P, A, Network, Network, C>, H>,
            Refused<queues::RefillStartFailure<D, P, A, Network, Network, C>, H>>
    {
        match self.child.begin_refill(selection, output, raw_status) {
            Ok(child) => Ok(Carried { child, held: self.held }),
            Err(start) => Err(Refused { start, held: self.held }),
        }
    }
}
impl<D, P, A, C, H>
    Carried<queues::Refilling<D, P, A, Network, Network, C>, H>
{
    pub fn poll(&mut self, budget: usize) -> queue::refill::Poll<'_> { self.child.poll(budget) }
    pub fn into_completed(self)
        -> Result<Carried<queues::Refilled<D, P, A, Network, Network, C>, H>, Self>
    {
        self.finish(queues::Refilling::into_completed)
    }
}
impl<'n, 'data, D, P, A, C>
    Carried<queues::Refilled<D, P, A, Network, Network, C>, Owners<'n, 'data>>
{

    pub fn into_filter<M, F, K: PairedKernel, V>(self, state: ltv_filter::State<M, F, K>,
        signal: Batch, frame: i32, observer: Option<V>)
        -> Result<Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, Owners<'n, 'data>>,
            Refused<filter::StartFailure<D, P, A, Network, Network, C, M, F, K, V>, Owners<'n, 'data>>>
    {
        begin_filter(self.child, self.held, state, signal, frame, observer)
    }

    pub fn into_filter_holding<H, M, F, K: PairedKernel, V>(self,
        state: ltv_filter::State<M, F, K>, signal: Batch, frame: i32, observer: Option<V>, held: H)
        -> Result<Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, H)>,
            Refused<filter::StartFailure<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, H)>>
    {
        begin_filter(self.child, (self.held, held), state, signal, frame, observer)
    }
}
impl<'n, 'data, D, P, A, C, H>
    Carried<queues::Refilled<D, P, A, Network, Network, C>, (Owners<'n, 'data>, H)>
{
    pub fn into_filter<M, F, K: PairedKernel, V>(self, state: ltv_filter::State<M, F, K>,
        signal: Batch, frame: i32, observer: Option<V>)
        -> Result<Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, H)>,
            Refused<filter::StartFailure<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, H)>>
    { begin_filter(self.child, self.held, state, signal, frame, observer) }

    pub fn into_filter_holding<J, M, F, K: PairedKernel, V>(self,
        state: ltv_filter::State<M, F, K>, signal: Batch, frame: i32, observer: Option<V>, held: J)
        -> Result<Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, (H, J))>,
            Refused<filter::StartFailure<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, (H, J))>>
    {
        let (owners, metadata) = self.held;
        begin_filter(self.child, (owners, (metadata, held)), state, signal, frame, observer)
    }
}

fn begin_filter<D, P, A, C, M, F, K: PairedKernel, V, H>(
    refill: queues::Refilled<D, P, A, Network, Network, C>, held: H,
    state: ltv_filter::State<M, F, K>, signal: Batch, frame: i32, observer: Option<V>)
    -> Result<Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, H>,
        Refused<filter::StartFailure<D, P, A, Network, Network, C, M, F, K, V>, H>>
{
    match refill.into_filter(state, signal, frame, observer) {
        Ok(child) => Ok(Carried { child, held }),
        Err(start) => Err(Refused { start, held }),
    }
}
impl<'n, 'data, D, P, A, C, M: SourceCoefficientMath, F: SourceLtvFft, K: PairedKernel, V: SpanObserver>
    Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, Owners<'n, 'data>>
{
    pub fn run(self)
        -> Result<Carried<filter::Completed<D, P, A, Network, Network, C, M, F, K, V>, Owners<'n, 'data>>,
            Carried<filter::Failed<D, P, A, Network, Network, C, M, F, K, V>, Owners<'n, 'data>>>
    {
        run_filter(self)
    }
}
impl<'n, 'data, D, P, A, C, M: SourceCoefficientMath, F: SourceLtvFft,
    K: PairedKernel, V: SpanObserver, H>
    Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, H)>
{
    pub fn run(self)
        -> Result<Carried<filter::Completed<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, H)>,
            Carried<filter::Failed<D, P, A, Network, Network, C, M, F, K, V>, (Owners<'n, 'data>, H)>>
    { run_filter(self) }
}

fn run_filter<D, P, A, C, M: SourceCoefficientMath, F: SourceLtvFft,
    K: PairedKernel, V: SpanObserver, H>(
    carried: Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, H>)
    -> Result<Carried<filter::Completed<D, P, A, Network, Network, C, M, F, K, V>, H>,
        Carried<filter::Failed<D, P, A, Network, Network, C, M, F, K, V>, H>>
{
    match carried.child.run() {
        Ok(child) => Ok(Carried { child, held: carried.held }),
        Err(child) => Err(Carried { child, held: carried.held }),
    }
}

pub struct FilterContext<'n, 'data, D, P, A, C, M, F, V> {
    root: Owners<'n, 'data>,
    filter: filter::Completed<D, P, A, Network, Network, C, M, F, Kernel<ValueOutputs>, V>,
}
impl<'n, 'data, D, P, A, C, M, F, V> FilterContext<'n, 'data, D, P, A, C, M, F, V> {
    pub fn root(&self) -> &Owners<'n, 'data> { &self.root }
    pub fn filter(&self) -> &filter::Completed<D, P, A, Network, Network, C, M, F, Kernel<ValueOutputs>, V> {
        &self.filter
    }

    pub fn into_parts(self) -> (Owners<'n, 'data>,
        filter::Completed<D, P, A, Network, Network, C, M, F, Kernel<ValueOutputs>, V>)
    {
        (self.root, self.filter)
    }
    pub fn into_remaining_coefficients(self) -> (queue::Completed<C>,
        ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V>)
    {
        let (coefficients, first) = self.filter.into_remaining_coefficients();
        (coefficients, ReturnedFirstFilter { root: self.root, first })
    }
}
pub struct ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V> {
    root: Owners<'n, 'data>,
    first: filter::ReturnedFirstFilter<D, P, A, Network, Network, M, F, Kernel<ValueOutputs>, V>,
}
impl<'n, 'data, D, P, A, M, F, V> ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V> {
    pub fn root(&self) -> &Owners<'n, 'data> { &self.root }
    pub fn first(&self) -> &filter::ReturnedFirstFilter<D, P, A, Network, Network, M, F, Kernel<ValueOutputs>, V> { &self.first }
    pub fn into_parts(self) -> (Owners<'n, 'data>,
        filter::ReturnedFirstFilter<D, P, A, Network, Network, M, F, Kernel<ValueOutputs>, V>)
    { (self.root, self.first) }
    fn split_upstream(self) -> (FirstFilterState<'n, 'data, M, F, V>,
        crate::cepstral_model::Upstream<D, P, A, Network, Network>)
    {
        let Self { root, first: filter::ReturnedFirstFilter {
            upstream, filter, first_refill_trace, second_signal } } = self;
        (FirstFilterState { root, filter, first_refill_trace, second_signal }, upstream)
    }
}

pub struct FirstFilterState<'n, 'data, M, F, V> {
    root: Owners<'n, 'data>,
    filter: ltv_filter::Completed<M, F, Kernel<ValueOutputs>, V>,
    first_refill_trace: queue::refill::Trace,
    second_signal: Option<second_refill::SecondHistory>,
}
impl<'n, 'data, M, F, V> FirstFilterState<'n, 'data, M, F, V> {
    pub fn root(&self) -> &Owners<'n, 'data> { &self.root }
    pub fn filter(&self) -> &ltv_filter::Completed<M, F, Kernel<ValueOutputs>, V> { &self.filter }
    pub fn first_refill_trace(&self) -> &queue::refill::Trace { &self.first_refill_trace }
    pub fn second_signal(&self) -> Option<&second_refill::SecondHistory> { self.second_signal.as_ref() }
    pub fn into_parts(self) -> (Owners<'n, 'data>, ltv_filter::Completed<M, F, Kernel<ValueOutputs>, V>,
        queue::refill::Trace, Option<second_refill::SecondHistory>)
    { (self.root, self.filter, self.first_refill_trace, self.second_signal) }
}

pub struct HeldSecondHistory<H> { producer: H, complete: call::CompletedCall }
impl<H> HeldSecondHistory<H> {
    pub fn producer(&self) -> &H { &self.producer }
    pub fn completion(&self) -> &call::CompletedCall { &self.complete }
    pub fn into_parts(self) -> (H, call::CompletedCall) { (self.producer, self.complete) }
}
pub enum SignalCause { Output(ltv_filter::output::Error) }
pub struct SignalFailure<'n, 'data, D, P, A, C, M, F, V> {
    context: FilterContext<'n, 'data, D, P, A, C, M, F, V>, cause: SignalCause,
}
impl<'n, 'data, D, P, A, C, M, F, V> SignalFailure<'n, 'data, D, P, A, C, M, F, V> {
    pub fn context(&self) -> &FilterContext<'n, 'data, D, P, A, C, M, F, V> { &self.context }
    pub fn cause(&self) -> &SignalCause { &self.cause }
}
impl<'n, 'data, D, P, A, C, M, F, V>
    Carried<filter::Completed<D, P, A, Network, Network, C, M, F, Kernel<ValueOutputs>, V>, Owners<'n, 'data>>
{

    pub fn into_signal(self)
        -> Result<Carried<Batch, FilterContext<'n, 'data, D, P, A, C, M, F, V>>,
            SignalFailure<'n, 'data, D, P, A, C, M, F, V>>
    {
        take_filter_signal(self.child, self.held)
    }
}

fn take_filter_signal<'n, 'data, D, P, A, C, M, F, V>(
    filter: filter::Completed<D, P, A, Network, Network, C, M, F, Kernel<ValueOutputs>, V>,
    root: Owners<'n, 'data>)
    -> Result<Carried<Batch, FilterContext<'n, 'data, D, P, A, C, M, F, V>>,
        SignalFailure<'n, 'data, D, P, A, C, M, F, V>>
{
    let mut context = FilterContext { root, filter };
    let signal = match context.filter.take_output_values().map_err(SignalCause::Output) {
        Ok(signal) => signal,
        Err(cause) => return Err(SignalFailure { context, cause }),
    };
    Ok(Carried { child: signal, held: context })
}
impl<'n, 'data, D, P, A, C, M, F, V, H>
    Carried<filter::Completed<D, P, A, Network, Network, C, M, F, Kernel<ValueOutputs>, V>, (Owners<'n, 'data>, H)>
{
    pub fn into_signal(self)
        -> Result<Carried<Batch, (FilterContext<'n, 'data, D, P, A, C, M, F, V>, H)>,
            Refused<SignalFailure<'n, 'data, D, P, A, C, M, F, V>, H>>
    {
        let (root, held) = self.held;
        match take_filter_signal(self.child, root) {
            Ok(Carried { child, held: context }) => Ok(Carried { child, held: (context, held) }),
            Err(start) => Err(Refused { start, held }),
        }
    }
}
impl<'n, 'data, D, P, A, C, M, F, V>
    Carried<Batch, FilterContext<'n, 'data, D, P, A, C, M, F, V>>
{

    pub fn into_paired_call<SM: paired_post::Module, SR: paired_input::Rows,
        W, X, Y, O, S, U: ScalarUpper, PF, G>(self,
        bound: call::Bound<SM, SR, W, X, Y, O, S, U, PF, G>, paired_features: Batch)
        -> Result<Carried<paired_input::Prepared<SM, SR,
                call::History<FilterContext<'n, 'data, D, P, A, C, M, F, V>>>,
                call::ScalarOwners<W, X, Y, O, S, U, PF, G>>,
            call::ScalarStartFailure<SM, SR, FilterContext<'n, 'data, D, P, A, C, M, F, V>,
                W, X, Y, O, S, U, PF, G>>
    {
        begin_paired_call(self.child, self.held, bound, paired_features)
    }

    pub fn into_paired_call_holding<H, SM: paired_post::Module, SR: paired_input::Rows,
        W, X, Y, O, S, U: ScalarUpper, PF, G>(self,
        bound: call::Bound<SM, SR, W, X, Y, O, S, U, PF, G>, paired_features: Batch, held: H)
        -> Result<Carried<paired_input::Prepared<SM, SR,
                call::History<(FilterContext<'n, 'data, D, P, A, C, M, F, V>, H)>>,
                call::ScalarOwners<W, X, Y, O, S, U, PF, G>>,
            call::ScalarStartFailure<SM, SR, (FilterContext<'n, 'data, D, P, A, C, M, F, V>, H),
                W, X, Y, O, S, U, PF, G>>
    {
        begin_paired_call(self.child, (self.held, held), bound, paired_features)
    }
}

impl<'n, 'data, D, P, A, C, M, F, V, H>
    Carried<Batch, (FilterContext<'n, 'data, D, P, A, C, M, F, V>, H)>
{
    pub fn into_paired_call<SM: paired_post::Module, SR: paired_input::Rows,
        W, X, Y, O, S, U: ScalarUpper, PF, G>(self,
        bound: call::Bound<SM, SR, W, X, Y, O, S, U, PF, G>, paired_features: Batch)
        -> Result<Carried<paired_input::Prepared<SM, SR,
                call::History<(FilterContext<'n, 'data, D, P, A, C, M, F, V>, H)>>,
                call::ScalarOwners<W, X, Y, O, S, U, PF, G>>,
            call::ScalarStartFailure<SM, SR, (FilterContext<'n, 'data, D, P, A, C, M, F, V>, H),
                W, X, Y, O, S, U, PF, G>>
    {
        begin_paired_call(self.child, self.held, bound, paired_features)
    }

    pub fn into_paired_call_holding<J, SM: paired_post::Module, SR: paired_input::Rows,
        W, X, Y, O, S, U: ScalarUpper, PF, G>(self,
        bound: call::Bound<SM, SR, W, X, Y, O, S, U, PF, G>, paired_features: Batch, held: J)
        -> Result<Carried<paired_input::Prepared<SM, SR,
                call::History<(FilterContext<'n, 'data, D, P, A, C, M, F, V>, (H, J))>>,
                call::ScalarOwners<W, X, Y, O, S, U, PF, G>>,
            call::ScalarStartFailure<SM, SR, (FilterContext<'n, 'data, D, P, A, C, M, F, V>, (H, J)),
                W, X, Y, O, S, U, PF, G>>
    {
        let (context, metadata) = self.held;
        begin_paired_call(self.child, (context, (metadata, held)), bound, paired_features)
    }
}

fn begin_paired_call<H, SM: paired_post::Module, SR: paired_input::Rows,
    W, X, Y, O, S, U: ScalarUpper, PF, G>(signal: Batch, held: H,
    bound: call::Bound<SM, SR, W, X, Y, O, S, U, PF, G>, paired_features: Batch)
    -> Result<Carried<paired_input::Prepared<SM, SR, call::History<H>>,
            call::ScalarOwners<W, X, Y, O, S, U, PF, G>>,
        call::ScalarStartFailure<SM, SR, H, W, X, Y, O, S, U, PF, G>>
{
    bound.begin(signal, paired_features, held)
}

impl<'n, 'data, D, P, A, C, M, F, V, X, Y, U: ScalarUpper, W, S,
    PF: paired_features::Frames, G: paired_features::Gaussian,
    SM: paired_post::Module, SR: paired_input::Rows, O, E>
    Carried<appended::Completed<X, Y, U,
        joined::History<W, S, PF, G, SM, SR, FilterContext<'n, 'data, D, P, A, C, M, F, V>>, O, E>,
        call::CompletedCall>
{

    pub fn into_remaining_refill(self, request: second_refill::Request)
        -> Result<Carried<second_refill::Prepared<C, appended::History<X, Y, U,
                joined::History<W, S, PF, G, SM, SR, ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V>>, O, E>>,
                call::CompletedCall>,
            Refused<second_refill::StartFailure<C, appended::History<X, Y, U,
                joined::History<W, S, PF, G, SM, SR, ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V>>, O, E>>,
                call::CompletedCall>>
    {
        begin_remaining_refill(self.child, self.held, request, |context| {
            let (coefficients, first) = context.into_remaining_coefficients();
            (first, coefficients)
        })
    }
}
impl<'n, 'data, D, P, A, C, M, F, V, H, X, Y, U: ScalarUpper, W, S,
    PF: paired_features::Frames, G: paired_features::Gaussian,
    SM: paired_post::Module, SR: paired_input::Rows, O, E>
    Carried<appended::Completed<X, Y, U,
        joined::History<W, S, PF, G, SM, SR, (FilterContext<'n, 'data, D, P, A, C, M, F, V>, H)>, O, E>,
        call::CompletedCall>
{
    pub fn into_remaining_refill(self, request: second_refill::Request)
        -> Result<Carried<second_refill::Prepared<C, appended::History<X, Y, U,
                joined::History<W, S, PF, G, SM, SR, (ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V>, H)>, O, E>>,
                call::CompletedCall>,
            Refused<second_refill::StartFailure<C, appended::History<X, Y, U,
                joined::History<W, S, PF, G, SM, SR, (ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V>, H)>, O, E>>,
                call::CompletedCall>>
    {
        begin_remaining_refill(self.child, self.held, request, |(context, metadata)| {
            let (coefficients, first) = context.into_remaining_coefficients();
            ((first, metadata), coefficients)
        })
    }
}

fn begin_remaining_refill<X, Y, U: ScalarUpper, W, S,
    PF: paired_features::Frames, G: paired_features::Gaussian,
    SM: paired_post::Module, SR: paired_input::Rows, H, T, C, O, E>(
    completed: appended::Completed<X, Y, U, joined::History<W, S, PF, G, SM, SR, H>, O, E>,
    marker: call::CompletedCall, request: second_refill::Request,
    split: impl FnOnce(H) -> (T, queue::Completed<C>))
    -> Result<Carried<second_refill::Prepared<C,
            appended::History<X, Y, U, joined::History<W, S, PF, G, SM, SR, T>, O, E>>, call::CompletedCall>,
        Refused<second_refill::StartFailure<C,
            appended::History<X, Y, U, joined::History<W, S, PF, G, SM, SR, T>, O, E>>, call::CompletedCall>>
{
    let appended::CompletedParts { signal, history: appended::History { retired, append_trace } } = completed.into_parts();
    let (history, coefficients) = split_second_history(appended::History { retired, append_trace }, split);
    match second_refill::Prepared::from_completed_queues(signal, coefficients, history, request) {
        Ok(child) => Ok(Carried { child, held: marker }),
        Err(start) => Err(Refused { start, held: marker }),
    }
}

fn split_second_history<X, Y, U: ScalarUpper, W, S,
    PF: paired_features::Frames, G: paired_features::Gaussian,
    SM: paired_post::Module, SR: paired_input::Rows, H, T, Extra, O, E>(
    history: appended::History<X, Y, U, joined::History<W, S, PF, G, SM, SR, H>, O, E>,
    split: impl FnOnce(H) -> (T, Extra))
    -> (appended::History<X, Y, U, joined::History<W, S, PF, G, SM, SR, T>, O, E>, Extra)
{
    let appended::History { retired, append_trace } = history;
    let paired_post::retirement::CompletedParts { continuation, erasure, retirement } = retired;
    let paired_post::packing::Continuation { post, packing, input240, held,
        post_trace, packing_trace, post_completion, packing_completion } = continuation;
    let (held, extra) = held.split_input_held(split);
    let retired = paired_post::retirement::CompletedParts {
        continuation: paired_post::packing::Continuation { post, packing, input240, held,
            post_trace, packing_trace, post_completion, packing_completion }, erasure, retirement,
    };
    (appended::History { retired, append_trace }, extra)
}

impl<'n, 'data, D, P, A, C, M, F, V, X, Y, U: ScalarUpper, W, S,
    PF: paired_features::Frames, G: paired_features::Gaussian,
    SM: paired_post::Module, SR: paired_input::Rows, O, E>
    Carried<second_refill::Completed<C, appended::History<X, Y, U,
        joined::History<W, S, PF, G, SM, SR, ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V>>, O, E>>,
        call::CompletedCall>
{

    pub fn into_remaining_filter<M2, F2, K2: PairedKernel, V2>(self,
        state: ltv_filter::State<M2, F2, K2>, frame: i32, observer: Option<V2>)
        -> Result<Carried<filter::Run<D, P, A, Network, Network, C, M2, F2, K2, V2>,
                HeldSecondHistory<appended::History<X, Y, U,
                    joined::History<W, S, PF, G, SM, SR, FirstFilterState<'n, 'data, M, F, V>>, O, E>>>,
            Refused<filter::StartFailure<D, P, A, Network, Network, C, M2, F2, K2, V2>,
                HeldSecondHistory<appended::History<X, Y, U,
                    joined::History<W, S, PF, G, SM, SR, FirstFilterState<'n, 'data, M, F, V>>, O, E>>>>
    {
        begin_remaining_filter(self.child, self.held, state, frame, observer,
            ReturnedFirstFilter::split_upstream)
    }

    pub fn into_remaining_source_filter<T2, U2: ScalarUpper, V2>(self,
        bound: SourceConfiguredBound<T2, U2>, frame: i32, observer: Option<V2>)
        -> Result<Carried<filter::Run<D, P, A, Network, Network, C,
                coefficient::Provider<U2>, ConfiguredFft, Kernel<ValueOutputs>, V2>,
                HeldSecondHistory<(appended::History<X, Y, U,
                    joined::History<W, S, PF, G, SM, SR, FirstFilterState<'n, 'data, M, F, V>>, O, E>,
                    SourceInitializationHistory<T2>)>>,
            Refused<filter::StartFailure<D, P, A, Network, Network, C,
                coefficient::Provider<U2>, ConfiguredFft, Kernel<ValueOutputs>, V2>,
                HeldSecondHistory<(appended::History<X, Y, U,
                    joined::History<W, S, PF, G, SM, SR, FirstFilterState<'n, 'data, M, F, V>>, O, E>,
                    SourceInitializationHistory<T2>)>>>
    {
        let (state, history) = bound.into_state_and_history();
        attach_remaining_source_history(self.into_remaining_filter(state, frame, observer), history)
    }
}
impl<'n, 'data, D, P, A, C, M, F, V, H, X, Y, U: ScalarUpper, W, S,
    PF: paired_features::Frames, G: paired_features::Gaussian,
    SM: paired_post::Module, SR: paired_input::Rows, O, E>
    Carried<second_refill::Completed<C, appended::History<X, Y, U,
        joined::History<W, S, PF, G, SM, SR, (ReturnedFirstFilter<'n, 'data, D, P, A, M, F, V>, H)>, O, E>>,
        call::CompletedCall>
{

    pub fn into_remaining_filter<M2, F2, K2: PairedKernel, V2>(self,
        state: ltv_filter::State<M2, F2, K2>, frame: i32, observer: Option<V2>)
        -> Result<Carried<filter::Run<D, P, A, Network, Network, C, M2, F2, K2, V2>,
                HeldSecondHistory<appended::History<X, Y, U,
                    joined::History<W, S, PF, G, SM, SR, (FirstFilterState<'n, 'data, M, F, V>, H)>, O, E>>>,
            Refused<filter::StartFailure<D, P, A, Network, Network, C, M2, F2, K2, V2>,
                HeldSecondHistory<appended::History<X, Y, U,
                    joined::History<W, S, PF, G, SM, SR, (FirstFilterState<'n, 'data, M, F, V>, H)>, O, E>>>>
    {
        begin_remaining_filter(self.child, self.held, state, frame, observer, |(first, metadata)| {
            let (first, upstream) = first.split_upstream();
            ((first, metadata), upstream)
        })
    }

    pub fn into_remaining_source_filter<T2, U2: ScalarUpper, V2>(self,
        bound: SourceConfiguredBound<T2, U2>, frame: i32, observer: Option<V2>)
        -> Result<Carried<filter::Run<D, P, A, Network, Network, C,
                coefficient::Provider<U2>, ConfiguredFft, Kernel<ValueOutputs>, V2>,
                HeldSecondHistory<(appended::History<X, Y, U,
                    joined::History<W, S, PF, G, SM, SR, (FirstFilterState<'n, 'data, M, F, V>, H)>, O, E>,
                    SourceInitializationHistory<T2>)>>,
            Refused<filter::StartFailure<D, P, A, Network, Network, C,
                coefficient::Provider<U2>, ConfiguredFft, Kernel<ValueOutputs>, V2>,
                HeldSecondHistory<(appended::History<X, Y, U,
                    joined::History<W, S, PF, G, SM, SR, (FirstFilterState<'n, 'data, M, F, V>, H)>, O, E>,
                    SourceInitializationHistory<T2>)>>>
    {
        let (state, history) = bound.into_state_and_history();
        attach_remaining_source_history(self.into_remaining_filter(state, frame, observer), history)
    }
}

fn attach_remaining_source_history<R, E, H, T>(
    result: Result<Carried<R, HeldSecondHistory<H>>, Refused<E, HeldSecondHistory<H>>>,
    history: SourceInitializationHistory<T>)
    -> Result<Carried<R, HeldSecondHistory<(H, SourceInitializationHistory<T>)>>,
        Refused<E, HeldSecondHistory<(H, SourceInitializationHistory<T>)>>>
{
    match result {
        Ok(Carried { child, held: HeldSecondHistory { producer, complete } }) =>
            Ok(Carried { child, held: HeldSecondHistory { producer: (producer, history), complete } }),
        Err(Refused { start, held: HeldSecondHistory { producer, complete } }) =>
            Err(Refused { start, held: HeldSecondHistory { producer: (producer, history), complete } }),
    }
}

fn begin_remaining_filter<X, Y, U: ScalarUpper, W, S,
    PF: paired_features::Frames, G: paired_features::Gaussian,
    SM: paired_post::Module, SR: paired_input::Rows, H, T, D, P, A, C, O, E,
    M, F, K: PairedKernel, V>(
    completed: second_refill::Completed<C,
        appended::History<X, Y, U, joined::History<W, S, PF, G, SM, SR, H>, O, E>>,
    marker: call::CompletedCall, state: ltv_filter::State<M, F, K>, frame: i32, observer: Option<V>,
    split: impl FnOnce(H) -> (T, crate::cepstral_model::Upstream<D, P, A, Network, Network>))
    -> Result<Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>,
            HeldSecondHistory<appended::History<X, Y, U, joined::History<W, S, PF, G, SM, SR, T>, O, E>>>,
        Refused<filter::StartFailure<D, P, A, Network, Network, C, M, F, K, V>,
            HeldSecondHistory<appended::History<X, Y, U, joined::History<W, S, PF, G, SM, SR, T>, O, E>>>>
{
    let second_refill::CompletedParts { held, signal, coefficients, input, trace, readiness } = completed.into_parts();
    let (producer, upstream) = split_second_history(held, split);
    let refill = queues::Refilled::from_completed_parts(upstream, coefficients);
    let second = second_refill::SecondHistory { signal, trace, readiness };
    let held = HeldSecondHistory { producer, complete: marker };
    match refill.into_second_filter(state, input, frame, observer, second) {
        Ok(child) => Ok(Carried { child, held }),
        Err(start) => Err(Refused { start, held }),
    }
}

impl<D, P, A, C, M: SourceCoefficientMath, F: SourceLtvFft,
    K: PairedKernel, V: SpanObserver, H>
    Carried<filter::Run<D, P, A, Network, Network, C, M, F, K, V>, HeldSecondHistory<H>>
{
    pub fn run(self)
        -> Result<Carried<filter::Completed<D, P, A, Network, Network, C, M, F, K, V>, HeldSecondHistory<H>>,
            Carried<filter::Failed<D, P, A, Network, Network, C, M, F, K, V>, HeldSecondHistory<H>>>
    { run_filter(self) }
}
impl<D, P, A, C, M, F, K, V, H>
    Carried<filter::Completed<D, P, A, Network, Network, C, M, F, K, V>, HeldSecondHistory<H>>
{

    pub fn into_completed_parts(self)
        -> (filter::Completed<D, P, A, Network, Network, C, M, F, K, V>, HeldSecondHistory<H>)
    { (self.child, self.held) }
}
impl<D, P, A, C, M, F, V, H>
    Carried<filter::Completed<D, P, A, Network, Network, C, M, F, Kernel<ValueOutputs>, V>, HeldSecondHistory<H>>
{

    pub fn write_output_wav<W: std::io::Write + ?Sized>(&self, writer: &mut W, sample_rate: u32)
        -> Result<crate::wave_output::Receipt, ltv_filter::output::Error>
    { self.child.filter().write_output_wav(writer, sample_rate) }

    #[cfg(windows)]
    pub fn play_output_wasapi(&self, parts: &crate::wasapi_render::Parts,
        wait: crate::wasapi_playback::WaitPolicy, cancel: &std::sync::atomic::AtomicBool)
        -> Result<crate::wasapi_playback::Outcome, ltv_filter::output::Error>
    { self.child.filter().play_output_wasapi(parts, wait, cancel) }
}
