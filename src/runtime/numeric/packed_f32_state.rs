#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind1Rounding { NearestEven, TowardNegative, TowardPositive, TowardZero }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind1Exceptions(pub(crate) u8);
impl Kind1Exceptions {
    pub const INVALID: Self = Self(1);
    pub const DENORMAL: Self = Self(2);
    pub const DIVIDE_BY_ZERO: Self = Self(4);
    pub const OVERFLOW: Self = Self(8);
    pub const UNDERFLOW: Self = Self(16);
    pub const PRECISION: Self = Self(32);
    pub fn from_explicit_bits(bits: u8) -> Option<Self> {
        if bits & !0x3f != 0 { None } else { Some(Self(bits)) }
    }
    pub fn bits(self) -> u8 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind1Mxcsr(u32);
impl Kind1Mxcsr {
    pub fn from_explicit_word(word: u32) -> Option<Self> {
        if word & !0xffff != 0 { None } else { Some(Self(word)) }
    }
    pub fn word(self) -> u32 { self.0 }
    pub fn sticky(self) -> Kind1Exceptions { Kind1Exceptions((self.0 & 0x3f) as u8) }
    pub fn masks(self) -> Kind1Exceptions { Kind1Exceptions(((self.0 >> 7) & 0x3f) as u8) }
    pub fn denormals_are_zero(self) -> bool { self.0 & 0x40 != 0 }
    pub fn flush_to_zero(self) -> bool { self.0 & 0x8000 != 0 }
    pub fn rounding(self) -> Kind1Rounding {
        match (self.0 >> 13) & 3 {
            0 => Kind1Rounding::NearestEven,
            1 => Kind1Rounding::TowardNegative,
            2 => Kind1Rounding::TowardPositive,
            _ => Kind1Rounding::TowardZero,
        }
    }
    pub(crate) fn with_raised(self, raised: Kind1Exceptions) -> Self { Self(self.0 | u32::from(raised.0)) }
    pub(crate) fn unmasked(self, raised: Kind1Exceptions) -> Kind1Exceptions {
        Kind1Exceptions(raised.0 & !self.masks().0)
    }

    pub fn with_explicit_raised_status(self, raised: Kind1Exceptions) -> (Self, Kind1Exceptions) {
        (self.with_raised(raised), self.unmasked(raised))
    }

    pub(crate) fn admits_transition(self, after: Self) -> bool {
        self.0 & !0x3f == after.0 & !0x3f && self.0 & 0x3f & !after.0 == 0
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Kind1PackedOperation {
    Vmulps { left: [u32; 8], right: [u32; 8] },
    Vaddps { left: [u32; 8], right: [u32; 8] },
    Vfmadd231ps { accumulator: [u32; 8], left: [u32; 8], right: [u32; 8] },
}

#[derive(Debug, Clone, Copy)]
pub enum Kind1PackedOutcome {
    Completed { result: [u32; 8], state: Kind1Mxcsr },
    Trapped { state: Kind1Mxcsr, unmasked: Kind1Exceptions },
}

