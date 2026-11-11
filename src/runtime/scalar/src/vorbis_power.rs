use super::finite_power::{ExplicitExceptionState, MathErrorCallbackCompletion,
    MathErrorConversionRequired, NormalArithmeticError, normal_fma_source_bits,
    normal_multiply_source_bits, normal_divide_source_bits, round_exact_normal_u128};
use super::retained_exp_constants::{EXP_TABLE, LN2_DIV64, LOG2_64, POLY_HALF, POLY_THIRD};
use super::selected_exp::rne_i32_lane;
use super::shared_scalar_f32::ScalarF32Architecture;

const ONE: u64 = 0x3ff0_0000_0000_0000;
const SIGN: u64 = 0x8000_0000_0000_0000;
const LN2: u64 = 0x3fe6_2e42_fefa_39ef;
const LOG_THIRD: u64 = 0x3fd5_5555_5555_5555;
const UPPER_RANGE: u64 = 0x4056_2e43_0000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VorbisScalarError {
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    BaseOutsideIntegral24Domain { bits: u32 },
    ExponentOutsideCodebookDomain { bits: u32 },
    ScaleValueOutsideSigned21Domain { bits: u64 },
    ScaleShiftOutside63Domain { shift: i32 },
    ArctangentOutsideNonnegativeNormal32Domain { bits: u32 },
    ArithmeticRequired { operation: &'static str, reason: NormalArithmeticError, advanced: ExplicitExceptionState },
    SourceContinuationRequired { operation: &'static str, advanced: ExplicitExceptionState },
    ConversionRequired(MathErrorConversionRequired),
}

impl VorbisScalarError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "explicit arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "explicit source state is not RNE/all-masked/gradual",
            Self::BaseOutsideIntegral24Domain { .. } => "584fc0 base is outside positive integral24 codebook domain",
            Self::ExponentOutsideCodebookDomain { .. } => "584fc0 exponent is outside positive codebook domain",
            Self::ScaleValueOutsideSigned21Domain { .. } => "581ea0 value is outside signed integral21 packed domain",
            Self::ScaleShiftOutside63Domain { .. } => "581ea0 shift is outside packed [-63,63] domain",
            Self::ArctangentOutsideNonnegativeNormal32Domain { .. } => "57da70 input is outside +zero/positive-normal/<32 domain",
            Self::ArithmeticRequired { operation, .. } | Self::SourceContinuationRequired { operation, .. } => operation,
            Self::ConversionRequired(_) => "source scalar final F64-to-F32 conversion required",
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::ArithmeticRequired { advanced, .. } | Self::SourceContinuationRequired { advanced, .. } => advanced,
            Self::ConversionRequired(required) => required.advanced,
            _ => entry,
        }
    }
}

pub struct SourceVorbisScalarMath {
    explicit_source_word: u32,
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
}

