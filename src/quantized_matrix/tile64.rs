use std::{collections::TryReserveError, fmt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Layout { RowMajor, Tile64, Auto }
#[derive(Debug)]
pub(super) enum PackError {
    Parameters(&'static str),
    Reserve(TryReserveError),
}
impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parameters(message) => f.write_str(message),
            Self::Reserve(error) => write!(f, "tile64 reservation failed: {error}"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Packed64 {
    input: usize,
    output: usize,
    pub(super) weights: Vec<i16>,
}
impl Packed64 {
    fn checked_count(input: usize, output: usize, source_len: usize)
        -> std::result::Result<usize, PackError>
    {
        let bad = PackError::Parameters;
        let count = input.checked_mul(output).ok_or(bad("tile64 matrix extent overflow"))?;
        let bytes = count.checked_mul(std::mem::size_of::<i16>())
            .ok_or(bad("tile64 matrix byte extent overflow"))?;
        if bytes > isize::MAX as usize {
            return Err(bad("tile64 matrix byte extent exceeds isize"));
        }
        if !(1..=65_536).contains(&input) || !(1..=65_536).contains(&output)
            || source_len != count {
            return Err(bad("tile64 matrix dimensions or extent mismatch"));
        }
        let columns = output / 64 * 64;
        let packed_count = input.checked_mul(columns).ok_or(bad("tile64 packed extent overflow"))?;
        let packed_bytes = packed_count.checked_mul(std::mem::size_of::<i16>())
            .ok_or(bad("tile64 packed byte extent overflow"))?;
        if packed_bytes > isize::MAX as usize {
            return Err(bad("tile64 packed byte extent exceeds isize"));
        }
        Ok(packed_count)
    }

    pub(super) fn prepare_with(input: usize, output: usize, source: &[i16],
        layout: Layout, avx2: bool,
        reserve: impl FnOnce(&mut Vec<i16>, usize) -> std::result::Result<(), TryReserveError>)
        -> std::result::Result<Option<Self>, PackError>
    {
        let count = Self::checked_count(input, output, source.len())?;
        if output <= 64 || layout == Layout::RowMajor || (layout == Layout::Auto && !avx2) {
            return Ok(None);
        }
        let mut weights = Vec::new();
        if let Err(error) = reserve(&mut weights, count) {

            return if layout == Layout::Auto {Ok(None)} else {Err(PackError::Reserve(error))};
        }
        for base in (0..output / 64 * 64).step_by(64) {
            for i in 0..input {
                let start = i * output + base;
                weights.extend_from_slice(&source[start..start + 64]);
            }
        }
        Ok(Some(Self { input, output, weights }))
    }
    pub(super) fn matches(&self, input: usize, output: usize) -> bool {
        self.input == input && self.output == output
    }
}

