use super::*;
use std::convert::Infallible;

struct LaneStorage<'a> { target: &'a mut f32, noise: &'a f32 }
impl noise::Storage for LaneStorage<'_> {
    type Error = Infallible;
    fn read_target_bits(&mut self) -> Result<u32, Infallible> { Ok(self.target.to_bits()) }
    fn read_noise_bits(&mut self) -> Result<u32, Infallible> { Ok(self.noise.to_bits()) }
    fn store_target_bits(&mut self, value: u32) -> Result<(), Infallible> { *self.target = f32::from_bits(value); Ok(()) }
}
impl<A: Module, B: Module, U: ScalarUpper, H> graph::PostServices for Owner<A, B, U, H> {
    type Error = ServiceError<A::Error, B::Error>;
    fn module56(&mut self) -> Result<(), Self::Error> {
        let p = &mut self.state.parts;
        p.module56.forward(&mut self.input240, &mut p.stage272).map_err(ServiceError::Module56)?;
        self.trace.module56_completed = true;
        Ok(())
    }
    fn prepare_noise(&mut self) -> Result<usize, Self::Error> {
        let p = &self.state.parts;
        let index = usize::try_from(p.cursor_index).map_err(|_| ServiceError::Cursor)?;
        self.trace.noise.index = Some(index);
        if index >= p.cursor_count { return Err(ServiceError::Cursor); }
        let count = p.stage272.rows.len();
        self.trace.noise.count = Some(count);
        if count > i32::MAX as usize { return Err(ServiceError::Count); }
        let word = p.cursor_words[index];
        self.trace.noise.cursor_word = Some(word);
        let fifo_count = p.noise_rows.len() as u64;
        self.trace.noise.fifo_count = Some(fifo_count);
        let available = (fifo_count as u32).wrapping_sub(word) as i32;
        self.trace.noise.available = Some(available);
        if available < count as i32 || p.ended != 0 && available != count as i32 { return Err(ServiceError::Cursor); }
        if count != 0 {
            self.trace.noise.environment_attempted = true;
            crate::matrix_f32::check_environment().map_err(ServiceError::Environment)?;
        }
        let start = usize::try_from(word as i32).map_err(|_| ServiceError::Cursor)?;
        self.trace.noise.start = Some(start);
        let end = start.checked_add(count).ok_or(ServiceError::Count)?;
        self.trace.noise.end = Some(end);
        if end as u64 > fifo_count { return Err(ServiceError::Cursor); }
        Ok(count)
    }
    fn admit_noise_row(&mut self, row: usize) -> Result<(), Self::Error> {
        self.trace.admitted_row = None;
        self.trace.noise_width = None; self.trace.target_width = None;
        let p = &self.state.parts;
        let at = self.trace.noise.start.ok_or(ServiceError::Control("prepared noise start"))? + row;
        let source = p.noise_rows.get(at).ok_or(ServiceError::Input)?;
        self.trace.noise_width = Some(source.len());
        let target = p.stage272.rows.get(row).ok_or(ServiceError::Input)?;
        self.trace.target_width = Some(target.len());
        if source.len() < 2 * p.groups || target.len() < p.groups { return Err(ServiceError::Width); }
        self.trace.admitted_row = Some(row);
        Ok(())
    }
    fn apply_noise_lane(&mut self, row: usize, lane_index: usize) -> Result<(), Self::Error> {
        if self.trace.admitted_row != Some(row) { return Err(ServiceError::Control("noise row admission")); }
        let p = &mut self.state.parts;
        let at = self.trace.noise.start.ok_or(ServiceError::Control("prepared noise start"))? + row;
        let source = p.noise_rows.get(at).ok_or(ServiceError::Input)?;
        let target = p.stage272.rows.get_mut(row).ok_or(ServiceError::Input)?;
        let target = target.get_mut(lane_index).ok_or(ServiceError::Width)?;
        let sample = source.get(p.groups + lane_index).ok_or(ServiceError::Width)?;

        let result = noise::apply(row, lane_index, &mut p.math, &mut LaneStorage { target, noise: sample });
        match result {
            Ok(completed) => { self.trace.last_lane = Some(completed.partial()); Ok(()) }
            Err(error) => { self.trace.last_lane = Some(error.partial()); Err(ServiceError::Lane(error)) }
        }
    }
    fn commit_cursor512(&mut self, count: usize) -> Result<(), Self::Error> {
        let index = self.trace.noise.index.ok_or(ServiceError::Control("prepared cursor index"))?;
        let word = &mut self.state.parts.cursor_words[index];
        *word = word.wrapping_add(count as u32);
        self.trace.cursor_after = Some(*word);
        Ok(())
    }
    fn module40(&mut self) -> Result<(), Self::Error> {
        let p = &mut self.state.parts;
        let output = p.final304.as_mut().ok_or(ServiceError::Control("owned final304"))?;
        p.module40.forward(&mut p.stage272, output).map_err(ServiceError::Module40)?;
        self.trace.module40_completed = true;
        Ok(())
    }
}
