use std::collections::VecDeque;
pub use crate::conditional_gate::Batch;
pub use crate::ltv_filter_values::{FilterProgress, FilterStage, FilterValueError};
pub use crate::model2_operator_binding::{FilterCallError, Ltvfs1Config13,
    OperatorBindingError, SourceLtvfs1Binding};
use crate::{ltv_filter_values as graph, ltv_synthesis,
    ltv_render::{SourceLtvBlockConfig, SourceLtvFft},
    model2_operator_binding::SourceFilterCallState};
#[path = "ltv_filter_services.rs"]
mod services;
#[path = "ltv_filter_output.rs"]
pub mod output;

pub trait PairedKernel {
    type Error;
    fn config(&self) -> SourceLtvBlockConfig;
    fn forward(&mut self, coefficients: &mut Batch, signal: &mut Batch,
        fft: &mut dyn SourceLtvFft) -> Result<(), Self::Error>;
}
pub trait SpanObserver {
    type Error;
    fn coefficient_span(&mut self, values: &mut [f32], length: &mut i32)
        -> Result<(), Self::Error>;
    fn finish_descriptor(&mut self) -> Result<(), Self::Error>;
}
impl<T: SpanObserver + ?Sized> SpanObserver for &mut T {
    type Error = T::Error;
    fn coefficient_span(&mut self, values: &mut [f32], length: &mut i32)
        -> Result<(), Self::Error> { T::coefficient_span(&mut **self, values, length) }
    fn finish_descriptor(&mut self) -> Result<(), Self::Error> {
        T::finish_descriptor(&mut **self)
    }
}

impl SpanObserver for std::convert::Infallible {
    type Error = std::convert::Infallible;
    fn coefficient_span(&mut self, _: &mut [f32], _: &mut i32) -> Result<(), Self::Error> {
        match *self {}
    }
    fn finish_descriptor(&mut self) -> Result<(), Self::Error> { match *self {} }
}

pub struct Parts<M, F, K> {
    pub binding: SourceLtvfs1Binding, pub frame: i32, pub ended_status: u8,
    pub signal_fifo: VecDeque<Option<Vec<f32>>>,
    pub coefficient_output: Batch, pub pair_signal: Batch,
    pub math: M, pub fft: F, pub kernel: K,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingError { Prefix(FilterCallError), Configuration, Fifo, Output }
pub struct BindingFailure<M, F, K> { pub cause: BindingError, pub parts: Parts<M, F, K> }

pub enum Backends<M, F> {
    Ready { math: M, fft: F },
    Coefficients(ltv_synthesis::Prepared<M, F>),
    Rejected(ltv_synthesis::StartFailure<M, F>),
    Vacant,
}
pub struct State<M, F, K> {
    binding: SourceLtvfs1Binding, frame: i32, ended_status: u8,
    config: SourceLtvBlockConfig,
    signal_fifo: VecDeque<Option<Vec<f32>>>, coefficient_output: Batch,
    pair_signal: Batch, backend: Backends<M, F>, kernel: K,
    prefix: SourceFilterCallState,
}
pub struct Input { pub coefficients: Batch, pub signal: Batch }
pub struct BeginFailure<M, F, K, O> {
    pub state: State<M, F, K>, pub input: Input, pub frame: i32, pub observer: Option<O>,
}
pub struct ObserverCall { pub ordinal: usize, pub initial_length: i32,
    pub current_length: i32, pub completed: bool }
pub struct Trace {
    pub input_signal_count: usize, pub input_signal_status: u8,
    pub moved_signal_prefix: usize, pub coefficient_input_loaned: bool,
    pub erased_signal_prefix: Option<usize>,
    pub retired_coefficient_output: Option<Batch>, pub retired_pair_rows: Vec<Vec<f32>>,
    pub observer_calls: Vec<ObserverCall>, pub kernel_attempted: bool,
    pub observer_finish_attempted: bool,
}
pub enum ServiceError<K, O> {
    CoefficientStart(ltv_synthesis::Error), Coefficient(ltv_synthesis::Failure),
    Allocation(std::collections::TryReserveError),
    SignalWidth { ordinal: usize, expected: usize, actual: usize },
    Control(&'static str), Kernel(K), Observer(O),
}
pub enum Error<K, O> {
    Prefix(FilterCallError), FrameConfiguration,
    Body(FilterValueError<ServiceError<K, O>>),
}
pub struct Run<M, F, K, O> {
    state: State<M, F, K>, input: Input, frame: i32, observer: Option<O>,
    trace: Trace, work: Option<graph::FilterWork>,
}
pub struct Failed<M, F, K: PairedKernel, O: SpanObserver> {
    run: Run<M, F, K, O>, error: Error<K::Error, O::Error>,
}
pub struct Completed<M, F, K, O> { run: Run<M, F, K, O> }

impl<M, F, K: PairedKernel> State<M, F, K> {
    pub fn from_explicit_parts(parts: Parts<M, F, K>) -> Result<Self, BindingFailure<M, F, K>> {
        let prefix = match SourceFilterCallState::from_explicit_initialized_frame(parts.frame, parts.ended_status) {
            Ok(prefix) => prefix,
            Err(error) => return Err(BindingFailure { cause: BindingError::Prefix(error), parts }),
        };
        let config = parts.kernel.config();
        let product = (parts.binding.config().raw_word0 as u32).wrapping_mul(parts.frame as u32) as i32;
        let cause = if product <= 0 || config.hop() != parts.frame as usize
            || config.window_length() != product as usize { Some(BindingError::Configuration) }
        else if parts.signal_fifo.len() > i32::MAX as usize || parts.signal_fifo.iter()
            .any(|row| row.as_ref().is_none_or(|row| row.len() != config.hop())) { Some(BindingError::Fifo) }
        else if parts.coefficient_output.rows.len() > i32::MAX as usize
            || parts.pair_signal.rows.len() > i32::MAX as usize
            || parts.coefficient_output.rows.iter().any(|row| row.len() != config.transform_length())
            || parts.pair_signal.rows.iter().any(|row| row.len() != config.hop()) { Some(BindingError::Output) }
        else { None };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }
        Ok(Self { binding: parts.binding, frame: parts.frame, ended_status: parts.ended_status, config,
            signal_fifo: parts.signal_fifo, coefficient_output: parts.coefficient_output,
            pair_signal: parts.pair_signal, backend: Backends::Ready { math: parts.math, fft: parts.fft },
            kernel: parts.kernel, prefix })
    }