impl SourceVorbisScalarMath {
    pub fn bind(explicit_source_word: u32, architecture: ScalarF32Architecture,
        state: ExplicitExceptionState) -> Result<Self, VorbisScalarError>
    {
        if architecture != ScalarF32Architecture::IntelSdm { return Err(VorbisScalarError::ArchitectureNotClosed); }

        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(VorbisScalarError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { explicit_source_word, architecture, state })
    }
    pub fn explicit_source_word(&self) -> u32 { self.explicit_source_word }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn name(&self) -> &'static str {
        if self.explicit_source_word == 0 { "source584fc0/codebook/separate" }
        else { "source584fc0/codebook/fused" }
    }
    pub fn scale_name(&self) -> &'static str { "source581ea0/packed21/shift63" }

    pub fn powf(&mut self, base: f32, exponent: f32) -> Result<f32, VorbisScalarError> {
        self.powf_bits(base.to_bits(), exponent.to_bits()).map(f32::from_bits)
    }
    pub fn scale_f64(&mut self, value: f64, shift: i32) -> Result<f64, VorbisScalarError> {
        self.scale_f64_bits(value.to_bits(), shift).map(f64::from_bits)
    }

    pub fn powf_bits(&mut self, base: u32, exponent: u32) -> Result<u32, VorbisScalarError> {
        if !positive_integral24(base) { return Err(VorbisScalarError::BaseOutsideIntegral24Domain { bits: base }); }
        if !(0x3780_0000..=0x3f80_0000).contains(&exponent) {
            return Err(VorbisScalarError::ExponentOutsideCodebookDomain { bits: exponent });
        }

        if exponent == 0x3f80_0000 { return Ok(base); }

        if base == 0x3f80_0000 { return Ok(base); }
        let entry = self.state;
        let result = self.general_power(base, exponent);
        match result {
            Ok((bits, advanced)) => { self.state = advanced; Ok(bits) }
            Err(error) => { self.state = error.known_advanced_state(entry); Err(error) }
        }
    }

    fn general_power(&self, base: u32, exponent: u32) -> Result<(u32, ExplicitExceptionState), VorbisScalarError> {
        let fused = self.explicit_source_word != 0;

        let (base_wide, state) = self.state.widen_f32_for_math_error(base);
        let (exponent_wide, state) = state.widen_f32_for_math_error(exponent);
        let mut arithmetic = Arithmetic { state };
        let fraction = base_wide & 0x000f_ffff_ffff_ffff;

        let index = ((fraction >> 44) + ((fraction >> 43) & 1)) as usize;
        let normalized = fraction | 0x3fe0_0000_0000_0000;
        let rounded = 0x3fe0_0000_0000_0000 | ((index as u64) << 44);
        let integer_exponent = ((base_wide >> 52) & 0x7ff) as u32 - 1023;
        let exponent_count = if integer_exponent == 0 { 0 }
            else { round_exact_normal_u128(u128::from(integer_exponent), 0).0 };
        let delta = arithmetic.subtract("584fc0/table SUBSD", rounded, normalized)?;
        let residual = arithmetic.multiply("584fc0/table reciprocal MULSD", delta, POW_RECIPROCAL[index])?;
        let correction = if fused {
            let polynomial = arithmetic.fma("584fc0/log first FMA", LOG_THIRD, residual, POLY_HALF.to_bits())?;
            let polynomial = arithmetic.fma("584fc0/log second FMA", polynomial, residual, ONE)?;
            arithmetic.multiply("584fc0/log residual MULSD", residual, polynomial)?
        } else {
            let polynomial = arithmetic.multiply("584fc0/log coefficient MULSD", LOG_THIRD, residual)?;
            let square = arithmetic.multiply("584fc0/log square MULSD", residual, residual)?;
            let polynomial = arithmetic.add("584fc0/log polynomial ADDSD", polynomial, POLY_HALF.to_bits())?;
            let product = arithmetic.multiply("584fc0/log polynomial MULSD", square, polynomial)?;
            arithmetic.add("584fc0/log residual ADDSD", product, residual)?
        };
        let logarithm = arithmetic.multiply("584fc0/log exponent MULSD", exponent_count, LN2)?;
        let logarithm = arithmetic.add("584fc0/log table ADDSD", logarithm, POW_LOG[index])?;
        let logarithm = arithmetic.subtract("584fc0/log correction SUBSD", logarithm, correction)?;
        let powered_log = arithmetic.multiply("584fc0/exponent MULSD", exponent_wide, logarithm)?;

        if powered_log >> 63 != 0 || powered_log > UPPER_RANGE {
            return Err(VorbisScalarError::SourceContinuationRequired { operation: "584fc0/range helper required", advanced: arithmetic.state });
        }
        let scaled = arithmetic.multiply("584fc0/packed LOG2_64 MULPD low", powered_log, LOG2_64.to_bits())?;

        let (count, flags) = rne_i32_lane(scaled);
        arithmetic.state = arithmetic.state.with_source_status(flags);
        if !(0..=2047).contains(&count) {
            return Err(VorbisScalarError::SourceContinuationRequired { operation: "584fc0/codebook count bound required", advanced: arithmetic.state });
        }
        let count_wide = if count == 0 { 0 } else { round_exact_normal_u128(count as u128, 0).0 };
        let remainder = if fused {
            arithmetic.fma("584fc0/reduction FNMADD", count_wide ^ SIGN, LN2_DIV64.to_bits(), powered_log)?
        } else {
            let product = arithmetic.multiply("584fc0/reduction count MULSD", count_wide, LN2_DIV64.to_bits())?;
            arithmetic.subtract("584fc0/reduction SUBSD", powered_log, product)?
        };
        let reduced = if fused {
            let polynomial = arithmetic.fma("584fc0/exp first FMA", POLY_THIRD.to_bits(), remainder, POLY_HALF.to_bits())?;
            let polynomial = arithmetic.fma("584fc0/exp second FMA", polynomial, remainder, ONE)?;
            arithmetic.multiply("584fc0/exp residual MULSD", remainder, polynomial)?
        } else {
            let polynomial = arithmetic.multiply("584fc0/exp coefficient MULSD", POLY_THIRD.to_bits(), remainder)?;
            let square = arithmetic.multiply("584fc0/exp square MULSD", remainder, remainder)?;
            let polynomial = arithmetic.add("584fc0/exp polynomial ADDSD", polynomial, POLY_HALF.to_bits())?;
            let product = arithmetic.multiply("584fc0/exp polynomial MULSD", square, polynomial)?;
            arithmetic.add("584fc0/exp residual ADDSD", product, remainder)?
        };
        let table = EXP_TABLE[(count as usize) & 63].to_bits();
        let expanded = if fused { arithmetic.fma("584fc0/exp table FMA", reduced, table, table)? }
        else {
            let product = arithmetic.multiply("584fc0/exp table MULSD", reduced, table)?;
            arithmetic.add("584fc0/exp table ADDSD", product, table)?
        };

        let wide = expanded.wrapping_add(((count >> 6) as u64) << 52);
        let narrowed = MathErrorCallbackCompletion { result_f64_bits: wide, advanced: arithmetic.state }
            .into_rne_f32().map_err(VorbisScalarError::ConversionRequired)?;
        Ok((narrowed.bits, narrowed.advanced))
    }

    pub fn scale_f64_bits(&mut self, bits: u64, shift: i32) -> Result<u64, VorbisScalarError> {
        if !(-63..=63).contains(&shift) { return Err(VorbisScalarError::ScaleShiftOutside63Domain { shift }); }
        let magnitude = bits & !SIGN;
        if magnitude == 0 { return Ok(bits); }
        let field = (magnitude >> 52) as i32;
        let exponent = field - 1023;
        if !(0..=20).contains(&exponent) { return Err(VorbisScalarError::ScaleValueOutsideSigned21Domain { bits }); }
        let discard = (52 - exponent) as u32;
        let significand = (magnitude & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
        if significand & ((1u64 << discard) - 1) != 0 || significand >> discard > 0x1f_ffff {
            return Err(VorbisScalarError::ScaleValueOutsideSigned21Domain { bits });
        }
        Ok((bits & 0x800f_ffff_ffff_ffff) | (((field + shift) as u64) << 52))
    }
}

