use super::*;

pub(super) struct Pair<'a, O> {
    pub stream: &'a mut Stream<O>, pub coefficients: &'a mut Batch,
    pub signal: &'a mut Batch, pub fft: &'a mut dyn SourceLtvFft, pub trace: &'a mut Trace,
}
impl<O: Outputs> paired::PairedValueServices for Pair<'_, O> {
    type Error = ServiceError<O::Error>;
    fn clear_output_and_set_status(&mut self, status: u8) -> Result<(), Self::Error> {
        self.stream.outputs.clear_output_and_set_status(status).map_err(ServiceError::Output)
    }
    fn move_signal_input(&mut self, ordinal: usize) -> Result<(), Self::Error> {
        if ordinal != self.trace.moved_signal { return Err(ServiceError::Control); }
        let row = self.signal.rows.get_mut(ordinal).ok_or(ServiceError::Control)?;
        self.trace.pending_enqueue = Some((LtvInputQueue::Signal40, std::mem::take(row)));
        self.trace.moved_signal += 1;
        self.stream.signal.try_reserve(1).map_err(ServiceError::Allocation)?;
        let (_, row) = self.trace.pending_enqueue.take().ok_or(ServiceError::Control)?;
        self.stream.signal.push_back(row);
        Ok(())
    }
    fn move_coefficient_input(&mut self, ordinal: usize) -> Result<(), Self::Error> {
        if ordinal != self.trace.moved_coefficients { return Err(ServiceError::Control); }
        let row = self.coefficients.rows.get_mut(ordinal).ok_or(ServiceError::Control)?;
        self.trace.pending_enqueue = Some((LtvInputQueue::Coefficient80, std::mem::take(row)));
        self.trace.moved_coefficients += 1;
        self.stream.coefficients.try_reserve(1).map_err(ServiceError::Allocation)?;
        let (_, row) = self.trace.pending_enqueue.take().ok_or(ServiceError::Control)?;
        self.stream.coefficients.push_back(row);
        Ok(())
    }
    fn signal_count(&self) -> usize { self.stream.signal.len() }
    fn coefficient_count(&self) -> usize { self.stream.coefficients.len() }
    fn emitted_rows(&self) -> u64 { self.stream.emitted_rows }
    fn process_front(&mut self) -> Result<(), Self::Error> {
        let c = self.stream.config;
        if self.stream.signal.len() != c.window_length() / c.hop()
            || self.stream.coefficients.is_empty() { return Err(ServiceError::Control); }
        if self.stream.halted { return Err(ServiceError::Control); }
        crate::matrix_f32::check_environment().map_err(ServiceError::Environment)?;
        let input = emission::EmissionInput::from_explicit_lengths(c.hop(), c.transform_length())
            .map_err(|_| ServiceError::Control)?;
        let mut work = emission::EmissionWork::after_explicit_environment();
        let mut services = Window { stream: &mut *self.stream, fft: &mut *self.fft, trace: &mut *self.trace };
        let result = emission::process_window(input, &mut work, &mut services);
        self.stream.work = Some(work);
        result.map_err(|error| ServiceError::Emission(Box::new(error)))
    }
    fn append_source_zero(&mut self, queue: LtvInputQueue) -> Result<(), Self::Error> {
        let width = match queue { LtvInputQueue::Signal40 => self.stream.config.hop(),
            LtvInputQueue::Coefficient80 => self.stream.config.transform_length() };
        let mut row = self.stream.outputs.acquire_terminal_row(queue, width).map_err(ServiceError::Output)?;
        if row.len() != width {
            self.trace.retained_terminal = Some(row);
            return Err(ServiceError::Width);
        }

        row.fill(0.0);
        self.trace.retained_terminal = Some(row);
        let destination = match queue { LtvInputQueue::Signal40 => &mut self.stream.signal,
            LtvInputQueue::Coefficient80 => &mut self.stream.coefficients };
        destination.try_reserve(1).map_err(ServiceError::Allocation)?;
        let row = self.trace.retained_terminal.take().ok_or(ServiceError::Control)?;
        destination.push_back(row);
        match queue { LtvInputQueue::Signal40 => self.trace.terminal_signal += 1,
            LtvInputQueue::Coefficient80 => self.trace.terminal_coefficients += 1 }
        Ok(())
    }
    fn finish_moved_input_headers(&mut self) -> Result<(), Self::Error> {
        if self.trace.moved_signal != self.signal.rows.len()
            || self.trace.moved_coefficients != self.coefficients.rows.len() {
            return Err(ServiceError::Control);
        }

        self.signal.rows.clear(); self.signal.status = 0;
        self.trace.signal_header_cleared = true;
        self.coefficients.rows.clear(); self.coefficients.status = 0;
        self.trace.coefficient_header_cleared = true;
        Ok(())
    }
    fn finish_input_owner_facts(&mut self) -> Result<(), Self::Error> {

        if !self.trace.signal_header_cleared || !self.trace.coefficient_header_cleared
            || !self.signal.rows.is_empty() || !self.coefficients.rows.is_empty()
            || self.signal.status != 0 || self.coefficients.status != 0 {
            return Err(ServiceError::Control);
        }
        Ok(())
    }
}

