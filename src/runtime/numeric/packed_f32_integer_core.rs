use super::{Kind1Exceptions, Kind1Mxcsr, Kind1PackedOperation, Kind1PackedOutcome, Kind1Rounding};
use std::cmp::Ordering;

const INVALID: u8 = 1;
const DENORMAL: u8 = 2;
const OVERFLOW: u8 = 8;
const UNDERFLOW: u8 = 16;
const PRECISION: u8 = 32;
const SIGN: u32 = 0x8000_0000;
const INFINITY: u32 = 0x7f80_0000;
const QUIET: u32 = 0x0040_0000;
const INDEFINITE: u32 = 0xffc0_0000;

pub(crate) fn evaluate_packed_instruction(operation: Kind1PackedOperation,
        state: Kind1Mxcsr) -> Kind1PackedOutcome
    {

        let mut prepared = [Prepared::Bits(0, 0); 8];
        let mut pre = 0u8;
        for lane in 0..8 {
            prepared[lane] = match operation {
                Kind1PackedOperation::Vmulps { left, right } =>
                    prepare(LaneOperation::Multiply, left[lane], right[lane], 0, state),
                Kind1PackedOperation::Vaddps { left, right } =>
                    prepare(LaneOperation::Add, left[lane], right[lane], 0, state),
                Kind1PackedOperation::Vfmadd231ps { accumulator, left, right } =>
                    prepare(LaneOperation::Fma, left[lane], right[lane], accumulator[lane], state),
            };
            pre |= prepared[lane].pre_flags();
        }
        if pre & !state.masks().bits() != 0 { return outcome(state, pre, [0; 8]); }
        let mut result = [0u32; 8];
        let mut post = 0u8;
        for lane in 0..8 {
            let rounded = match prepared[lane] {
                Prepared::Bits(bits, _) => Rounded { bits: Some(bits), post: 0 },
                Prepared::Exact(value, _) => round(value, state),
            };

            if let Some(bits) = rounded.bits { result[lane] = bits; }
            post |= rounded.post;
        }
        outcome(state, pre | post, result)
    }

fn outcome(before: Kind1Mxcsr, raised: u8, result: [u32; 8]) -> Kind1PackedOutcome {
    let flags = Kind1Exceptions(raised);
    let state = before.with_raised(flags);
    let unmasked = before.unmasked(flags);
    if unmasked.bits() == 0 { Kind1PackedOutcome::Completed { result, state } }
    else { Kind1PackedOutcome::Trapped { state, unmasked } }
}

#[derive(Clone, Copy)]
enum LaneOperation { Multiply, Add, Fma }
#[derive(Clone, Copy)]
enum Prepared { Bits(u32, u8), Exact(Exact, u8) }
impl Prepared {
    fn pre_flags(self) -> u8 {
        match self { Self::Bits(_, flags) | Self::Exact(_, flags) => flags }
    }
}

#[derive(Clone, Copy)]
struct Operand(u32);
impl Operand {
    fn sign(self) -> bool { self.0 & SIGN != 0 }
    fn fraction(self) -> u32 { self.0 & 0x007f_ffff }
    fn exponent(self) -> u32 { (self.0 >> 23) & 0xff }
    fn nan(self) -> bool { self.exponent() == 255 && self.fraction() != 0 }
    fn signaling(self) -> bool { self.nan() && self.0 & QUIET == 0 }
    fn infinite(self) -> bool { self.0 & !SIGN == INFINITY }
    fn zero(self) -> bool { self.0 & !SIGN == 0 }
    fn denormal(self) -> bool { self.exponent() == 0 && self.fraction() != 0 }
    fn daz(self, state: Kind1Mxcsr) -> Self {
        if state.denormals_are_zero() && self.denormal() { Self(self.0 & SIGN) } else { self }
    }

    fn finite_parts(self) -> (u64, i32) {
        if self.exponent() == 0 { (u64::from(self.fraction()), -149) }
        else { (u64::from(self.fraction() | 0x0080_0000), self.exponent() as i32 - 150) }
    }
    fn exact(self) -> Exact {
        let (significand, scale) = self.finite_parts();
        Exact { negative: self.sign(), magnitude: Magnitude::shifted(significand, (scale + 298) as usize) }
    }
}

