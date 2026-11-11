use super::*;
impl<P, H> graph::SecondSignalServices for Owner<P, H> {
    type Error = ServiceError;
    fn store_signal_status(&mut self, raw: u8) -> Result<(), ServiceError> {
        self.output.status = raw; Ok(())
    }
    fn snapshot_old_signal_range(&mut self) -> Result<usize, ServiceError> {
        if self.trace.old_count.is_some() { return Err(ServiceError::Control("old range already captured")); }
        let count = self.output.rows.len(); self.trace.old_count = Some(count); Ok(count)
    }
    fn release_old_signal(&mut self, ordinal: usize) -> Result<(), ServiceError> {
        if ordinal != self.trace.released_old.len() || self.trace.pending_retirement.is_some() {
            return Err(ServiceError::Control("old row retirement is not the next ordinal"));
        }
        let row = self.output.rows.get_mut(ordinal).ok_or(ServiceError::Control("old signal row absent"))?;
        self.trace.pending_retirement = Some(std::mem::take(row));
        self.trace.released_old.try_reserve(1).map_err(ServiceError::Allocation)?;
        self.trace.released_old.push(self.trace.pending_retirement.take().expect("retired row retained"));
        Ok(())
    }
    fn reset_signal_end_to_original_begin(&mut self) -> Result<(), ServiceError> {
        if self.trace.old_count != Some(self.trace.released_old.len()) || self.trace.pending_retirement.is_some()
            || self.output.rows.iter().any(|row| !row.is_empty()) { return Err(ServiceError::Control("old range still owns rows")); }

        self.output.rows.clear(); self.trace.reset_end = true; Ok(())
    }
    fn reserve_signal_for_selected_count(&mut self, selected: i64) -> Result<(), ServiceError> {
        if !self.trace.reset_end { return Err(ServiceError::Control("signal end was not reset")); }
        let count = usize::try_from(selected).map_err(|_| ServiceError::CountDomain(selected))?;
        self.output.rows.try_reserve(count).map_err(ServiceError::Allocation)
    }
    fn refill_coefficients328(&mut self) -> Result<(), ServiceError> {
        if self.coefficient_output.is_none() { return Err(ServiceError::Control("coefficient destination absent")); }
        let before = match std::mem::replace(&mut self.coefficients, Coefficients::Vacant) {
            Coefficients::Before(before) => before,
            other => { self.coefficients = other; return Err(ServiceError::Control("coefficient child already entered")); }
        };
        let output = self.coefficient_output.take().expect("coefficient destination admitted before owner move");
        let child = match before.begin_refill(self.trace.selection, output, self.coefficient_status) {
            Ok(child) => child,
            Err(failure) => {
                let cause = failure.refill.cause;
                self.coefficients = Coefficients::Refused(failure); return Err(ServiceError::CoefficientStart(cause));
            }
        };
        self.coefficients = Coefficients::Active(child);
        let complete = match &mut self.coefficients {
            Coefficients::Active(child) => matches!(child.poll(usize::MAX), queue::refill::Poll::Complete),
            _ => false,
        };
        if !complete { return Err(ServiceError::Coefficient); }
        let child = match std::mem::replace(&mut self.coefficients, Coefficients::Vacant) {
            Coefficients::Active(child) => child,
            other => { self.coefficients = other; return Err(ServiceError::Control("completed coefficient child absent")); }
        };
        match child.into_completed() {
            Ok(completed) => { self.coefficients = Coefficients::After(completed); Ok(()) }
            Err(child) => { self.coefficients = Coefficients::Active(child); Err(ServiceError::Coefficient) }
        }
    }
    fn snapshot_signal_prefix(&mut self, selected: i64) -> Result<(), ServiceError> {
        let count = usize::try_from(selected).map_err(|_| ServiceError::CountDomain(selected))?;
        if count > self.signal.parts.fifo.len() || self.trace.snapshot_count.is_some() {
            return Err(ServiceError::Control("signal prefix unavailable or already captured"));
        }
        self.trace.snapshot_count = Some(self.signal.parts.fifo.len()); Ok(())
    }
    fn move_signal_ordinal(&mut self, ordinal: usize) -> Result<(), ServiceError> {
        if self.trace.moved != ordinal || self.output.rows.len() != ordinal {
            return Err(ServiceError::Control("signal move is not the next ordinal"));
        }
        let slot = self.signal.parts.fifo.get_mut(ordinal).ok_or(ServiceError::Control("signal FIFO slot absent"))?;
        if slot.as_ref().is_none_or(|row| row.len() != self.signal.parts.width) {
            return Err(ServiceError::Control("signal FIFO row absent or wrong width"));
        }

        let row = slot.take().ok_or(ServiceError::Control("signal FIFO row already moved"))?;
        self.output.rows.push(row); self.trace.moved += 1; Ok(())
    }
    fn erase_moved_prefix_using_fresh_head(&mut self, selected: i64) -> Result<(), ServiceError> {
        let count = usize::try_from(selected).map_err(|_| ServiceError::CountDomain(selected))?;
        let fresh_count = self.signal.parts.fifo.len();
        self.trace.fresh_count_before_erase = Some(fresh_count);
        if self.trace.moved != count || fresh_count < count || self.signal.parts.fifo.iter().take(count).any(Option::is_some) {
            return Err(ServiceError::Control("signal erased prefix still owns rows"));
        }

        self.signal.parts.fifo.drain(..count); self.trace.erased = Some(count); Ok(())
    }
}
