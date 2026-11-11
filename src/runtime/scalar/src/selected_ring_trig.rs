use super::finite_power::{ExplicitExceptionState, MathErrorCallbackCompletion};
use super::selected_coefficient_trig::{DIRECT_LIMIT,
    SIN_COEFFICIENTS, COS_COEFFICIENTS, tiny_sine_source_status};
use super::separate_reduction::{reduce, SeparateReductionError};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::vorbis_power::{Arithmetic, VorbisScalarError};

const ONE: u64 = 0x3ff0_0000_0000_0000;
const HALF: u64 = 0x3fe0_0000_0000_0000;
const THIRD_FACTORIAL: u64 = 0x3fc5_5555_5555_5555;

pub struct SourceFiniteCosineWordZero { large: SourceLargeCosineWordZero }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFiniteCosineWordZeroError {
    NonFiniteInput { bits: u32 },
    Short(SourceRingTrigError),
    Large(SourceLargeCosineWordZeroError),
}
impl SourceFiniteCosineWordZeroError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NonFiniteInput { .. } => "CPU0 finite cosine requires non-CRT finite F32 input",
            Self::Short(error) => error.reason(),
            Self::Large(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::NonFiniteInput { .. } => entry,
            Self::Short(error) => error.known_advanced_state(entry),
            Self::Large(error) => error.known_advanced_state(entry),
        }
    }
}
impl SourceFiniteCosineWordZero {
    pub(super) fn install_source_sine_state(&mut self, advanced: ExplicitExceptionState) { self.large.leaf.state = advanced; }
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceFiniteCosineWordZeroError> {
        Ok(Self { large: SourceLargeCosineWordZero::bind(word, architecture, state)
            .map_err(SourceFiniteCosineWordZeroError::Large)? })
    }
    pub fn name(&self) -> &'static str { "source57e330/word0/finite/separate" }
    pub fn explicit_source_word(&self) -> u32 { self.large.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.large.architecture() }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.large.exception_state() }
    pub fn cosine_f32(&mut self, value: f32) -> Result<f32, SourceFiniteCosineWordZeroError> {
        self.cosine_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn cosine_bits(&mut self, bits: u32) -> Result<u32, SourceFiniteCosineWordZeroError> {
        use self::SourceFiniteCosineWordZeroError as E;
        if bits & 0x7fff_ffff >= 0x7f80_0000 { return Err(E::NonFiniteInput { bits }); }
        let entry = self.exception_state();

        let (wide, _) = entry.widen_f32_for_math_error(bits);
        let result = if wide & 0x7fff_ffff_ffff_ffff < 0x411e_8480_0000_0000 {
            self.large.leaf.graph(bits, false).map_err(E::Short)
        } else { self.large.graph(bits).map_err(E::Large) };
        match result {
            Ok((bits, advanced)) => { self.large.leaf.state = advanced; Ok(bits) }
            Err(error) => { self.large.leaf.state = error.known_advanced_state(entry); Err(error) }
        }
    }
}

pub(super) fn finite_sine_word_zero_graph(leaf: &SourceFiniteCosineWordZero, bits: u32)
    -> Result<(u32, ExplicitExceptionState), super::selected_finite_trig_word_zero::SourceFiniteTrigWordZeroError> {
    use super::selected_finite_trig_word_zero::SourceFiniteTrigWordZeroError as E;
    if bits & 0x7fff_ffff >= 0x7f80_0000 { return Err(E::NonFiniteInput { bits }); }
    let entry = leaf.exception_state();
    let (wide, advanced) = entry.widen_f32_for_math_error(bits);
    let magnitude = wide & 0x7fff_ffff_ffff_ffff;
    if magnitude < 0x416e_8480_0000_0000 {
        return leaf.large.leaf.graph(bits, true).map_err(E::Short);
    }
    let mut a = Arithmetic { state: advanced };

    let absolute = if bits >> 31 != 0 {
        a.subtract("587011/large sine absolute zero SUBSD", 0, wide)
            .map_err(|error| E::LargeSineKernel(SourceRingTrigError::Arithmetic(error)))?
    } else { magnitude };
    let (quadrant, head, advanced) = super::selected_phase::cpu0_sine_large_head_with_explicit_state(absolute, a.state)
        .map_err(E::LargeSine)?;
    a.state = advanced;
    let mut result = separate_kernel(&mut a, head, quadrant & 1 == 0)
        .map_err(|error| E::LargeSineKernel(SourceRingTrigError::Arithmetic(error)))?;

    if (quadrant & 2 != 0) ^ (bits >> 31 != 0) {
        result = a.subtract("587252/large sine quadrant zero SUBSD", 0, result)
            .map_err(|error| E::LargeSineKernel(SourceRingTrigError::Arithmetic(error)))?;
    }
    narrow_source(result, a.state).map_err(E::LargeSineKernel)
}

