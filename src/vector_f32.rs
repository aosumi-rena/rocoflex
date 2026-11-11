use crate::{matrix_f32, matrix_policy::FactoryState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Dimensions, Extent, Width, NonfiniteRead, NonfiniteResult,
    Environment(matrix_f32::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReciprocalPolicy { CurrentX86HostInstruction }

pub struct BindingFailure {
    pub error: Error,
    pub factory: FactoryState,
    pub logical_width: usize,
    pub capacity_bytes: usize,
    pub all_words: Vec<u32>,
}

pub struct Backing {
    factory: FactoryState,
    width: usize,
    capacity_bytes: usize,
    words: Vec<u32>,
}

impl Backing {
    pub fn minimum_capacity_bytes(width: usize) -> Result<usize, Error> {
        if width == 0 || width > i32::MAX as usize { return Err(Error::Dimensions); }
        width.checked_mul(4).and_then(|n| n.checked_add(63))
            .filter(|&n| n <= i32::MAX as usize).map(|n| n & !63usize)
            .ok_or(Error::Dimensions)
    }

    pub fn from_explicit_state(factory: FactoryState, logical_width: usize,
        capacity_bytes: usize, all_words: Vec<u32>) -> Result<Self, BindingFailure>
    {
        let admitted = Self::minimum_capacity_bytes(logical_width).and_then(|minimum| {
            if capacity_bytes < minimum || capacity_bytes > i32::MAX as usize
                || capacity_bytes % 64 != 0 || all_words.len() != capacity_bytes / 4
            { Err(Error::Extent) } else { Ok(()) }
        });
        if let Err(error) = admitted {
            return Err(BindingFailure { error, factory, logical_width, capacity_bytes, all_words });
        }
        Ok(Self { factory, width: logical_width, capacity_bytes, words: all_words })
    }

    pub fn factory(&self) -> FactoryState { self.factory }
    pub fn logical_width(&self) -> usize { self.width }
    pub fn capacity_bytes(&self) -> usize { self.capacity_bytes }
    pub fn logical_bits(&self) -> &[u32] { &self.words[..self.width] }
    pub fn full_bits(&self) -> &[u32] { &self.words }
    pub fn full_bits_mut(&mut self) -> &mut [u32] { &mut self.words }
    pub fn into_parts(self) -> (FactoryState, usize, usize, Vec<u32>) {
        (self.factory, self.width, self.capacity_bytes, self.words)
    }

    pub fn clear_logical(&mut self) { self.words[..self.width].fill(0); }

    pub fn initialize_full_zero(&mut self) { self.words.fill(0); }

    pub fn copy_into(&self, destination: &mut Self) -> Result<(), Error> {
        if self.width != destination.width { return Err(Error::Width); }
        destination.words[..self.width].copy_from_slice(&self.words[..self.width]);
        Ok(())
    }

    pub fn split_into(&self, first: &mut Self, second: &mut Self) -> Result<(), Error> {
        if first.width.checked_add(second.width) != Some(self.width) { return Err(Error::Width); }
        first.words[..first.width].copy_from_slice(&self.words[..first.width]);
        second.words[..second.width].copy_from_slice(&self.words[first.width..self.width]);
        Ok(())
    }

    fn padded_width(&self, destination: &Self) -> Result<usize, Error> {
        if self.width != destination.width || self.width > (i32::MAX - 7) as usize {
            return Err(Error::Width);
        }
        let lanes = if self.factory.flags()[2] != 0 { 8 } else { 4 };
        let count = self.width.div_ceil(lanes) * lanes;
        if count > self.words.len() || count > destination.words.len() { return Err(Error::Extent); }
        Ok(count)
    }

    pub fn add_into(&self, destination: &mut Self) -> Result<(), Error> {
        self.binary_into(destination, false)
    }
    pub fn multiply_into(&self, destination: &mut Self) -> Result<(), Error> {
        self.binary_into(destination, true)
    }

    pub fn sigmoid_into(&self, destination: &mut Self, reciprocal: ReciprocalPolicy)
        -> Result<(), Error>
    {
        matrix_f32::check_environment().map_err(Error::Environment)?;
        let count = self.padded_width(destination)?;
        let source = &self.words[..count];
        if source.iter().any(|&word| !f32::from_bits(word).is_finite()) {
            return Err(Error::NonfiniteRead);
        }
        let arithmetic = crate::activation::ArithmeticMode {
            fused_multiply_add: self.factory.flags()[4] != 0,
        };
        let pending = match reciprocal {
            ReciprocalPolicy::CurrentX86HostInstruction => source.iter().map(|&word|
                arithmetic.sigmoid(f32::from_bits(word)).to_bits()).collect::<Vec<_>>(),
        };
        if pending.iter().any(|&word| !f32::from_bits(word).is_finite()) {
            return Err(Error::NonfiniteResult);
        }
        destination.words[..count].copy_from_slice(&pending);
        Ok(())
    }

    fn binary_into(&self, destination: &mut Self, multiply: bool) -> Result<(), Error> {
        matrix_f32::check_environment().map_err(Error::Environment)?;
        let count = self.padded_width(destination)?;
        let source = &self.words[..count];
        let old = &destination.words[..count];
        if source.iter().chain(old).any(|&v| !f32::from_bits(v).is_finite()) {
            return Err(Error::NonfiniteRead);
        }
        let pending = old.iter().zip(source).map(|(&a,&b)| {
            let (a,b) = (f32::from_bits(a), f32::from_bits(b));
            if multiply { (a*b).to_bits() } else { (a+b).to_bits() }
        }).collect::<Vec<_>>();
        if pending.iter().any(|&v| !f32::from_bits(v).is_finite()) {
            return Err(Error::NonfiniteResult);
        }
        destination.words[..count].copy_from_slice(&pending);
        Ok(())
    }

    pub fn multiply_with_into(&self, other: &Self, destination: &mut Self) -> Result<(), Error> {
        matrix_f32::check_environment().map_err(Error::Environment)?;
        let count = self.padded_width(destination)?;
        if self.width != other.width || count > other.words.len() { return Err(Error::Extent); }
        let a = &self.words[..count];
        let b = &other.words[..count];
        if a.iter().chain(b).any(|&v| !f32::from_bits(v).is_finite()) {
            return Err(Error::NonfiniteRead);
        }
        let pending = a.iter().zip(b).map(|(&x,&y)|
            (f32::from_bits(x)*f32::from_bits(y)).to_bits()).collect::<Vec<_>>();
        if pending.iter().any(|&v| !f32::from_bits(v).is_finite()) {
            return Err(Error::NonfiniteResult);
        }
        destination.words[..count].copy_from_slice(&pending);
        Ok(())
    }

    pub fn one_minus_in_place(&mut self) -> Result<(), Error> {
        matrix_f32::check_environment().map_err(Error::Environment)?;
        if self.logical_bits().iter().any(|&v| !f32::from_bits(v).is_finite()) {
            return Err(Error::NonfiniteRead);
        }
        let pending = self.logical_bits().iter().map(|&v|
            (1.0_f32-f32::from_bits(v)).to_bits()).collect::<Vec<_>>();
        if pending.iter().any(|&v| !f32::from_bits(v).is_finite()) {
            return Err(Error::NonfiniteResult);
        }
        self.words[..self.width].copy_from_slice(&pending);
        Ok(())
    }
}
