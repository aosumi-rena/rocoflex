use crate::{cepstral, cepstral_model, cepstral_queue_model, conditional_gate::Batch,
    ltv_filter::{self, PairedKernel, SpanObserver},
    ltv_render::SourceLtvFft, ltv_synthesis::SourceCoefficientMath};
use cepstral::queue;

pub struct ContextParts<D, P, A, X, Y, C> {
    pub upstream: cepstral_model::Upstream<D, P, A, X, Y>,
    pub mixer: cepstral::Completed<C>, pub queues: queue::State,
    pub append_trace: queue::Trace, pub refill_trace: queue::refill::Trace,
    pub second_signal: Option<crate::ltv_second_refill::SecondHistory>,
}
pub struct Context<D, P, A, X, Y, C> { parts: ContextParts<D, P, A, X, Y, C> }
impl<D, P, A, X, Y, C> Context<D, P, A, X, Y, C> {
    pub fn upstream(&self) -> &cepstral_model::Upstream<D, P, A, X, Y> { &self.parts.upstream }
    pub fn mixer(&self) -> &cepstral::Completed<C> { &self.parts.mixer }
    pub fn queues(&self) -> &queue::State { &self.parts.queues }
    pub fn append_trace(&self) -> &queue::Trace { &self.parts.append_trace }
    pub fn refill_trace(&self) -> &queue::refill::Trace { &self.parts.refill_trace }
    pub fn second_signal(&self) -> Option<&crate::ltv_second_refill::SecondHistory> { self.parts.second_signal.as_ref() }

    pub fn into_parts(self) -> ContextParts<D, P, A, X, Y, C> { self.parts }
}
pub struct StartFailure<D, P, A, X, Y, C, M, F, K, O> {
    pub context: Context<D, P, A, X, Y, C>,
    pub filter: ltv_filter::BeginFailure<M, F, K, O>,
}
pub struct Run<D, P, A, X, Y, C, M, F, K, O> {
    context: Context<D, P, A, X, Y, C>, filter: ltv_filter::Run<M, F, K, O>,
}
pub struct Failed<D, P, A, X, Y, C, M, F, K: PairedKernel, O: SpanObserver> {
    context: Context<D, P, A, X, Y, C>, filter: ltv_filter::Failed<M, F, K, O>,
}
pub struct Completed<D, P, A, X, Y, C, M, F, K, O> {
    context: Context<D, P, A, X, Y, C>, filter: ltv_filter::Completed<M, F, K, O>,
}

pub struct ReturnedFirstFilter<D, P, A, X, Y, M, F, K, O> {
    pub upstream: cepstral_model::Upstream<D, P, A, X, Y>,
    pub filter: ltv_filter::Completed<M, F, K, O>,
    pub first_refill_trace: queue::refill::Trace,
    pub second_signal: Option<crate::ltv_second_refill::SecondHistory>,
}

pub struct Continuation<D, P, A, X, Y, C, M, F, K, O> {
    pub condition: crate::condition::Condition<D, P>,
    pub recurrent: crate::quantized_gru::QuantizedGru<A>,
    pub stages: crate::conditioned_stages::State<X, Y>,
    pub mixer: cepstral::State<C>, pub queues: queue::State,
    pub filter: ltv_filter::State<M, F, K>, pub observer: Option<O>, pub history: History,
    pub second_signal: Option<crate::ltv_second_refill::State>,
}
pub struct History {
    pub feature_input: Vec<Vec<f32>>, pub phase_words: Vec<u32>,
    pub condition_work: crate::condition::Work, pub recurrent_input: Vec<Vec<f32>>, pub recurrent_status: u8,
    pub stages: crate::conditioned_stages::Trace,
    pub mixer: cepstral::Trace, pub moved_mixer_packets: [Batch; 2],
    pub append: queue::Trace, pub refill: queue::refill::Trace,
    pub filter_input_after_call: ltv_filter::Input, pub filter: ltv_filter::Trace,
    pub second_refill: Option<crate::ltv_second_refill::SecondTrace>,
}
impl<D, P, A, X, Y, C> cepstral_queue_model::Refilled<D, P, A, X, Y, C> {

    pub fn into_filter<M, F, K: PairedKernel, O>(self, state: ltv_filter::State<M, F, K>,
        signal: Batch, frame: i32, observer: Option<O>)
        -> Result<Run<D, P, A, X, Y, C, M, F, K, O>, StartFailure<D, P, A, X, Y, C, M, F, K, O>>
    {
        self.into_filter_context(state, signal, frame, observer, None)
    }
    pub(crate) fn into_second_filter<M, F, K: PairedKernel, O>(self, state: ltv_filter::State<M, F, K>,
        signal: Batch, frame: i32, observer: Option<O>, second: crate::ltv_second_refill::SecondHistory)
        -> Result<Run<D, P, A, X, Y, C, M, F, K, O>, StartFailure<D, P, A, X, Y, C, M, F, K, O>>
    {
        self.into_filter_context(state, signal, frame, observer, Some(second))
    }
    fn into_filter_context<M, F, K: PairedKernel, O>(self, state: ltv_filter::State<M, F, K>,
        signal: Batch, frame: i32, observer: Option<O>, second_signal: Option<crate::ltv_second_refill::SecondHistory>)
        -> Result<Run<D, P, A, X, Y, C, M, F, K, O>, StartFailure<D, P, A, X, Y, C, M, F, K, O>>
    {
        let (upstream, refill) = self.into_parts();
        let (mixer, append_trace, refill) = refill.into_parts();
        let (queues, coefficients, refill_trace) = refill.into_parts();
        let context = Context { parts: ContextParts { upstream, mixer, queues, append_trace, refill_trace, second_signal } };
        match state.begin(ltv_filter::Input { coefficients, signal }, frame, observer) {
            Ok(filter) => Ok(Run { context, filter }),
            Err(filter) => Err(StartFailure { context, filter }),
        }
    }

