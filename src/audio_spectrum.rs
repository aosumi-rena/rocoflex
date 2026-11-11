use crate::{
    Result,
    fft::{rdft_forward, SourceFftMath, SourceFftFailure, SourceFftDirection},
    model::ModelTree,
    model_tags::decoded_hashes,
    timbre::{f32_at, u32_at},
};

#[path = "audio_spectrum_stream.rs"]
mod streaming;
pub use streaming::{SpectrumBatch, SpectrumStream, SpectrumStreamFailure};
#[path = "spectrum_magnitude_math.rs"]
pub(crate) mod magnitude_math;
pub use magnitude_math::{SourceSpectrumMath, SourceSpectrumMathBindingFailure,
    SourceSpectrumMathBindingCause, SourceSpectrumMathFailure, SourceSpectrumMathCause,
    SourceMagnitudeStage, SourceMagnitudeAttempt, SourceWindowCause, SourceWindowAttempt,
    SourceSpectrumWindowBindingFailure, SourceSpectrumWindowBindingCause};
use std::convert::Infallible;

trait SpectrumFft {
    type Error;
    type MagnitudeError;
    type WindowError;
    fn transform(&mut self, values: &mut [f32]) -> std::result::Result<(), Self::Error>;
    fn magnitude(&mut self, bin: usize, re: f32, im: f32) -> std::result::Result<f32, Self::MagnitudeError>;
    fn window(&mut self, index: usize, sample: f32, coefficient: f32) -> std::result::Result<f32, Self::WindowError>;
}
struct HostSpectrumFft;
impl SpectrumFft for HostSpectrumFft {
    type Error = String;
    type MagnitudeError = Infallible;
    type WindowError = Infallible;
    fn transform(&mut self, values: &mut [f32]) -> Result<()> { rdft_forward(values) }
    fn magnitude(&mut self, bin: usize, re: f32, im: f32) -> std::result::Result<f32, Infallible> {
        magnitude_math::host_magnitude(bin, re, im)
    }
    fn window(&mut self, index: usize, sample: f32, coefficient: f32) -> std::result::Result<f32, Infallible> {
        magnitude_math::host_window(index, sample, coefficient)
    }
}
impl SpectrumFft for SourceFftMath {
    type Error = SourceFftFailure;
    type MagnitudeError = Infallible;
    type WindowError = Infallible;
    fn transform(&mut self, values: &mut [f32]) -> std::result::Result<(), Self::Error> {
        self.transform_in_place(SourceFftDirection::ForwardPlus1, values)
    }
    fn magnitude(&mut self, bin: usize, re: f32, im: f32) -> std::result::Result<f32, Infallible> {
        magnitude_math::host_magnitude(bin, re, im)
    }
    fn window(&mut self, index: usize, sample: f32, coefficient: f32) -> std::result::Result<f32, Infallible> {
        magnitude_math::host_window(index, sample, coefficient)
    }
}
impl SpectrumFft for SourceSpectrumMath {
    type Error = SourceSpectrumMathCause;
    type MagnitudeError = SourceSpectrumMathCause;
    type WindowError = SourceSpectrumMathCause;
    fn transform(&mut self, values: &mut [f32]) -> std::result::Result<(), Self::Error> {
        self.transform_forward(values)
    }
    fn magnitude(&mut self, bin: usize, re: f32, im: f32) -> std::result::Result<f32, Self::MagnitudeError> {
        SourceSpectrumMath::magnitude(self, bin, re, im)
    }
    fn window(&mut self, index: usize, sample: f32, coefficient: f32) -> std::result::Result<f32, Self::WindowError> {
        SourceSpectrumMath::window(self, index, sample, coefficient)
    }
}
#[derive(Clone, Copy)]
pub enum SourceFftSpectrumStage { Input, Framing, Window, Prefft, Fft, Magnitude, Postlog }
enum FrameCause<E, G = Infallible, H = Infallible> {
    Source { stage: SourceFftSpectrumStage, reason: String }, Fft(E), Magnitude(G), Window(H),
}
fn host_reason(cause: FrameCause<String>) -> String {
    match cause {
        FrameCause::Source { reason, .. } | FrameCause::Fft(reason) => reason,
        FrameCause::Magnitude(never) => match never {},
        FrameCause::Window(never) => match never {},
    }
}
struct FrameWork {
    stage: SourceFftSpectrumStage, window_input: Option<Vec<f32>>, window_sample_index: Option<usize>,
    windowed1024: Option<Vec<f32>>, packed1024: Option<Vec<f32>>,
    magnitude513: Option<Vec<f32>>, magnitude_bin: Option<usize>, postlog513: Option<Vec<f32>>,
}
impl FrameWork {
    fn empty(stage: SourceFftSpectrumStage) -> Self {
        Self { stage, window_input: None, window_sample_index: None, windowed1024: None, packed1024: None, magnitude513: None, magnitude_bin: None, postlog513: None }
    }
}
struct FrameFailure<E, G = Infallible, H = Infallible> { cause: FrameCause<E, G, H>, work: FrameWork }
struct StreamFailure<E, G = Infallible, H = Infallible> {
    cause: FrameCause<E, G, H>, completed: Vec<f32>, current: Option<SourceFftSpectrumAttempt>,
}
enum WindowFailure<E> {
    Source(String), Operation { cause: E, output: Vec<f32>, index: usize },
}