fn prepare(op: LaneOperation, a: u32, b: u32, c: u32, state: Kind1Mxcsr) -> Prepared {
    let a = Operand(a).daz(state);
    let b = Operand(b).daz(state);
    let c = Operand(c).daz(state);
    let fused = matches!(op, LaneOperation::Fma);

    let invalid_nan = a.signaling() || b.signaling() || (fused && c.signaling());
    let chosen_nan = if a.nan() { Some(a) } else if b.nan() { Some(b) }
        else if fused && c.nan() { Some(c) } else { None };
    if let Some(nan) = chosen_nan {
        return Prepared::Bits(nan.0 | QUIET, if invalid_nan { INVALID } else { 0 });
    }
    let product_sign = a.sign() ^ b.sign();
    match op {
        LaneOperation::Add => {
            if a.infinite() && b.infinite() && a.sign() != b.sign() {
                return Prepared::Bits(INDEFINITE, INVALID);
            }
        }
        LaneOperation::Multiply | LaneOperation::Fma => {
            if (a.infinite() && b.zero()) || (b.infinite() && a.zero()) {
                return Prepared::Bits(INDEFINITE, INVALID);
            }
            if fused && (a.infinite() || b.infinite()) && c.infinite() && product_sign != c.sign() {
                return Prepared::Bits(INDEFINITE, INVALID);
            }
        }
    }

    let pre = if a.denormal() || b.denormal() || (fused && c.denormal()) { DENORMAL } else { 0 };
    match op {
        LaneOperation::Add => {
            if a.infinite() { return Prepared::Bits(a.0, pre); }
            if b.infinite() { return Prepared::Bits(b.0, pre); }
            Prepared::Exact(a.exact().add(b.exact(), state.rounding()), pre)
        }
        LaneOperation::Multiply | LaneOperation::Fma => {
            if a.infinite() || b.infinite() {
                return Prepared::Bits(INFINITY | if product_sign { SIGN } else { 0 }, pre);
            }
            if fused && c.infinite() { return Prepared::Bits(c.0, pre); }
            let (a_sig, a_scale) = a.finite_parts();
            let (b_sig, b_scale) = b.finite_parts();
            let product = Exact { negative: product_sign,
                magnitude: Magnitude::shifted(a_sig * b_sig, (a_scale + b_scale + 298) as usize) };
            Prepared::Exact(if fused { product.add(c.exact(), state.rounding()) } else { product }, pre)
        }
    }
}

#[derive(Clone, Copy)]
struct Magnitude([u64; 9]);
impl Magnitude {
    fn shifted(value: u64, shift: usize) -> Self {
        let mut words = [0u64; 9];
        let limb = shift / 64;
        let bit = shift % 64;
        words[limb] = value << bit;
        if bit != 0 && limb + 1 < 9 { words[limb + 1] = value >> (64 - bit); }
        Self(words)
    }
    fn highest(self) -> Option<usize> {
        for limb in (0..9).rev() {
            if self.0[limb] != 0 { return Some(limb * 64 + 63 - self.0[limb].leading_zeros() as usize); }
        }
        None
    }
    fn compare(self, other: Self) -> Ordering {
        for limb in (0..9).rev() {
            match self.0[limb].cmp(&other.0[limb]) { Ordering::Equal => (), order => return order }
        }
        Ordering::Equal
    }
    fn add(self, other: Self) -> Self {
        let mut words = [0u64; 9];
        let mut carry = false;
        for limb in 0..9 {
            let (sum, first) = self.0[limb].overflowing_add(other.0[limb]);
            let (sum, second) = sum.overflowing_add(u64::from(carry));
            words[limb] = sum;
            carry = first || second;
        }
        Self(words)
    }

    fn subtract(self, other: Self) -> Self {
        let mut words = [0u64; 9];
        let mut borrow = false;
        for limb in 0..9 {
            let (difference, first) = self.0[limb].overflowing_sub(other.0[limb]);
            let (difference, second) = difference.overflowing_sub(u64::from(borrow));
            words[limb] = difference;
            borrow = first || second;
        }
        Self(words)
    }
    fn bit(self, at: usize) -> bool { self.0[at / 64] & (1u64 << (at % 64)) != 0 }

    fn any_below(self, limit: usize) -> bool {
        let whole = limit / 64;
        if self.0[..whole].iter().any(|&word| word != 0) { return true; }
        let tail = limit % 64;
        tail != 0 && self.0[whole] & ((1u64 << tail) - 1) != 0
    }

    fn right_u64(self, shift: usize) -> u64 {
        let limb = shift / 64;
        let bit = shift % 64;
        let low = self.0[limb] >> bit;
        if bit == 0 || limb + 1 == 9 { low } else { low | (self.0[limb + 1] << (64 - bit)) }
    }
    fn rounded(self, shift: usize, negative: bool, rc: Kind1Rounding) -> (u64, bool) {
        let truncated = self.right_u64(shift);
        let inexact = self.any_below(shift);
        let increment = match rc {
            Kind1Rounding::NearestEven => shift != 0 && self.bit(shift - 1)
                && (self.any_below(shift - 1) || truncated & 1 != 0),
            Kind1Rounding::TowardNegative => negative && inexact,
            Kind1Rounding::TowardPositive => !negative && inexact,
            Kind1Rounding::TowardZero => false,
        };
        (truncated + u64::from(increment), inexact)
    }
}

#[derive(Clone, Copy)]
struct Exact { negative: bool, magnitude: Magnitude }
impl Exact {
    fn add(self, other: Self, rc: Kind1Rounding) -> Self {
        if self.negative == other.negative {
            return Self { negative: self.negative, magnitude: self.magnitude.add(other.magnitude) };
        }
        match self.magnitude.compare(other.magnitude) {
            Ordering::Greater => Self { negative: self.negative, magnitude: self.magnitude.subtract(other.magnitude) },
            Ordering::Less => Self { negative: other.negative, magnitude: other.magnitude.subtract(self.magnitude) },
            Ordering::Equal => Self { negative: rc == Kind1Rounding::TowardNegative, magnitude: Magnitude([0; 9]) },
        }
    }
}

