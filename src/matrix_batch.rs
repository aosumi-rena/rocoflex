mod sealed {
    pub trait Sealed {}
    impl Sealed for Vec<u32> {}
    impl Sealed for crate::vector_f32::Backing {}
}

pub(crate) trait FullWords: sealed::Sealed {
    fn full_words(&self) -> &[u32];
    fn full_words_mut(&mut self) -> &mut [u32];
    fn logical_width(&self) -> Option<usize>;
}
impl FullWords for Vec<u32> {
    fn full_words(&self) -> &[u32] { self.as_slice() }
    fn full_words_mut(&mut self) -> &mut [u32] { self.as_mut_slice() }
    fn logical_width(&self) -> Option<usize> { None }
}
impl FullWords for crate::vector_f32::Backing {
    fn full_words(&self) -> &[u32] { self.full_bits() }
    fn full_words_mut(&mut self) -> &mut [u32] { self.full_bits_mut() }
    fn logical_width(&self) -> Option<usize> {
        Some(crate::vector_f32::Backing::logical_width(self))
    }
}

pub(crate) fn execute<'a, E, D: FullWords>(count: usize, columns: usize, output_width: usize,
    mut read_row: impl FnMut(usize) -> Option<&'a [f32]>,
    destinations: &mut [D], extent: E,
    mut execute_row: impl FnMut(&[f32], &mut [u32]) -> Result<(), E>)
    -> Result<(), (Option<usize>, E)>
{
    if count != destinations.len() { return Err((None, extent)); }
    for row in 0..count {
        if read_row(row).is_none_or(|input| input.len() != columns) {
            return Err((Some(row), extent));
        }
    }

    for (row, destination) in destinations.iter().enumerate() {
        if destination.logical_width().is_some_and(|width| width != output_width) {
            return Err((Some(row), extent));
        }
    }

    let mut pending: Vec<Vec<u32>> = destinations.iter()
        .map(|destination| destination.full_words().to_vec()).collect();
    for (row, output) in pending.iter_mut().enumerate() {
        let input = match read_row(row) {
            Some(input) if input.len() == columns => input,
            _ => return Err((Some(row), extent)),
        };
        execute_row(input, output).map_err(|error| (Some(row), error))?;
    }
    for (destination, output) in destinations.iter_mut().zip(pending) {
        destination.full_words_mut().copy_from_slice(&output);
    }
    Ok(())
}