pub struct SourceFftSpectrumAttempt {
    row: usize, prepared_samples: Option<[f32; 1024]>, work: FrameWork,
}
impl SourceFftSpectrumAttempt {
    pub fn row(&self) -> usize { self.row }
    pub fn stage(&self) -> SourceFftSpectrumStage { self.work.stage }
    pub fn prepared_samples(&self) -> Option<&[f32; 1024]> { self.prepared_samples.as_ref() }

    pub fn window_input(&self) -> Option<&[f32]> { self.work.window_input.as_deref() }
    pub fn window_sample_index(&self) -> Option<usize> { self.work.window_sample_index }
    pub fn windowed1024(&self) -> Option<&[f32]> { self.work.windowed1024.as_deref() }
    pub fn packed1024(&self) -> Option<&[f32]> { self.work.packed1024.as_deref() }
    pub fn magnitude513(&self) -> Option<&[f32]> { self.work.magnitude513.as_deref() }

    pub fn magnitude_bin(&self) -> Option<usize> { self.work.magnitude_bin }
    pub fn postlog513(&self) -> Option<&[f32]> { self.work.postlog513.as_deref() }
}
pub enum SourceFftSpectrumCause {
    Source { stage: SourceFftSpectrumStage, reason: String }, Fft(SourceFftFailure),
}
pub struct SourceFftSpectrum { postlog: Vec<f32>, fft: SourceFftMath }
impl SourceFftSpectrum {
    pub fn postlog(&self) -> &[f32] { &self.postlog }
    pub fn fft(&self) -> &SourceFftMath { &self.fft }
    pub fn into_parts(self) -> (Vec<f32>, SourceFftMath) { (self.postlog, self.fft) }
}

pub struct SourceFftSpectrumFailure {
    fft: SourceFftMath, cause: SourceFftSpectrumCause,
    completed: Vec<f32>, current: Option<SourceFftSpectrumAttempt>,
}
impl SourceFftSpectrumFailure {
    pub fn fft(&self) -> &SourceFftMath { &self.fft }
    pub fn cause(&self) -> &SourceFftSpectrumCause { &self.cause }
    pub fn completed_postlog(&self) -> &[f32] { &self.completed }
    pub fn current(&self) -> Option<&SourceFftSpectrumAttempt> { self.current.as_ref() }
}

pub enum SourceMathSpectrumCause {
    Source { stage: SourceFftSpectrumStage, reason: String }, Math(SourceSpectrumMathCause),
}
pub struct SourceMathSpectrum { postlog: Vec<f32>, math: SourceSpectrumMath }
impl SourceMathSpectrum {
    pub fn postlog(&self) -> &[f32] { &self.postlog }
    pub fn math(&self) -> &SourceSpectrumMath { &self.math }
    pub fn into_parts(self) -> (Vec<f32>, SourceSpectrumMath) { (self.postlog, self.math) }
}

pub struct SourceMathSpectrumFailure {
    math: SourceSpectrumMath, cause: SourceMathSpectrumCause,
    completed: Vec<f32>, current: Option<SourceFftSpectrumAttempt>,
}
impl SourceMathSpectrumFailure {
    pub fn math(&self) -> &SourceSpectrumMath { &self.math }
    pub fn cause(&self) -> &SourceMathSpectrumCause { &self.cause }
    pub fn completed_postlog(&self) -> &[f32] { &self.completed }
    pub fn current(&self) -> Option<&SourceFftSpectrumAttempt> { self.current.as_ref() }
}

