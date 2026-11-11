use crate::{ltv_render::{ConfiguredFft, LtvArithmeticPolicy, SourceLtvBlockConfig},
    ltv_synthesis::{CoefficientError, SourceCoefficientMath}};

pub const MODEL_SAMPLE_RATE: u32 = 48_000;
pub const MODEL_HOP: usize = 240;
pub const MODEL_WINDOW: usize = 480;
pub const MODEL_TRANSFORM: usize = 1024;
pub const MODEL_CEPSTRAL_WIDTH: usize = 512;

#[derive(Clone, Copy, Debug, Default)]
pub struct StreamProgress {
    pub accepted_input_frames: u64,
    pub model_hop_attempts: u64,
    pub model_hop_completed: u64,
    pub emitted_output_frames: u64,
}

pub struct FixedHopStream {
    hop: usize,
    input: Vec<f32>,
    rendered: Vec<f32>,
    progress: StreamProgress,
    failed: Option<String>,
    in_flight: bool,
}

impl FixedHopStream {
    pub fn new(hop: usize) -> Result<Self, String> {
        if hop == 0 { return Err("Model hop must be positive".into()); }
        let mut input = Vec::new();
        input.try_reserve_exact(hop).map_err(|e| format!("Cannot allocate model input hop: {e}"))?;
        let mut rendered = Vec::new();
        rendered.try_reserve_exact(hop).map_err(|e| format!("Cannot allocate model output hop: {e}"))?;
        Ok(Self { hop, input, rendered, progress: StreamProgress::default(), failed: None, in_flight: false })
    }

    pub fn hop(&self) -> usize { self.hop }
    pub fn buffered_input_frames(&self) -> usize { self.input.len() }
    pub fn partial_input(&self) -> &[f32] { &self.input }
    pub fn current_rendered(&self) -> &[f32] { &self.rendered }
    pub fn progress(&self) -> StreamProgress { self.progress }
    pub fn failure(&self) -> Option<&str> { self.failed.as_deref() }

    pub fn framing_delay_bound_frames(&self) -> usize { self.hop - 1 }

    fn fail(&mut self, error: String) -> String {
        if self.failed.is_none() { self.failed = Some(error); }
        self.failed.as_ref().unwrap().clone()
    }

    pub fn process(&mut self, input_mono: &[f32], output_mono: &mut Vec<f32>,
        render_hop: impl FnMut(&[f32], &mut Vec<f32>) -> Result<(), String>) -> Result<(), String>
    {
        self.process_with_input(input_mono,output_mono,|part,buffer| {
            buffer.extend_from_slice(part); Ok(())
        },render_hop)
    }

    pub(crate) fn process_with_input(&mut self,input_mono:&[f32],output_mono:&mut Vec<f32>,
        mut append_input:impl FnMut(&[f32],&mut Vec<f32>)->Result<(),String>,
        mut render_hop:impl FnMut(&[f32],&mut Vec<f32>)->Result<(),String>)->Result<(),String>
    {
        if let Some(error) = &self.failed { return Err(error.clone()); }
        if self.in_flight { return Err("Previous model hop did not return; restart the stream".into()); }
        if input_mono.iter().any(|sample| !sample.is_finite()) {
            return Err(self.fail("Capture audio contains nonfinite samples".into()));
        }
        let mut cursor = 0;
        while cursor < input_mono.len() {
            let count = (self.hop - self.input.len()).min(input_mono.len() - cursor);
            let before=self.input.len();
            if let Err(error)=append_input(&input_mono[cursor..cursor+count],&mut self.input) {
                return Err(self.fail(error));
            }
            if self.input.len()!=before+count {
                return Err(self.fail("Input transform changed the framing extent".into()));
            }
            cursor += count;
            self.progress.accepted_input_frames = self.progress.accepted_input_frames.saturating_add(count as u64);
            if self.input.len() < self.hop { break; }

            self.rendered.clear();
            self.in_flight = true;
            self.progress.model_hop_attempts = self.progress.model_hop_attempts.saturating_add(1);
            let result = render_hop(&self.input, &mut self.rendered);
            self.in_flight = false;
            if let Err(error) = result { return Err(self.fail(error)); }
            self.progress.model_hop_completed = self.progress.model_hop_completed.saturating_add(1);
            if self.rendered.iter().any(|sample| !sample.is_finite()) {
                return Err(self.fail("Synthesis produced nonfinite samples".into()));
            }
            if let Err(error) = output_mono.try_reserve(self.rendered.len()) {
                return Err(self.fail(format!("Cannot queue synthesized audio: {error}")));
            }
            output_mono.extend_from_slice(&self.rendered);
            self.progress.emitted_output_frames = self.progress.emitted_output_frames.saturating_add(self.rendered.len() as u64);
            self.input.clear();
        }
        Ok(())
    }
}

