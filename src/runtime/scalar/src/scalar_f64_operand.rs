#[derive(Clone, Copy)]
pub(super) struct F64Operand(u64);
impl F64Operand {
    pub fn from_bits(bits: u64) -> Self { Self(bits) }
    pub fn bits(self) -> u64 { self.0 }
    pub fn negative(self) -> bool { self.0 >> 63 != 0 }
    pub fn magnitude(self) -> u64 { self.0 & 0x7fff_ffff_ffff_ffff }
    pub fn field(self) -> i32 { (self.magnitude() >> 52) as i32 }
    pub fn fraction(self) -> u64 { self.0 & 0x000f_ffff_ffff_ffff }
    pub fn nan(self) -> bool { self.magnitude() > 0x7ff0_0000_0000_0000 }
    pub fn signaling(self) -> bool { self.nan() && self.0 & 0x0008_0000_0000_0000 == 0 }
    pub fn infinite(self) -> bool { self.magnitude() == 0x7ff0_0000_0000_0000 }
    pub fn zero(self) -> bool { self.magnitude() == 0 }
    pub fn subnormal(self) -> bool { self.field() == 0 && self.fraction() != 0 }
    pub fn with_daz(self, enabled: bool) -> Self {
        if enabled && self.subnormal() { Self(self.0 & 0x8000_0000_0000_0000) } else { self }
    }

    pub fn finite_parts(self) -> (u64, i32) {
        if self.field() == 0 { (self.fraction(), -1074) }
        else { (self.fraction() | (1u64 << 52), self.field() - 1075) }
    }

    pub fn normalized_nonzero_parts(self) -> (u64, i32) {
        let (significand, power) = self.finite_parts();
        let highest = 63 - significand.leading_zeros();
        let shift = 52 - highest;
        (significand << shift, power - shift as i32)
    }
}

pub(super) fn binary_nan_result(first: F64Operand, second: F64Operand) -> Option<(u64, u8)> {
    if !first.nan() && !second.nan() { return None; }
    let chosen = if first.nan() { first } else { second };
    Some((chosen.bits() | 0x0008_0000_0000_0000, u8::from(first.signaling() || second.signaling())))
}