pub struct AudioSpectrum {
    window: Vec<f32>,
}

pub struct SpectrumStages {
    pub windowed1024: Vec<f32>,
    pub packed1024: Vec<f32>,
    pub magnitude513: Vec<f32>,
    pub postlog513: Vec<f32>,
}

pub fn packed_magnitude513(packed: &[f32]) -> Result<Vec<f32>> {
    packed_magnitude_using(packed, &mut HostSpectrumFft).map_err(|failure| match failure {
        PackedMagnitudeFailure::Source(reason) => reason,
        PackedMagnitudeFailure::Operation { cause: never, .. } => match never {},
    })
}
enum PackedMagnitudeFailure<E> {
    Source(String), Operation { cause: E, output: Vec<f32>, bin: usize },
}
fn packed_magnitude_using<F: SpectrumFft>(packed: &[f32], fft: &mut F)
    -> std::result::Result<Vec<f32>, PackedMagnitudeFailure<F::MagnitudeError>>
{
    if packed.len() != 1024 || packed.iter().any(|v| !v.is_finite()) {
        return Err(PackedMagnitudeFailure::Source("magnitude requires finite 1024-lane packed RDFT".into()));
    }
    let mut out = vec![0.0; 513];
    out[0] = packed[0].abs();
    out[512] = packed[1].abs();
    for i in 1..512 {
        match fft.magnitude(i, packed[2 * i], packed[2 * i + 1]) {
            Ok(value) => out[i] = value,
            Err(cause) => return Err(PackedMagnitudeFailure::Operation { cause, output: out, bin: i }),
        }
    }
    Ok(out)
}

pub fn spectrum_postlog513(raw: &[f32]) -> Result<Vec<f32>> {
    if raw.is_empty() || raw.len() % 513 != 0 || raw.iter().any(|&v| !v.is_finite() || v < 0.0) {
        return Err("postlog requires nonnegative finite rows x513".into());
    }
    let mut out = raw.to_vec();
    for row in out.chunks_exact_mut(513) {
        for v in &mut row[..512] {
            *v = (((*v as f64) + 0.001_f64).ln() * 0.10000000149011612_f64) as f32;
        }
    }
    Ok(out)
}

impl AudioSpectrum {

