use crate::{
    Result,
    fft::{rdft_forward, SourceFftMath, SourceFftDirection, SourceFftFailure},
    audio_spectrum::{SourceSpectrumMath, SourceSpectrumMathCause},
    model::ModelTree,
    model_tags::decoded_hashes,
    timbre::{f32_at, u32_at},
};

#[path = "pitch_audio_spectrum_stream.rs"]
mod streaming;
pub use streaming::{PitchSpectrumBatch, PitchSpectrumStream, PitchSpectrumStreamFailure};
pub(crate) use streaming::PitchSpectrumState;
#[path = "pitch_spectrum_postlog.rs"]
mod postlog;
pub use postlog::{SourcePitchPostlogMath, SourcePitchPostlogBindingCause,
    SourcePitchPostlogBindingFailure, SourcePitchPostlogStage, SourcePitchPostlogCause,
    SourcePitchPostlogAttempt, SourcePitchPostlogFailure};

trait FramePostlog {
    type Error;
    fn apply(&mut self, magnitude: f32, epsilon: f32) -> std::result::Result<f32, Self::Error>;
}

struct MagnitudesOnly;
impl FramePostlog for MagnitudesOnly {
    type Error = std::convert::Infallible;
    fn apply(&mut self, magnitude: f32, _epsilon: f32) -> std::result::Result<f32, Self::Error> {
        Ok(magnitude)
    }
}
struct HostPostlog;
impl FramePostlog for HostPostlog {
    type Error = std::convert::Infallible;
    fn apply(&mut self, magnitude: f32, epsilon: f32) -> std::result::Result<f32, Self::Error> {
        let shifted = magnitude + epsilon;
        let logged = shifted.ln();
        Ok(logged * 0.1_f32)
    }
}
trait FrameFft {
    type Error;
    type MagnitudeError;
    type WindowError;
    fn transform(&mut self, frame: &mut [f32]) -> std::result::Result<(), Self::Error>;
    fn magnitude(&mut self, bin: usize, real: f32, imaginary: f32)
        -> std::result::Result<f32, Self::MagnitudeError>;
    fn window(&mut self, index: usize, sample: f32, coefficient: f32)
        -> std::result::Result<f32, Self::WindowError>;
}
struct HostFrameFft;
impl FrameFft for HostFrameFft {
    type Error = String;
    type MagnitudeError = std::convert::Infallible;
    type WindowError = std::convert::Infallible;
    fn transform(&mut self, frame: &mut [f32]) -> Result<()> { rdft_forward(frame) }
    fn magnitude(&mut self, bin: usize, real: f32, imaginary: f32) -> std::result::Result<f32, Self::MagnitudeError> {
        crate::audio_spectrum::magnitude_math::host_magnitude(bin, real, imaginary)
    }
    fn window(&mut self, index: usize, sample: f32, coefficient: f32) -> std::result::Result<f32, Self::WindowError> {
        crate::audio_spectrum::magnitude_math::host_window(index, sample, coefficient)
    }
}
impl FrameFft for SourceFftMath {
    type Error = SourceFftFailure;
    type MagnitudeError = std::convert::Infallible;
    type WindowError = std::convert::Infallible;
    fn transform(&mut self, frame: &mut [f32]) -> std::result::Result<(), Self::Error> {
        self.transform_in_place(SourceFftDirection::ForwardPlus1, frame)
    }
    fn magnitude(&mut self, bin: usize, real: f32, imaginary: f32) -> std::result::Result<f32, Self::MagnitudeError> {
        crate::audio_spectrum::magnitude_math::host_magnitude(bin, real, imaginary)
    }
    fn window(&mut self, index: usize, sample: f32, coefficient: f32) -> std::result::Result<f32, Self::WindowError> {
        crate::audio_spectrum::magnitude_math::host_window(index, sample, coefficient)
    }
}
impl FrameFft for SourceSpectrumMath {
    type Error = SourceSpectrumMathCause;
    type MagnitudeError = SourceSpectrumMathCause;
    type WindowError = SourceSpectrumMathCause;
    fn transform(&mut self, frame: &mut [f32]) -> std::result::Result<(), Self::Error> {
        self.transform_forward(frame)
    }
    fn magnitude(&mut self, bin: usize, real: f32, imaginary: f32) -> std::result::Result<f32, Self::MagnitudeError> {
        SourceSpectrumMath::magnitude(self, bin, real, imaginary)
    }
    fn window(&mut self, index: usize, sample: f32, coefficient: f32) -> std::result::Result<f32, Self::WindowError> {
        SourceSpectrumMath::window(self, index, sample, coefficient)
    }
}
enum FrameCause<E, F, G, H> { Source(String), Fft(F), Magnitude(G), Window(H), Postlog(E) }
struct FrameFailure<E, F, G, H> {
    cause: FrameCause<E, F, G, H>, frame: Vec<f32>, output: Vec<f32>, bin: Option<usize>,
    window_sample_index: Option<usize>,
}
struct FinalFailure<E, F, G, H> {
    cause: FrameCause<E, F, G, H>, spectra: [Vec<f32>; 3], current: Option<CurrentPitchSpectrumFrame>,
}
pub struct CurrentPitchSpectrumFrame {
    window_index: usize, row: usize, bin: Option<usize>, frame: Vec<f32>, output: Vec<f32>,
    window_sample_index: Option<usize>,
}
impl CurrentPitchSpectrumFrame {
    pub fn window_index(&self) -> usize { self.window_index }
    pub fn row(&self) -> usize { self.row }
    pub fn bin(&self) -> Option<usize> { self.bin }

