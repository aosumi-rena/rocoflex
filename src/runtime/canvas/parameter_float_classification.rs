pub trait Memory {
    type Pointer;
    type Error;
    fn read_word(&mut self, pointer: &Self::Pointer, offset: u8) -> Result<u16, Self::Error>;
    fn read_byte(&mut self, pointer: &Self::Pointer, offset: u8) -> Result<u8, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    InitialWord2,
    MaxExponentByte2,
    MaxExponentWord0,
    OtherWord2,
    ZeroCandidateWord0,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Captures {
    pub initial_word2: Option<u16>,

    pub saved_dx: Option<u16>,
    pub fresh_byte2: Option<u8>,
    pub fresh_word2: Option<u16>,

    pub fresh_word0: Option<u16>,
}

#[derive(Debug)]
pub struct Failure<E> {
    pub error: E,
    pub attempted: Phase,
    pub captures: Captures,
}

pub fn classify_with_memory<M: Memory + ?Sized>(memory: &mut M, pointer: &M::Pointer)
    -> Result<i16, Failure<M::Error>> {
    let mut captures = Captures::default();
    let initial = memory.read_word(pointer, 2).map_err(|error| Failure {
        error, attempted: Phase::InitialWord2, captures,
    })?;
    captures.initial_word2 = Some(initial);
    let saved_dx = initial & 0x7f80;
    captures.saved_dx = Some(saved_dx);

    if saved_dx == 0x7f80 {
        let byte = memory.read_byte(pointer, 2).map_err(|error| Failure {
            error, attempted: Phase::MaxExponentByte2, captures,
        })?;
        captures.fresh_byte2 = Some(byte);
        if byte & 0x7f != 0 { return Ok(2); }
        let low = memory.read_word(pointer, 0).map_err(|error| Failure {
            error, attempted: Phase::MaxExponentWord0, captures,
        })?;
        return Ok(if low != 0 { 2 } else { 1 });
    }

    let fresh = memory.read_word(pointer, 2).map_err(|error| Failure {
        error, attempted: Phase::OtherWord2, captures,
    })?;
    captures.fresh_word2 = Some(fresh);
    if fresh & 0x7fff == 0 {
        let low = memory.read_word(pointer, 0).map_err(|error| Failure {
            error, attempted: Phase::ZeroCandidateWord0, captures,
        })?;
        if low == 0 { return Ok(0); }
    }

    Ok(if saved_dx != 0 { -1 } else { -2 })
}
