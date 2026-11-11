//! Real FFT adapted from Takuya Ooura's General Purpose FFT Package.
//! Copyright Takuya OOURA, 1996-2001. See ../licenses/Ooura.txt.
//! One retained source contains the existing forward and inverse bodies.
//! Inverse output is unscaled; the filter applies its separate 2/N factor.
#[path = "runtime/fft/native_fft.rs"]
mod source;
pub use source::{rdft_forward, rdft_inverse as rdft_inverse512, rdft_inverse_supported};
pub(crate) use source::cdft_in_place;
pub use source::{rdft_forward_with_twiddle, rdft_inverse_supported_with_twiddle,
    TwiddleMath, TwiddleRequest, TwiddleSite, TwiddleOperation, TwiddleScale, TwiddleFftError};

#[path = "fft_source_math.rs"]
pub(crate) mod source_math;
pub use source_math::{SourceFftMath, SourceFftMathBindingFailure, SourceFftMathBindingCause,
    SourceFftDirection, TransformAdmissionError, TwiddleStage, SourceFftMathFailure,
    SourceFftFailure, SourceTwiddleCompletion};