pub struct SourceLargeCosineWordZero { leaf: SourceRingTrigWordZero }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLargeCosineWordZeroError {
    OutsideLargeDomain { bits: u32 },
    Source(SourceRingTrigError),
    Reduction(super::selected_phase::PhaseError),
}
impl SourceLargeCosineWordZeroError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::OutsideLargeDomain { .. } => "CPU0 large cosine requires finite magnitude reaching actual602810 reduction threshold",
            Self::Source(error) => error.reason(),
            Self::Reduction(super::selected_phase::PhaseError::ConsumedUndefinedBitScan) => "CPU0 cosine head consumed undefined zero BSR result",
            Self::Reduction(_) => "CPU0 cosine head arithmetic/source continuation required",
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::Source(error) => error.known_advanced_state(entry),
            Self::Reduction(super::selected_phase::PhaseError::NormalArithmeticRequired { advanced, .. }) => advanced,
            _ => entry,
        }
    }
}
impl SourceLargeCosineWordZero {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceLargeCosineWordZeroError> {
        Ok(Self { leaf: SourceRingTrigWordZero::bind(word, architecture, state)
            .map_err(SourceLargeCosineWordZeroError::Source)? })
    }
    pub fn name(&self) -> &'static str { "source57e330/word0/large597b40/separate" }
    pub fn explicit_source_word(&self) -> u32 { self.leaf.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.leaf.architecture() }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.leaf.state }
    pub fn cosine_f32(&mut self, value: f32) -> Result<f32, SourceLargeCosineWordZeroError> {
        self.cosine_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn cosine_bits(&mut self, bits: u32) -> Result<u32, SourceLargeCosineWordZeroError> {
        let entry = self.leaf.state;
        match self.graph(bits) {
            Ok((bits, advanced)) => { self.leaf.state = advanced; Ok(bits) }
            Err(error) => { self.leaf.state = error.known_advanced_state(entry); Err(error) }
        }
    }
    fn graph(&self, bits: u32) -> Result<(u32, ExplicitExceptionState), SourceLargeCosineWordZeroError> {
        use self::SourceLargeCosineWordZeroError as E;
        if bits & 0x7fff_ffff >= 0x7f80_0000 { return Err(E::OutsideLargeDomain { bits }); }
        let (wide, advanced) = self.leaf.state.widen_f32_for_math_error(bits);
        let magnitude = wide & 0x7fff_ffff_ffff_ffff;
        if magnitude < 0x411e_8480_0000_0000 { return Err(E::OutsideLargeDomain { bits }); }

        let (quadrant, head, advanced) = super::selected_phase::cpu0_cosine_large_head_with_explicit_state(magnitude, advanced)
            .map_err(E::Reduction)?;
        let mut a = Arithmetic { state: advanced };

        let mut result = separate_kernel(&mut a, head, quadrant & 1 != 0)
            .map_err(|error| E::Source(SourceRingTrigError::Arithmetic(error)))?;
        if (quadrant + 1) & 2 != 0 {
            result = a.subtract("57e5e0/CPU0 large cosine zero SUBSD", 0, result)
                .map_err(|error| E::Source(SourceRingTrigError::Arithmetic(error)))?;
        }
        narrow_source(result, a.state).map_err(E::Source)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceRingTrigError {
    WordZeroRequired { supplied: u32 },
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    OutsideRingDomain { bits: u32 },
    OutsideFloor0CosineDomain { bits: u32 },
    OutsideProjectionDomain { bits: u32 },
    CountContinuationRequired { scaled_bits: u64, advanced: ExplicitExceptionState },
    Arithmetic(VorbisScalarError),
}

impl SourceRingTrigError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::WordZeroRequired { .. } => "57e330/586ed0 ring SSE body requires explicitly supplied word zero",
            Self::ArchitectureNotClosed => "ring arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "ring source state is not RNE/all-masked/gradual",
            Self::OutsideRingDomain { .. } => "ring trig input requires +zero or positive normal [1/8,8)",
            Self::OutsideFloor0CosineDomain { .. } => "word-zero floor0 cosine requires finite F32 magnitude below8, including signed zeros/subnormals",
            Self::OutsideProjectionDomain { .. } => "word-zero projection trig requires signed zero or normal magnitude [2^-23,8)",
            Self::CountContinuationRequired { .. } => "ring bounded CVTTPD2DQ continuation required",
            Self::Arithmetic(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::CountContinuationRequired { advanced, .. } => advanced,
            Self::Arithmetic(error) => error.known_advanced_state(entry),
            _ => entry,
        }
    }
}