struct OrdinaryCoefficientMath;
impl SourceCoefficientMath for OrdinaryCoefficientMath {
    fn scalar_57f8e0(&mut self, value: f32) -> Result<f32, CoefficientError> {
        let result = value.exp();
        if result.is_finite() { Ok(result) } else { Err(CoefficientError::Math) }
    }
    fn complex_516c0(&mut self, real: f32, imaginary: f32) -> Result<[f32; 2], CoefficientError> {
        let magnitude = real.exp();
        let result = [magnitude * imaginary.cos(), magnitude * imaginary.sin()];
        if result.iter().all(|value| value.is_finite()) { Ok(result) } else { Err(CoefficientError::Math) }
    }
}
struct OrdinaryWindowCosine;
impl crate::ltv_analysis_window::SourceLtvWindowCosine for OrdinaryWindowCosine {
    type Error = String;
    fn cosf_57e330(&mut self, angle: f32) -> Result<f32, String> {
        let value = angle.cos();
        if value.is_finite() { Ok(value) } else { Err("Nonfinite LTV window cosine".into()) }
    }
}

struct LtvFilter {
    config: SourceLtvBlockConfig,
    fft: ConfiguredFft,
    window: Vec<f32>,
    signal_rows: Vec<Vec<f32>>,
    packed: Vec<f32>,
    overlap: Vec<f32>,
    scratch: Vec<f32>,
    startup_remaining: usize,
    publication: Vec<f32>,
    published_hops: u64,
}
impl LtvFilter {
    fn new() -> Result<Self, String> {

        let config = SourceLtvBlockConfig::bind(MODEL_WINDOW, MODEL_TRANSFORM, MODEL_HOP)
            .map_err(|error| format!("Invalid model LTV configuration: {error:?}"))?;
        let fft = ConfiguredFft::from_explicit_config(config)
            .map_err(|_| "Cannot initialize the model's 1024-point LTV FFT".to_string())?;
        let built = crate::ltv_analysis_window::build(config, OrdinaryWindowCosine)
            .map_err(|_| "Cannot initialize model LTV window in the current arithmetic environment".to_string())?;
        let (_, window, _, _, _) = built.into_parts();
        Ok(Self { config, fft, window, signal_rows: vec![vec![0.0; MODEL_HOP]; MODEL_WINDOW / MODEL_HOP],
            packed: vec![0.0; MODEL_TRANSFORM], overlap: vec![0.0; MODEL_TRANSFORM],
            scratch: vec![0.0; MODEL_TRANSFORM], startup_remaining: MODEL_TRANSFORM / 2,
            publication: Vec::with_capacity(MODEL_HOP), published_hops: 0 })
    }
    fn process(&mut self, signal: &[f32], coefficients: &[f32], output: &mut [f32]) -> Result<(), String> {
        crate::ltv_coefficient_values::transform_values_into(MODEL_TRANSFORM, coefficients,
            &mut self.packed, &mut self.fft, &mut OrdinaryCoefficientMath)
            .map_err(|error| format!("Learned LTV coefficient transform failed: {error:?}"))?;
        self.signal_rows.rotate_left(1);
        self.signal_rows.last_mut().unwrap().copy_from_slice(signal);
        self.config.accumulate_block_in_scratch(
            LtvArithmeticPolicy::NearestEvenSeparateOperationsGradualUnderflow,
            &mut self.fft, &self.window, &self.signal_rows, &self.packed,
            &mut self.overlap, &mut self.scratch)
            .map_err(|error| format!("LTV overlap-add processing failed: {error:?}"))?;
        if self.overlap.iter().any(|value| !value.is_finite()) {
            return Err("LTV overlap-add produced nonfinite samples".into());
        }
        output.copy_from_slice(&self.overlap[..MODEL_HOP]);
        self.overlap.copy_within(MODEL_HOP.., 0);
        self.overlap[MODEL_TRANSFORM - MODEL_HOP..].fill(0.0);
        Ok(())
    }

    fn publish(&mut self, raw: &[f32], output: &mut Vec<f32>) {

        let skip = self.startup_remaining.min(raw.len());
        self.startup_remaining -= skip;
        let mut visible = &raw[skip..];
        while !visible.is_empty() {
            let take = (MODEL_HOP - self.publication.len()).min(visible.len());
            self.publication.extend_from_slice(&visible[..take]);
            visible = &visible[take..];
            if self.publication.len() == MODEL_HOP {
                output.extend_from_slice(&self.publication);
                self.publication.clear();
                self.published_hops = self.published_hops.saturating_add(1);
            }
        }
    }
}

#[derive(Clone,Copy,serde::Serialize)]
pub(crate) struct LtvStageStartupProgress {
    pub startup_remaining:usize,
    pub publication_samples:usize,
    pub completed_hops:u64,
    pub published_hops:u64,
}
#[derive(Clone,Copy,serde::Serialize)]
pub(crate) struct LtvPairStartupProgress {
    pub first:LtvStageStartupProgress,
    pub second:LtvStageStartupProgress,
    pub failed:bool,
}