fn positive_integral24(bits: u32) -> bool {
    if !(0x3f80_0000..=0x4b7f_ffff).contains(&bits) { return false; }
    let exponent = (bits >> 23) - 127;
    let discard = 23 - exponent;
    bits & ((1u32 << discard) - 1) == 0
}

pub(super) struct Arithmetic { pub(super) state: ExplicitExceptionState }

pub(super) trait ArithmeticContext {
    fn exception_state(&self) -> ExplicitExceptionState;
    fn record_completed(&mut self, flags: u32);
    fn finish(&mut self, operation: &'static str, result: Result<(u64, u32), NormalArithmeticError>) -> Result<u64, VorbisScalarError> {
        let (bits, flags) = result.map_err(|reason| VorbisScalarError::ArithmeticRequired {
            operation, reason, advanced: self.exception_state() })?;
        self.record_completed(flags);
        Ok(bits)
    }
    fn multiply(&mut self, operation: &'static str, left: u64, right: u64) -> Result<u64, VorbisScalarError> {
        self.finish(operation, normal_multiply_source_bits(left, right))
    }
    fn add(&mut self, operation: &'static str, left: u64, right: u64) -> Result<u64, VorbisScalarError> {

        self.finish(operation, normal_fma_source_bits(left, ONE, right))
    }
    fn subtract(&mut self, operation: &'static str, left: u64, right: u64) -> Result<u64, VorbisScalarError> {
        self.add(operation, left, right ^ SIGN)
    }
    fn divide(&mut self, operation: &'static str, left: u64, right: u64) -> Result<u64, VorbisScalarError> {
        self.finish(operation, normal_divide_source_bits(left, right))
    }
    fn fma(&mut self, operation: &'static str, left: u64, right: u64, addend: u64) -> Result<u64, VorbisScalarError> {
        self.finish(operation, normal_fma_source_bits(left, right, addend))
    }
}
impl ArithmeticContext for Arithmetic {
    fn exception_state(&self) -> ExplicitExceptionState { self.state }
    fn record_completed(&mut self, flags: u32) { self.state = self.state.with_source_status(flags); }
}
impl ArithmeticContext for super::phase_status::PhaseStatusAccumulator {
    fn exception_state(&self) -> ExplicitExceptionState { self.state }
    fn record_completed(&mut self, flags: u32) { self.raise(flags); }
}
impl Arithmetic {
    pub(super) fn multiply(&mut self, operation: &'static str, left: u64, right: u64) -> Result<u64, VorbisScalarError> {
        ArithmeticContext::multiply(self, operation, left, right)
    }
    pub(super) fn add(&mut self, operation: &'static str, left: u64, right: u64) -> Result<u64, VorbisScalarError> {
        ArithmeticContext::add(self, operation, left, right)
    }
    pub(super) fn subtract(&mut self, operation: &'static str, left: u64, right: u64) -> Result<u64, VorbisScalarError> {
        ArithmeticContext::subtract(self, operation, left, right)
    }
    pub(super) fn divide(&mut self, operation: &'static str, left: u64, right: u64) -> Result<u64, VorbisScalarError> {
        ArithmeticContext::divide(self, operation, left, right)
    }
    pub(super) fn fma(&mut self, operation: &'static str, left: u64, right: u64, addend: u64) -> Result<u64, VorbisScalarError> {
        ArithmeticContext::fma(self, operation, left, right, addend)
    }
}

