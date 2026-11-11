use std::collections::VecDeque;
use crate::{conditional_gate::Batch, ltv_analysis_window as window, ltv_output_values as output,
    ltv_render::{LtvArithmeticPolicy, LtvError, SourceLtvBlockConfig}, ltv_stream,
    ltv_value_binding as binding, model2_operator_binding::{Ltvfs1Config13, OperatorBindingError, SourceLtvfs1Binding},
    userdata::{self, Role}, value_row_allocation as allocation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeError { Frame, WindowProduct(i32), Configuration(LtvError) }

pub fn shape(configuration: Ltvfs1Config13, frame: i32, transform_length: usize)
    -> Result<SourceLtvBlockConfig, ShapeError>
{
    if frame <= 0 { return Err(ShapeError::Frame); }
    let product = configuration.raw_word0.wrapping_mul(frame);
    if product <= 0 { return Err(ShapeError::WindowProduct(product)); }
    SourceLtvBlockConfig::bind(product as usize, transform_length, frame as usize)
        .map_err(ShapeError::Configuration)
}

#[derive(Clone, Copy, Debug)]
pub struct OutputPolicy {
    pub output_rows: usize, pub output_status: u8, pub output_word: u32,
    pub next_current_rows: usize, pub next_current_word: u32,
    pub terminal_signal_rows: usize, pub terminal_signal_word: u32,
    pub terminal_coefficient_rows: usize, pub terminal_coefficient_word: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputSite { Output, NextCurrent, TerminalSignal, TerminalCoefficient }
pub enum AllocationCause { Capacity(std::collections::TryReserveError), Row(allocation::LogicalError) }
pub enum OutputFailure {
    Count { config: SourceLtvBlockConfig, policy: OutputPolicy },
    Allocation { config: SourceLtvBlockConfig, policy: OutputPolicy,
        parts: output::Parts, site: OutputSite, cause: AllocationCause },
    Binding { config: SourceLtvBlockConfig, policy: OutputPolicy, failure: output::BindingFailure },
}
fn fill_rows(count: usize, width: usize, word: u32, mut push: impl FnMut(Vec<f32>))
    -> Result<(), allocation::LogicalError>
{
    for _ in 0..count { push(allocation::allocate_logical_row(width, word)?); }
    Ok(())
}

pub fn allocate_outputs(config: SourceLtvBlockConfig, policy: OutputPolicy)
    -> Result<output::ValueOutputs, OutputFailure>
{
    if [policy.output_rows, policy.next_current_rows, policy.terminal_signal_rows,
        policy.terminal_coefficient_rows].iter().any(|&count| count > i32::MAX as usize)
    { return Err(OutputFailure::Count { config, policy }); }
    let mut parts = output::Parts { hop: config.hop(), transform: config.transform_length(),
        output: Batch { rows: Vec::new(), status: policy.output_status },
        next_current: VecDeque::new(), terminal_signal: VecDeque::new(), terminal_coefficients: VecDeque::new() };
    let reservations = [OutputSite::Output, OutputSite::NextCurrent, OutputSite::TerminalSignal, OutputSite::TerminalCoefficient];
    for site in reservations {
        let result = match site {
            OutputSite::Output => parts.output.rows.try_reserve_exact(policy.output_rows),
            OutputSite::NextCurrent => parts.next_current.try_reserve(policy.next_current_rows),
            OutputSite::TerminalSignal => parts.terminal_signal.try_reserve(policy.terminal_signal_rows),
            OutputSite::TerminalCoefficient => parts.terminal_coefficients.try_reserve(policy.terminal_coefficient_rows),
        };
        if let Err(cause) = result {
            return Err(OutputFailure::Allocation { config, policy, parts, site, cause: AllocationCause::Capacity(cause) });
        }
    }
    for site in reservations {
        let result = match site {
            OutputSite::Output => fill_rows(policy.output_rows, config.hop(), policy.output_word,
                |row| parts.output.rows.push(row)),
            OutputSite::NextCurrent => fill_rows(policy.next_current_rows, config.hop(), policy.next_current_word,
                |row| parts.next_current.push_back(row)),
            OutputSite::TerminalSignal => fill_rows(policy.terminal_signal_rows, config.hop(), policy.terminal_signal_word,
                |row| parts.terminal_signal.push_back(row)),
            OutputSite::TerminalCoefficient => fill_rows(policy.terminal_coefficient_rows, config.transform_length(), policy.terminal_coefficient_word,
                |row| parts.terminal_coefficients.push_back(row)),
        };
        if let Err(cause) = result {
            return Err(OutputFailure::Allocation { config, policy, parts, site, cause: AllocationCause::Row(cause) });
        }
    }
    output::ValueOutputs::from_explicit_parts(parts)
        .map_err(|failure| OutputFailure::Binding { config, policy, failure })
}

pub struct History {
    pub ended_status: u8,
    pub outer_signal_fifo: VecDeque<Option<Vec<f32>>>,
    pub coefficient_output: Batch, pub pair_signal: Batch,
    pub signal: VecDeque<Vec<f32>>, pub coefficients: VecDeque<Vec<f32>>,
    pub paired: ltv_stream::PairedState, pub phase: i64, pub partial_count: usize,
    pub emitted_rows: u64, pub stream_halted: bool,
}
#[derive(Clone, Copy, Debug)]

pub struct StorageWords { pub overlap: u32, pub scratch: u32, pub current: u32 }
pub struct Inputs<M, F> {
    pub identity: binding::Identity, pub frame: i32, pub transform_length: usize,
    pub arithmetic: LtvArithmeticPolicy, pub storage: StorageWords, pub outputs: OutputPolicy,
    pub history: History, pub math: M, pub fft: F,
}

pub(crate) struct InitialInputs<M, F> {
    pub identity: binding::Identity, pub frame: i32,
    pub arithmetic: LtvArithmeticPolicy, pub outputs: OutputPolicy,
    pub math: M, pub fft: F,
}
pub struct Context<M, F> { pub role: Role, pub binding: SourceLtvfs1Binding, pub inputs: Inputs<M, F> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError { Role(Role), MissingConfiguration(Role), Identity(OperatorBindingError), Shape(ShapeError) }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageSite { Overlap, Scratch, Current }
pub struct WindowHistory<C> {
    pub config: SourceLtvBlockConfig, pub cosine: C, pub calls: Vec<window::Call>, pub normalizer: f32,
}
pub struct Bound<C, M, F> {
    pub initialized: binding::Bound<M, F, ltv_stream::Kernel<output::ValueOutputs>>,
    pub window: WindowHistory<C>,
}
pub enum Failure<C: window::SourceLtvWindowCosine, M, F> {
    Admission { role: Role, configuration: Option<Ltvfs1Config13>, cause: AdmissionError,
        inputs: Inputs<M, F>, cosine: C },
    Window { context: Context<M, F>, failure: window::Failure<C> },
    Storage { context: Context<M, F>, window: window::Completed<C>,
        rows: [Option<Vec<f32>>; 3], site: StorageSite, cause: allocation::LogicalError },
    Outputs { context: Context<M, F>, window: window::Completed<C>,
        rows: [Option<Vec<f32>>; 3], failure: OutputFailure },
    Binding { window: WindowHistory<C>, failure: binding::StreamFailure<M, F, output::ValueOutputs> },
}

pub enum InitialFailure<C: window::SourceLtvWindowCosine, M, F> {
    Initialization(Failure<C, M, F>),
    Prefill(InitialPrefillFailure<C, M, F>),
}

pub struct InitialPrefillFailure<C, M, F> {
    context: Context<M, F>, window: window::Completed<C>,
    rows: [Option<Vec<f32>>; 3], required_rows: usize, cause: AllocationCause,
}
impl<C, M, F> InitialPrefillFailure<C, M, F> {
    pub fn context(&self) -> &Context<M, F> { &self.context }
    pub fn window(&self) -> &window::Completed<C> { &self.window }
    pub fn storage_rows(&self) -> &[Option<Vec<f32>>; 3] { &self.rows }
    pub fn required_rows(&self) -> usize { self.required_rows }
    pub fn cause(&self) -> &AllocationCause { &self.cause }
}

pub fn initialize_userdata<C: window::SourceLtvWindowCosine, M, F>(userdata: &userdata::Decoded<'_>,
    role: Role, inputs: Inputs<M, F>, cosine: C) -> Result<Bound<C, M, F>, Failure<C, M, F>>
{
    if !matches!(role, Role::Filter136 | Role::Filter144 | Role::ExtraFilter) {
        return Err(Failure::Admission { role, configuration: None, cause: AdmissionError::Role(role), inputs, cosine });
    }
    let configuration = match userdata.primary().filter(role) {
        Some(configuration) => configuration,
        None => return Err(Failure::Admission { role, configuration: None,
            cause: AdmissionError::MissingConfiguration(role), inputs, cosine }),
    };
    let identity = inputs.identity;
    let admitted = match SourceLtvfs1Binding::bind(identity.decoded_tag, identity.outer_class, identity.inner_class, configuration) {
        Ok(binding) => binding,
        Err(cause) => return Err(Failure::Admission { role, configuration: Some(configuration),
            cause: AdmissionError::Identity(cause), inputs, cosine }),
    };
    initialize_admitted(role, admitted, inputs, cosine)
}

pub(crate) fn initialize_admitted<C: window::SourceLtvWindowCosine, M, F>(role: Role,
    admitted: SourceLtvfs1Binding, inputs: Inputs<M, F>, cosine: C)
    -> Result<Bound<C, M, F>, Failure<C, M, F>>
{
    let configuration = admitted.config();
    let config = match shape(configuration, inputs.frame, inputs.transform_length) {
        Ok(config) => config,
        Err(cause) => return Err(Failure::Admission { role, configuration: Some(configuration),
            cause: AdmissionError::Shape(cause), inputs, cosine }),
    };
    initialize_shaped(role, admitted, inputs, config, cosine)
}

pub(crate) fn initialize_shaped<C: window::SourceLtvWindowCosine, M, F>(role: Role,
    admitted: SourceLtvfs1Binding, inputs: Inputs<M, F>, config: SourceLtvBlockConfig, cosine: C)
    -> Result<Bound<C, M, F>, Failure<C, M, F>>
{
    let context = Context { role, binding: admitted, inputs };
    let (context, window, rows) = build_workspace(context, config, cosine)?;
    finish_workspace(context, window, rows)
}

pub(crate) fn initialize_initial_shaped<C: window::SourceLtvWindowCosine, M, F>(role: Role,
    admitted: SourceLtvfs1Binding, inputs: InitialInputs<M, F>,
    config: SourceLtvBlockConfig, cosine: C)
    -> Result<Bound<C, M, F>, InitialFailure<C, M, F>>
{
    let InitialInputs { identity, frame, arithmetic, outputs, math, fft } = inputs;
    let history = History { ended_status: 0, outer_signal_fifo: VecDeque::new(),
        coefficient_output: Batch { rows: Vec::new(), status: 0 },
        pair_signal: Batch { rows: Vec::new(), status: 0 },
        signal: VecDeque::new(), coefficients: VecDeque::new(),
        paired: ltv_stream::PairedState { accepted_rows208: 0, terminal248: false, halted: false },
        phase: -((config.transform_length() / 2) as i64), partial_count: 0,
        emitted_rows: 0, stream_halted: false };
    let context = Context { role, binding: admitted, inputs: Inputs {
        identity, frame, transform_length: config.transform_length(), arithmetic,
        storage: StorageWords { overlap: 0, scratch: 0, current: 0 }, outputs, history, math, fft } };
    let (mut context, window, rows) = build_workspace(context, config, cosine)
        .map_err(InitialFailure::Initialization)?;

    let required_rows = config.window_length() / (2 * config.hop());
    if let Err(cause) = context.inputs.history.signal.try_reserve(required_rows) {
        return Err(InitialFailure::Prefill(InitialPrefillFailure { context, window,
            rows, required_rows, cause: AllocationCause::Capacity(cause) }));
    }
    if let Err(cause) = fill_rows(required_rows, config.hop(), 0,
        |row| context.inputs.history.signal.push_back(row))
    {
        return Err(InitialFailure::Prefill(InitialPrefillFailure { context, window,
            rows, required_rows, cause: AllocationCause::Row(cause) }));
    }
    finish_workspace(context, window, rows).map_err(InitialFailure::Initialization)
}

type Workspace<C, M, F> = (Context<M, F>, window::Completed<C>, [Option<Vec<f32>>; 3]);
fn build_workspace<C: window::SourceLtvWindowCosine, M, F>(context: Context<M, F>,
    config: SourceLtvBlockConfig, cosine: C) -> Result<Workspace<C, M, F>, Failure<C, M, F>>
{
    let window = match window::build(config, cosine) {
        Ok(window) => window,
        Err(failure) => return Err(Failure::Window { context, failure }),
    };
    let mut rows = [None, None, None];
    for (index, site, width, word) in [
        (0, StorageSite::Overlap, config.transform_length(), context.inputs.storage.overlap),
        (1, StorageSite::Scratch, config.transform_length(), context.inputs.storage.scratch),
        (2, StorageSite::Current, config.hop(), context.inputs.storage.current),
    ] {
        match allocation::allocate_logical_row(width, word) {
            Ok(row) => rows[index] = Some(row),
            Err(cause) => return Err(Failure::Storage { context, window, rows, site, cause }),
        }
    }
    Ok((context, window, rows))
}

fn finish_workspace<C: window::SourceLtvWindowCosine, M, F>(context: Context<M, F>,
    window: window::Completed<C>, rows: [Option<Vec<f32>>; 3])
    -> Result<Bound<C, M, F>, Failure<C, M, F>>
{
    let config = window.config();
    let outputs = match allocate_outputs(config, context.inputs.outputs) {
        Ok(outputs) => outputs,
        Err(failure) => return Err(Failure::Outputs { context, window, rows, failure }),
    };
    let (config, samples, cosine, calls, normalizer) = window.into_parts();
    let window = WindowHistory { config, cosine, calls, normalizer };
    let Context { role, binding: admitted, inputs } = context;
    let Inputs { identity, frame, arithmetic, history, math, fft, .. } = inputs;
    let History { ended_status, outer_signal_fifo, coefficient_output, pair_signal,
        signal, coefficients, paired, phase, partial_count, emitted_rows, stream_halted } = history;
    let [overlap, scratch, current] = rows;

    let kernel = ltv_stream::Parts { config, arithmetic, window: samples,
        overlap: overlap.expect("allocated overlap"), scratch: scratch.expect("allocated scratch"),
        current, signal, coefficients, paired, phase, partial_count, emitted_rows, stream_halted, outputs };
    match binding::bind_admitted_stream(role, admitted, binding::Parts { identity,
        outer: binding::Outer { frame, ended_status, signal_fifo: outer_signal_fifo,
            coefficient_output, pair_signal, math, fft }, kernel })
    {
        Ok(initialized) => Ok(Bound { initialized, window }),
        Err(failure) => Err(Failure::Binding { window, failure }),
    }
}
