use super::finite_power::ExplicitExceptionState;
use super::selected_phase::{SIN_COEFFICIENTS, COS_COEFFICIENTS};
use super::selected_coefficient_trig::DIRECT_LIMIT;
use super::separate_reduction::{reduce, SeparateReductionError};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::vorbis_power::{Arithmetic, ArithmeticContext, VorbisScalarError};

const ONE: u64 = 0x3ff0_0000_0000_0000;
const HALF: u64 = 0x3fe0_0000_0000_0000;
const NEGATIVE_HALF: u64 = 0xbfe0_0000_0000_0000;
const SMALL_COSINE: u64 = 0x3f20_0000_0000_0000;
const TWO32: u64 = 0x41f0_0000_0000_0000;
const PI: u64 = 0x4009_21fb_5444_2d18;

pub struct SourcePhaseWordZeroLarge { leaf: SourcePhaseWordZeroMdct }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLargePhaseWordZeroError {
    OutsideLargeDomain { bits: u64 },
    Binding(SourceMdctPhaseWordZeroError),
    Integer(super::selected_phase::PhaseError),
    ReductionArithmetic(VorbisScalarError),
    Kernel(SourceMdctPhaseWordZeroError),
}
impl SourceLargePhaseWordZeroError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::OutsideLargeDomain { .. } => "CPU0 large F64 phase requires finite magnitude reaching actual411e848000000000 threshold",
            Self::Binding(error) | Self::Kernel(error) => error.reason(),
            Self::Integer(_) => "CPU0 large F64 integer head/tail continuation required, including consumed zero BSR",
            Self::ReductionArithmetic(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::OutsideLargeDomain { .. } => entry,
            Self::Binding(error) | Self::Kernel(error) => error.known_advanced_state(entry),
            Self::Integer(super::selected_phase::PhaseError::NormalArithmeticRequired { advanced, .. }) => advanced,
            Self::Integer(_) => entry,
            Self::ReductionArithmetic(error) => error.known_advanced_state(entry),
        }
    }
}
impl SourcePhaseWordZeroLarge {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceLargePhaseWordZeroError> {
        Ok(Self { leaf: SourcePhaseWordZeroMdct::bind(word, architecture, state)
            .map_err(SourceLargePhaseWordZeroError::Binding)? })
    }
    pub fn name(&self) -> &'static str { "source57ddb0+586960/word0/large5973e0/separate" }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.leaf.state }
    pub fn explicit_source_word(&self) -> u32 { self.leaf.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.leaf.architecture() }
    pub fn sine_f64(&mut self, value: f64) -> Result<f64, SourceLargePhaseWordZeroError> {
        self.sine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn cosine_f64(&mut self, value: f64) -> Result<f64, SourceLargePhaseWordZeroError> {
        self.cosine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn sine_bits(&mut self, bits: u64) -> Result<u64, SourceLargePhaseWordZeroError> { self.evaluate(bits, true) }
    pub fn cosine_bits(&mut self, bits: u64) -> Result<u64, SourceLargePhaseWordZeroError> { self.evaluate(bits, false) }
    fn evaluate(&mut self, bits: u64, sine: bool) -> Result<u64, SourceLargePhaseWordZeroError> {
        let entry = self.leaf.state;
        let mut a = Arithmetic { state: entry };
        match self.evaluate_observed(bits, sine, &mut a) {
            Ok(bits) => { self.leaf.state = a.state; Ok(bits) }
            Err(error) => { self.leaf.state = error.known_advanced_state(entry); Err(error) }
        }
    }
    pub(super) fn evaluate_observed(&self, bits: u64, sine: bool, a: &mut impl ArithmeticContext)
        -> Result<u64, SourceLargePhaseWordZeroError> {
        let magnitude = bits & 0x7fff_ffff_ffff_ffff;
        if !(0x411e_8480_0000_0000..0x7ff0_0000_0000_0000).contains(&magnitude) {
            return Err(SourceLargePhaseWordZeroError::OutsideLargeDomain { bits });
        }
        let reduced = super::separate_large_reduction::reduce(a, magnitude);
        match reduced {
            Ok(reduced) => reduced_graph(a, bits, sine, reduced.quadrant, reduced.head, reduced.tail)
                .map_err(SourceLargePhaseWordZeroError::Kernel),
            Err(super::separate_large_reduction::SeparateLargeReductionError::Integer(error)) => Err(SourceLargePhaseWordZeroError::Integer(error)),
            Err(super::separate_large_reduction::SeparateLargeReductionError::Arithmetic(error)) => Err(SourceLargePhaseWordZeroError::ReductionArithmetic(error)),
        }
    }
}

pub struct SourcePhaseWordZeroModerate { leaf: SourcePhaseWordZeroMdct }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceModeratePhaseWordZeroError {
    OutsideModerateDomain { bits: u64 },
    Source(SourceMdctPhaseWordZeroError),
}
impl SourceModeratePhaseWordZeroError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::OutsideModerateDomain { .. } => "CPU0 F64 phase moderate entry requires magnitude[8,actual411e848000000000 threshold)",
            Self::Source(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self { Self::Source(error) => error.known_advanced_state(entry), _ => entry }
    }
}
impl SourcePhaseWordZeroModerate {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceModeratePhaseWordZeroError> {
        Ok(Self { leaf: SourcePhaseWordZeroMdct::bind(word, architecture, state)
            .map_err(SourceModeratePhaseWordZeroError::Source)? })
    }
    pub fn name(&self) -> &'static str { "source57ddb0+586960/word0/moderate/separate" }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.leaf.state }
    pub fn explicit_source_word(&self) -> u32 { self.leaf.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.leaf.architecture() }
    pub fn sine_f64(&mut self, value: f64) -> Result<f64, SourceModeratePhaseWordZeroError> {
        self.sine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn cosine_f64(&mut self, value: f64) -> Result<f64, SourceModeratePhaseWordZeroError> {
        self.cosine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn sine_bits(&mut self, bits: u64) -> Result<u64, SourceModeratePhaseWordZeroError> { self.evaluate(bits, true) }
    pub fn cosine_bits(&mut self, bits: u64) -> Result<u64, SourceModeratePhaseWordZeroError> { self.evaluate(bits, false) }
    fn evaluate(&mut self, bits: u64, sine: bool) -> Result<u64, SourceModeratePhaseWordZeroError> {
        let entry = self.leaf.state;
        let mut a = Arithmetic { state: entry };
        match self.evaluate_observed(bits, sine, &mut a) {
            Ok(bits) => { self.leaf.state = a.state; Ok(bits) }
            Err(error) => { self.leaf.state = error.known_advanced_state(entry); Err(error) }
        }
    }
    pub(super) fn evaluate_observed(&self, bits: u64, sine: bool, a: &mut impl ArithmeticContext)
        -> Result<u64, SourceModeratePhaseWordZeroError> {
        let magnitude = bits & 0x7fff_ffff_ffff_ffff;
        if !(0x4020_0000_0000_0000..0x411e_8480_0000_0000).contains(&magnitude) {
            return Err(SourceModeratePhaseWordZeroError::OutsideModerateDomain { bits });
        }
        graph(a, bits, sine).map_err(SourceModeratePhaseWordZeroError::Source)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceMdctPhaseWordZeroError {
    WordZeroRequired { supplied: u32 },
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    OutsideMdctAngleDomain { bits: u64 },
    OutsideShortAngleDomain { bits: u64 },
    CountContinuationRequired { scaled_bits: u64, advanced: ExplicitExceptionState },
    Arithmetic(VorbisScalarError),
}
impl SourceMdctPhaseWordZeroError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::WordZeroRequired { .. } => "57ddb0/586960 SSE phase requires explicitly supplied word zero",
            Self::ArchitectureNotClosed => "MDCT phase arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "MDCT phase state is not RNE/all-masked/gradual",
            Self::OutsideMdctAngleDomain { .. } => "word-zero phase requires +zero or positive F64 [2^-16,pi)",
            Self::OutsideShortAngleDomain { .. } => "word-zero short phase requires finite magnitude below8",
            Self::CountContinuationRequired { .. } => "word-zero phase bounded CVTTPD2DQ continuation required",
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

pub struct SourcePhaseWordZeroShort {
    leaf: SourcePhaseWordZeroMdct,
}
impl SourcePhaseWordZeroShort {
    pub(super) fn install_source_continuation_state(&mut self, state: ExplicitExceptionState) { self.leaf.state = state; }
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceMdctPhaseWordZeroError>
    {
        Ok(Self { leaf: SourcePhaseWordZeroMdct::bind(word, architecture, state)? })
    }
    pub fn name(&self) -> &'static str { "source57ddb0+586960/word0/signed-short/separate" }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.leaf.state }
    pub fn explicit_source_word(&self) -> u32 { self.leaf.explicit_source_word() }
    pub fn architecture(&self) -> ScalarF32Architecture { self.leaf.architecture() }
    pub fn sine_f64(&mut self, value: f64) -> Result<f64, SourceMdctPhaseWordZeroError> {
        self.sine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn cosine_f64(&mut self, value: f64) -> Result<f64, SourceMdctPhaseWordZeroError> {
        self.cosine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn sine_bits(&mut self, bits: u64) -> Result<u64, SourceMdctPhaseWordZeroError> { self.evaluate(bits, true) }
    pub fn cosine_bits(&mut self, bits: u64) -> Result<u64, SourceMdctPhaseWordZeroError> { self.evaluate(bits, false) }
    fn evaluate(&mut self, bits: u64, sine: bool) -> Result<u64, SourceMdctPhaseWordZeroError> {
        let entry = self.leaf.state;
        let mut a = Arithmetic { state: entry };
        match self.evaluate_observed(bits, sine, &mut a) {
            Ok(bits) => { self.leaf.state = a.state; Ok(bits) }
            Err(error) => { self.leaf.state = error.known_advanced_state(entry); Err(error) }
        }
    }
    pub(super) fn evaluate_observed(&self, bits: u64, sine: bool, a: &mut impl ArithmeticContext)
        -> Result<u64, SourceMdctPhaseWordZeroError> {
        let magnitude = bits & 0x7fff_ffff_ffff_ffff;
        if magnitude == 0 { return Ok(if sine { bits } else { ONE }); }
        if magnitude >= 0x4020_0000_0000_0000 {
            return Err(SourceMdctPhaseWordZeroError::OutsideShortAngleDomain { bits });
        }
        if sine && magnitude <= 0x3e40_0000_0000_0000 {

            let flags = if magnitude < 0x0010_0000_0000_0000 { 0x32 }
                else if magnitude == 0x0010_0000_0000_0000 { 0x30 } else { 0x20 };
            a.record_completed(flags);
            return Ok(bits);
        }
        if !sine && magnitude < 0x3e40_0000_0000_0000 {

            let flags = if magnitude < 0x0010_0000_0000_0000 { 0x22 } else { 0x20 };
            a.record_completed(flags);
            return Ok(ONE);
        }

        graph(a, bits, sine)
    }
}

pub struct SourcePhaseWordZeroMdct {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
}
impl SourcePhaseWordZeroMdct {
    pub fn bind(word: u32, architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceMdctPhaseWordZeroError>
    {
        if word != 0 { return Err(SourceMdctPhaseWordZeroError::WordZeroRequired { supplied: word }); }
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceMdctPhaseWordZeroError::ArchitectureNotClosed); }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceMdctPhaseWordZeroError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { architecture, state })
    }
    pub fn name(&self) -> &'static str { "source57ddb0+586960/word0/mdct/separate" }
    pub fn explicit_source_word(&self) -> u32 { 0 }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn sine_f64(&mut self, value: f64) -> Result<f64, SourceMdctPhaseWordZeroError> {
        self.sine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn cosine_f64(&mut self, value: f64) -> Result<f64, SourceMdctPhaseWordZeroError> {
        self.cosine_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn sine_bits(&mut self, bits: u64) -> Result<u64, SourceMdctPhaseWordZeroError> { self.evaluate(bits, true) }
    pub fn cosine_bits(&mut self, bits: u64) -> Result<u64, SourceMdctPhaseWordZeroError> { self.evaluate(bits, false) }
    fn evaluate(&mut self, bits: u64, sine: bool) -> Result<u64, SourceMdctPhaseWordZeroError> {
        if bits != 0 && !(0x3ef0_0000_0000_0000..PI).contains(&bits) {
            return Err(SourceMdctPhaseWordZeroError::OutsideMdctAngleDomain { bits });
        }
        if bits == 0 { return Ok(if sine { 0 } else { ONE }); }
        let mut a = Arithmetic { state: self.state };
        let result = graph(&mut a, bits, sine);
        match result {
            Ok(bits) => { self.state = a.state; Ok(bits) }
            Err(error) => { self.state = error.known_advanced_state(self.state); Err(error) }
        }
    }
}

fn graph(a: &mut impl ArithmeticContext, bits: u64, sine: bool) -> Result<u64, SourceMdctPhaseWordZeroError> {
    use self::SourceMdctPhaseWordZeroError::Arithmetic as E;
    let magnitude = bits & 0x7fff_ffff_ffff_ffff;

    if magnitude < DIRECT_LIMIT {
        if !sine && magnitude < SMALL_COSINE {
            let _ = a.add("57e0c4/discarded small cosine ADDSD", bits, TWO32).map_err(E)?;
            let square = a.multiply("57e0e1/small cosine square MULSD", bits, bits).map_err(E)?;
            let half = a.multiply("57e0e5/small cosine half MULSD", square, HALF).map_err(E)?;
            return a.subtract("57e0ed/small cosine SUBSD", ONE, half).map_err(E);
        }
        let square = a.multiply("word0 F64/direct square MULSD", bits, bits).map_err(E)?;
        return if sine { sine_kernel(a, bits, square, None).map_err(E) }
            else { direct_cosine(a, square).map_err(E) };
    }
    let reduced = reduce(a, magnitude, true).map_err(|error| match error {
        SeparateReductionError::Count { scaled_bits, advanced } => SourceMdctPhaseWordZeroError::CountContinuationRequired { scaled_bits, advanced },
        SeparateReductionError::Arithmetic(error) => E(error),
    })?;
    reduced_graph(a, bits, sine, reduced.count, reduced.head, reduced.tail)
}

fn reduced_graph(a: &mut impl ArithmeticContext, bits: u64, sine: bool, count: u32, head: u64, tail: u64)
    -> Result<u64, SourceMdctPhaseWordZeroError> {
    use self::SourceMdctPhaseWordZeroError::Arithmetic as E;
    let square = a.multiply("word0 F64/reduced square MULSD", head, head).map_err(E)?;
    let sine_kernel_selected = if sine { count & 1 == 0 } else { count & 1 != 0 };
    let result = if sine_kernel_selected {
        sine_kernel(a, head, square, Some(tail)).map_err(E)?
    } else { reduced_cosine(a, head, tail, square).map_err(E)? };
    let negative = if sine { (count & 2 != 0) ^ (bits >> 63 != 0) }
        else { (count + 1) & 2 != 0 };
    if negative { a.subtract("word0 F64/quadrant zero SUBSD", 0, result).map_err(E) } else { Ok(result) }
}

fn sine_kernel(a: &mut impl ArithmeticContext, head: u64, square: u64, tail: Option<u64>) -> Result<u64, VorbisScalarError> {
    let p = a.multiply("word0 F64/sine sixth MULSD", SIN_COEFFICIENTS[5].to_bits(), square)?;
    let fourth = a.multiply("word0 F64/sine fourth MULSD", square, square)?;
    let q = a.multiply("word0 F64/sine third MULSD", SIN_COEFFICIENTS[2].to_bits(), square)?;
    let p = a.add("word0 F64/sine fifth ADDSD", p, SIN_COEFFICIENTS[4].to_bits())?;
    let sixth = a.multiply("word0 F64/sine sixth power MULSD", fourth, square)?;
    let p = a.multiply("word0 F64/sine high multiply MULSD", p, square)?;
    let cube = a.multiply("word0 F64/sine cube MULSD", head, square)?;
    let q = a.add("word0 F64/sine second ADDSD", q, SIN_COEFFICIENTS[1].to_bits())?;
    let q = a.multiply("word0 F64/sine low multiply MULSD", q, square)?;
    let p = a.add("word0 F64/sine fourth ADDSD", p, SIN_COEFFICIENTS[3].to_bits())?;
    let half_square = if tail.is_some() {
        a.multiply("word0 F64/reduced sine half square MULSD", square, HALF)?
    } else { 0 };
    let p = a.multiply("word0 F64/sine high group MULSD", p, sixth)?;
    let q = a.add("word0 F64/sine first ADDSD", q, SIN_COEFFICIENTS[0].to_bits())?;
    let tail_product = if let Some(tail) = tail {
        a.multiply("word0 F64/reduced sine tail product MULSD", half_square, tail)?
    } else { 0 };
    let polynomial = a.add("word0 F64/sine groups ADDSD", p, q)?;
    let correction = a.multiply("word0 F64/sine correction MULSD", cube, polynomial)?;
    if let Some(tail) = tail {
        let correction = a.subtract("word0 F64/reduced sine correction SUBSD", correction, tail_product)?;
        let result = a.add("word0 F64/reduced sine tail ADDSD", tail, correction)?;
        a.add("word0 F64/reduced sine finish ADDSD", result, head)
    } else { a.add("word0 F64/direct sine finish ADDSD", head, correction) }
}

fn direct_cosine(a: &mut impl ArithmeticContext, square: u64) -> Result<u64, VorbisScalarError> {
    let fourth = a.multiply("57e031/direct cosine fourth MULSD", square, square)?;
    let p = a.multiply("57e03d/direct cosine second MULSD", COS_COEFFICIENTS[1].to_bits(), square)?;
    let q = a.multiply("57e049/direct cosine fourth coefficient MULSD", COS_COEFFICIENTS[3].to_bits(), square)?;
    let t = a.multiply("57e051/direct cosine sixth coefficient MULSD", COS_COEFFICIENTS[5].to_bits(), square)?;
    let eighth = a.multiply("57e055/direct cosine eighth MULSD", fourth, fourth)?;
    let p = a.add("57e059/direct cosine first ADDSD", p, COS_COEFFICIENTS[0].to_bits())?;
    let p = a.multiply("57e061/direct cosine low group MULSD", p, fourth)?;
    let q = a.add("57e065/direct cosine third ADDSD", q, COS_COEFFICIENTS[2].to_bits())?;
    let negative_half = a.multiply("57e06d/direct cosine negative half MULSD", square, NEGATIVE_HALF)?;
    let t = a.add("57e075/direct cosine fifth ADDSD", t, COS_COEFFICIENTS[4].to_bits())?;
    let q = a.multiply("57e07d/direct cosine middle group MULSD", q, eighth)?;
    let twelfth = a.multiply("57e081/direct cosine twelfth MULSD", fourth, eighth)?;
    let t = a.multiply("57e085/direct cosine high group MULSD", twelfth, t)?;
    let p = a.add("57e091/direct cosine first groups ADDSD", p, q)?;
    let leading = a.add("57e099/direct cosine leading ADDSD", negative_half, ONE)?;
    let loss = a.subtract("57e09d/direct cosine first loss SUBSD", ONE, leading)?;
    let loss = a.add("57e0a1/direct cosine half loss ADDSD", loss, negative_half)?;
    let p = a.add("57e0a5/direct cosine high group ADDSD", p, t)?;
    let result = a.add("57e0a9/direct cosine correction ADDSD", loss, p)?;
    a.add("57e0ad/direct cosine finish ADDSD", result, leading)
}

fn reduced_cosine(a: &mut impl ArithmeticContext, head: u64, tail: u64, square: u64) -> Result<u64, VorbisScalarError> {
    let head_tail = a.multiply("word0 F64/reduced cosine head tail MULSD", tail, head)?;
    let half = a.multiply("word0 F64/reduced cosine half MULSD", HALF, square)?;
    let p = a.multiply("word0 F64/reduced cosine sixth MULSD", COS_COEFFICIENTS[5].to_bits(), square)?;
    let q = a.multiply("word0 F64/reduced cosine third MULSD", COS_COEFFICIENTS[2].to_bits(), square)?;
    let minus_leading = a.subtract("word0 F64/reduced cosine negative leading SUBSD", half, ONE)?;
    let fourth = a.multiply("word0 F64/reduced cosine fourth MULSD", square, square)?;
    let p = a.add("word0 F64/reduced cosine fifth ADDSD", p, COS_COEFFICIENTS[4].to_bits())?;
    let q = a.add("word0 F64/reduced cosine second ADDSD", q, COS_COEFFICIENTS[1].to_bits())?;
    let loss = a.add("word0 F64/reduced cosine first loss ADDSD", minus_leading, ONE)?;
    let sixth = a.multiply("word0 F64/reduced cosine sixth power MULSD", fourth, square)?;
    let p = a.multiply("word0 F64/reduced cosine high multiply MULSD", p, square)?;
    let q = a.multiply("word0 F64/reduced cosine low multiply MULSD", q, square)?;
    let half = a.multiply("word0 F64/reduced cosine repeated half MULSD", square, HALF)?;
    let p = a.add("word0 F64/reduced cosine fourth ADDSD", p, COS_COEFFICIENTS[3].to_bits())?;
    let q = a.add("word0 F64/reduced cosine first ADDSD", q, COS_COEFFICIENTS[0].to_bits())?;
    let fourth = a.multiply("word0 F64/reduced cosine repeated fourth MULSD", square, square)?;
    let loss = a.subtract("word0 F64/reduced cosine half loss SUBSD", loss, half)?;
    let p = a.multiply("word0 F64/reduced cosine high group MULSD", p, sixth)?;
    let polynomial = a.add("word0 F64/reduced cosine groups ADDSD", q, p)?;
    let minus_leading = a.subtract("word0 F64/reduced cosine repeated leading SUBSD", half, ONE)?;
    let loss = a.subtract("word0 F64/reduced cosine head tail SUBSD", loss, head_tail)?;
    let polynomial = a.multiply("word0 F64/reduced cosine correction MULSD", polynomial, fourth)?;
    let correction = a.add("word0 F64/reduced cosine correction ADDSD", polynomial, loss)?;
    a.subtract("word0 F64/reduced cosine finish SUBSD", correction, minus_leading)
}