    pub fn into_filter_without_observer<M, F, K: PairedKernel>(self,
        state: ltv_filter::State<M, F, K>, signal: Batch, frame: i32)
        -> Result<Run<D, P, A, X, Y, C, M, F, K, std::convert::Infallible>,
            StartFailure<D, P, A, X, Y, C, M, F, K, std::convert::Infallible>>
    {
        self.into_filter(state, signal, frame, None)
    }
}
impl<D, P, A, X, Y, C, M, F, K, O> Run<D, P, A, X, Y, C, M, F, K, O> {
    pub fn context(&self) -> &Context<D, P, A, X, Y, C> { &self.context }
    pub fn filter(&self) -> &ltv_filter::Run<M, F, K, O> { &self.filter }
}
impl<D, P, A, X, Y, C, M: SourceCoefficientMath, F: SourceLtvFft, K: PairedKernel, O: SpanObserver>
    Run<D, P, A, X, Y, C, M, F, K, O>
{

    pub fn run(self) -> Result<Completed<D, P, A, X, Y, C, M, F, K, O>,
        Failed<D, P, A, X, Y, C, M, F, K, O>>
    {
        match self.filter.run() {
            Ok(filter) => Ok(Completed { context: self.context, filter }),
            Err(filter) => Err(Failed { context: self.context, filter }),
        }
    }
}
impl<D, P, A, X, Y, C, M, F, K: PairedKernel, O: SpanObserver> Failed<D, P, A, X, Y, C, M, F, K, O> {
    pub fn context(&self) -> &Context<D, P, A, X, Y, C> { &self.context }
    pub fn filter(&self) -> &ltv_filter::Failed<M, F, K, O> { &self.filter }
}
impl<D, P, A, X, Y, C, M, F, K, O> Completed<D, P, A, X, Y, C, M, F, K, O> {
    pub fn context(&self) -> &Context<D, P, A, X, Y, C> { &self.context }
    pub fn filter(&self) -> &ltv_filter::Completed<M, F, K, O> { &self.filter }
    pub fn into_parts(self) -> (Context<D, P, A, X, Y, C>, ltv_filter::Completed<M, F, K, O>) {
        (self.context, self.filter)
    }

    pub fn into_remaining_coefficients(self) -> (queue::Completed<C>,
        ReturnedFirstFilter<D, P, A, X, Y, M, F, K, O>)
    {
        let ContextParts { upstream, mixer, queues, append_trace, refill_trace, second_signal } = self.context.into_parts();
        (queue::Completed::from_returned_filter(queues, mixer, append_trace),
            ReturnedFirstFilter { upstream, filter: self.filter, first_refill_trace: refill_trace, second_signal })
    }

    pub fn into_continuation(self) -> Continuation<D, P, A, X, Y, C, M, F, K, O> {
        let context = self.context.into_parts();
        let upstream = context.upstream.into_parts();
        let recurrent = upstream.condition_and_gru.into_parts();
        let (mixer, mixer_trace, moved_mixer_packets) = context.mixer.into_parts();
        let (filter, filter_input_after_call, observer, filter_trace) = self.filter.into_parts();
        let (second_signal, second_refill) = match context.second_signal {
            Some(second) => (Some(second.signal), Some(crate::ltv_second_refill::SecondTrace {
                trace: second.trace, readiness: second.readiness })),
            None => (None, None),
        };
        Continuation { condition: recurrent.condition, recurrent: recurrent.recurrent,
            stages: upstream.stages, mixer, queues: context.queues, filter, observer, second_signal,
            history: History { feature_input: recurrent.feature_input, phase_words: recurrent.phase_words,
                condition_work: recurrent.condition_work, recurrent_input: recurrent.intermediate,
                recurrent_status: recurrent.status, stages: upstream.stage_trace_after_move,
                mixer: mixer_trace, moved_mixer_packets, append: context.append_trace,
                refill: context.refill_trace, filter_input_after_call, filter: filter_trace, second_refill } }
    }
}

impl<D, P, A, X, Y, C, M, F, O>
    Completed<D, P, A, X, Y, C, M, F, crate::ltv_stream::Kernel<crate::ltv_output_values::ValueOutputs>, O>
{

    pub fn take_output_values(&mut self) -> Result<Batch, ltv_filter::output::Error> {
        self.filter.take_output_values()
    }
}