    pub fn stream_with_limit(&self, maximum_samples: usize) -> Result<SpectrumStream<'_>> {
        SpectrumStream::new(self, maximum_samples)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("Unsupported feature model version".into());
        }
        let root = tree.root;
        let node = root
            .children
            .first()
            .and_then(|n| n.children.first())
            .ok_or("spectrum module missing")?;
        node.require(0x7fca40ff, b"", &[0xbeeb7256bd03fcc5])?;
        let cfg = [1u32, 1024, 240, 1024, 512, 0xffffefff, 0xffffffff, 0];
        if node.data.len() != 32 || node.children.len() != 1 {
            return Err("spectrum topology mismatch".into());
        }
        for (i, v) in cfg.into_iter().enumerate() {
            if u32_at(node.data, i * 4)? != v {
                return Err("unqualified spectrum configuration".into());
            }
        }
        let window = &node.children[0];
        window.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
        if !window.children.is_empty()
            || window.data.len() != 4100
            || u32_at(window.data, 0)? != 1024
        {
            return Err("spectrum window must be 1024-element model vector".into());
        }
        let values = (0..1024)
            .map(|i| f32_at(window.data, 4 + 4 * i))
            .collect::<Result<Vec<_>>>()?;
        if values.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite spectrum window".into());
        }
        Ok(Self { window: values })
    }

    pub fn window1024(&self, samples: &[f32]) -> Result<Vec<f32>> {
        self.window1024_using(samples, &mut HostSpectrumFft).map_err(|failure| match failure {
            WindowFailure::Source(reason) => reason,
            WindowFailure::Operation { cause: never, .. } => match never {},
        })
    }

    fn window1024_using<F: SpectrumFft>(&self, samples: &[f32], fft: &mut F)
        -> std::result::Result<Vec<f32>, WindowFailure<F::WindowError>>
    {
        if samples.len() != 1024 || samples.iter().any(|v| !v.is_finite()) {
            return Err(WindowFailure::Source("spectrum window requires finite 1024 samples".into()));
        }
        let mut output = Vec::with_capacity(samples.len().min(self.window.len()));
        for (index, (&sample, &coefficient)) in samples.iter().zip(&self.window).enumerate() {
            match fft.window(index, sample, coefficient) {
                Ok(value) => output.push(value),
                Err(cause) => return Err(WindowFailure::Operation { cause, output, index }),
            }
        }
        Ok(output)
    }

    pub fn from_window_samples(&self, samples: &[f32]) -> Result<SpectrumStages> {
        self.from_window_samples_using(samples, &mut HostSpectrumFft).map_err(|failure| host_reason(failure.cause))
    }

    fn from_window_samples_using<F: SpectrumFft>(&self, samples: &[f32], fft: &mut F)
        -> std::result::Result<SpectrumStages, FrameFailure<F::Error, F::MagnitudeError, F::WindowError>>
    {
        let window_input = match self.window1024_using(samples, fft) {
            Ok(value) => value,
            Err(WindowFailure::Source(reason)) => return Err(FrameFailure {
                cause: FrameCause::Source { stage: SourceFftSpectrumStage::Window, reason },
                work: FrameWork::empty(SourceFftSpectrumStage::Window),
            }),
            Err(WindowFailure::Operation { cause, output, index }) => {
                let mut work = FrameWork::empty(SourceFftSpectrumStage::Window);
                work.window_input = Some(output); work.window_sample_index = Some(index);
                return Err(FrameFailure { cause: FrameCause::Window(cause), work });
            }
        };
        match self.from_prefft_using(&window_input, fft) {
            Ok(stages) => Ok(stages),
            Err(mut failure) => {
                failure.work.window_input = Some(window_input);
                Err(failure)
            }
        }
    }

    pub fn stream_frame1024(&self, stream: &[f32], row: usize) -> Result<[f32; 1024]> {
        let center = row.checked_mul(240).ok_or("frame position overflow")?;
        center.checked_add(1024).ok_or("frame position overflow")?;
        let mut samples = [0.0; 1024];
        for (i, v) in samples.iter_mut().enumerate() {
            if center + i >= 512 {
                *v = stream.get(center + i - 512).copied().unwrap_or(0.0);
            }
        }
        Ok(samples)
    }

    pub fn from_prefft(&self, prefft: &[f32]) -> Result<SpectrumStages> {
        self.from_prefft_using(prefft, &mut HostSpectrumFft).map_err(|failure| host_reason(failure.cause))
    }

    fn from_prefft_using<F: SpectrumFft>(&self, prefft: &[f32], fft: &mut F)
        -> std::result::Result<SpectrumStages, FrameFailure<F::Error, F::MagnitudeError, F::WindowError>>
    {
        if prefft.len() != 1024 || prefft.iter().any(|v| !v.is_finite()) {
            return Err(FrameFailure {
                cause: FrameCause::Source { stage: SourceFftSpectrumStage::Prefft, reason: "prefft must have 1024 finite lanes".into() },
                work: FrameWork::empty(SourceFftSpectrumStage::Prefft),
            });
        }
        let windowed1024 = prefft.to_vec();
        let mut packed1024 = windowed1024.clone();
        if let Err(cause) = fft.transform(&mut packed1024) {
            return Err(FrameFailure { cause: FrameCause::Fft(cause), work: FrameWork {
                stage: SourceFftSpectrumStage::Fft, window_input: None, window_sample_index: None,
                windowed1024: Some(windowed1024), packed1024: Some(packed1024), magnitude513: None, magnitude_bin: None, postlog513: None,
            } });
        }
        let magnitude513 = match packed_magnitude_using(&packed1024, fft) {
            Ok(value) => value,
            Err(PackedMagnitudeFailure::Source(reason)) => return Err(FrameFailure {
                cause: FrameCause::Source { stage: SourceFftSpectrumStage::Magnitude, reason },
                work: FrameWork { stage: SourceFftSpectrumStage::Magnitude, window_input: None, window_sample_index: None,
                    windowed1024: Some(windowed1024), packed1024: Some(packed1024), magnitude513: None, magnitude_bin: None, postlog513: None },
            }),
            Err(PackedMagnitudeFailure::Operation { cause, output, bin }) => return Err(FrameFailure {
                cause: FrameCause::Magnitude(cause),
                work: FrameWork { stage: SourceFftSpectrumStage::Magnitude, window_input: None, window_sample_index: None,
                    windowed1024: Some(windowed1024), packed1024: Some(packed1024), magnitude513: Some(output),
                    magnitude_bin: Some(bin), postlog513: None },
            }),
        };
        let postlog513 = match spectrum_postlog513(&magnitude513) {
            Ok(value) => value,
            Err(reason) => return Err(FrameFailure {
                cause: FrameCause::Source { stage: SourceFftSpectrumStage::Postlog, reason },
                work: FrameWork { stage: SourceFftSpectrumStage::Postlog, window_input: None, window_sample_index: None,
                    windowed1024: Some(windowed1024), packed1024: Some(packed1024), magnitude513: Some(magnitude513), magnitude_bin: None, postlog513: None },
            }),
        };
        Ok(SpectrumStages {
            windowed1024,
            packed1024,
            magnitude513,
            postlog513,
        })
    }

    pub fn from_stream48k(&self, stream: &[f32]) -> Result<Vec<f32>> {
        self.from_stream_using(stream, &mut HostSpectrumFft).map_err(|failure| host_reason(failure.cause))
    }

    pub fn from_stream48k_with_source_fft(&self, stream: &[f32], mut fft: SourceFftMath)
        -> std::result::Result<SourceFftSpectrum, SourceFftSpectrumFailure>
    {
        match self.from_stream_using(stream, &mut fft) {
            Ok(postlog) => Ok(SourceFftSpectrum { postlog, fft }),
            Err(StreamFailure { cause, completed, current }) => Err(SourceFftSpectrumFailure {
                fft, cause: match cause {
                    FrameCause::Source { stage, reason } => SourceFftSpectrumCause::Source { stage, reason },
                    FrameCause::Fft(cause) => SourceFftSpectrumCause::Fft(cause),
                    FrameCause::Magnitude(never) => match never {},
                    FrameCause::Window(never) => match never {},
                }, completed, current }),
        }
    }

    pub fn from_stream48k_with_source_math(&self, stream: &[f32], mut math: SourceSpectrumMath)
        -> std::result::Result<SourceMathSpectrum, SourceMathSpectrumFailure>
    {
        match self.from_stream_using(stream, &mut math) {
            Ok(postlog) => Ok(SourceMathSpectrum { postlog, math }),
            Err(StreamFailure { cause, completed, current }) => Err(SourceMathSpectrumFailure {
                math, cause: match cause {
                    FrameCause::Source { stage, reason } => SourceMathSpectrumCause::Source { stage, reason },
                    FrameCause::Fft(cause) | FrameCause::Magnitude(cause) | FrameCause::Window(cause) => SourceMathSpectrumCause::Math(cause),
                }, completed, current }),
        }
    }

    fn from_stream_using<F: SpectrumFft>(&self, stream: &[f32], fft: &mut F)
        -> std::result::Result<Vec<f32>, StreamFailure<F::Error, F::MagnitudeError, F::WindowError>>
    {
        if stream.is_empty() || stream.iter().any(|v| !v.is_finite()) {
            return Err(StreamFailure {
                cause: FrameCause::Source { stage: SourceFftSpectrumStage::Input, reason: "spectrum requires nonempty finite normalized stream".into() },
                completed: Vec::new(), current: None,
            });
        }
        let Some(padded) = stream.len().checked_add(512) else {
            return Err(StreamFailure {
                cause: FrameCause::Source { stage: SourceFftSpectrumStage::Input, reason: "stream length overflow".into() },
                completed: Vec::new(), current: None,
            });
        };
        let n = padded.div_ceil(240);
        let Some(cap) = n.checked_mul(513) else {
            return Err(StreamFailure {
                cause: FrameCause::Source { stage: SourceFftSpectrumStage::Input, reason: "spectrum size overflow".into() },
                completed: Vec::new(), current: None,
            });
        };
        let mut output = Vec::with_capacity(cap);
        for row in 0..n {
            let samples = match self.stream_frame1024(stream, row) {
                Ok(value) => value,
                Err(reason) => return Err(StreamFailure {
                    cause: FrameCause::Source { stage: SourceFftSpectrumStage::Framing, reason },
                    completed: output, current: Some(SourceFftSpectrumAttempt {
                        row, prepared_samples: None, work: FrameWork::empty(SourceFftSpectrumStage::Framing),
                    }),
                }),
            };
            match self.from_window_samples_using(&samples, fft) {
                Ok(stages) => output.extend(stages.postlog513),
                Err(FrameFailure { cause, work }) => return Err(StreamFailure {
                    cause, completed: output,
                    current: Some(SourceFftSpectrumAttempt { row, prepared_samples: Some(samples), work }),
                }),
            }
        }
        Ok(output)
    }
}
