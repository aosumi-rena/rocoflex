pub use crate::ltv_block::{LtvArithmeticPolicy, LtvError, LtvFftDirection,
    SourceLtvBlockConfig, SourceLtvFft};
pub use crate::ltv_history::{LtvBatchOwnership, LtvBlockOwnership, LtvHistoryError,
    LtvRowOrigin};
use crate::ltv_history::SourceLtvHistory;
use crate::source_scalar::SourceF64ValueMath;
use crate::fft::{SourceFftMath, SourceFftMathBindingCause};
pub use crate::fft::{TransformAdmissionError, TwiddleStage, SourceFftMathFailure,
    SourceFftFailure, SourceTwiddleCompletion};
use crate::fft::source_math::{admit_transform, admission_message};

pub struct Fft512 { failure: Option<String> }

impl Fft512 {
    pub fn new() -> Self { Self { failure: None } }
    pub fn failure(&self) -> Option<&str> { self.failure.as_deref() }
}

impl SourceLtvFft for Fft512 {
    fn transform_in_place(&mut self, direction: LtvFftDirection, values: &mut [f32])
        -> Result<(), LtvError>
    {
        transform_values(512, &mut self.failure, direction, values)
    }
}

pub struct ConfiguredFft { config: SourceLtvBlockConfig, failure: Option<String>, math: ConfiguredMath }
enum ConfiguredMath { Host, Source(SourceFftMath) }
pub struct FftBindingFailure { pub config: SourceLtvBlockConfig }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFftBindingCause { UnsupportedSize, AlreadyFailed }

pub struct SourceFftBindingFailure {
    config: SourceLtvBlockConfig, math: SourceF64ValueMath,
    source_word: u32, cause: SourceFftBindingCause,
}
impl SourceFftBindingFailure {
    pub fn config(&self) -> SourceLtvBlockConfig { self.config }
    pub fn math(&self) -> &SourceF64ValueMath { &self.math }
    pub fn source_word(&self) -> u32 { self.source_word }
    pub fn cause(&self) -> SourceFftBindingCause { self.cause }
}
impl ConfiguredFft {

    pub fn from_explicit_config(config: SourceLtvBlockConfig) -> Result<Self, FftBindingFailure> {
        if ![512, 1024, 2048].contains(&config.transform_length()) {
            return Err(FftBindingFailure { config });
        }
        Ok(Self { config, failure: None, math: ConfiguredMath::Host })
    }

    pub fn from_explicit_source_math(config: SourceLtvBlockConfig, math: SourceF64ValueMath,
        source_word: u32) -> Result<Self, SourceFftBindingFailure>
    {
        if ![512, 1024, 2048].contains(&config.transform_length()) {
            return Err(SourceFftBindingFailure { config, math, source_word, cause: SourceFftBindingCause::UnsupportedSize });
        }
        let source = SourceFftMath::bind_with_failure(math, source_word, |math, source_word, cause|
            SourceFftBindingFailure { config, math, source_word, cause: match cause {
                SourceFftMathBindingCause::AlreadyFailed => SourceFftBindingCause::AlreadyFailed,
            } })?;
        Ok(Self { config, failure: None, math: ConfiguredMath::Source(source) })
    }
    pub fn config(&self) -> SourceLtvBlockConfig { self.config }
    pub fn failure(&self) -> Option<&str> {
        match &self.math {
            ConfiguredMath::Host => self.failure.as_deref(),
            ConfiguredMath::Source(source) => source.failure_message(),
        }
    }
    pub fn source_math(&self) -> Option<&SourceF64ValueMath> {
        match &self.math { ConfiguredMath::Host => None, ConfiguredMath::Source(source) => Some(source.math()) }
    }
    pub fn source_word(&self) -> Option<u32> {
        match &self.math { ConfiguredMath::Host => None, ConfiguredMath::Source(source) => Some(source.source_word()) }
    }
    pub fn source_failure(&self) -> Option<&SourceFftFailure> {
        match &self.math { ConfiguredMath::Host => None, ConfiguredMath::Source(source) => source.failure() }
    }