    pub fn window_sample_index(&self) -> Option<usize> { self.window_sample_index }

    pub fn frame(&self) -> &[f32] { &self.frame }
    pub fn output_prefix(&self) -> &[f32] { &self.output }
}
pub enum SourcePitchSpectrumCause { Source(String), Postlog(SourcePitchPostlogCause) }
pub struct SourcePitchSpectrum {
    spectra: [Vec<f32>; 3], math: SourcePitchPostlogMath,
}
impl SourcePitchSpectrum {
    pub fn spectra(&self) -> &[Vec<f32>; 3] { &self.spectra }
    pub fn math(&self) -> &SourcePitchPostlogMath { &self.math }
    pub fn into_parts(self) -> ([Vec<f32>; 3], SourcePitchPostlogMath) { (self.spectra, self.math) }
}

pub struct SourcePitchSpectrumFailure {
    math: SourcePitchPostlogMath, cause: SourcePitchSpectrumCause,
    spectra: [Vec<f32>; 3], current: Option<CurrentPitchSpectrumFrame>,
}
impl SourcePitchSpectrumFailure {
    pub fn math(&self) -> &SourcePitchPostlogMath { &self.math }
    pub fn cause(&self) -> &SourcePitchSpectrumCause { &self.cause }
    pub fn completed_spectra(&self) -> &[Vec<f32>; 3] { &self.spectra }
    pub fn current_frame(&self) -> Option<&CurrentPitchSpectrumFrame> { self.current.as_ref() }
}

pub enum SourceFftPitchSpectrumCause {
    Source(String), Fft(SourceFftFailure), Postlog(SourcePitchPostlogCause),
}
pub struct SourceFftPitchSpectrum {
    spectra: [Vec<f32>; 3], fft: SourceFftMath, postlog: SourcePitchPostlogMath,
}
impl SourceFftPitchSpectrum {
    pub fn spectra(&self) -> &[Vec<f32>; 3] { &self.spectra }
    pub fn fft(&self) -> &SourceFftMath { &self.fft }
    pub fn postlog(&self) -> &SourcePitchPostlogMath { &self.postlog }
    pub fn into_parts(self) -> ([Vec<f32>; 3], SourceFftMath, SourcePitchPostlogMath) {
        (self.spectra, self.fft, self.postlog)
    }
}

pub struct SourceFftPitchSpectrumFailure {
    fft: SourceFftMath, postlog: SourcePitchPostlogMath, cause: SourceFftPitchSpectrumCause,
    spectra: [Vec<f32>; 3], current: Option<CurrentPitchSpectrumFrame>,
}
impl SourceFftPitchSpectrumFailure {
    pub fn fft(&self) -> &SourceFftMath { &self.fft }
    pub fn postlog(&self) -> &SourcePitchPostlogMath { &self.postlog }
    pub fn cause(&self) -> &SourceFftPitchSpectrumCause { &self.cause }
    pub fn completed_spectra(&self) -> &[Vec<f32>; 3] { &self.spectra }
    pub fn current_frame(&self) -> Option<&CurrentPitchSpectrumFrame> { self.current.as_ref() }
}

