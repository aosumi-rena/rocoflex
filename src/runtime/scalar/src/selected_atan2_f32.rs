use super::finite_power::{ExplicitExceptionState, MathErrorCallbackCompletion, round_exact_normal_u128};
use super::shared_scalar_f32::ScalarF32Architecture;
use super::vorbis_power::{Arithmetic, VorbisScalarError};

const SIGN64: u64 = 0x8000_0000_0000_0000;
const SIGN32: u32 = 0x8000_0000;
const PI_F32: u32 = 0x4049_0fdb;
const HALF_PI_F32: u32 = 0x3fc9_0fdb;
const HALF_PI_F64: u64 = 0x3ff9_21fb_5444_2d18;
const PI_F64: u64 = 0x4009_21fb_5444_2d18;
const GRID_SCALE: u64 = 0x4070_0000_0000_0000;
const HALF: u64 = 0x3fe0_0000_0000_0000;
const GRID_LIMIT: u64 = 0x3fb0_0000_0000_0000;
const SMALL_LIMIT: u64 = 0x3f1a_36e2_eb1c_432d;
const TABLE_CUBIC: u64 = 0x3fd5_5555_5555_0877;
const SMALL_FIFTH: u64 = 0x3fc9_9999_9996_43a3;
const SMALL_SEVENTH: u64 = 0x3fc2_4924_82bd_6be1;
const SMALL_THIRD: u64 = 0x3fd5_5555_5555_5538;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceAtan2F32Error {
    ArchitectureNotClosed,
    UnsupportedEnvironment { mxcsr: u32 },
    NonFinitePairRequired { numerator_bits: u32, denominator_bits: u32 },
    TableIndexContinuationRequired { grid_bits: u64, advanced: ExplicitExceptionState },
    Arithmetic(VorbisScalarError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceAtan2F32CallReceipt {
    pub numerator_bits: u32,
    pub denominator_bits: u32,
    pub entry: ExplicitExceptionState,
    pub advanced: ExplicitExceptionState,
    pub result_bits: Option<u32>,
    pub error: Option<SourceAtan2F32Error>,
}

impl SourceAtan2F32Error {
    pub fn reason(self) -> &'static str {
        match self {
            Self::ArchitectureNotClosed => "57d678 explicit arithmetic architecture is not source-closed",
            Self::UnsupportedEnvironment { .. } => "57d678 state is not RNE/all-masked/gradual",
            Self::NonFinitePairRequired { .. } => "57d678 nonfinite pair requires its separate source continuation",
            Self::TableIndexContinuationRequired { .. } => "57d678 retained table index continuation required",
            Self::Arithmetic(error) => error.reason(),
        }
    }
    pub fn known_advanced_state(self, entry: ExplicitExceptionState) -> ExplicitExceptionState {
        match self {
            Self::TableIndexContinuationRequired { advanced, .. } => advanced,
            Self::Arithmetic(error) => error.known_advanced_state(entry),
            _ => entry,
        }
    }
}

pub struct SourceAtan2F32 {
    architecture: ScalarF32Architecture,
    state: ExplicitExceptionState,
    last_call: Option<SourceAtan2F32CallReceipt>,
    first_failure: Option<SourceAtan2F32CallReceipt>,
}

impl SourceAtan2F32 {
    pub fn bind(architecture: ScalarF32Architecture, state: ExplicitExceptionState)
        -> Result<Self, SourceAtan2F32Error>
    {
        if architecture != ScalarF32Architecture::IntelSdm { return Err(SourceAtan2F32Error::ArchitectureNotClosed); }
        if state.mxcsr() & 0xffff_ffc0 != 0x1f80 {
            return Err(SourceAtan2F32Error::UnsupportedEnvironment { mxcsr: state.mxcsr() });
        }
        Ok(Self { architecture, state, last_call: None, first_failure: None })
    }
    pub fn name(&self) -> &'static str { "source57d678/two-f32/finite/separate" }
    pub fn architecture(&self) -> ScalarF32Architecture { self.architecture }
    pub fn exception_state(&self) -> ExplicitExceptionState { self.state }
    pub fn last_call(&self) -> Option<&SourceAtan2F32CallReceipt> { self.last_call.as_ref() }

    pub fn first_failure(&self) -> Option<&SourceAtan2F32CallReceipt> { self.first_failure.as_ref() }
    pub fn atan2_f32(&mut self, numerator: f32, denominator: f32) -> Result<f32, SourceAtan2F32Error> {
        self.atan2_f32_bits(numerator.to_bits(), denominator.to_bits()).map(f32::from_bits)
    }
    pub fn atan2_f32_bits(&mut self, numerator: u32, denominator: u32) -> Result<u32, SourceAtan2F32Error> {
        let entry = self.state;
        let result = self.evaluate_source_bits(numerator, denominator);
        let receipt = SourceAtan2F32CallReceipt { numerator_bits: numerator, denominator_bits: denominator,
            entry, advanced: self.state, result_bits: result.as_ref().ok().copied(),
            error: result.as_ref().err().copied() };
        self.last_call = Some(receipt);
        if result.is_err() && self.first_failure.is_none() { self.first_failure = Some(receipt); }
        result
    }

    fn evaluate_source_bits(&mut self, numerator: u32, denominator: u32) -> Result<u32, SourceAtan2F32Error> {
        if (numerator & !SIGN32) >= 0x7f80_0000 || (denominator & !SIGN32) >= 0x7f80_0000 {
            return Err(SourceAtan2F32Error::NonFinitePairRequired { numerator_bits: numerator, denominator_bits: denominator });
        }
        let entry = self.state;
        match self.graph(numerator, denominator) {
            Ok((bits, advanced)) => { self.state = advanced; Ok(bits) }
            Err(error) => { self.state = error.known_advanced_state(entry); Err(error) }
        }
    }

    fn narrow(bits: u64, arithmetic: Arithmetic) -> Result<(u32, ExplicitExceptionState), SourceAtan2F32Error> {
        let result = MathErrorCallbackCompletion { result_f64_bits: bits, advanced: arithmetic.state }
            .into_rne_f32().map_err(|required| SourceAtan2F32Error::Arithmetic(VorbisScalarError::ConversionRequired(required)))?;
        Ok((result.bits, result.advanced))
    }
    fn fixed(bits: u32, state: ExplicitExceptionState, flags: u32) -> (u32, ExplicitExceptionState) {

        (bits, state.with_source_status(flags))
    }

    fn graph(&self, numerator: u32, denominator: u32) -> Result<(u32, ExplicitExceptionState), SourceAtan2F32Error> {

        let (x, advanced) = self.state.widen_f32_for_math_error(denominator);
        let (y, advanced) = advanced.widen_f32_for_math_error(numerator);
        let mut arithmetic = Arithmetic { state: advanced };
        let ax = x & !SIGN64;
        let ay = y & !SIGN64;
        let ysign = numerator & SIGN32;
        let negative_x = x & SIGN64 != 0;

        if ay == 0 {
            return if negative_x { Ok(Self::fixed(PI_F32 | ysign, arithmetic.state, 0x20)) }
                else { Self::narrow(y, arithmetic) };
        }
        if ax == 0 {

            return Ok(Self::fixed(HALF_PI_F32 | ysign, arithmetic.state, 0x20));
        }
        let exponent_difference = ((ay >> 52) as i32) - ((ax >> 52) as i32);
        if exponent_difference > 26 {
            return Ok(Self::fixed(HALF_PI_F32 | ysign, arithmetic.state, 0x20));
        }
        if exponent_difference < -13 && !negative_x {
            if exponent_difference < -150 {

                return Ok(Self::fixed(ysign, arithmetic.state, 0x30));
            }
            let ratio = if exponent_difference < -126 {

                let scaled = arithmetic.multiply("57d7ff/tiny ratio scaled MULSD", 0x4630_0000_0000_0000, y)
                    .map_err(SourceAtan2F32Error::Arithmetic)?;
                let scaled = arithmetic.divide("57d803/tiny ratio scaled DIVSD", scaled, x)
                    .map_err(SourceAtan2F32Error::Arithmetic)?;
                let field = ((scaled & !SIGN64) >> 52) - 100;
                (scaled & 0x800f_ffff_ffff_ffff) | (field << 52)
            } else {
                arithmetic.divide("57d8b0/tiny ratio DIVSD", y, x).map_err(SourceAtan2F32Error::Arithmetic)?
            };

            return Self::narrow(ratio, arithmetic);
        }
        if exponent_difference < -26 && negative_x {
            return Ok(Self::fixed(PI_F32 | ysign, arithmetic.state, 0x20));
        }

        let swapped = ay > ax;
        let (smaller, larger) = if swapped { (ax, ay) } else { (ay, ax) };
        let ratio = arithmetic.divide("57d956/general ratio DIVSD", smaller, larger)
            .map_err(SourceAtan2F32Error::Arithmetic)?;
        let mut angle = if ratio > GRID_LIMIT {

            let grid = arithmetic.multiply("57d96f/grid MULSD", ratio, GRID_SCALE)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let scaled_smaller = arithmetic.multiply("57d976/numerator scale MULSD", smaller, GRID_SCALE)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let grid = arithmetic.add("57d97a/grid half ADDSD", grid, HALF)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let (integer, flags) = truncate_grid(grid).ok_or(SourceAtan2F32Error::TableIndexContinuationRequired {
                grid_bits: grid, advanced: arithmetic.state,
            })?;
            arithmetic.state = arithmetic.state.with_source_status(flags);
            let count = round_exact_normal_u128(u128::from(integer), 0).0;
            let numerator_product = arithmetic.multiply("57d99a/count numerator MULSD", count, smaller)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let denominator_product = arithmetic.multiply("57d99e/count denominator MULSD", count, larger)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let scaled_larger = arithmetic.multiply("57d9a2/denominator scale MULSD", larger, GRID_SCALE)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let residual_numerator = arithmetic.subtract("57d9a6/reduction numerator SUBSD", scaled_smaller, denominator_product)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let residual_denominator = arithmetic.add("57d9aa/reduction denominator ADDSD", numerator_product, scaled_larger)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let reduced = arithmetic.divide("57d9ae/reduction DIVSD", residual_numerator, residual_denominator)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let table_angle = arithmetic.add("57d9b8/table ADDSD", reduced, ATAN_TABLE[(integer - 16) as usize])
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let square = arithmetic.multiply("57d9bd/table square MULSD", reduced, reduced)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let cube = arithmetic.multiply("57d9c1/table cube MULSD", square, reduced)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let correction = arithmetic.multiply("57d9c5/table correction MULSD", cube, TABLE_CUBIC)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            arithmetic.subtract("57da13/table correction SUBSD", table_angle, correction).map_err(SourceAtan2F32Error::Arithmetic)?
        } else if ratio < SMALL_LIMIT { ratio }
        else {
            let square = arithmetic.multiply("57d9e8/small square MULSD", ratio, ratio)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let high = arithmetic.multiply("57d9ef/small seventh MULSD", square, SMALL_SEVENTH)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let p = arithmetic.subtract("57d9f7/small fifth SUBSD", SMALL_FIFTH, high)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let p = arithmetic.multiply("57da03/small polynomial MULSD", p, square)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let cube = arithmetic.multiply("57da07/small cube MULSD", square, ratio)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let p = arithmetic.subtract("57da0b/small third SUBSD", SMALL_THIRD, p)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            let correction = arithmetic.multiply("57da0f/small correction MULSD", p, cube)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
            arithmetic.subtract("57da13/small correction SUBSD", ratio, correction).map_err(SourceAtan2F32Error::Arithmetic)?
        };
        if swapped {
            angle = arithmetic.subtract("57da23/swapped half-pi SUBSD", HALF_PI_F64, angle)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
        }
        if negative_x {
            angle = arithmetic.subtract("57da37/negative-x pi SUBSD", PI_F64, angle)
                .map_err(SourceAtan2F32Error::Arithmetic)?;
        }
        if ysign != 0 { angle ^= SIGN64; }
        Self::narrow(angle, arithmetic)
    }
}

fn truncate_grid(bits: u64) -> Option<(u32, u32)> {
    let exponent = (bits >> 52) as i32 - 1023;
    if !(4..=8).contains(&exponent) { return None; }
    let discard = (52 - exponent) as u32;
    let significand = (bits & 0x000f_ffff_ffff_ffff) | (1u64 << 52);
    let integer = (significand >> discard) as u32;
    if !(16..=256).contains(&integer) { return None; }
    let flags = if significand & ((1u64 << discard) - 1) != 0 { 0x20 } else { 0 };
    Some((integer, flags))
}

const ATAN_TABLE: [u64; 241] = [
    0x3faff55bb72cfde9, 0x3fb0f99ea71d52a6, 0x3fb1f86dbf082d58, 0x3fb2f719318a4a9a,
    0x3fb3f59f0e7c559d, 0x3fb4f3fd677292fb, 0x3fb5f2324fd2d7b2, 0x3fb6f03bdcea4b0c,
    0x3fb7ee182602f10e, 0x3fb8ebc54478fb28, 0x3fb9e94153cfdcf1, 0x3fbae68a71c722b8,
    0x3fbbe39ebe6f07c3, 0x3fbce07c5c3cca32, 0x3fbddd21701eba6e, 0x3fbed98c2190043a,
    0x3fbfd5ba9aac2f6d, 0x3fc068d584212b3d, 0x3fc0e6adccf40881, 0x3fc1646541060850,
    0x3fc1e1fafb043726, 0x3fc25f6e171a535c, 0x3fc2dcbdb2fba1ff, 0x3fc359e8edeb99a3,
    0x3fc3d6eee8c6626c, 0x3fc453cec6092a9e, 0x3fc4d087a9da4f17, 0x3fc54d18ba11570a,
    0x3fc5c9811e3ec269, 0x3fc645bfffb3aa73, 0x3fc6c1d4898933d8, 0x3fc73dbde8a7d201,
    0x3fc7b97b4bce5b02, 0x3fc8350be398ebc7, 0x3fc8b06ee2879c28, 0x3fc92ba37d050271,
    0x3fc9a6a8e96c8626, 0x3fca217e601081a5, 0x3fca9c231b403279, 0x3fcb1696574d780b,
    0x3fcb90d7529260a2, 0x3fcc0ae54d768466, 0x3fcc84bf8a742e6d, 0x3fccfe654e1d5395,
    0x3fcd77d5df205736, 0x3fcdf110864c9d9d, 0x3fce6a148e96ec4d, 0x3fcee2e1451d980c,
    0x3fcf5b75f92c80dd, 0x3fcfd3d1fc40dbe4, 0x3fd025fa510665b5, 0x3fd061eea03d6290,
    0x3fd09dc597d86362, 0x3fd0d97ee509acb3, 0x3fd1151a362431c9, 0x3fd150973a9ce546,
    0x3fd18bf5a30bf178, 0x3fd1c735212dd883, 0x3fd2025567e47c95, 0x3fd23d562b381041,
    0x3fd278372057ef45, 0x3fd2b2f7fd9b5fe2, 0x3fd2ed987a823cfe, 0x3fd328184fb58951,
    0x3fd362773707ebcb, 0x3fd39cb4eb76157b, 0x3fd3d6d129271134, 0x3fd410cbad6c7d32,
    0x3fd44aa436c2af09, 0x3fd4845a84d0c21b, 0x3fd4bdee586890e6, 0x3fd4f75f73869978,
    0x3fd530ad9951cd49, 0x3fd569d88e1b4cd7, 0x3fd5a2e0175e0f4e, 0x3fd5dbc3fbbe768d,
    0x3fd614840309cfe1, 0x3fd64d1ff635c1c5, 0x3fd685979f5fa6fd, 0x3fd6bdeac9cbd76c,
    0x3fd6f61941e4def0, 0x3fd72e22d53aa2a9, 0x3fd7660752817501, 0x3fd79dc6899118d1,
    0x3fd7d5604b63b3f7, 0x3fd80cd46a14b1d0, 0x3fd84422b8df95d7, 0x3fd87b4b0c1ebedb,
    0x3fd8b24d394a1b25, 0x3fd8e92916f5cde8, 0x3fd91fde7cd0c662, 0x3fd9566d43a34907,
    0x3fd98cd5454d6b18, 0x3fd9c3165cc58107, 0x3fd9f93066168001, 0x3fda2f233e5e530b,
    0x3fda64eec3cc23fc, 0x3fda9a92d59e98cf, 0x3fdad00f5422058b, 0x3fdb056420ae9343,
    0x3fdb3a911da65c6c, 0x3fdb6f962e737efb, 0x3fdba473378624a5, 0x3fdbd9281e528191,
    0x3fdc0db4c94ec9ef, 0x3fdc42191ff11eb6, 0x3fdc76550aad71f8, 0x3fdcaa6872f3631b,
    0x3fdcde53432c1350, 0x3fdd121566b7f2ad, 0x3fdd45aec9ec862b, 0x3fdd791f5a1226f4,
    0x3fddac670561bb4f, 0x3fdddf85bb026974, 0x3fde127b6b0744af, 0x3fde4548066cf51a,
    0x3fde77eb7f175a34, 0x3fdeaa65c7cf28c4, 0x3fdedcb6d43f8434, 0x3fdf0ede98f393cf,
    0x3fdf40dd0b541417, 0x3fdf72b221a4e495, 0x3fdfa45dd3029258, 0x3fdfd5e0175fdf83,
    0x3fe0039c73c1a40b, 0x3fe01c341e82422d, 0x3fe034b709250488, 0x3fe04d25314342e5,
    0x3fe0657e94db30cf, 0x3fe07dc3324e9b38, 0x3fe095f30861a58f, 0x3fe0ae0e1639866c,
    0x3fe0c6145b5b43da, 0x3fe0de05d7aa6f7c, 0x3fe0f5e28b67e295, 0x3fe10daa77307a0d,
    0x3fe1255d9bfbd2a8, 0x3fe13cfbfb1b056e, 0x3fe1548596376469, 0x3fe16bfa6f5137e1,
    0x3fe1835a88be7c13, 0x3fe19aa5e5299f99, 0x3fe1b1dc87904284, 0x3fe1c8fe7341f64f,
    0x3fe1e00babdefeb3, 0x3fe1f7043557138a, 0x3fe20de813e823b1, 0x3fe224b74c1d192a,
    0x3fe23b71e2cc9e6a, 0x3fe25217dd17e501, 0x3fe268a940696da6, 0x3fe27f261273d1b3,
    0x3fe2958e59308e30, 0x3fe2abe21aded073, 0x3fe2c2215e024465, 0x3fe2d84c2961e48b,
    0x3fe2ee628406cbca, 0x3fe30464753b090a, 0x3fe31a52048874be, 0x3fe3302b39b78856,
    0x3fe345f01cce37bb, 0x3fe35ba0b60eccce, 0x3fe3713d0df6c503, 0x3fe386c52d3db11e,
    0x3fe39c391cd41719, 0x3fe3b198e5e2564a, 0x3fe3c6e491c78dc4, 0x3fe3dc1c2a188504,
    0x3fe3f13fb89e96f4, 0x3fe4064f47569f48, 0x3fe41b4ae06fea41, 0x3fe430328e4b26d5,
    0x3fe445065b795b55, 0x3fe459c652badc7f, 0x3fe46e727efe4715, 0x3fe4830aeb5f7bfd,
    0x3fe4978fa3269ee1, 0x3fe4ac00b1c71762, 0x3fe4c05e22de94e4, 0x3fe4d4a8023414e8,
    0x3fe4e8de5bb6ec04, 0x3fe4fd013b7dd17e, 0x3fe51110adc5ed81, 0x3fe5250cbef1e9fa,
    0x3fe538f57b89061e, 0x3fe54ccaf0362c8f, 0x3fe5608d29c70c34, 0x3fe5743c352b33b9,
    0x3fe587d81f732fba, 0x3fe59b60f5cfab9d, 0x3fe5aed6c5909517, 0x3fe5c2399c244260,
    0x3fe5d58987169b18, 0x3fe5e8c6941043cf, 0x3fe5fbf0d0d5cc49, 0x3fe60f084b46e05e,
    0x3fe6220d115d7b8d, 0x3fe634ff312d1f3b, 0x3fe647deb8e20b8f, 0x3fe65aabb6c07b02,
    0x3fe66d663923e086, 0x3fe6800e4e7e2857, 0x3fe692a40556fb6a, 0x3fe6a5276c4b0575,
    0x3fe6b798920b3d98, 0x3fe6c9f7855c3198, 0x3fe6dc44551553ae, 0x3fe6ee7f10204aef,
    0x3fe700a7c5784633, 0x3fe712be84295198, 0x3fe724c35b4fae7b, 0x3fe736b65a172dff,
    0x3fe748978fba8e0f, 0x3fe75a670b82d8d8, 0x3fe76c24dcc6c6c0, 0x3fe77dd112ea22c7,
    0x3fe78f6bbd5d315e, 0x3fe7a0f4eb9c19a2, 0x3fe7b26cad2e50fd, 0x3fe7c3d311a6092b,
    0x3fe7d528289fa093, 0x3fe7e66c01c114fd, 0x3fe7f79eacb97898, 0x3fe808c03940694a,
    0x3fe819d0b7158a4c, 0x3fe82ad036000005, 0x3fe83bbec5cdee22, 0x3fe84c9c7653f7ea,
    0x3fe85d69576cc2c5, 0x3fe86e2578f87ae5, 0x3fe87ed0eadc5a2a, 0x3fe88f6bbd023118,
    0x3fe89ff5ff57f1f7, 0x3fe8b06fc1cf3dfe, 0x3fe8c0d9145cf49d, 0x3fe8d13206f8c4ca,
    0x3fe8e17aa99cc05d, 0x3fe8f1b30c44f167, 0x3fe901db3eeef187, 0x3fe911f35199833b,
    0x3fe921fb54442d18,
];