    pub fn last_source_twiddle(&self) -> Option<&SourceTwiddleCompletion> {
        match &self.math { ConfiguredMath::Host => None, ConfiguredMath::Source(source) => source.last_source_twiddle() }
    }
}
impl SourceLtvFft for ConfiguredFft {
    fn transform_in_place(&mut self, direction: LtvFftDirection, values: &mut [f32])
        -> Result<(), LtvError>
    {
        let transform = self.config.transform_length();
        match &mut self.math {
            ConfiguredMath::Host => transform_values(transform, &mut self.failure, direction, values),
            ConfiguredMath::Source(source) => {
                match source.transform_with_expected_width(transform, direction, values) {
                    Ok(()) => Ok(()),
                    Err(_) => {

                        Err(LtvError::FftBackend)
                    }
                }
            }
        }
    }
}

fn transform_values(transform: usize, failure: &mut Option<String>,
    direction: LtvFftDirection, values: &mut [f32]) -> Result<(), LtvError>
{
    if let Err(cause) = admit_transform(transform, failure, values) {
        if cause != TransformAdmissionError::PriorFailure { *failure = Some(admission_message(transform, cause)); }
        return Err(LtvError::FftBackend);
    }
    let result = {
        match direction {
            LtvFftDirection::ForwardPlus1 => crate::fft::rdft_forward(values),
            LtvFftDirection::InverseMinus1 => crate::fft::rdft_inverse_supported(values),
        }
    };
    if let Err(error) = result {
        *failure = Some(error);
        return Err(LtvError::FftBackend);
    }
    Ok(())
}

pub struct Input {
    pub config: SourceLtvBlockConfig,
    pub arithmetic: LtvArithmeticPolicy,
    pub window: Vec<f32>,
    pub signal: Vec<Vec<f32>>,
    pub coefficients: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Dimensions,
    NonfiniteInput,
    Extent,
    History(LtvHistoryError),
    Environment(crate::matrix_f32::Error),
    Block(LtvError),
    NonfiniteBlock,
    OutputPlan,
}

pub struct StartFailure<F> {
    pub error: Error,
    pub input: Input,
    pub fft: F,
}

pub struct Attempt {
    pub block: usize,
    pub signal: Vec<Vec<f32>>,
    pub coefficients: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub block: usize,
    pub error: Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    Pending { completed_blocks: usize, total_blocks: usize },
    Complete,
    Failed(Failure),
}

#[derive(Clone, Copy)]
enum State { Pending, Complete, Failed(Failure) }

pub struct Prepared<F> {
    input: Input,
    fft: F,
    plan: LtvBatchOwnership,
    next: usize,
    overlap: Vec<f32>,
    scratch: Vec<f32>,
    output: Vec<f32>,
    attempt: Option<Attempt>,
    state: State,
}

pub struct Completed<F> {
    pub input: Input,
    pub fft: F,
    pub plan: LtvBatchOwnership,
    pub samples: Vec<f32>,
    pub overlap: Vec<f32>,
    pub scratch: Vec<f32>,
}

impl<F> Completed<F> {

    #[cfg(windows)]
    pub fn play_wasapi(&self, parts: &crate::wasapi_render::Parts,
        wait: crate::wasapi_playback::WaitPolicy, cancel: &std::sync::atomic::AtomicBool)
        -> crate::wasapi_playback::Outcome
    {
        crate::wasapi_playback::play_mono(&self.samples, parts, wait, cancel)
    }

    pub fn write_wav<W: std::io::Write + ?Sized>(&self, writer: &mut W, sample_rate: u32)
        -> Result<crate::wave_output::Receipt, crate::wave_output::Error>
    {
        crate::wave_output::write_mono_samples(writer, sample_rate, &self.samples)
    }
}

impl<F: SourceLtvFft> Prepared<F> {

    pub fn from_reset(input: Input, fft: F) -> Result<Self, StartFailure<F>> {
        let config = input.config;
        let hop = config.hop();
        let n = config.transform_length();
        let error = if input.window.len() != config.window_length()
            || input.signal.len() != input.coefficients.len()
            || input.signal.iter().any(|row| row.len() != hop)
            || input.coefficients.iter().any(|row| row.len() != n)
        { Some(Error::Dimensions) }
        else if input.window.iter().chain(input.signal.iter().flatten())
            .chain(input.coefficients.iter().flatten()).any(|v| !v.is_finite())
        { Some(Error::NonfiniteInput) }
        else if input.signal.len() > i32::MAX as usize
            || input.signal.len().checked_mul(hop)
                .and_then(|n| n.checked_mul(std::mem::size_of::<f32>()))
                .is_none_or(|bytes| bytes > isize::MAX as usize)
        { Some(Error::Extent) }
        else { None };
        if let Some(error) = error { return Err(StartFailure { error, input, fft }); }
        let mut history = SourceLtvHistory::from_kernel_reset(config);
        let plan = match history.plan_paired_batch(input.signal.len(), 1, 1) {
            Ok(plan) => plan,
            Err(error) => return Err(StartFailure { error: Error::History(error), input, fft }),
        };

        if plan.output_rows != input.signal.len() {
            return Err(StartFailure { error: Error::OutputPlan, input, fft });
        }
        Ok(Self { input, fft, plan, next: 0, overlap: vec![0.0; n],
            scratch: vec![0.0; n], output: Vec::new(), attempt: None,
            state: State::Pending })
    }

