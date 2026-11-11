use std::collections::VecDeque;
use crate::{conditional_gate::Batch, ltv_filter::PairedKernel,
    ltv_render::{LtvArithmeticPolicy, LtvError, SourceLtvBlockConfig, SourceLtvFft},
    ltv_paired_values as paired, ltv_emission_values as emission};
pub use paired::{PairedState, PairedProgress, LtvInputQueue};
pub use emission::{EmissionProgress, EmissionPlan};
#[path = "ltv_stream_services.rs"]
mod services;

pub struct ExchangeFailure<E> { pub reason: E, pub returned_completed: Option<Vec<f32>> }
pub trait Outputs {
    type Error;
    fn clear_output_and_set_status(&mut self, status: u8) -> Result<(), Self::Error>;
    fn emit_then_acquire(&mut self, completed: Vec<f32>) -> Result<Vec<f32>, ExchangeFailure<Self::Error>>;

    fn acquire_terminal_row(&mut self, queue: LtvInputQueue, width: usize) -> Result<Vec<f32>, Self::Error>;
}
pub struct Parts<O> {
    pub config: SourceLtvBlockConfig, pub arithmetic: LtvArithmeticPolicy,
    pub window: Vec<f32>, pub overlap: Vec<f32>, pub scratch: Vec<f32>,
    pub signal: VecDeque<Vec<f32>>, pub coefficients: VecDeque<Vec<f32>>,
    pub paired: PairedState, pub phase: i64, pub partial_count: usize,
    pub current: Option<Vec<f32>>, pub emitted_rows: u64, pub stream_halted: bool,
    pub outputs: O,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingError { Width, Queue, Counter }
pub struct BindingFailure<O> { pub error: BindingError, pub parts: Parts<O> }
pub struct Trace {
    pub moved_signal: usize, pub moved_coefficients: usize,
    pub signal_header_cleared: bool, pub coefficient_header_cleared: bool,
    pub released_signal: Vec<Vec<f32>>, pub released_coefficients: Vec<Vec<f32>>,
    pub terminal_signal: usize, pub terminal_coefficients: usize,
    pub retained_terminal: Option<Vec<f32>>,
    pub pending_enqueue: Option<(LtvInputQueue, Vec<f32>)>,
}
pub enum ServiceError<E> {
    Allocation(std::collections::TryReserveError), Output(E), Exchange(ExchangeFailure<E>),
    Environment(crate::matrix_f32::Error), Arithmetic(LtvError),
    Width, Control, Emission(Box<emission::EmissionValueError<ServiceError<E>>>),
}
pub enum Failure<E> {
    PriorFailure, AlreadyEnded, StatusMismatch, CountMismatch, CountDomain,
    CounterOverflow, InvalidQueues, InputWidth,
    Body(paired::PairedValueError<ServiceError<E>>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stopped;
pub struct Kernel<O: Outputs> {
    paired: PairedState, stream: Stream<O>, trace: Option<Trace>,
    paired_work: Option<paired::PairedWork>, failure: Option<Failure<O::Error>>,
}
struct Stream<O> {
    config: SourceLtvBlockConfig, arithmetic: LtvArithmeticPolicy,
    window: Vec<f32>, overlap: Vec<f32>, scratch: Vec<f32>,
    signal: VecDeque<Vec<f32>>, coefficients: VecDeque<Vec<f32>>,
    phase: i64, partial_count: usize, current: Option<Vec<f32>>,
    emitted_rows: u64, halted: bool, outputs: O,
    work: Option<emission::EmissionWork>,
}
impl<O: Outputs> Kernel<O> {
    pub fn from_explicit_parts(parts: Parts<O>) -> Result<Self, BindingFailure<O>> {
        let c = parts.config;
        let error = if parts.window.len() != c.window_length() || parts.overlap.len() != c.transform_length()
            || parts.scratch.len() != c.transform_length() || parts.partial_count >= c.hop()
            || parts.current.as_ref().is_none_or(|row| row.len() != c.hop()) { Some(BindingError::Width) }
        else if parts.signal.len() > c.window_length() / c.hop()
            || parts.coefficients.len() > i32::MAX as usize
            || parts.signal.iter().any(|row| row.len() != c.hop())
            || parts.coefficients.iter().any(|row| row.len() != c.transform_length()) { Some(BindingError::Queue) }
        else if parts.paired.accepted_rows208 > i64::MAX as u64
            || parts.emitted_rows > parts.paired.accepted_rows208 { Some(BindingError::Counter) }
        else { None };
        if let Some(error) = error { return Err(BindingFailure { error, parts }); }
        Ok(Self { paired: parts.paired, stream: Stream { config: c, arithmetic: parts.arithmetic,
            window: parts.window, overlap: parts.overlap, scratch: parts.scratch,
            signal: parts.signal, coefficients: parts.coefficients, phase: parts.phase,
            partial_count: parts.partial_count, current: parts.current, emitted_rows: parts.emitted_rows,
            halted: parts.stream_halted, outputs: parts.outputs, work: None },
            trace: None, paired_work: None, failure: None })
    }
    pub fn paired_state(&self) -> &PairedState { &self.paired }
    pub fn outputs(&self) -> &O { &self.stream.outputs }
    fn output_access_allowed(&self) -> bool {
        self.failure.is_none() && !self.paired.halted && !self.stream.halted
    }
    pub fn outputs_if_idle(&self) -> Option<&O> {
        if self.output_access_allowed() { Some(&self.stream.outputs) } else { None }
    }

    pub fn outputs_mut_if_idle(&mut self) -> Option<&mut O> {
        if self.output_access_allowed() { Some(&mut self.stream.outputs) } else { None }
    }
    pub fn overlap(&self) -> &[f32] { &self.stream.overlap }
    pub fn scratch(&self) -> &[f32] { &self.stream.scratch }
    pub fn current(&self) -> Option<&[f32]> { self.stream.current.as_deref() }
    pub fn phase(&self) -> i64 { self.stream.phase }
    pub fn partial_count(&self) -> usize { self.stream.partial_count }
    pub fn emitted_rows(&self) -> u64 { self.stream.emitted_rows }
    pub fn signal_queue(&self) -> &VecDeque<Vec<f32>> { &self.stream.signal }
    pub fn coefficient_queue(&self) -> &VecDeque<Vec<f32>> { &self.stream.coefficients }
    pub fn failure(&self) -> Option<&Failure<O::Error>> { self.failure.as_ref() }
    pub fn trace(&self) -> Option<&Trace> { self.trace.as_ref() }
    pub fn paired_progress(&self) -> Option<PairedProgress> { self.paired_work.as_ref().map(paired::PairedWork::progress) }
    pub fn emission_progress(&self) -> Option<EmissionProgress> { self.stream.work.as_ref().map(emission::EmissionWork::progress) }

    pub fn into_parts(self) -> Result<(Parts<O>, Option<Trace>), Self> {
        if self.failure.is_some() || self.paired.halted || self.stream.halted { return Err(self); }
        let s = self.stream;
        Ok((Parts { config: s.config, arithmetic: s.arithmetic, window: s.window,
            overlap: s.overlap, scratch: s.scratch, signal: s.signal, coefficients: s.coefficients,
            paired: self.paired, phase: s.phase, partial_count: s.partial_count, current: s.current,
            emitted_rows: s.emitted_rows, stream_halted: s.halted, outputs: s.outputs }, self.trace))
    }
}
impl<O: Outputs> PairedKernel for Kernel<O> {
    type Error = Stopped;
    fn config(&self) -> SourceLtvBlockConfig { self.stream.config }
    fn forward(&mut self, coefficients: &mut Batch, signal: &mut Batch,
        fft: &mut dyn SourceLtvFft) -> Result<(), Stopped>
    {

        if self.failure.is_some() { return Err(Stopped); }
        let count = signal.rows.len();
        let window_rows = self.stream.config.window_length() / self.stream.config.hop();
        let error = if self.paired.halted || self.stream.halted { Some(Failure::PriorFailure) }
            else if self.paired.terminal248 { Some(Failure::AlreadyEnded) }
            else if coefficients.status != signal.status { Some(Failure::StatusMismatch) }
            else if coefficients.rows.len() != count { Some(Failure::CountMismatch) }
            else if count > i32::MAX as usize { Some(Failure::CountDomain) }
            else if self.paired.accepted_rows208.checked_add(count as u64)
                .is_none_or(|n| n > i64::MAX as u64) { Some(Failure::CounterOverflow) }
            else if signal.rows.iter().any(|row| row.len() != self.stream.config.hop())
                || coefficients.rows.iter().any(|row| row.len() != self.stream.config.transform_length()) {
                Some(Failure::InputWidth)
            } else if self.stream.signal.len() >= window_rows { Some(Failure::InvalidQueues) }
            else { None };
        if let Some(error) = error { self.failure = Some(error); return Err(Stopped); }
        self.trace = Some(Trace { moved_signal: 0, moved_coefficients: 0,
            signal_header_cleared: false, coefficient_header_cleared: false,
            released_signal: Vec::new(), released_coefficients: Vec::new(),
            terminal_signal: 0, terminal_coefficients: 0, retained_terminal: None, pending_enqueue: None });
        let mut work = paired::PairedWork::after_explicit_admission(count, coefficients.status, window_rows);
        let mut services = services::Pair { stream: &mut self.stream, coefficients, signal, fft,
            trace: self.trace.as_mut().expect("trace installed before paired control") };
        let result = paired::forward_admitted(&mut self.paired, &mut work, &mut services);
        self.paired_work = Some(work);
        match result {
            Ok(()) => Ok(()),
            Err(error) => { self.failure = Some(Failure::Body(error)); Err(Stopped) }
        }
    }
}
