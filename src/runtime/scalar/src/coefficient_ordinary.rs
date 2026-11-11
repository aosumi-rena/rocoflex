use super::finite_power::CoefficientScaleResult;
use super::shared_scalar_f32::comparison::F32Operand;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCoefficientExpOperands {
    pub input_bits: u32,
    pub multiplier_bits: u32,
    pub exponent2: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCoefficientOrdinaryDomainRequired { FiniteOperands, OuterEntryBranch }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCoefficientOrdinaryDomainError {
    pub operands: SourceCoefficientExpOperands,
    pub required: SourceCoefficientOrdinaryDomainRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCoefficientOrdinaryArguments(SourceCoefficientExpOperands);

impl SourceCoefficientOrdinaryArguments {
    pub fn from_finite_bits(input_bits: u32, multiplier_bits: u32, exponent2: i16)
        -> Result<Self, SourceCoefficientOrdinaryDomainError>
    {
        let operands = SourceCoefficientExpOperands { input_bits, multiplier_bits, exponent2 };
        let input = F32Operand::from_bits(input_bits);
        let multiplier = F32Operand::from_bits(multiplier_bits);
        if input.magnitude() >= 0x7f80_0000 || multiplier.magnitude() >= 0x7f80_0000 {
            return Err(SourceCoefficientOrdinaryDomainError {
                operands, required: SourceCoefficientOrdinaryDomainRequired::FiniteOperands });
        }
        if input.magnitude() > 0x4365_0000 || multiplier.magnitude() == 0 {
            return Err(SourceCoefficientOrdinaryDomainError {
                operands, required: SourceCoefficientOrdinaryDomainRequired::OuterEntryBranch });
        }
        Ok(Self(operands))
    }
    pub fn operands(self) -> SourceCoefficientExpOperands { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCoefficientBinaryOpcode { Addss, Subss, Mulss, Divss }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCoefficientComparisonFlags {
    pub carry: bool,
    pub parity: bool,
    pub zero: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCoefficientOrdinaryRequest {
    Binary { site: u32, opcode: SourceCoefficientBinaryOpcode, left: u32, right: u32 },
    Comiss { site: u32, left: u32, right: u32 },
    Cvttss2siI32 { site: u32, source: u32 },
    Cvtdq2psXmm { site: u32, source: [i32; 4] },
    Scale562bf0 { site: u32, bits: u32, shift: i32 },
}

pub trait SourceCoefficientOrdinaryServices {
    type Error;
    fn binary(&mut self, site: u32, opcode: SourceCoefficientBinaryOpcode,
        left: u32, right: u32) -> Result<u32, Self::Error>;
    fn comiss(&mut self, site: u32, left: u32, right: u32)
        -> Result<SourceCoefficientComparisonFlags, Self::Error>;
    fn cvttss2si_i32(&mut self, site: u32, source: u32) -> Result<u32, Self::Error>;
    fn cvtdq2ps_xmm(&mut self, site: u32, source: [i32; 4]) -> Result<[u32; 4], Self::Error>;
    fn scale_562bf0(&mut self, site: u32, bits: u32, shift: i32)
        -> Result<CoefficientScaleResult, Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceCoefficientOrdinaryFailure<E> {
    pub arguments: SourceCoefficientOrdinaryArguments,
    pub request: SourceCoefficientOrdinaryRequest,
    pub error: E,
}

fn invoke<S: SourceCoefficientOrdinaryServices, T>(services: &mut S,
    arguments: SourceCoefficientOrdinaryArguments, request: SourceCoefficientOrdinaryRequest,
    call: impl FnOnce(&mut S) -> Result<T, S::Error>)
    -> Result<T, SourceCoefficientOrdinaryFailure<S::Error>>
{
    call(services).map_err(|error| SourceCoefficientOrdinaryFailure { arguments, request, error })
}

fn binary<S: SourceCoefficientOrdinaryServices>(services: &mut S,
    arguments: SourceCoefficientOrdinaryArguments, site: u32,
    opcode: SourceCoefficientBinaryOpcode, left: u32, right: u32)
    -> Result<u32, SourceCoefficientOrdinaryFailure<S::Error>>
{
    invoke(services, arguments, SourceCoefficientOrdinaryRequest::Binary { site, opcode, left, right },
        |services| services.binary(site, opcode, left, right))
}

fn comiss<S: SourceCoefficientOrdinaryServices>(services: &mut S,
    arguments: SourceCoefficientOrdinaryArguments, site: u32, left: u32, right: u32)
    -> Result<SourceCoefficientComparisonFlags, SourceCoefficientOrdinaryFailure<S::Error>>
{
    invoke(services, arguments, SourceCoefficientOrdinaryRequest::Comiss { site, left, right },
        |services| services.comiss(site, left, right))
}

pub fn run_coefficient_ordinary<S: SourceCoefficientOrdinaryServices>(services: &mut S,
    arguments: SourceCoefficientOrdinaryArguments)
    -> Result<CoefficientScaleResult, SourceCoefficientOrdinaryFailure<S::Error>>
{
    use self::SourceCoefficientBinaryOpcode::{Addss, Subss, Mulss, Divss};
    let source = arguments.operands();
    let scaled = binary(services, arguments, 0x56287c, Mulss, source.input_bits, 0x3fb8_aa3b)?;
    let flags = comiss(services, arguments, 0x562884, 0, scaled)?;

    let bias = if flags.carry || flags.zero { 0x3f00_0000 } else { 0xbf00_0000 };
    let biased = binary(services, arguments, 0x5628a3, Addss, bias, scaled)?;
    let count = invoke(services, arguments,
        SourceCoefficientOrdinaryRequest::Cvttss2siI32 { site: 0x5628a7, source: biased },
        |services| services.cvttss2si_i32(0x5628a7, biased))?;
    let mut count_word = count as u16;
    let lanes = [i32::from(count_word as i16), 0, 0, 0];
    let converted = invoke(services, arguments,
        SourceCoefficientOrdinaryRequest::Cvtdq2psXmm { site: 0x5628b2, source: lanes },
        |services| services.cvtdq2ps_xmm(0x5628b2, lanes))?;
    let low = binary(services, arguments, 0x5628b8, Mulss, converted[0], 0x35bf_be8e)?;
    let high = binary(services, arguments, 0x5628c0, Mulss, converted[0], 0x3f31_7200)?;
    let first = binary(services, arguments, 0x5628c8, Subss, source.input_bits, high)?;
    let reduced = binary(services, arguments, 0x5628d6, Subss, first, low)?;
    let lower = comiss(services, arguments, 0x5628da, reduced, 0xb300_0000)?;
    let rational = if lower.carry || lower.zero { true } else {
        let upper = comiss(services, arguments, 0x5628df, 0x3300_0000, reduced)?;

        upper.carry || upper.zero
    };
    let fraction = if rational {
        count_word = count_word.wrapping_sub(1);
        let square = binary(services, arguments, 0x5628ea, Mulss, reduced, reduced)?;
        let denominator_product = binary(services, arguments, 0x5628f1, Mulss, square, 0x4140_3e28)?;
        let numerator_factor = binary(services, arguments, 0x5628f9, Addss, square, 0x4270_5d55)?;
        let denominator = binary(services, arguments, 0x562901, Addss, denominator_product, 0x42f0_5d55)?;
        let numerator = binary(services, arguments, 0x562909, Mulss, reduced, numerator_factor)?;
        let sum = binary(services, arguments, 0x562910, Addss, denominator, numerator)?;
        let difference = binary(services, arguments, 0x562914, Subss, denominator, numerator)?;
        let ratio = binary(services, arguments, 0x562918, Divss, sum, difference)?;
        let doubled = binary(services, arguments, 0x56291c, Addss, ratio, ratio)?;
        binary(services, arguments, 0x562920, Mulss, doubled, source.multiplier_bits)?
    } else { source.multiplier_bits };
    let shift = i32::from(count_word as i16) + i32::from(source.exponent2);
    invoke(services, arguments,
        SourceCoefficientOrdinaryRequest::Scale562bf0 { site: 0x562bf0, bits: fraction, shift },
        |services| services.scale_562bf0(0x562bf0, fraction, shift))
}