    pub fn input(&self) -> &Input { &self.input }
    pub fn fft(&self) -> &F { &self.fft }
    pub fn plan(&self) -> &LtvBatchOwnership { &self.plan }
    pub fn output_prefix(&self) -> &[f32] { &self.output }
    pub fn overlap(&self) -> &[f32] { &self.overlap }
    pub fn scratch(&self) -> &[f32] { &self.scratch }
    pub fn attempt(&self) -> Option<&Attempt> { self.attempt.as_ref() }

    fn fail(&mut self, error: Error) -> Poll {
        let failure = Failure { block: self.next, error };
        self.state = State::Failed(failure);
        Poll::Failed(failure)
    }

    pub fn poll(&mut self, budget: usize) -> Poll {
        match self.state {
            State::Failed(failure) => return Poll::Failed(failure),
            State::Complete => return Poll::Complete,
            State::Pending => {},
        }
        let config = self.input.config;
        let hop = config.hop();
        let n = config.transform_length();
        for _ in 0..budget {
            if self.next == self.plan.blocks.len() { break; }
            if let Err(error) = crate::matrix_f32::check_environment() {
                return self.fail(Error::Environment(error));
            }
            let block = &self.plan.blocks[self.next];

            let signal = block.time_rows.iter().map(|origin| match origin {
                LtvRowOrigin::Input(index) => self.input.signal[*index as usize].clone(),
                LtvRowOrigin::ResetZero | LtvRowOrigin::FlushZero => vec![0.0; hop],
            }).collect();
            let coefficients = match block.coefficient {
                LtvRowOrigin::Input(index) => self.input.coefficients[index as usize].clone(),
                LtvRowOrigin::ResetZero | LtvRowOrigin::FlushZero => vec![0.0; n],
            };
            self.attempt = Some(Attempt { block: self.next, signal, coefficients });
            let attempt = self.attempt.as_ref().expect("installed attempt");
            if let Err(error) = config.accumulate_block_in_scratch(self.input.arithmetic,
                &mut self.fft, &self.input.window, &attempt.signal, &attempt.coefficients,
                &mut self.overlap, &mut self.scratch)
            { return self.fail(Error::Block(error)); }
            if self.overlap.iter().chain(&self.scratch).any(|v| !v.is_finite()) {
                return self.fail(Error::NonfiniteBlock);
            }
            let block = &self.plan.blocks[self.next];
            let count = block.visible_sample_count;

            if count != 0 {
                let start = block.visible_sample_start;
                let Some(end) = start.checked_add(count).filter(|&end| end <= n) else {
                    return self.fail(Error::OutputPlan);
                };
                let before = self.output.len() / hop;
                self.output.extend_from_slice(&self.overlap[start..end]);
                if (self.output.len() / hop != before) != block.completes_output_row {
                    return self.fail(Error::OutputPlan);
                }
            }
            self.overlap.copy_within(hop..n, 0);
            self.overlap[n-hop..].fill(0.0);
            self.next += 1;
            self.attempt = None;
        }
        if self.next == self.plan.blocks.len() {
            if self.output.len() != self.plan.output_rows * hop {
                return self.fail(Error::OutputPlan);
            }
            self.state = State::Complete;
            Poll::Complete
        } else { Poll::Pending { completed_blocks: self.next, total_blocks: self.plan.blocks.len() } }
    }

    pub fn into_completed(self) -> Result<Completed<F>, Self> {
        if !matches!(self.state, State::Complete) { return Err(self); }
        Ok(Completed { input: self.input, fft: self.fft, plan: self.plan,
            samples: self.output, overlap: self.overlap, scratch: self.scratch })
    }
}
