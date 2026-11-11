use crate::{ltv_coefficient_math::{Provider, ScalarUpper}, paired_post_values::lane};
pub use lane::{LaneComplete, LaneFailure, LanePartial, LaneStage};

pub trait Storage {
    type Error;
    fn read_target_bits(&mut self) -> Result<u32, Self::Error>;
    fn read_noise_bits(&mut self) -> Result<u32, Self::Error>;
    fn store_target_bits(&mut self, value: u32) -> Result<(), Self::Error>;
}
pub enum Error<E> { Storage(E), Math(crate::ltv_coefficient_values::CoefficientError) }
struct Bound<'a, U: ScalarUpper, S> { math: &'a mut Provider<U>, storage: &'a mut S }

pub fn apply<U: ScalarUpper, S: Storage>(row: usize, index: usize,
    math: &mut Provider<U>, storage: &mut S)
    -> Result<LaneComplete, LaneFailure<Error<S::Error>>>
{
    lane::apply_lane(row, index, &mut Bound { math, storage })
}
impl<U: ScalarUpper, S: Storage> lane::LaneServices for Bound<'_, U, S> {
    type Error = Error<S::Error>;
    fn read_target_bits(&mut self) -> Result<u32, Self::Error> {
        self.storage.read_target_bits().map_err(Error::Storage)
    }
    fn scalar_57f8e0_bits(&mut self, value: u32) -> Result<u32, Self::Error> {
        self.math.scalar_57f8e0_bits(value).map_err(Error::Math)
    }
    fn addss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.math.addss_bits(left, right).map_err(Error::Math)
    }
    fn divss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.math.divss_bits(left, right).map_err(Error::Math)
    }
    fn read_noise_bits(&mut self) -> Result<u32, Self::Error> {
        self.storage.read_noise_bits().map_err(Error::Storage)
    }
    fn mulss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.math.mulss_bits(left, right).map_err(Error::Math)
    }
    fn store_target_bits(&mut self, value: u32) -> Result<(), Self::Error> {
        self.storage.store_target_bits(value).map_err(Error::Storage)
    }
}