    pub fn begin<O>(self, input: Input, frame: i32, observer: Option<O>)
        -> Result<Run<M, F, K, O>, BeginFailure<M, F, K, O>>
    {
        if input.coefficients.rows.len() > i32::MAX as usize {
            return Err(BeginFailure { state: self, input, frame, observer });
        }
        let trace = Trace { input_signal_count: input.signal.rows.len(), input_signal_status: input.signal.status,
            moved_signal_prefix: 0, coefficient_input_loaned: false, erased_signal_prefix: None,
            retired_coefficient_output: None, retired_pair_rows: Vec::new(), observer_calls: Vec::new(),
            kernel_attempted: false, observer_finish_attempted: false };
        Ok(Run { state: self, input, frame, observer, trace, work: None })
    }

    pub fn begin_without_observer(self, input: Input, frame: i32)
        -> Result<Run<M, F, K, std::convert::Infallible>, BeginFailure<M, F, K, std::convert::Infallible>>
    {
        self.begin(input, frame, None)
    }
}
impl<M, F, K> State<M, F, K> {
    pub fn frame(&self) -> i32 { self.frame }
    pub fn config(&self) -> SourceLtvBlockConfig { self.config }
    pub fn ended_status(&self) -> u8 { self.ended_status }
    pub fn signal_fifo(&self) -> &VecDeque<Option<Vec<f32>>> { &self.signal_fifo }
    pub fn coefficient_output(&self) -> &Batch { &self.coefficient_output }
    pub fn pair_signal(&self) -> &Batch { &self.pair_signal }
    pub fn backends(&self) -> &Backends<M, F> { &self.backend }
    pub fn kernel(&self) -> &K { &self.kernel }

    pub fn kernel_mut(&mut self) -> &mut K { &mut self.kernel }
    pub fn into_explicit_parts(mut self) -> Result<Parts<M, F, K>, Self> {
        match std::mem::replace(&mut self.backend, Backends::Vacant) {
            Backends::Ready { math, fft } => Ok(Parts { binding: self.binding, frame: self.frame,
                ended_status: self.ended_status, signal_fifo: self.signal_fifo,
                coefficient_output: self.coefficient_output, pair_signal: self.pair_signal, math, fft, kernel: self.kernel }),
            backend => { self.backend = backend; Err(self) }
        }
    }
}
impl<M, F, K, O> Run<M, F, K, O> {
    pub fn state(&self) -> &State<M, F, K> { &self.state }
    pub fn input(&self) -> &Input { &self.input }
    pub fn trace(&self) -> &Trace { &self.trace }
    pub fn observer(&self) -> Option<&O> { self.observer.as_ref() }
    pub fn progress(&self) -> Option<FilterProgress> { self.work.as_ref().map(graph::FilterWork::progress) }
}
impl<M: ltv_synthesis::SourceCoefficientMath, F: SourceLtvFft, K: PairedKernel, O: SpanObserver> Run<M, F, K, O> {

    pub fn run(mut self) -> Result<Completed<M, F, K, O>, Failed<M, F, K, O>> {
        let prefix = self.state.prefix.observe_positive_frame(self.state.binding, self.frame,
            self.input.coefficients.status, self.input.signal.status,
            self.input.coefficients.rows.len(), self.input.signal.rows.len());

        self.state.ended_status = self.state.prefix.ended_status();
        let error = match prefix {
            Err(error) => Some(Error::Prefix(error)),
            Ok(prefix) if prefix.initialize => Some(Error::FrameConfiguration),
            Ok(_) => {
                let mut work = graph::FilterWork::after_explicit_prefix(self.frame,
                    self.input.signal.rows.len(), self.observer.is_some());
                let result = graph::forward_initialized(&mut work, &mut self);
                self.work = Some(work);
                result.err().map(Error::Body)
            }
        };
        if let Some(error) = error { Err(Failed { run: self, error }) }
        else { Ok(Completed { run: self }) }
    }
}
impl<M, F, K: PairedKernel, O: SpanObserver> Failed<M, F, K, O> {
    pub fn pending(&self) -> &Run<M, F, K, O> { &self.run }
    pub fn error(&self) -> &Error<K::Error, O::Error> { &self.error }
}
impl<M, F, K, O> Completed<M, F, K, O> {
    pub fn state(&self) -> &State<M, F, K> { &self.run.state }
    pub fn trace(&self) -> &Trace { &self.run.trace }
    pub fn into_parts(self) -> (State<M, F, K>, Input, Option<O>, Trace) {
        (self.run.state, self.run.input, self.run.observer, self.run.trace)
    }
}