struct Rounded { bits: Option<u32>, post: u8 }
fn round(exact: Exact, state: Kind1Mxcsr) -> Rounded {
    let sign = if exact.negative { SIGN } else { 0 };
    let highest = match exact.magnitude.highest() {
        Some(highest) => highest,
        None => return Rounded { bits: Some(sign), post: 0 },
    };
    let rc = state.rounding();

    let (unbounded, unbounded_inexact) = if highest < 23 {
        (exact.magnitude.right_u64(0) << (23 - highest), false)
    } else { exact.magnitude.rounded(highest - 23, exact.negative, rc) };
    let rounded_highest = highest + usize::from(unbounded == 0x0100_0000);
    let tiny = rounded_highest < 172;
    if rounded_highest >= 426 {
        if state.masks().bits() & OVERFLOW == 0 {

            return Rounded { bits: None, post: OVERFLOW
                | if unbounded_inexact { PRECISION } else { 0 } };
        }
        let infinity = match rc {
            Kind1Rounding::NearestEven => true,
            Kind1Rounding::TowardNegative => exact.negative,
            Kind1Rounding::TowardPositive => !exact.negative,
            Kind1Rounding::TowardZero => false,
        };
        return Rounded { bits: Some(sign | if infinity { INFINITY } else { 0x7f7f_ffff }),
            post: OVERFLOW | PRECISION };
    }
    if tiny && state.masks().bits() & UNDERFLOW == 0 {

        return Rounded { bits: None, post: UNDERFLOW
            | if unbounded_inexact { PRECISION } else { 0 } };
    }
    if tiny && state.flush_to_zero() {

        return Rounded { bits: Some(sign), post: UNDERFLOW | PRECISION };
    }

    let shift = if highest < 172 { 149 } else { highest - 23 };
    let (mut significand, inexact) = exact.magnitude.rounded(shift, exact.negative, rc);
    let mut exponent = shift as i32 - 298 + 23;
    if significand == 0x0100_0000 { significand >>= 1; exponent += 1; }
    let bits = if significand < 0x0080_0000 { sign | significand as u32 }
        else { sign | (((exponent + 127) as u32) << 23) | (significand as u32 & 0x007f_ffff) };
    let underflow = tiny && inexact;
    Rounded { bits: Some(bits), post: (if inexact { PRECISION } else { 0 })
        | (if underflow { UNDERFLOW } else { 0 }) }
}

pub(crate) fn evaluate_subtract_low(left: u32, right: u32, state: Kind1Mxcsr) -> Kind1PackedOutcome {
    let addend = if Operand(left).nan() || Operand(right).nan() { right } else { right ^ SIGN };
    evaluate_packed_instruction(Kind1PackedOperation::Vaddps { left: [left; 8], right: [addend; 8] }, state)
}

pub(crate) fn evaluate_convert_u32_low(value: u32, state: Kind1Mxcsr) -> Kind1PackedOutcome {
    let exact = Exact { negative: false, magnitude: Magnitude::shifted(u64::from(value), 298) };
    let rounded = round(exact, state);
    let bits = rounded.bits.expect("unsigned DWORD conversion cannot overflow or underflow F32");
    outcome(state, rounded.post, [bits; 8])
}

fn round_convert_signed_magnitude(negative: bool, magnitude: u64, state: Kind1Mxcsr) -> Rounded {
    let exact = Exact { negative, magnitude: Magnitude::shifted(magnitude, 298) };
    round(exact, state)
}

fn round_convert_i32_lane(value: i32, state: Kind1Mxcsr) -> Rounded {
    let magnitude = if value < 0 { (-i64::from(value)) as u64 } else { value as u64 };
    round_convert_signed_magnitude(value < 0, magnitude, state)
}

pub(crate) fn evaluate_convert_i32_low(value: i32, state: Kind1Mxcsr) -> Kind1PackedOutcome {
    let rounded = round_convert_i32_lane(value, state);
    let bits = rounded.bits.expect("signed DWORD conversion cannot overflow or underflow F32");
    outcome(state, rounded.post, [bits; 8])
}

pub(crate) fn evaluate_convert_i32_xmm(values: [i32; 4], state: Kind1Mxcsr) -> Kind1PackedOutcome {
    let mut result = [0u32; 8];
    let mut raised = 0u8;
    for lane in 0..4 {
        let rounded = round_convert_i32_lane(values[lane], state);
        result[lane] = rounded.bits.expect("signed DWORD conversion cannot overflow or underflow F32");
        result[lane + 4] = result[lane];
        raised |= rounded.post;
    }
    outcome(state, raised, result)
}

pub(crate) fn evaluate_convert_i64_low(value: i64, state: Kind1Mxcsr) -> Kind1PackedOutcome {
    let rounded = round_convert_signed_magnitude(value < 0, value.unsigned_abs(), state);
    let bits = rounded.bits.expect("signed QWORD conversion cannot overflow or underflow F32");
    outcome(state, rounded.post, [bits; 8])
}
