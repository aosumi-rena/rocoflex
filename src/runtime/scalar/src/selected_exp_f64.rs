use super::finite_power::{ExplicitExceptionState, round_exact_normal_u128};
use super::retained_exp_constants::{EXP_TABLE, LOG2_64, POLY_HALF, POLY_THIRD};
use super::selected_exp::rne_i32_lane;
use super::shared_scalar_f32::ScalarF32Architecture;
use super::vorbis_power::{Arithmetic, VorbisScalarError};

const SIGN: u64 = 0x8000_0000_0000_0000;
const ONE: u64 = 0x3ff0_0000_0000_0000;
const TWO32: u64 = 0x41f0_0000_0000_0000;
const TINY: u64 = 0x3e50_0000_0000_0000;
const BELOW_HALF: u64 = 0x3c80_0000_0000_0000;
const WHEEL_LIMIT: u64 = 0x3ff9_0000_0000_0000;
const NEGATIVE_LN2_64_HIGH: u64 = 0xbf86_2e42_fefa_0000;
const NEGATIVE_LN2_64_LOW: u64 = 0xbd1c_f79a_bc9e_3b39;
const ONE_720: u64 = 0x3f56_c16c_16c1_6c17;
const ONE_120: u64 = 0x3f81_1111_1111_1111;
const ONE_24: u64 = 0x3fa5_5555_5555_5555;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceF64ExpError {
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    OutsideWheelDomain { bits: u64 },
    ReductionRequired { scaled_bits: u64, advanced: ExplicitExceptionState },
    Arithmetic(VorbisScalarError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceF64ExpCallReceipt {
    pub input_bits: u64,
    pub explicit_source_word: u32,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
    pub result_bits: Option<u64>,
    pub error: Option<SourceF64ExpError>,
}

impl SourceF64ExpError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "explicit F64 arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "explicit F64 exp state is not RNE/all-masked/gradual",
            Self::OutsideWheelDomain { .. } => "57ee70 input is outside finite [-25/16,+25/16] wheel domain",
            Self::ReductionRequired { .. } => "57ee70 bounded truncation continuation required",
            Self::Arithmetic(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::ReductionRequired { advanced, .. } => advanced,
            Self::Arithmetic(error) => error.known_advanced_state(entry),
            _ => entry,
        }
    }
}

pub struct SourceF64Exp {
    explicit_source_word: u32,
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
    last_call: Option<SourceF64ExpCallReceipt>,
    first_failure: Option<SourceF64ExpCallReceipt>,
}