const POW_LOG: [u64; 257] = [
    0x0000000000000000, 0x3f6ff00aa2b10bc0, 0x3f7fe02a6b106789, 0x3f87dc475f810a77,
    0x3f8fc0a8b0fc03e4, 0x3f93cea44346a575, 0x3f97b91b07d5b11b, 0x3f9b9fc027af9198,
    0x3f9f829b0e783300, 0x3fa1b0d98923d980, 0x3fa39e87b9febd60, 0x3fa58a5bafc8e4d5,
    0x3fa77458f632dcfc, 0x3fa95c830ec8e3eb, 0x3fab42dd711971bf, 0x3fad276b8adb0b52,
    0x3faf0a30c01162a6, 0x3fb075983598e471, 0x3fb16536eea37ae1, 0x3fb253f62f0a1417,
    0x3fb341d7961bd1d1, 0x3fb42edcbea646f0, 0x3fb51b073f06183f, 0x3fb60658a93750c4,
    0x3fb6f0d28ae56b4c, 0x3fb7da766d7b12cd, 0x3fb8c345d6319b21, 0x3fb9ab42462033ad,
    0x3fba926d3a4ad563, 0x3fbb78c82bb0eda1, 0x3fbc5e548f5bc743, 0x3fbd4313d66cb35d,
    0x3fbe27076e2af2e6, 0x3fbf0a30c01162a6, 0x3fbfec9131dbeabb, 0x3fc0671512ca596e,
    0x3fc0d77e7cd08e59, 0x3fc14785846742ac, 0x3fc1b72ad52f67a0, 0x3fc2266f190a5acb,
    0x3fc29552f81ff523, 0x3fc303d718e47fd3, 0x3fc371fc201e8f74, 0x3fc3dfc2b0ecc62a,
    0x3fc44d2b6ccb7d1e, 0x3fc4ba36f39a55e5, 0x3fc526e5e3a1b438, 0x3fc59338d9982086,
    0x3fc5ff3070a793d4, 0x3fc66acd4272ad51, 0x3fc6d60fe719d21d, 0x3fc740f8f54037a5,
    0x3fc7ab890210d909, 0x3fc815c0a14357eb, 0x3fc87fa06520c911, 0x3fc8e928de886d41,
    0x3fc9525a9cf456b4, 0x3fc9bb362e7dfb83, 0x3fca23bc1fe2b563, 0x3fca8becfc882f19,
    0x3fcaf3c94e80bff3, 0x3fcb5b519e8fb5a4, 0x3fcbc286742d8cd6, 0x3fcc2968558c18c1,
    0x3fcc8ff7c79a9a22, 0x3fccf6354e09c5dc, 0x3fcd5c216b4fbb91, 0x3fcdc1bca0abec7d,
    0x3fce27076e2af2e6, 0x3fce8c0252aa5a60, 0x3fcef0adcbdc5936, 0x3fcf550a564b7b37,
    0x3fcfb9186d5e3e2b, 0x3fd00e6c45ad501d, 0x3fd0402594b4d041, 0x3fd071b85fcd590d,
    0x3fd0a324e27390e3, 0x3fd0d46b579ab74b, 0x3fd1058bf9ae4ad5, 0x3fd136870293a8b0,
    0x3fd1675cababa60e, 0x3fd1980d2dd4236f, 0x3fd1c898c16999fb, 0x3fd1f8ff9e48a2f3,
    0x3fd22941fbcf7966, 0x3fd2596010df763a, 0x3fd2895a13de86a3, 0x3fd2b9303ab89d25,
    0x3fd2e8e2bae11d31, 0x3fd31871c9544185, 0x3fd347dd9a987d55, 0x3fd3772662bfd85b,
    0x3fd3a64c556945ea, 0x3fd3d54fa5c1f710, 0x3fd404308686a7e4, 0x3fd432ef2a04e814,
    0x3fd4618bc21c5ec2, 0x3fd49006804009d1, 0x3fd4be5f957778a1, 0x3fd4ec973260026a,
    0x3fd51aad872df82d, 0x3fd548a2c3add263, 0x3fd5767717455a6c, 0x3fd5a42ab0f4cfe2,
    0x3fd5d1bdbf5809ca, 0x3fd5ff3070a793d4, 0x3fd62c82f2b9c795, 0x3fd659b57303e1f3,
    0x3fd686c81e9b14af, 0x3fd6b3bb2235943e, 0x3fd6e08eaa2ba1e4, 0x3fd70d42e2789236,
    0x3fd739d7f6bbd007, 0x3fd7664e1239dbcf, 0x3fd792a55fdd47a2, 0x3fd7bede0a37afc0,
    0x3fd7eaf83b82afc3, 0x3fd816f41da0d496, 0x3fd842d1da1e8b17, 0x3fd86e919a330ba0,
    0x3fd89a3386c1425b, 0x3fd8c5b7c858b48b, 0x3fd8f11e873662c7, 0x3fd91c67eb45a83e,
    0x3fd947941c2116fb, 0x3fd972a341135158, 0x3fd99d958117e08b, 0x3fd9c86b02dc0863,
    0x3fd9f323ecbf984c, 0x3fda1dc064d5b995, 0x3fda484090e5bb0a, 0x3fda72a4966bd9ea,
    0x3fda9cec9a9a084a, 0x3fdac718c258b0e4, 0x3fdaf1293247786b, 0x3fdb1b1e0ebdfc5b,
    0x3fdb44f77bcc8f63, 0x3fdb6eb59d3cf35e, 0x3fdb9858969310fb, 0x3fdbc1e08b0dad0a,
    0x3fdbeb4d9da71b7c, 0x3fdc149ff115f027, 0x3fdc3dd7a7cdad4d, 0x3fdc66f4e3ff6ff8,
    0x3fdc8ff7c79a9a22, 0x3fdcb8e0744d7aca, 0x3fdce1af0b85f3eb, 0x3fdd0a63ae721e64,
    0x3fdd32fe7e00ebd5, 0x3fdd5b7f9ae2c684, 0x3fdd83e7258a2f3e, 0x3fddac353e2c5954,
    0x3fddd46a04c1c4a1, 0x3fddfc859906d5b5, 0x3fde24881a7c6c26, 0x3fde4c71a8687704,
    0x3fde744261d68788, 0x3fde9bfa659861f5, 0x3fdec399d2468cc0, 0x3fdeeb20c640ddf4,
    0x3fdf128f5faf06ed, 0x3fdf39e5bc811e5c, 0x3fdf6123fa7028ac, 0x3fdf884a36fe9ec2,
    0x3fdfaf588f78f31f, 0x3fdfd64f20f61572, 0x3fdffd2e0857f498, 0x3fe011fab125ff8a,
    0x3fe02552a5a5d0ff, 0x3fe0389eefce633b, 0x3fe04bdf9da926d2, 0x3fe05f14bd26459c,
    0x3fe0723e5c1cdf40, 0x3fe0855c884b450e, 0x3fe0986f4f573521, 0x3fe0ab76bece14d2,
    0x3fe0be72e4252a83, 0x3fe0d163ccb9d6b8, 0x3fe0e44985d1cc8c, 0x3fe0f7241c9b497d,
    0x3fe109f39e2d4c97, 0x3fe11cb81787ccf8, 0x3fe12f719593efbc, 0x3fe1422025243d45,
    0x3fe154c3d2f4d5ea, 0x3fe1675cababa60e, 0x3fe179eabbd899a1, 0x3fe18c6e0ff5cf06,
    0x3fe19ee6b467c96f, 0x3fe1b154b57da29f, 0x3fe1c3b81f713c25, 0x3fe1d610fe677003,
    0x3fe1e85f5e7040d0, 0x3fe1faa34b87094c, 0x3fe20cdcd192ab6e, 0x3fe21f0bfc65beec,
    0x3fe23130d7bebf43, 0x3fe2434b6f483934, 0x3fe2555bce98f7cb, 0x3fe26762013430e0,
    0x3fe2795e1289b11b, 0x3fe28b500df60783, 0x3fe29d37fec2b08b, 0x3fe2af15f02640ad,
    0x3fe2c0e9ed448e8c, 0x3fe2d2b4012edc9e, 0x3fe2e47436e40268, 0x3fe2f62a99509546,
    0x3fe307d7334f10be, 0x3fe3197a0fa7fe6a, 0x3fe32b1339121d71, 0x3fe33ca2ba328995,
    0x3fe34e289d9ce1d3, 0x3fe35fa4edd36ea0, 0x3fe37117b54747b6, 0x3fe38280fe58797f,
    0x3fe393e0d3562a1a, 0x3fe3a5373e7ebdfa, 0x3fe3b68449fffc23, 0x3fe3c7c7fff73206,
    0x3fe3d9026a7156fb, 0x3fe3ea33936b2f5c, 0x3fe3fb5b84d16f42, 0x3fe40c7a4880dce9,
    0x3fe41d8fe84672ae, 0x3fe42e9c6ddf80bf, 0x3fe43f9fe2f9ce67, 0x3fe4509a5133bb0a,
    0x3fe4618bc21c5ec2, 0x3fe472743f33aaad, 0x3fe48353d1ea88df, 0x3fe4942a83a2fc07,
    0x3fe4a4f85db03ebb, 0x3fe4b5bd6956e274, 0x3fe4c679afccee3a, 0x3fe4d72d3a39fd00,
    0x3fe4e7d811b75bb1, 0x3fe4f87a3f5026e9, 0x3fe50913cc01686b, 0x3fe519a4c0ba3446,
    0x3fe52a2d265bc5ab, 0x3fe53aad05b99b7d, 0x3fe54b2467999498, 0x3fe55b9354b40bcd,
    0x3fe56bf9d5b3f399, 0x3fe57c57f336f191, 0x3fe58cadb5cd7989, 0x3fe59cfb25fae87e,
    0x3fe5ad404c359f2d, 0x3fe5bd7d30e71c73, 0x3fe5cdb1dc6c1765, 0x3fe5ddde57149923,
    0x3fe5ee02a9241675, 0x3fe5fe1edad18919, 0x3fe60e32f44788d9, 0x3fe61e3efda46467,
    0x3fe62e42fefa39ef,
];

