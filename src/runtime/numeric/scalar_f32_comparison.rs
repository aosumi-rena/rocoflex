#[derive(Clone, Copy)]
pub struct F32Operand(u32);
impl F32Operand {
    pub fn from_bits(bits: u32) -> Self { Self(bits) }
    pub fn bits(self) -> u32 { self.0 }
    pub fn magnitude(self) -> u32 { self.0 & 0x7fff_ffff }
    pub fn negative(self) -> bool { self.0 >> 31 != 0 }
    pub fn nan(self) -> bool { self.magnitude() > 0x7f80_0000 }
    pub fn signaling(self) -> bool { self.nan() && self.0 & 0x0040_0000 == 0 }
    pub fn subnormal(self) -> bool {
        self.magnitude() != 0 && self.magnitude() < 0x0080_0000
    }
    pub fn with_daz(self, enabled: bool) -> Self {
        if enabled && self.subnormal() { Self(self.0 & 0x8000_0000) } else { self }
    }
}

pub fn fdtest_f32_low_word(bits: u32) -> i16 {
    let operand = F32Operand::from_bits(bits);
    if operand.nan() { 2 }
    else if operand.magnitude() == 0x7f80_0000 { 1 }
    else if operand.magnitude() == 0 { 0 }
    else if operand.subnormal() { -2 }
    else { -1 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F32Relation { Less, Equal, Greater, Unordered }
impl F32Relation {

    pub fn flag_bits(self) -> (bool, bool, bool) {
        match self {
            Self::Less => (true, false, false),
            Self::Equal => (false, false, true),
            Self::Greater => (false, false, false),
            Self::Unordered => (true, true, true),
        }
    }
}

pub fn f32_comparison_candidate(left: u32, right: u32,
    quiet: bool, denormals_are_zero: bool) -> (F32Relation, u8) {
    let a = F32Operand::from_bits(left);
    let b = F32Operand::from_bits(right);
    if a.nan() || b.nan() {
        let invalid = !quiet || a.signaling() || b.signaling();
        return (F32Relation::Unordered, u8::from(invalid));
    }
    let a = a.with_daz(denormals_are_zero);
    let b = b.with_daz(denormals_are_zero);
    let raised = if a.subnormal() || b.subnormal() { 2 } else { 0 };
    (ordered_sign_magnitude_relation(u64::from(a.magnitude()), a.negative(),
        u64::from(b.magnitude()), b.negative()), raised)
}

pub fn ordered_sign_magnitude_relation(left_magnitude: u64, left_negative: bool,
    right_magnitude: u64, right_negative: bool) -> F32Relation {
    if (left_magnitude == right_magnitude && left_negative == right_negative)
        || (left_magnitude == 0 && right_magnitude == 0) { return F32Relation::Equal; }
    let less = if left_negative != right_negative { left_negative }
        else if left_negative { left_magnitude > right_magnitude }
        else { left_magnitude < right_magnitude };
    if less { F32Relation::Less } else { F32Relation::Greater }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F32MinMaxOpcode { Minss, Maxss }

pub fn select_minmax_f32_bits(opcode: F32MinMaxOpcode,
    left: u32, right: u32, carry: bool, parity: bool, zero: bool) -> u32 {
    let take_left = match opcode {
        F32MinMaxOpcode::Minss => carry && !parity,
        F32MinMaxOpcode::Maxss => !carry && !zero && !parity,
    };
    if take_left { left } else { right }
}

pub fn minmax_f32_candidate(opcode: F32MinMaxOpcode,
    left: u32, right: u32, denormals_are_zero: bool) -> (Option<u32>, u8) {
    let (relation, raised) = f32_comparison_candidate(left, right, false, denormals_are_zero);
    let a = F32Operand::from_bits(left);
    let b = F32Operand::from_bits(right);
    if denormals_are_zero && a.nan() && b.subnormal() { return (None, raised); }
    let (carry, parity, zero) = relation.flag_bits();
    (Some(select_minmax_f32_bits(opcode,
        a.with_daz(denormals_are_zero).bits(), b.with_daz(denormals_are_zero).bits(),
        carry, parity, zero)), raised)
}