impl SourceF64Exp {
    pub fn bind(explicit_source_word: u32, architecture: ScalarF32Architecture,
        state: ExplicitExceptionState) -> Result<Self, SourceF64ExpError>
    {
        if architecture != ScalarF32Architecture::IntelSdm {
            return Err(SourceF64ExpError::ArchitectureNotClosed);
        }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceF64ExpError::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { explicit_source_word, architecture, state, last_call: None, first_failure: None })
    }
    pub fn explicit_source_word(&self) -> u32 { self.explicit_source_word }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn last_call(&self) -> Option<&SourceF64ExpCallReceipt> { self.last_call.as_ref() }

    pub fn first_failure(&self) -> Option<&SourceF64ExpCallReceipt> { self.first_failure.as_ref() }
    pub fn name(&self) -> &'static str {
        if self.explicit_source_word == 0 { "source57ee70/f64/wheel/separate" }
        else { "source57ee70/f64/wheel/fused" }
    }
    pub fn evaluate(&mut self, value: f64) -> Result<f64, SourceF64ExpError> {
        self.evaluate_bits(value.to_bits()).map(f64::from_bits)
    }
    pub fn evaluate_bits(&mut self, bits: u64) -> Result<u64, SourceF64ExpError> {
        let entry = self.state;
        let result = self.evaluate_source_bits(bits);
        let receipt = SourceF64ExpCallReceipt { input_bits: bits,
            explicit_source_word: self.explicit_source_word, entry, advanced: self.state,
            result_bits: result.as_ref().ok().copied(), error: result.as_ref().err().copied() };
        self.last_call = Some(receipt);
        if result.is_err() && self.first_failure.is_none() { self.first_failure = Some(receipt); }
        result
    }

    fn evaluate_source_bits(&mut self, bits: u64) -> Result<u64, SourceF64ExpError> {
        let magnitude = bits & !SIGN;
        if magnitude > WHEEL_LIMIT { return Err(SourceF64ExpError::OutsideWheelDomain { bits }); }
        let entry = self.state;
        match self.graph(bits) {
            Ok((bits, advanced)) => { self.state = advanced; Ok(bits) }
            Err(error) => { self.state = error.known_advanced_state(entry); Err(error) }
        }
    }

    fn graph(&self, bits: u64) -> Result<(u64, ExplicitExceptionState), SourceF64ExpError> {
        let magnitude = bits & !SIGN;

        let mut arithmetic = Arithmetic { state: self.state };
        if magnitude <= TINY {

            if magnitude <= BELOW_HALF {
                let flags = if magnitude == 0 { 0 }
                    else { 0x20 | if magnitude < 0x0010_0000_0000_0000 { 2 } else { 0 } };
                return Ok((ONE, arithmetic.state.with_source_status(flags)));
            }
            let _discarded = arithmetic.add("57ee70/tiny discarded ADDSD", bits, TWO32)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let result = arithmetic.add("57ee70/tiny result ADDSD", bits, ONE)
                .map_err(SourceF64ExpError::Arithmetic)?;
            return Ok((result, arithmetic.state));
        }

        let fused = self.explicit_source_word != 0;
        let scaled = arithmetic.multiply("57ee70/LOG2_64 MULSD", bits, LOG2_64.to_bits())
            .map_err(SourceF64ExpError::Arithmetic)?;
        let (integer, truncated_bits, flags) = truncate_bounded_count(scaled, arithmetic.state)?;
        arithmetic.state = arithmetic.state.with_source_status(flags);

        let count = if fused { truncated_bits } else if integer == 0 { 0 }
            else { round_exact_normal_u128(u128::from(integer.unsigned_abs()), 0).0
                | if integer < 0 { SIGN } else { 0 } };
        let reduced = if fused {
            let first = arithmetic.fma("57f129/reduction VFMADD231SD", count, NEGATIVE_LN2_64_HIGH, bits)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let low = arithmetic.multiply("57f132/reduction low VMULSD", count, NEGATIVE_LN2_64_LOW)
                .map_err(SourceF64ExpError::Arithmetic)?;
            arithmetic.add("57f13a/reduction VADDSD", low, first).map_err(SourceF64ExpError::Arithmetic)?
        } else {
            let high = arithmetic.multiply("57ef0e/reduction high MULSD", count, NEGATIVE_LN2_64_HIGH)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let low = arithmetic.multiply("57ef22/reduction low MULSD", count, NEGATIVE_LN2_64_LOW)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let first = arithmetic.add("57ef2a/reduction first ADDSD", bits, high)
                .map_err(SourceF64ExpError::Arithmetic)?;
            arithmetic.add("57ef2e/reduction second ADDSD", low, first).map_err(SourceF64ExpError::Arithmetic)?
        };

        let polynomial = if fused {
            let p = arithmetic.fma("57f152/polynomial FMA720", ONE_720, reduced, ONE_120)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let p = arithmetic.fma("57f15b/polynomial FMA120", p, reduced, ONE_24)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let p = arithmetic.fma("57f164/polynomial FMA24", p, reduced, POLY_THIRD.to_bits())
                .map_err(SourceF64ExpError::Arithmetic)?;
            let p = arithmetic.fma("57f16d/polynomial FMA6", p, reduced, POLY_HALF.to_bits())
                .map_err(SourceF64ExpError::Arithmetic)?;
            let square = arithmetic.multiply("57f176/polynomial square VMULSD", reduced, reduced)
                .map_err(SourceF64ExpError::Arithmetic)?;
            arithmetic.fma("57f17a/polynomial final FMA", square, p, reduced).map_err(SourceF64ExpError::Arithmetic)?
        } else {
            let upper = arithmetic.multiply("57ef3a/polynomial MULSD720", ONE_720, reduced)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let lower = arithmetic.multiply("57ef4a/polynomial MULSD6", POLY_THIRD.to_bits(), reduced)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let upper = arithmetic.add("57ef4e/polynomial ADDSD120", upper, ONE_120)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let square = arithmetic.multiply("57ef56/polynomial square MULSD", reduced, reduced)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let lower = arithmetic.add("57ef5a/polynomial ADDSDhalf", lower, POLY_HALF.to_bits())
                .map_err(SourceF64ExpError::Arithmetic)?;
            let fourth = arithmetic.multiply("57ef66/polynomial fourth MULSD", square, square)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let upper = arithmetic.multiply("57ef6a/polynomial upper MULSD", upper, reduced)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let lower = arithmetic.multiply("57ef6e/polynomial lower MULSD", lower, square)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let upper = arithmetic.add("57ef72/polynomial ADDSD24", upper, ONE_24)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let lower = arithmetic.add("57ef7a/polynomial ADDSDlinear", lower, reduced)
                .map_err(SourceF64ExpError::Arithmetic)?;
            let upper = arithmetic.multiply("57ef7e/polynomial upper fourth MULSD", upper, fourth)
                .map_err(SourceF64ExpError::Arithmetic)?;
            arithmetic.add("57ef82/polynomial final ADDSD", lower, upper).map_err(SourceF64ExpError::Arithmetic)?
        };
        let index = (integer as u32 & 63) as usize;
        let expanded = arithmetic.multiply("57ee70/table MULSD", polynomial, EXP_TABLE[index].to_bits())
            .map_err(SourceF64ExpError::Arithmetic)?;
        let expanded = arithmetic.add("57ee70/table tail ADDSD", expanded, EXP_TAIL[index])
            .map_err(SourceF64ExpError::Arithmetic)?;
        let expanded = arithmetic.add("57ee70/table head ADDSD", expanded, EXP_HEAD[index])
            .map_err(SourceF64ExpError::Arithmetic)?;

        let scale = ((i64::from(integer >> 6)) as u64).wrapping_shl(52);
        Ok((expanded.wrapping_add(scale), arithmetic.state))
    }
}