struct Window<'a, O> { stream: &'a mut Stream<O>, fft: &'a mut dyn SourceLtvFft, trace: &'a mut Trace }
impl<O: Outputs> emission::EmissionValueServices for Window<'_, O> {
    type Error = ServiceError<O::Error>;
    fn phase(&self) -> i64 { self.stream.phase }
    fn partial_count(&self) -> usize { self.stream.partial_count }
    fn emitted_rows(&self) -> u64 { self.stream.emitted_rows }
    fn admit_window(&self, plan: EmissionPlan) -> Result<(), Self::Error> {
        let c = self.stream.config;
        if plan.visible_count() > c.hop()
            || plan.visible_start().checked_add(plan.visible_count()).is_none_or(|n| n > c.transform_length())
            || self.stream.partial_count >= c.hop()
            || self.stream.current.as_ref().is_none_or(|row| row.len() != c.hop()) {
            return Err(ServiceError::Width);
        }
        Ok(())
    }
    fn set_halted(&mut self, value: bool) { self.stream.halted = value; }
    fn accumulate_block(&mut self) -> Result<(), Self::Error> {
        let s = &mut *self.stream;
        let rows = s.signal.make_contiguous();
        let packed = s.coefficients.front().ok_or(ServiceError::Control)?;
        s.config.accumulate_block_in_scratch(s.arithmetic, &mut *self.fft,
            &s.window, rows, packed, &mut s.overlap, &mut s.scratch).map_err(ServiceError::Arithmetic)
    }
    fn copy_overlap_to_current(&mut self, source: usize, destination: usize, count: usize)
        -> Result<(), Self::Error>
    {
        let source_end = source.checked_add(count).ok_or(ServiceError::Width)?;
        let destination_end = destination.checked_add(count).ok_or(ServiceError::Width)?;
        let input = self.stream.overlap.get(source..source_end).ok_or(ServiceError::Width)?;
        let output = self.stream.current.as_mut().and_then(|row| row.get_mut(destination..destination_end))
            .ok_or(ServiceError::Width)?;
        output.copy_from_slice(input);
        Ok(())
    }
    fn set_partial_count(&mut self, value: usize) { self.stream.partial_count = value; }
    fn exchange_completed_current(&mut self) -> Result<(), Self::Error> {
        let completed = self.stream.current.take().ok_or(ServiceError::Control)?;
        let next = self.stream.outputs.emit_then_acquire(completed).map_err(ServiceError::Exchange)?;

        self.stream.current = Some(next);
        if self.stream.current.as_ref().is_none_or(|row| row.len() != self.stream.config.hop()) {
            return Err(ServiceError::Width);
        }
        Ok(())
    }
    fn set_emitted_rows(&mut self, value: u64) { self.stream.emitted_rows = value; }
    fn set_phase(&mut self, value: i64) { self.stream.phase = value; }
    fn shift_overlap_and_zero_tail(&mut self, hop: usize) -> Result<(), Self::Error> {
        let n = self.stream.overlap.len();
        if hop > n { return Err(ServiceError::Width); }
        self.stream.overlap.copy_within(hop..n, 0);
        self.stream.overlap[n-hop..].fill(0.0);
        Ok(())
    }
    fn release_signal_then_coefficient_fronts(&mut self) -> Result<(), Self::Error> {
        self.trace.released_signal.try_reserve(1).map_err(ServiceError::Allocation)?;
        let signal = self.stream.signal.pop_front().ok_or(ServiceError::Control)?;
        self.trace.released_signal.push(signal);

        self.trace.released_coefficients.try_reserve(1).map_err(ServiceError::Allocation)?;
        let coefficient = self.stream.coefficients.pop_front().ok_or(ServiceError::Control)?;
        self.trace.released_coefficients.push(coefficient);
        Ok(())
    }
}