pub struct SourceProjectionTrigWordZero {
    leaf: SourceRingTrigWordZero,
}
impl SourceProjectionTrigWordZero {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceRingTrigError>
    {
        Ok(Self { leaf: SourceRingTrigWordZero::bind(word, architecture, state)? })
    }
    pub fn name(&self) -> &'static str { "source57e330+586ed0/word0/projection/separate" }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.leaf.state }
    pub fn explicit_source_word(&self) -> u32 { self.leaf.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.leaf.architecture() }
    pub fn sinf(&mut self, value: f32) -> Result<f32, SourceRingTrigError> {
        self.sinf_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn cosf(&mut self, value: f32) -> Result<f32, SourceRingTrigError> {
        self.cosf_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn sinf_bits(&mut self, bits: u32) -> Result<u32, SourceRingTrigError> { self.evaluate(bits, true) }
    pub fn cosf_bits(&mut self, bits: u32) -> Result<u32, SourceRingTrigError> { self.evaluate(bits, false) }
    fn evaluate(&mut self, bits: u32, sine: bool) -> Result<u32, SourceRingTrigError> {
        let magnitude = bits & 0x7fff_ffff;
        if magnitude != 0 && !(0x3400_0000..0x4100_0000).contains(&magnitude) {
            return Err(SourceRingTrigError::OutsideProjectionDomain { bits });
        }
        let entry = self.leaf.state;
        match self.leaf.graph(bits, sine) {
            Ok((bits, advanced)) => { self.leaf.state = advanced; Ok(bits) }
            Err(error) => { self.leaf.state = error.known_advanced_state(entry); Err(error) }
        }
    }
}

pub struct SourceFloor0CosineWordZero {
    leaf: SourceRingTrigWordZero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCoefficientTrigWordZeroShortError {
    OutsideShortDomain { bits: u32 },
    Source(SourceRingTrigError),
}
impl SourceCoefficientTrigWordZeroShortError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::OutsideShortDomain { .. } => "word-zero coefficient trig requires finite F32 magnitude below8, including signed zeros/subnormals",
            Self::Source(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self { Self::Source(error) => error.known_advanced_state(entry), _ => entry }
    }
}

pub struct SourceCoefficientTrigWordZeroShort { leaf: SourceRingTrigWordZero }
impl SourceCoefficientTrigWordZeroShort {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceCoefficientTrigWordZeroShortError>
    {
        Ok(Self { leaf: SourceRingTrigWordZero::bind(word, architecture, state)
            .map_err(SourceCoefficientTrigWordZeroShortError::Source)? })
    }
    pub fn name(&self) -> &'static str { "source57e330+586ed0/word0/coefficient-finite-short/separate" }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.leaf.exception_state() }
    pub fn explicit_source_word(&self) -> u32 { self.leaf.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.leaf.architecture() }
    pub fn sinf(&mut self, value: f32) -> Result<f32, SourceCoefficientTrigWordZeroShortError> {
        self.sinf_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn cosf(&mut self, value: f32) -> Result<f32, SourceCoefficientTrigWordZeroShortError> {
        self.cosf_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn sinf_bits(&mut self, bits: u32) -> Result<u32, SourceCoefficientTrigWordZeroShortError> { self.evaluate(bits, true) }
    pub fn cosf_bits(&mut self, bits: u32) -> Result<u32, SourceCoefficientTrigWordZeroShortError> { self.evaluate(bits, false) }
    fn evaluate(&mut self, bits: u32, sine: bool) -> Result<u32, SourceCoefficientTrigWordZeroShortError> {
        if bits & 0x7fff_ffff >= 0x4100_0000 { return Err(SourceCoefficientTrigWordZeroShortError::OutsideShortDomain { bits }); }
        let entry = self.leaf.state;
        match self.leaf.graph(bits, sine) {
            Ok((bits, advanced)) => { self.leaf.state = advanced; Ok(bits) }
            Err(error) => {
                self.leaf.state = error.known_advanced_state(entry);
                Err(SourceCoefficientTrigWordZeroShortError::Source(error))
            }
        }
    }
}
impl SourceFloor0CosineWordZero {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceRingTrigError>
    {
        Ok(Self { leaf: SourceRingTrigWordZero::bind(word, architecture, state)? })
    }
    pub fn name(&self) -> &'static str { "source57e330/word0/floor0-finite-short/separate" }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.leaf.exception_state() }
    pub fn explicit_source_word(&self) -> u32 { self.leaf.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.leaf.architecture() }
    pub fn cosine_f32(&mut self, value: f32) -> Result<f32, SourceRingTrigError> {
        self.cosine_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn cosine_bits(&mut self, bits: u32) -> Result<u32, SourceRingTrigError> {
        if bits & 0x7fff_ffff >= 0x4100_0000 {
            return Err(SourceRingTrigError::OutsideFloor0CosineDomain { bits });
        }

        let entry = self.leaf.state;
        match self.leaf.graph(bits, false) {
            Ok((bits, advanced)) => { self.leaf.state = advanced; Ok(bits) }
            Err(error) => { self.leaf.state = error.known_advanced_state(entry); Err(error) }
        }
    }
}

pub struct SourceRingTrigWordZero {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
}

impl SourceRingTrigWordZero {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceRingTrigError>
    {
        if word != 0 { return Err(SourceRingTrigError::WordZeroRequired { supplied: word }); }
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceRingTrigError::ArchitectureNotClosed); }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceRingTrigError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { architecture, state })
    }
    pub fn name(&self) -> &'static str { "source57e330+586ed0/word0/ring/separate" }
    pub fn explicit_source_word(&self) -> u32 { 0 }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn sinf(&mut self, value: f32) -> Result<f32, SourceRingTrigError> {
        self.sinf_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn cosf(&mut self, value: f32) -> Result<f32, SourceRingTrigError> {
        self.cosf_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn sinf_bits(&mut self, bits: u32) -> Result<u32, SourceRingTrigError> { self.evaluate(bits, true) }
    pub fn cosf_bits(&mut self, bits: u32) -> Result<u32, SourceRingTrigError> { self.evaluate(bits, false) }

    fn evaluate(&mut self, bits: u32, sine: bool) -> Result<u32, SourceRingTrigError> {
        if bits != 0 && !(0x3e00_0000..0x4100_0000).contains(&bits) {
            return Err(SourceRingTrigError::OutsideRingDomain { bits });
        }
        let entry = self.state;
        match self.graph(bits, sine) {
            Ok((bits, advanced)) => { self.state = advanced; Ok(bits) }
            Err(error) => { self.state = error.known_advanced_state(entry); Err(error) }
        }
    }

    fn graph(&self, bits: u32, sine: bool) -> Result<(u32, ExplicitExceptionState), SourceRingTrigError> {
        let (wide, state) = self.state.widen_f32_for_math_error(bits);
        let mut a = Arithmetic { state };
        let magnitude = wide & 0x7fff_ffff_ffff_ffff;
        if sine && magnitude < 0x3f20_0000_0000_0000 {

            return Ok((bits, a.state.with_source_status(tiny_sine_source_status(bits))));
        }
        if sine && magnitude < 0x3f80_0000_0000_0000 {

            let square = a.multiply("586f7c/small sine square MULSD", wide, wide)
                .map_err(SourceRingTrigError::Arithmetic)?;
            let cube = a.multiply("586f80/small sine cube MULSD", square, wide)
                .map_err(SourceRingTrigError::Arithmetic)?;
            let correction = a.multiply("586f84/small sine correction MULSD", cube, THIRD_FACTORIAL)
                .map_err(SourceRingTrigError::Arithmetic)?;
            let result = a.subtract("586f88/small sine result SUBSD", wide, correction)
                .map_err(SourceRingTrigError::Arithmetic)?;
            return narrow_source(result, a.state);
        }
        let mut head = wide;
        let mut count = 0u32;
        let mut short_exponent = None;
        if magnitude <= DIRECT_LIMIT {
            if !sine {

                let _ = a.multiply("57e38b/discarded direct cosine square", wide, wide)
                    .map_err(SourceRingTrigError::Arithmetic)?;
            }
        } else {

            let absolute = if sine && bits >> 31 != 0 {
                a.subtract("587011/negative sine absolute zero SUBSD", 0, wide)
                    .map_err(SourceRingTrigError::Arithmetic)?
            } else { magnitude };
            let reduced = reduce(&mut a, absolute, false).map_err(|error| match error {
                SeparateReductionError::Count { scaled_bits, advanced } => SourceRingTrigError::CountContinuationRequired { scaled_bits, advanced },
                SeparateReductionError::Arithmetic(error) => SourceRingTrigError::Arithmetic(error),
            })?;
            count = reduced.count;
            head = reduced.head;
            short_exponent = Some(reduced.first_exponent);
        }
        let sine_kernel = if sine { count & 1 == 0 } else { count & 1 != 0 };
        let mut result = if let Some(exponent) = short_exponent.filter(|&exponent| exponent < 0x3f2) {
            if exponent <= 0x3de {
                if sine_kernel { head } else { ONE }
            } else {
                let square = a.multiply("ring SSE/small reduced square MULSD", head, head)
                    .map_err(SourceRingTrigError::Arithmetic)?;
                if sine_kernel {

                    let correction = a.multiply("ring SSE/small reduced coefficient MULSD", THIRD_FACTORIAL, head)
                        .map_err(SourceRingTrigError::Arithmetic)?;
                    let correction = a.multiply("ring SSE/small reduced cube MULSD", correction, square)
                        .map_err(SourceRingTrigError::Arithmetic)?;
                    a.subtract("ring SSE/small reduced sine SUBSD", head, correction)
                        .map_err(SourceRingTrigError::Arithmetic)?
                } else {
                    let half_square = a.multiply("ring SSE/small reduced half square MULSD", square, HALF)
                        .map_err(SourceRingTrigError::Arithmetic)?;
                    a.subtract("ring SSE/small reduced cosine SUBSD", ONE, half_square)
                        .map_err(SourceRingTrigError::Arithmetic)?
                }
            }
        } else { separate_kernel(&mut a, head, sine_kernel).map_err(SourceRingTrigError::Arithmetic)? };
        let negative = if sine {

            (count & 2 != 0) ^ (short_exponent.is_some() && bits >> 31 != 0)
        } else { (count + 1) & 2 != 0 };
        if negative {

            result = a.subtract("ring SSE/quadrant zero SUBSD", 0, result)
                .map_err(SourceRingTrigError::Arithmetic)?;
        }
        narrow_source(result, a.state)
    }
}

fn narrow_source(bits: u64, advanced: ExplicitExceptionState) -> Result<(u32, ExplicitExceptionState), SourceRingTrigError> {
    let narrowed = MathErrorCallbackCompletion { result_f64_bits: bits, advanced }
        .into_rne_f32().map_err(|required| SourceRingTrigError::Arithmetic(VorbisScalarError::ConversionRequired(required)))?;
    Ok((narrowed.bits, narrowed.advanced))
}

fn separate_kernel(a: &mut Arithmetic, head: u64, sine: bool) -> Result<u64, VorbisScalarError> {
    let square = a.multiply("ring SSE/kernel square MULSD", head, head)?;
    if sine {
        let p = a.multiply("ring SSE/sine seventh MULSD", SIN_COEFFICIENTS[3].to_bits(), square)?;
        let fourth = a.multiply("ring SSE/sine fourth MULSD", square, square)?;
        let q = a.multiply("ring SSE/sine cubic MULSD", SIN_COEFFICIENTS[1].to_bits(), square)?;
        let cube = a.multiply("ring SSE/sine cube MULSD", head, square)?;
        let p = a.add("ring SSE/sine fifth ADDSD", p, SIN_COEFFICIENTS[2].to_bits())?;
        let p = a.multiply("ring SSE/sine high pair MULSD", p, fourth)?;
        let q = a.add("ring SSE/sine first ADDSD", q, SIN_COEFFICIENTS[0].to_bits())?;
        let p = a.add("ring SSE/sine pairs ADDSD", p, q)?;
        let p = a.multiply("ring SSE/sine correction MULSD", p, cube)?;
        a.add("ring SSE/sine result ADDSD", head, p)
    } else {
        let p = a.multiply("ring SSE/cosine sixth MULSD", COS_COEFFICIENTS[3].to_bits(), square)?;
        let fourth = a.multiply("ring SSE/cosine fourth MULSD", square, square)?;
        let q = a.multiply("ring SSE/cosine second MULSD", COS_COEFFICIENTS[1].to_bits(), square)?;
        let leading = a.multiply("ring SSE/cosine negative half square MULSD", HALF | 0x8000_0000_0000_0000, square)?;
        let p = a.add("ring SSE/cosine fourth ADDSD", p, COS_COEFFICIENTS[2].to_bits())?;
        let p = a.multiply("ring SSE/cosine high pair MULSD", p, fourth)?;
        let q = a.add("ring SSE/cosine constant ADDSD", q, COS_COEFFICIENTS[0].to_bits())?;
        let p = a.add("ring SSE/cosine pairs ADDSD", p, q)?;
        let p = a.multiply("ring SSE/cosine correction MULSD", p, fourth)?;
        let leading = a.add("ring SSE/cosine leading ADDSD", leading, ONE)?;
        a.add("ring SSE/cosine result ADDSD", leading, p)
    }
}