pub(super) fn truncate_bounded_count(bits: u64, advanced: ExplicitExceptionState)
    -> Result<(i32, u64, u32), SourceF64ExpError>
{
    truncate_count_with_exponent_limit(bits, advanced, 7)
}

pub(super) fn truncate_positive_trig_count(bits: u64, advanced: ExplicitExceptionState)
    -> Result<(i32, u64, u32), SourceF64ExpError>
{
    if bits & SIGN != 0 { return Err(SourceF64ExpError::ReductionRequired { scaled_bits: bits, advanced }); }
    truncate_count_with_exponent_limit(bits, advanced, 23)
}

fn truncate_count_with_exponent_limit(bits: u64, advanced: ExplicitExceptionState, limit: i32)
    -> Result<(i32, u64, u32), SourceF64ExpError>
{
    let magnitude = bits & !SIGN;
    let exponent = ((magnitude >> 52) & 0x7ff) as i32 - 1023;
    if exponent > limit { return Err(SourceF64ExpError::ReductionRequired { scaled_bits: bits, advanced }); }
    let (truncated, flags) = if exponent < 0 {
        (bits & SIGN, if magnitude == 0 { 0 } else { 0x20 })
    } else {
        let mask = (1u64 << (52 - exponent)) - 1;
        (bits & !mask, if magnitude & mask != 0 { 0x20 } else { 0 })
    };
    let (integer, conversion_flags) = rne_i32_lane(truncated);
    Ok((integer, truncated, flags | conversion_flags))
}

