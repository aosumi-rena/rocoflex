#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F32Operation {
    Cvtsi2ssSigned32(i32),
    Mul { destination: u32, source: u32 },
    Div { destination: u32, source: u32 },
    Add { destination: u32, source: u32 },
    Sub { destination: u32, source: u32 },
    Min { destination: u32, source: u32 },
    Max { destination: u32, source: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompareFlags { pub carry: bool, pub zero: bool, pub parity: bool }
impl CompareFlags {
    pub(super) fn below_or_equal(self) -> bool { self.carry || self.zero }
}