pub enum SourceMathPitchSpectrumCause {
    Source(String), Math(SourceSpectrumMathCause), Postlog(SourcePitchPostlogCause),
}
pub struct SourceMathPitchSpectrum {
    spectra: [Vec<f32>; 3], math: SourceSpectrumMath, postlog: SourcePitchPostlogMath,
}
impl SourceMathPitchSpectrum {
    pub fn spectra(&self) -> &[Vec<f32>; 3] { &self.spectra }
    pub fn math(&self) -> &SourceSpectrumMath { &self.math }
    pub fn postlog(&self) -> &SourcePitchPostlogMath { &self.postlog }
    pub fn into_parts(self) -> ([Vec<f32>; 3], SourceSpectrumMath, SourcePitchPostlogMath) {
        (self.spectra, self.math, self.postlog)
    }
}
pub struct SourceMathPitchSpectrumFailure {
    math: SourceSpectrumMath, postlog: SourcePitchPostlogMath, cause: SourceMathPitchSpectrumCause,
    spectra: [Vec<f32>; 3], current: Option<CurrentPitchSpectrumFrame>,
}
impl SourceMathPitchSpectrumFailure {
    pub fn math(&self) -> &SourceSpectrumMath { &self.math }
    pub fn postlog(&self) -> &SourcePitchPostlogMath { &self.postlog }
    pub fn cause(&self) -> &SourceMathPitchSpectrumCause { &self.cause }
    pub fn completed_spectra(&self) -> &[Vec<f32>; 3] { &self.spectra }
    pub fn current_frame(&self) -> Option<&CurrentPitchSpectrumFrame> { self.current.as_ref() }
}

pub struct PitchAudioSpectrum {
    windows: [Vec<f32>; 3],
    epsilon: f32,
}