pub struct LtvPair {
    first: LtvFilter,
    second: LtvFilter,
    first_output: Vec<f32>,
    second_output: Vec<f32>,
    first_completed_hops: u64,
    second_completed_hops: u64,
    failed: Option<String>,
}
impl LtvPair {
    pub fn new() -> Result<Self, String> {
        Ok(Self { first: LtvFilter::new()?, second: LtvFilter::new()?,
            first_output: vec![0.0; MODEL_HOP], second_output: vec![0.0; MODEL_HOP],
            first_completed_hops: 0, second_completed_hops: 0, failed: None })
    }
    pub fn hop(&self) -> usize { MODEL_HOP }

    pub fn latency_frames(&self) -> usize { (MODEL_TRANSFORM - MODEL_WINDOW) / 2 + MODEL_WINDOW - MODEL_HOP }
    pub fn completed_hops(&self) -> u64 { self.first_completed_hops.min(self.second_completed_hops) }
    pub fn first_completed_hops(&self) -> u64 { self.first_completed_hops }
    pub fn second_completed_hops(&self) -> u64 { self.second_completed_hops }
    pub fn first_published_hops(&self) -> u64 { self.first.published_hops }
    pub fn second_published_hops(&self) -> u64 { self.second.published_hops }
    pub fn failure(&self) -> Option<&str> { self.failed.as_deref() }

    pub(crate) fn startup_progress(&self) -> LtvPairStartupProgress {
        LtvPairStartupProgress {
            first:LtvStageStartupProgress {
                startup_remaining:self.first.startup_remaining,
                publication_samples:self.first.publication.len(),
                completed_hops:self.first_completed_hops,
                published_hops:self.first.published_hops,
            },
            second:LtvStageStartupProgress {
                startup_remaining:self.second.startup_remaining,
                publication_samples:self.second.publication.len(),
                completed_hops:self.second_completed_hops,
                published_hops:self.second.published_hops,
            },
            failed:self.failed.is_some(),
        }
    }

    pub fn raw_filtered_channels(&self) -> (&[f32], &[f32]) { (&self.first_output, &self.second_output) }

    pub fn process_first(&mut self, signal: &[f32], coefficients: &[f32], output: &mut Vec<f32>) -> Result<(), String> {
        self.process_stage(false, signal, coefficients, output)
    }

    pub fn process_second(&mut self, signal: &[f32], coefficients: &[f32], output: &mut Vec<f32>) -> Result<(), String> {
        self.process_stage(true, signal, coefficients, output)
    }

    fn process_stage(&mut self, second: bool, signal: &[f32], coefficients: &[f32], output: &mut Vec<f32>) -> Result<(), String> {
        if let Some(error) = &self.failed { return Err(error.clone()); }
        let result: Result<(), String> = (|| {
            Self::validate_hop(signal, coefficients)?;
            output.try_reserve(MODEL_HOP).map_err(|error| format!("Cannot queue LTV stage output: {error}"))?;
            let (filter, rendered, count) = if second {
                (&mut self.second, &mut self.second_output, &mut self.second_completed_hops)
            } else {
                (&mut self.first, &mut self.first_output, &mut self.first_completed_hops)
            };
            filter.process(signal, coefficients, rendered)?;
            filter.publish(rendered, output);
            *count = count.saturating_add(1);
            Ok(())
        })();
        if let Err(error) = &result { self.failed = Some(error.clone()); }
        result
    }

    pub fn process_channels(&mut self, signal0: &[f32], signal1: &[f32],
        coef0: &[f32], coef1: &[f32], output0: &mut Vec<f32>, output1: &mut Vec<f32>) -> Result<(), String>
    {
        if let Some(error) = &self.failed { return Err(error.clone()); }
        if let Err(error) = output0.try_reserve(MODEL_HOP).and_then(|_| output1.try_reserve(MODEL_HOP)) {
            let error = format!("Cannot queue LTV channel output: {error}");
            self.failed = Some(error.clone());
            return Err(error);
        }
        if let Err(error) = self.run_channels(signal0, signal1, coef0, coef1) {
            self.failed = Some(error.clone());
            return Err(error);
        }
        self.first.publish(&self.first_output, output0);
        self.second.publish(&self.second_output, output1);
        Ok(())
    }

    fn run_channels(&mut self, signal0: &[f32], signal1: &[f32], coef0: &[f32], coef1: &[f32]) -> Result<(), String> {
        Self::validate_hop(signal0, coef0)?;
        Self::validate_hop(signal1, coef1)?;
        self.first.process(signal0, coef0, &mut self.first_output)?;
        self.first_completed_hops = self.first_completed_hops.saturating_add(1);
        self.second.process(signal1, coef1, &mut self.second_output)?;
        self.second_completed_hops = self.second_completed_hops.saturating_add(1);
        Ok(())
    }

    fn validate_hop(signal: &[f32], coefficients: &[f32]) -> Result<(), String> {
        if signal.len() != MODEL_HOP || coefficients.len() != MODEL_CEPSTRAL_WIDTH {
            return Err("LTV needs a 240-sample signal hop and a 512-word cepstral row".into());
        }
        if signal.iter().chain(coefficients).any(|value| !value.is_finite()) {
            return Err("LTV inputs contain nonfinite samples or coefficients".into());
        }
        Ok(())
    }
}
