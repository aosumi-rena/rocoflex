use super::*;
use std::marker::PhantomData;

pub(super) struct RowValues<'a, E, U: ScalarUpper> {
    pub current: &'a [f32], pub history: &'a mut Vec<f32>, pub output: &'a mut Batch,
    pub seen: &'a mut u64, pub math: &'a mut Provider<U>, pub trace: &'a mut Trace,
    pub error: PhantomData<fn() -> E>,
}
impl<E, U: ScalarUpper> graph::FractionServices for RowValues<'_, E, U> {
    type Error = ServiceError<E>;
    fn signed_i32_to_f32_bits(&mut self, value: i32) -> Result<u32, Self::Error> {
        self.math.signed_i32_to_f32_bits(value).map_err(ServiceError::Math)
    }
    fn addss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.math.addss_bits(left, right).map_err(ServiceError::Math)
    }
    fn divss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.math.divss_bits(left, right).map_err(ServiceError::Math)
    }
}
impl<E, U: ScalarUpper> graph::RowServices for RowValues<'_, E, U> {
    type Target<'a> = Target<'a, E, U> where Self: 'a;
    fn copy_current_to_history(&mut self, _first: bool) -> Result<(), Self::Error> {
        if self.current.len() != self.history.len() { return Err(ServiceError::Width); }
        self.history.copy_from_slice(self.current); self.trace.copies += 1; Ok(())
    }
    fn copy_current_to_output(&mut self, written: usize) -> Result<(), Self::Error> {
        let target = self.output.rows.get_mut(written).ok_or(ServiceError::Output(written))?;
        if target.len() != self.current.len() { return Err(ServiceError::Width); }
        target.copy_from_slice(self.current); self.trace.copies += 1; Ok(())
    }
    fn bind_interpolation_target(&mut self, written: usize, _step: usize) -> Result<Self::Target<'_>, Self::Error> {
        let target = self.output.rows.get_mut(written).ok_or(ServiceError::Output(written))?;
        if target.len() != self.current.len() { return Err(ServiceError::Width); }
        Ok(Target { current: self.current, history: self.history, target,
            math: self.math, trace: self.trace, error: PhantomData })
    }
    fn increment_seen16(&mut self) -> Result<(), Self::Error> { *self.seen += 1; Ok(()) }
}
pub(super) struct Target<'a, E, U: ScalarUpper> {
    current: &'a [f32], history: &'a [f32], target: &'a mut [f32],
    math: &'a mut Provider<U>, trace: &'a mut Trace, error: PhantomData<fn() -> E>,
}
impl<E, U: ScalarUpper> graph::LaneServices for Target<'_, E, U> {
    type Error = ServiceError<E>;
    fn read_next_bits(&mut self, lane: usize) -> Result<u32, Self::Error> {
        self.current.get(lane).map(|v| v.to_bits()).ok_or(ServiceError::Width)
    }
    fn read_previous_bits(&mut self, lane: usize) -> Result<u32, Self::Error> {
        self.history.get(lane).map(|v| v.to_bits()).ok_or(ServiceError::Width)
    }
    fn subss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.math.subss_bits(left, right).map_err(ServiceError::Math)
    }
    fn mulss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.math.mulss_bits(left, right).map_err(ServiceError::Math)
    }
    fn addss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.math.addss_bits(left, right).map_err(ServiceError::Math)
    }
    fn store_bits(&mut self, lane: usize, bits: u32) -> Result<(), Self::Error> {
        let target = self.target.get_mut(lane).ok_or(ServiceError::Width)?;
        *target = f32::from_bits(bits); self.trace.lane_stores += 1; self.trace.last_bits = Some(bits); Ok(())
    }
}
pub(super) struct EndValues<'a, E> {
    pub history: &'a [f32], pub output: &'a mut Batch, pub trace: &'a mut Trace,
    pub error: PhantomData<fn() -> E>,
}
impl<E> graph::EndServices for EndValues<'_, E> {
    type Error = ServiceError<E>;
    fn copy_history_to_output(&mut self, written: usize) -> Result<(), Self::Error> {
        let target = self.output.rows.get_mut(written).ok_or(ServiceError::Output(written))?;
        if target.len() != self.history.len() { return Err(ServiceError::Width); }
        target.copy_from_slice(self.history); self.trace.copies += 1; Ok(())
    }
}
