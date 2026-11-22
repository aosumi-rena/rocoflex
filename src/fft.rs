//! FFT adapted from Takuya Ooura. Copyright Takuya OOURA, 1996-2001.
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
