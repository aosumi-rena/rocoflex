use super::finite_power::{ExplicitExceptionState, MathErrorCallbackCompletion};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::vorbis_power::{Arithmetic, VorbisScalarError};

const ONE: u64 = 0x3ff0_0000_0000_0000;
const TWO: u64 = 0x4000_0000_0000_0000;
const THREE_HALVES: u64 = 0x3ff8_0000_0000_0000;
const NEGATIVE_ONE: u64 = 0xbff0_0000_0000_0000;
const HALF_REDUCTION_BASE: u64 = 0x3fdd_ac67_0561_bb4f;
const ONE_REDUCTION_BASE: u64 = 0x3fe9_21fb_5444_2d18;
const THREE_HALVES_REDUCTION_BASE: u64 = 0x3fef_730b_d281_f69b;
const RECIPROCAL_REDUCTION_BASE: u64 = 0x3ff9_21fb_5444_2d18;
const NUMERATOR_HIGH: u64 = 0x3f73_476a_758d_a22a;
const NUMERATOR_MIDDLE: u64 = 0x3fc8_9e17_3a81_ee7f;
const NUMERATOR_LOW: u64 = 0x3fd2_fa53_1690_7834;
const DENOMINATOR_HIGH: u64 = 0x3fd3_27e3_df2c_f2aa;
const DENOMINATOR_LOW: u64 = 0x3fec_777c_a210_53f0;
const DENOMINATOR_MIDDLE: u64 = 0x3ff1_c587_93da_6ea4;

pub struct SourceVorbisArctangent {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
}

impl SourceVorbisArctangent {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, VorbisScalarError>
    {
        if architecture != ScalarF32Architecture::IntelSdm {
            return Err(VorbisScalarError::ArchitectureNotClosed);
        }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(VorbisScalarError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { architecture, state })
    }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn name(&self) -> &'static str { "source57da70/positive-normal32/separate" }
    pub fn arctangent_f32(&mut self, value: f32) -> Result<f32, VorbisScalarError> {
        self.arctangent_f32_bits(value.to_bits()).map(f32::from_bits)
    }
    pub fn arctangent_f32_bits(&mut self, bits: u32) -> Result<u32, VorbisScalarError> {
        if bits != 0 && !(0x0080_0000..0x4200_0000).contains(&bits) {
            return Err(VorbisScalarError::ArctangentOutsideNonnegativeNormal32Domain { bits });
        }
        let entry = self.state;
        match self.evaluate(bits) {
            Ok((result, advanced)) => { self.state = advanced; Ok(result) }
            Err(error) => { self.state = error.known_advanced_state(entry); Err(error) }
        }
    }

    fn evaluate(&self, bits: u32) -> Result<(u32, ExplicitExceptionState), VorbisScalarError> {

        let (wide, state) = self.state.widen_f32_for_math_error(bits);
        let mut arithmetic = Arithmetic { state };
        let (reduced, base) = if wide < 0x3fdc_0000_0000_0000 {

            (wide, 0)
        } else if wide < 0x3fe6_0000_0000_0000 {

            let doubled = arithmetic.add("ADDSD twice argument", wide, wide)?;
            let denominator = arithmetic.add("ADDSD argument+2", wide, TWO)?;
            let numerator = arithmetic.subtract("SUBSD twice argument-1", doubled, ONE)?;
            (arithmetic.divide("DIVSD half reduction", numerator, denominator)?, HALF_REDUCTION_BASE)
        } else if wide < 0x3ff3_0000_0000_0000 {
            let numerator = arithmetic.subtract("SUBSD argument-1", wide, ONE)?;
            let denominator = arithmetic.add("ADDSD argument+1", wide, ONE)?;
            (arithmetic.divide("DIVSD one reduction", numerator, denominator)?, ONE_REDUCTION_BASE)
        } else if wide < 0x4003_8000_0000_0000 {

            let scaled = arithmetic.multiply("MULSD argument*1.5", wide, THREE_HALVES)?;
            let numerator = arithmetic.subtract("SUBSD argument-1.5", wide, THREE_HALVES)?;
            let denominator = arithmetic.add("ADDSD scaled+1", scaled, ONE)?;
            (arithmetic.divide("DIVSD three-halves reduction", numerator, denominator)?, THREE_HALVES_REDUCTION_BASE)
        } else {

            (arithmetic.divide("DIVSD negative reciprocal", NEGATIVE_ONE, wide)?, RECIPROCAL_REDUCTION_BASE)
        };

        let square = arithmetic.multiply("MULSD reduced square", reduced, reduced)?;
        let numerator = arithmetic.multiply("MULSD numerator high", square, NUMERATOR_HIGH)?;
        let denominator = arithmetic.multiply("MULSD denominator high", square, DENOMINATOR_HIGH)?;
        let cube = arithmetic.multiply("MULSD reduced cube", square, reduced)?;
        let numerator = arithmetic.add("ADDSD numerator middle", numerator, NUMERATOR_MIDDLE)?;
        let denominator = arithmetic.add("ADDSD denominator middle", denominator, DENOMINATOR_MIDDLE)?;
        let numerator = arithmetic.multiply("MULSD numerator square", numerator, square)?;
        let denominator = arithmetic.multiply("MULSD denominator square", denominator, square)?;
        let numerator = arithmetic.add("ADDSD numerator low", numerator, NUMERATOR_LOW)?;
        let denominator = arithmetic.add("ADDSD denominator low", denominator, DENOMINATOR_LOW)?;
        let numerator = arithmetic.multiply("MULSD numerator cube", numerator, cube)?;
        let quotient = arithmetic.divide("DIVSD rational", numerator, denominator)?;
        let correction = arithmetic.subtract("SUBSD rational-reduced", quotient, reduced)?;
        let result = arithmetic.subtract("SUBSD base-correction", base, correction)?;

        let narrowed = MathErrorCallbackCompletion { result_f64_bits: result, advanced: arithmetic.state }
            .into_rne_f32().map_err(VorbisScalarError::ConversionRequired)?;
        Ok((narrowed.bits, narrowed.advanced))
    }
}