impl PitchAudioSpectrum {
    pub fn stream_with_limit(&self, maximum_samples: usize) -> Result<PitchSpectrumStream<'_>> {
        PitchSpectrumStream::new(self, maximum_samples)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("pitch spectra require v2".into());
        }
        let root = tree.root;
        let owner = root.children.get(1).ok_or("pitch frontend absent")?;
        owner.require(
            0x7fca41ff,
            b"",
            &[560151423416826003 ^ 0xf22ee5adeedbda94 ^ 0xb2f45c29815fb08d],
        )?;
        if owner.children.len() != 14 || owner.data.len() != 76 || u32_at(owner.data, 12)? != 3 {
            return Err("pitch root config".into());
        }
        let epsilon = f32_at(owner.data, 0)?;
        if epsilon.to_bits() != 0x3a83126f {
            return Err("pitch log epsilon config".into());
        }
        let mut windows = Vec::new();
        for (i, n) in [512_usize, 1024, 2048].into_iter().enumerate() {
            let node = &owner.children[i];
            let tag = if i == 1 {
                18316645265828189916_u64
            } else {
                18267558758069216379_u64
            };
            if node.kind != 0x7fca40ff
                || node.tag != tag
                || node.children.len() != 1
                || node.data.len() != 32
            {
                return Err("pitch spectral node".into());
            }
            let config = [
                1,
                n as u32,
                240,
                n as u32,
                (n - 240) as u32,
                (-4097_i32) as u32,
                u32::MAX,
                0,
            ];
            for (j, x) in config.into_iter().enumerate() {
                if u32_at(node.data, j * 4)? != x {
                    return Err("pitch spectral config".into());
                }
            }
            let w = &node.children[0];
            w.require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if !w.children.is_empty()
                || w.data.len() != 4 + 4 * n
                || u32_at(w.data, 0)? as usize != n
            {
                return Err("pitch model window".into());
            }
            let window = (0..n)
                .map(|j| f32_at(w.data, 4 + 4 * j))
                .collect::<Result<Vec<_>>>()?;
            if window.iter().any(|v| !v.is_finite()) {
                return Err("nonfinite pitch window".into());
            }
            windows.push(window);
        }
        Ok(Self {
            windows: windows.try_into().map_err(|_| "pitch windows")?,
            epsilon,
        })
    }

    pub fn frame_magnitudes_512(&self, frame: Vec<f32>) -> Result<Vec<f32>> {
        self.process_frame_with_choices(0, frame, 0..512, &mut MagnitudesOnly, &mut HostFrameFft)
            .map_err(|failure| match failure.cause {
                FrameCause::Source(reason) | FrameCause::Fft(reason) => reason,
                FrameCause::Magnitude(never) => match never {},
                FrameCause::Window(never) => match never {},
                FrameCause::Postlog(never) => match never {},
            })
    }

    fn process_frame(
        &self,
        window_index: usize,
        frame: Vec<f32>,
        real_samples: std::ops::Range<usize>,
    ) -> Result<Vec<f32>> {
        self.process_frame_using(window_index, frame, real_samples, &mut HostPostlog)
            .map_err(|failure| match failure.cause {
                FrameCause::Source(reason) | FrameCause::Fft(reason) => reason,
                FrameCause::Magnitude(never) => match never {},
                FrameCause::Window(never) => match never {},
                FrameCause::Postlog(never) => match never {},
            })
    }

    fn process_frame_using<M: FramePostlog>(&self, window_index: usize, frame: Vec<f32>,
        real_samples: std::ops::Range<usize>, math: &mut M)
        -> std::result::Result<Vec<f32>, FrameFailure<M::Error, String, std::convert::Infallible, std::convert::Infallible>>
    {
        self.process_frame_with_choices(window_index, frame, real_samples, math, &mut HostFrameFft)
    }

    fn process_frame_with_choices<M: FramePostlog, F: FrameFft>(&self, window_index: usize, mut frame: Vec<f32>,
        real_samples: std::ops::Range<usize>, math: &mut M, fft: &mut F)
        -> std::result::Result<Vec<f32>, FrameFailure<M::Error, F::Error, F::MagnitudeError, F::WindowError>>
    {
        let Some(window) = self.windows.get(window_index) else {
            return Err(FrameFailure { cause: FrameCause::Source("pitch window index".into()),
                frame, output: Vec::new(), bin: None, window_sample_index: None });
        };
        let n = window.len();
        if frame.len() != n || real_samples.start > real_samples.end
            || real_samples.end > n || frame.iter().any(|v| !v.is_finite())
        {
            return Err(FrameFailure { cause: FrameCause::Source("invalid pitch spectral frame".into()),
                frame, output: Vec::new(), bin: None, window_sample_index: None });
        }
        for k in real_samples {
            match fft.window(k, frame[k], window[k]) {
                Ok(value) => frame[k] = value,
                Err(cause) => return Err(FrameFailure { cause: FrameCause::Window(cause),
                    frame, output: Vec::new(), bin: None, window_sample_index: Some(k) }),
            }
        }
        if let Err(cause) = fft.transform(&mut frame) {
            return Err(FrameFailure { cause: FrameCause::Fft(cause), frame, output: Vec::new(), bin: None, window_sample_index: None });
        }
        let mut output = Vec::with_capacity(n / 2 + 1);
        for k in 0..n / 2 + 1 {
            let magnitude = if k == 0 {
                frame[0].abs()
            } else if k == n / 2 {
                frame[1].abs()
            } else {
                match fft.magnitude(k, frame[2 * k], frame[2 * k + 1]) {
                    Ok(value) => value,
                    Err(cause) => return Err(FrameFailure { cause: FrameCause::Magnitude(cause),
                        frame, output, bin: Some(k), window_sample_index: None }),
                }
            };
            match math.apply(magnitude, self.epsilon) {
                Ok(value) => output.push(value),
                Err(cause) => return Err(FrameFailure { cause: FrameCause::Postlog(cause),
                    frame, output, bin: Some(k), window_sample_index: None }),
            }
        }
        Ok(output)
    }

    pub fn forward_final(&self, input: &[f32]) -> Result<[Vec<f32>; 3]> {
        self.forward_final_using(input, &mut HostPostlog).map_err(|failure| match failure.cause {
            FrameCause::Source(reason) | FrameCause::Fft(reason) => reason,
            FrameCause::Magnitude(never) => match never {},
            FrameCause::Window(never) => match never {},
            FrameCause::Postlog(never) => match never {},
        })
    }

    pub fn forward_final_with_source_postlog(&self, input: &[f32], mut math: SourcePitchPostlogMath)
        -> std::result::Result<SourcePitchSpectrum, SourcePitchSpectrumFailure>
    {
        match self.forward_final_using(input, &mut math) {
            Ok(spectra) => Ok(SourcePitchSpectrum { spectra, math }),
            Err(FinalFailure { cause, spectra, current }) => Err(SourcePitchSpectrumFailure {
                math, cause: match cause {
                    FrameCause::Source(reason) | FrameCause::Fft(reason) => SourcePitchSpectrumCause::Source(reason),
                    FrameCause::Magnitude(never) => match never {},
                    FrameCause::Window(never) => match never {},
                    FrameCause::Postlog(cause) => SourcePitchSpectrumCause::Postlog(cause),
                }, spectra, current }),
        }
    }

    pub fn forward_final_with_source_fft_and_postlog(&self, input: &[f32], mut fft: SourceFftMath,
        mut postlog: SourcePitchPostlogMath)
        -> std::result::Result<SourceFftPitchSpectrum, SourceFftPitchSpectrumFailure>
    {
        match self.forward_final_with_choices(input, &mut postlog, &mut fft) {
            Ok(spectra) => Ok(SourceFftPitchSpectrum { spectra, fft, postlog }),
            Err(FinalFailure { cause, spectra, current }) => Err(SourceFftPitchSpectrumFailure {
                fft, postlog, cause: match cause {
                    FrameCause::Source(reason) => SourceFftPitchSpectrumCause::Source(reason),
                    FrameCause::Fft(cause) => SourceFftPitchSpectrumCause::Fft(cause),
                    FrameCause::Magnitude(never) => match never {},
                    FrameCause::Window(never) => match never {},
                    FrameCause::Postlog(cause) => SourceFftPitchSpectrumCause::Postlog(cause),
                }, spectra, current }),
        }
    }

    pub fn forward_final_with_source_math_and_postlog(&self, input: &[f32], mut math: SourceSpectrumMath,
        mut postlog: SourcePitchPostlogMath)
        -> std::result::Result<SourceMathPitchSpectrum, SourceMathPitchSpectrumFailure>
    {
        match self.forward_final_with_choices(input, &mut postlog, &mut math) {
            Ok(spectra) => Ok(SourceMathPitchSpectrum { spectra, math, postlog }),
            Err(FinalFailure { cause, spectra, current }) => Err(SourceMathPitchSpectrumFailure {
                math, postlog, cause: match cause {
                    FrameCause::Source(reason) => SourceMathPitchSpectrumCause::Source(reason),
                    FrameCause::Fft(cause) | FrameCause::Magnitude(cause) | FrameCause::Window(cause) => SourceMathPitchSpectrumCause::Math(cause),
                    FrameCause::Postlog(cause) => SourceMathPitchSpectrumCause::Postlog(cause),
                }, spectra, current }),
        }
    }

    fn forward_final_using<M: FramePostlog>(&self, input: &[f32], math: &mut M)
        -> std::result::Result<[Vec<f32>; 3], FinalFailure<M::Error, String, std::convert::Infallible, std::convert::Infallible>>
    {
        self.forward_final_with_choices(input, math, &mut HostFrameFft)
    }

    fn forward_final_with_choices<M: FramePostlog, F: FrameFft>(&self, input: &[f32], math: &mut M, fft: &mut F)
        -> std::result::Result<[Vec<f32>; 3], FinalFailure<M::Error, F::Error, F::MagnitudeError, F::WindowError>>
    {
        if input.is_empty() || input.iter().any(|v| !v.is_finite()) {
            return Err(FinalFailure { cause: FrameCause::Source("invalid pitch stream".into()),
                spectra: std::array::from_fn(|_| Vec::new()), current: None });
        }
        let mut spectra = std::array::from_fn(|_| Vec::new());
        for (i, window) in self.windows.iter().enumerate() {
            let n = window.len();
            let left = n - 240;
            let rows = (input.len() + left).div_ceil(240);
            let bins = n / 2 + 1;
            spectra[i].reserve(rows * bins);
            for row in 0..rows {
                let mut frame = vec![0.0_f32; n];
                let mut first = n;
                let mut end = 0;
                for k in 0..n {
                    let at = (row * 240 + k) as i64 - left as i64;
                    if at >= 0 && (at as usize) < input.len() {
                        frame[k] = input[at as usize];
                        first = first.min(k);
                        end = k + 1;
                    }
                }
                let range = if end == 0 { 0..0 } else { first..end };
                match self.process_frame_with_choices(i, frame, range, math, fft) {
                    Ok(output) => spectra[i].extend(output),
                    Err(FrameFailure { cause, frame, output, bin, window_sample_index }) => return Err(FinalFailure {
                        cause, spectra, current: Some(CurrentPitchSpectrumFrame {
                            window_index: i, row, bin, frame, output, window_sample_index }),
                    }),
                }
            }
        }
        Ok(spectra)
    }
}
