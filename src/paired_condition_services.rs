use super::*;

impl crate::logical_read_rows::ReadRows for Borrowed176<'_> {
    fn row_count(&self) -> usize { Borrowed176::row_count(self) }
    fn source_status(&self) -> u8 { Borrowed176::status(self) }
    fn read_row(&self, ordinal: usize) -> Option<&[f32]> { Borrowed176::row(self, ordinal) }
}

impl<W: Worker, P: paired_features::Frames, G: paired_features::Gaussian, M: Module, R: paired_input::Rows, H>
    graph::ConditionServices for Owner<W, P, G, M, R, H> {
    type Error = ServiceError<W::Error>;
    fn set208status_then_bind176(&mut self, cursor: i32, count: usize, ended517: u8) -> Result<(), Self::Error> {
        self.state.parts.condition208.status = ended517;

        let start = usize::try_from(cursor).map_err(|_| ServiceError::Range)?;
        let end = start.checked_add(count).ok_or(ServiceError::Range)?;
        if end > self.upstream.prepared().state().parts().queue400.len() { return Err(ServiceError::Range); }
        let p = &mut self.state.parts;
        p.input176_status = ended517;
        for (slot, ordinal) in p.input176_indices[..count].iter_mut().zip(start..end) { *slot = Some(ordinal); }
        p.input176_len = count; self.trace.bound = Some((start, end)); Ok(())
    }
    fn move_condition208_ordinal(&mut self, ordinal: usize) -> Result<(), Self::Error> {
        let p = &mut self.state.parts;
        let row = p.conditions432.get_mut(ordinal).and_then(Option::take).ok_or(ServiceError::Input)?;

        p.condition208.rows.push(row); self.trace.moved += 1; Ok(())
    }
    fn erase_condition432_prefix(&mut self, count: usize) -> Result<(), Self::Error> {
        if self.state.parts.conditions432.len() < count
            || self.state.parts.conditions432.iter().take(count).any(Option::is_some) {
            return Err(ServiceError::Control("moved condition432 prefix"));
        }
        for _ in 0..count { self.state.parts.conditions432.pop_front(); self.trace.erased += 1; }
        Ok(())
    }
    fn worker64_44180(&mut self) -> Result<(), Self::Error> {
        let p = &mut self.state.parts;
        let input = Borrowed176 { queue: &self.upstream.prepared().state().parts().queue400,
            indices: &p.input176_indices[..p.input176_len], status: p.input176_status };
        p.worker64.forward176(input, &mut p.condition208, &mut p.retained240).map_err(ServiceError::Worker)?;
        self.trace.worker_completed = true; Ok(())
    }
    fn zero176_pointer_words(&mut self) -> Result<(), Self::Error> {
        let p = &mut self.state.parts;
        p.input176_indices[..p.input176_len].fill(None); self.trace.zeroed176 = true; Ok(())
    }
    fn advance_current_cursor508_then_finish176(&mut self, count: usize) -> Result<(), Self::Error> {
        let p = &mut self.state.parts;
        let current = p.cursor_words.get_mut(self.index).ok_or(ServiceError::Cursor)?;
        *current = current.wrapping_add(count as u32); self.trace.cursor_after = Some(*current);
        p.input176_len = 0; p.input176_status = 0; self.trace.finished176 = true; Ok(())
    }
    fn cleanup208(&mut self) -> Result<(), Self::Error> {

        self.state.parts.condition208.status = 0;
        for row in &mut self.state.parts.condition208.rows {
            self.trace.pending_retirement = Some(std::mem::take(row));
            self.trace.retired208.try_reserve(1).map_err(ServiceError::Allocation)?;
            self.trace.retired208.push(self.trace.pending_retirement.take().expect("retained condition row"));
        }
        self.state.parts.condition208.rows.clear(); self.state.parts.condition208.status = 0; Ok(())
    }
}