const EXP_HEAD: [u64; 64] = [
    0x3ff0000000000000, 0x3ff02c9a30000000, 0x3ff059b0d0000000, 0x3ff0874510000000,
    0x3ff0b55860000000, 0x3ff0e3ec30000000, 0x3ff11301d0000000, 0x3ff1429aa0000000,
    0x3ff172b830000000, 0x3ff1a35be0000000, 0x3ff1d48730000000, 0x3ff2063b80000000,
    0x3ff2387a60000000, 0x3ff26b4560000000, 0x3ff29e9df0000000, 0x3ff2d285a0000000,
    0x3ff306fe00000000, 0x3ff33c08b0000000, 0x3ff371a730000000, 0x3ff3a7db30000000,
    0x3ff3dea640000000, 0x3ff4160a20000000, 0x3ff44e0860000000, 0x3ff486a2b0000000,
    0x3ff4bfdad0000000, 0x3ff4f9b270000000, 0x3ff5342b50000000, 0x3ff56f4730000000,
    0x3ff5ab07d0000000, 0x3ff5e76f10000000, 0x3ff6247eb0000000, 0x3ff6623880000000,
    0x3ff6a09e60000000, 0x3ff6dfb230000000, 0x3ff71f75e0000000, 0x3ff75feb50000000,
    0x3ff7a11470000000, 0x3ff7e2f330000000, 0x3ff8258990000000, 0x3ff868d990000000,
    0x3ff8ace540000000, 0x3ff8f1ae90000000, 0x3ff93737b0000000, 0x3ff97d8290000000,
    0x3ff9c49180000000, 0x3ffa0c6670000000, 0x3ffa5503b0000000, 0x3ffa9e6b50000000,
    0x3ffae89f90000000, 0x3ffb33a2b0000000, 0x3ffb7f76f0000000, 0x3ffbcc1e90000000,
    0x3ffc199bd0000000, 0x3ffc67f120000000, 0x3ffcb720d0000000, 0x3ffd072d40000000,
    0x3ffd5818d0000000, 0x3ffda9e600000000, 0x3ffdfc9730000000, 0x3ffe502ee0000000,
    0x3ffea4afa0000000, 0x3ffefa1be0000000, 0x3fff507650000000, 0x3fffa7c180000000,
];
const EXP_TAIL: [u64; 64] = [
    0x0000000000000000, 0x3e6cef00c1dcdef9, 0x3e48ac2ba1d73e2a, 0x3e60eb37901186be,
    0x3e69f3121ec53172, 0x3e469e8d10103a17, 0x3df25b50a4ebbf1a, 0x3e6d525bbf668203,
    0x3e68faa2f5b9bef9, 0x3e66df96ea796d31, 0x3e368b9aa7805b80, 0x3e60c519ac771dd6,
    0x3e6ceac470cd83f5, 0x3e5789f37495e99c, 0x3e547f7b84b09745, 0x3e5b900c2d002475,
    0x3e64636e2a5bd1ab, 0x3e4320b7fa64e430, 0x3e5ceaa72a9c5154, 0x3e53967fdba86f24,
    0x3e682468446b6824, 0x3e3f72e29f84325b, 0x3e18624b40c4dbd0, 0x3e5704f3404f068e,
    0x3e54d8a89c750e5e, 0x3e5a74b29ab4cf62, 0x3e5a753e077c2a0f, 0x3e5ad49f699bb2c0,
    0x3e6a90a852b19260, 0x3e56b48521ba6f93, 0x3e0d2ac258f87d03, 0x3e42a91124893ecf,
    0x3e59fcef32422cbe, 0x3e68ca345de441c5, 0x3e61d8bee7ba46e1, 0x3e59099f22fdba6a,
    0x3e4f580c36bea881, 0x3e5b3d398841740a, 0x3e62999c25159f11, 0x3e668925d901c83b,
    0x3e415506dadd3e2a, 0x3e622aee6c57304e, 0x3e29b8bc9e8a0387, 0x3e6fbc9c9f173d24,
    0x3e451f8480e3e235, 0x3e66bbcac96535b5, 0x3e41f12ae45a1224, 0x3e55e7f6fd0fac90,
    0x3e62b5a75abd0e69, 0x3e609e2bf5ed7fa1, 0x3e47daf237553d84, 0x3e12f074891ee83d,
    0x3e6b0aa538444196, 0x3e6cafa29694426f, 0x3e69df20d22a0797, 0x3e640f12f71a1e45,
    0x3e69f7490e4bb40b, 0x3e4ed9942b84600d, 0x3e4bdcdaf5cb4656, 0x3e5e2cffd89cf44c,
    0x3e452486cc2c7b9d, 0x3e6cc2b44eee3fa4, 0x3e66dc8a80ce9f09, 0x3e39e90d82e90a7e,
];
