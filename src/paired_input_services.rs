use super::*;
impl<M: Module, R: Rows, H> Owner<M, R, H> {
    fn retain_removed(&mut self, row: Vec<f32>) -> Result<(), ServiceError<M::Error, R::Error>> {
        self.trace.pending_retirement = Some(row);
        self.trace.retired_rows.try_reserve(1).map_err(ServiceError::Allocation)?;
        self.trace.retired_rows.push(self.trace.pending_retirement.take().expect("retained removed feature row"));
        Ok(())
    }
}
impl<M: Module, R: Rows, H> graph::ScalarInputServices for Owner<M, R, H> {
    type Error = ServiceError<M::Error, R::Error>;
    fn prepare112_and_set_status(&mut self, required_rows: usize, ended_status: u8) -> Result<(), Self::Error> {
        self.trace.preparation = Some(required_rows);
        let p = &mut self.state.parts;
        p.rows.prepare112(&mut p.input112, required_rows).map_err(ServiceError::Prepare)?;
        p.input112.status = ended_status;
        Ok(())
    }
    fn snapshot_signal_frame_count(&mut self) -> Result<usize, Self::Error> { Ok(self.signal.rows.len()) }
    fn admit_signal_frame(&mut self, frame: usize) -> Result<usize, Self::Error> {
        self.trace.frame = None; self.trace.source_width = None;
        let source = self.signal.rows.get(frame).ok_or(ServiceError::Frame(frame))?;
        self.trace.source_width = Some(source.len());
        if source.len() < self.state.parts.hop { return Err(ServiceError::Frame(frame)); }

        self.trace.frame = Some(frame);
        Ok(self.state.parts.hop)
    }
    fn copy_signal_sample(&mut self, frame: usize, sample: usize) -> Result<(), Self::Error> {
        if self.trace.frame != Some(frame) { return Err(ServiceError::Control("captured signal frame")); }
        let index = frame * self.state.parts.hop + sample;
        let target = self.state.parts.input112.rows.get_mut(index).ok_or(ServiceError::Row(index))?;
        if target.len() != 1 { return Err(ServiceError::Row(index)); }
        let bits = self.signal.rows[frame][sample].to_bits();
        target[0] = f32::from_bits(bits);
        self.trace.copied += 1; self.trace.last_bits = Some(bits);
        Ok(())
    }
    fn module48(&mut self) -> Result<(), Self::Error> {
        self.trace.frame = None;
        let p = &mut self.state.parts;
        p.module48.forward(&mut p.input112, &mut p.feature144).map_err(ServiceError::Module48)?;
        self.trace.module48_completed = true;
        Ok(())
    }
    fn begin_front_trim(&mut self) -> Result<i32, Self::Error> { Ok((self.state.parts.groups / 2) as i32 - 1) }
    fn current_latency496(&mut self) -> Result<i32, Self::Error> { Ok(self.state.parts.latency496) }
    fn current_feature_count(&mut self) -> Result<usize, Self::Error> { Ok(self.state.parts.feature144.rows.len()) }
    fn trim_front_and_refresh_end(&mut self) -> Result<(), Self::Error> {
        if self.state.parts.feature144.rows.is_empty() { return Err(ServiceError::Control("nonempty front trim")); }
        let row = self.state.parts.feature144.rows.remove(0);
        self.retain_removed(row)?;
        self.trace.retired_front += 1;
        Ok(())
    }
    fn increment_latency496(&mut self) -> Result<(), Self::Error> { self.state.parts.latency496 += 1; Ok(()) }
    fn begin_end_trim(&mut self) -> Result<bool, Self::Error> { Ok(self.state.parts.ended517 != 0) }
    fn trim_last_and_refresh_end(&mut self) -> Result<(), Self::Error> {
        let row = self.state.parts.feature144.rows.pop().ok_or(ServiceError::Control("nonempty end trim"))?;
        self.retain_removed(row)?;
        self.trace.retired_last = true;
        Ok(())
    }
}