const POW_RECIPROCAL: [u64; 257] = [
    0x4000000000000000, 0x3fffe01fe01fe020, 0x3fffc07f01fc07f0, 0x3fffa11caa01fa12,
    0x3fff81f81f81f820, 0x3fff6310aca0dbb5, 0x3fff44659e4a4271, 0x3fff25f644230ab5,
    0x3fff07c1f07c1f08, 0x3ffee9c7f8458e02, 0x3ffecc07b301ecc0, 0x3ffeae807aba01eb,
    0x3ffe9131abf0b767, 0x3ffe741aa59750e4, 0x3ffe573ac901e574, 0x3ffe3a9179dc1a73,
    0x3ffe1e1e1e1e1e1e, 0x3ffe01e01e01e01e, 0x3ffde5d6e3f8868a, 0x3ffdca01dca01dca,
    0x3ffdae6076b981db, 0x3ffd92f2231e7f8a, 0x3ffd77b654b82c34, 0x3ffd5cac807572b2,
    0x3ffd41d41d41d41d, 0x3ffd272ca3fc5b1a, 0x3ffd0cb58f6ec074, 0x3ffcf26e5c44bfc6,
    0x3ffcd85689039b0b, 0x3ffcbe6d9601cbe7, 0x3ffca4b3055ee191, 0x3ffc8b265afb8a42,
    0x3ffc71c71c71c71c, 0x3ffc5894d10d4986, 0x3ffc3f8f01c3f8f0, 0x3ffc26b5392ea01c,
    0x3ffc0e070381c0e0, 0x3ffbf583ee868d8b, 0x3ffbdd2b899406f7, 0x3ffbc4fd65883e7b,
    0x3ffbacf914c1bad0, 0x3ffb951e2b18ff23, 0x3ffb7d6c3dda338b, 0x3ffb65e2e3beee05,
    0x3ffb4e81b4e81b4f, 0x3ffb37484ad806ce, 0x3ffb2036406c80d9, 0x3ffb094b31d922a4,
    0x3ffaf286bca1af28, 0x3ffadbe87f94905e, 0x3ffac5701ac5701b, 0x3ffaaf1d2f87ebfd,
    0x3ffa98ef606a63be, 0x3ffa82e65130e159, 0x3ffa6d01a6d01a6d, 0x3ffa574107688a4a,
    0x3ffa41a41a41a41a, 0x3ffa2c2a87c51ca0, 0x3ffa16d3f97a4b02, 0x3ffa01a01a01a01a,
    0x3ff9ec8e951033d9, 0x3ff9d79f176b682d, 0x3ff9c2d14ee4a102, 0x3ff9ae24ea5510da,
    0x3ff999999999999a, 0x3ff9852f0d8ec0ff, 0x3ff970e4f80cb872, 0x3ff95cbb0be377ae,
    0x3ff948b0fcd6e9e0, 0x3ff934c67f9b2ce6, 0x3ff920fb49d0e229, 0x3ff90d4f120190d5,
    0x3ff8f9c18f9c18fa, 0x3ff8e6527af1373f, 0x3ff8d3018d3018d3, 0x3ff8bfce8062ff3a,
    0x3ff8acb90f6bf3aa, 0x3ff899c0f601899c, 0x3ff886e5f0abb04a, 0x3ff87427bcc092b9,
    0x3ff8618618618618, 0x3ff84f00c2780614, 0x3ff83c977ab2bedd, 0x3ff82a4a0182a4a0,
    0x3ff8181818181818, 0x3ff8060180601806, 0x3ff7f405fd017f40, 0x3ff7e225515a4f1d,
    0x3ff7d05f417d05f4, 0x3ff7beb3922e017c, 0x3ff7ad2208e0ecc3, 0x3ff79baa6bb6398b,
    0x3ff78a4c8178a4c8, 0x3ff77908119ac60d, 0x3ff767dce434a9b1, 0x3ff756cac201756d,
    0x3ff745d1745d1746, 0x3ff734f0c541fe8d, 0x3ff724287f46debc, 0x3ff713786d9c7c09,
    0x3ff702e05c0b8170, 0x3ff6f26016f26017, 0x3ff6e1f76b4337c7, 0x3ff6d1a62681c861,
    0x3ff6c16c16c16c17, 0x3ff6b1490aa31a3d, 0x3ff6a13cd1537290, 0x3ff691473a88d0c0,
    0x3ff6816816816817, 0x3ff6719f3601671a, 0x3ff661ec6a5122f9, 0x3ff6524f853b4aa3,
    0x3ff642c8590b2164, 0x3ff63356b88ac0de, 0x3ff623fa77016240, 0x3ff614b36831ae94,
    0x3ff6058160581606, 0x3ff5f66434292dfc, 0x3ff5e75bb8d015e7, 0x3ff5d867c3ece2a5,
    0x3ff5c9882b931057, 0x3ff5babcc647fa91, 0x3ff5ac056b015ac0, 0x3ff59d61f123ccaa,
    0x3ff58ed2308158ed, 0x3ff5805601580560, 0x3ff571ed3c506b3a, 0x3ff56397ba7c52e2,
    0x3ff5555555555555, 0x3ff54725e6bb82fe, 0x3ff5390948f40feb, 0x3ff52aff56a8054b,
    0x3ff51d07eae2f815, 0x3ff50f22e111c4c5, 0x3ff5015015015015, 0x3ff4f38f62dd4c9b,
    0x3ff4e5e0a72f0539, 0x3ff4d843bedc2c4c, 0x3ff4cab88725af6e, 0x3ff4bd3edda68fe1,
    0x3ff4afd6a052bf5b, 0x3ff4a27fad76014a, 0x3ff49539e3b2d067, 0x3ff4880522014880,
    0x3ff47ae147ae147b, 0x3ff46dce34596066, 0x3ff460cbc7f5cf9a, 0x3ff453d9e2c776ca,
    0x3ff446f86562d9fb, 0x3ff43a2730abee4d, 0x3ff42d6625d51f87, 0x3ff420b5265e5951,
    0x3ff4141414141414, 0x3ff40782d10e6566, 0x3ff3fb013fb013fb, 0x3ff3ee8f42a5af07,
    0x3ff3e22cbce4a902, 0x3ff3d5d991aa75c6, 0x3ff3c995a47babe7, 0x3ff3bd60d9232955,
    0x3ff3b13b13b13b14, 0x3ff3a524387ac822, 0x3ff3991c2c187f63, 0x3ff38d22d366088e,
    0x3ff3813813813814, 0x3ff3755bd1c945ee, 0x3ff3698df3de0748, 0x3ff35dce5f9f2af8,
    0x3ff3521cfb2b78c1, 0x3ff34679ace01346, 0x3ff33ae45b57bcb2, 0x3ff32f5ced6a1dfa,
    0x3ff323e34a2b10bf, 0x3ff3187758e9ebb6, 0x3ff30d190130d190, 0x3ff301c82ac40260,
    0x3ff2f684bda12f68, 0x3ff2eb4ea1fed14b, 0x3ff2e025c04b8097, 0x3ff2d50a012d50a0,
    0x3ff2c9fb4d812ca0, 0x3ff2bef98e5a3711, 0x3ff2b404ad012b40, 0x3ff2a91c92f3c105,
    0x3ff29e4129e4129e, 0x3ff293725bb804a5, 0x3ff288b01288b013, 0x3ff27dfa38a1ce4d,
    0x3ff27350b8812735, 0x3ff268b37cd60127, 0x3ff25e22708092f1, 0x3ff2539d7e9177b2,
    0x3ff2492492492492, 0x3ff23eb79717605b, 0x3ff23456789abcdf, 0x3ff22a0122a0122a,
    0x3ff21fb78121fb78, 0x3ff21579804855e6, 0x3ff20b470c67c0d9, 0x3ff2012012012012,
    0x3ff1f7047dc11f70, 0x3ff1ecf43c7fb84c, 0x3ff1e2ef3b3fb874, 0x3ff1d8f5672e4abd,
    0x3ff1cf06ada2811d, 0x3ff1c522fc1ce059, 0x3ff1bb4a4046ed29, 0x3ff1b17c67f2bae3,
    0x3ff1a7b9611a7b96, 0x3ff19e0119e0119e, 0x3ff19453808ca29c, 0x3ff18ab083902bdb,
    0x3ff1811811811812, 0x3ff1778a191bd684, 0x3ff16e0689427379, 0x3ff1648d50fc3201,
    0x3ff15b1e5f75270d, 0x3ff151b9a3fdd5c9, 0x3ff1485f0e0acd3b, 0x3ff13f0e8d344724,
    0x3ff135c81135c811, 0x3ff12c8b89edc0ac, 0x3ff12358e75d3033, 0x3ff11a3019a74826,
    0x3ff1111111111111, 0x3ff107fbbe011080, 0x3ff0fef010fef011, 0x3ff0f5edfab325a2,
    0x3ff0ecf56be69c90, 0x3ff0e40655826011, 0x3ff0db20a88f4696, 0x3ff0d24456359e3a,
    0x3ff0c9714fbcda3b, 0x3ff0c0a7868b4171, 0x3ff0b7e6ec259dc8, 0x3ff0af2f722eecb5,
    0x3ff0a6810a6810a7, 0x3ff09ddba6af8360, 0x3ff0953f39010954, 0x3ff08cabb37565e2,
    0x3ff0842108421084, 0x3ff07b9f29b8eae2, 0x3ff073260a47f7c6, 0x3ff06ab59c7912fb,
    0x3ff0624dd2f1a9fc, 0x3ff059eea0727586, 0x3ff05197f7d73404, 0x3ff04949cc1664c5,
    0x3ff0410410410410, 0x3ff038c6b78247fc, 0x3ff03091b51f5e1a, 0x3ff02864fc7729e9,
    0x3ff0204081020408, 0x3ff0182436517a37, 0x3ff0101010101010, 0x3ff0080402010080,
    0x3ff0000000000000,
];
