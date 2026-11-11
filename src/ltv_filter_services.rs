use super::*;

impl<M: ltv_synthesis::SourceCoefficientMath, F: SourceLtvFft, K: PairedKernel, O: SpanObserver>
    graph::FilterValueServices for Run<M, F, K, O>
{
    type Error = ServiceError<K::Error, O::Error>;

    fn transform_coefficients(&mut self) -> Result<(), Self::Error> {
        if self.trace.coefficient_input_loaned || self.trace.retired_coefficient_output.is_some() {
            return Err(ServiceError::Control("coefficient attempt already entered"));
        }
        let (math, fft) = match std::mem::replace(&mut self.state.backend, Backends::Vacant) {
            Backends::Ready { math, fft } => (math, fft),
            backend => {
                self.state.backend = backend;
                return Err(ServiceError::Control("numeric providers are not ready"));
            }
        };
        self.trace.coefficient_input_loaned = true;
        let rows = std::mem::take(&mut self.input.coefficients.rows);
        let child = ltv_synthesis::Prepared::from_explicit_rows(
            self.state.config.transform_length(), rows, math, fft);
        self.state.backend = match child {
            Ok(child) => Backends::Coefficients(child),
            Err(failure) => {
                let cause = failure.error;
                self.state.backend = Backends::Rejected(failure);
                return Err(ServiceError::CoefficientStart(cause));
            }
        };
        let result = match &mut self.state.backend {
            Backends::Coefficients(child) => child.poll(usize::MAX),
            _ => return Err(ServiceError::Control("coefficient child is not retained")),
        };
        match result {
            ltv_synthesis::Poll::Failed(failure) => return Err(ServiceError::Coefficient(failure)),
            ltv_synthesis::Poll::Pending { .. } => return Err(ServiceError::Control("coefficient child did not finish")),
            ltv_synthesis::Poll::Complete => {}
        }
        let child = match std::mem::replace(&mut self.state.backend, Backends::Vacant) {
            Backends::Coefficients(child) => child,
            backend => {
                self.state.backend = backend;
                return Err(ServiceError::Control("completed coefficient child is absent"));
            }
        };
        let completed = match child.into_completed() {
            Ok(completed) => completed,
            Err(child) => {
                self.state.backend = Backends::Coefficients(child);
                return Err(ServiceError::Control("coefficient completion was refused"));
            }
        };
        let parts = completed.into_parts();
        self.input.coefficients.rows = parts.original_rows;
        self.trace.coefficient_input_loaned = false;
        let produced = Batch { rows: parts.packed_rows, status: self.input.coefficients.status };
        self.trace.retired_coefficient_output = Some(std::mem::replace(
            &mut self.state.coefficient_output, produced));
        self.state.backend = Backends::Ready { math: parts.math, fft: parts.fft };
        Ok(())
    }
    fn signal_input_count(&self) -> usize { self.input.signal.rows.len() }
    fn append_signal(&mut self, ordinal: usize, frame: i32) -> Result<(), Self::Error> {
        if ordinal != self.trace.moved_signal_prefix {
            return Err(ServiceError::Control("signal move is not the next ordinal"));
        }
        let row = self.input.signal.rows.get(ordinal)
            .ok_or(ServiceError::Control("signal ordinal is absent"))?;
        if row.len() != frame as usize {
            return Err(ServiceError::SignalWidth { ordinal, expected: frame as usize, actual: row.len() });
        }
        if self.state.signal_fifo.len() >= i32::MAX as usize {
            return Err(ServiceError::Control("signal FIFO count exceeds the value domain"));
        }
        self.state.signal_fifo.try_reserve(1).map_err(ServiceError::Allocation)?;

        let row = std::mem::take(&mut self.input.signal.rows[ordinal]);
        self.state.signal_fifo.push_back(Some(row));
        self.trace.moved_signal_prefix += 1;
        Ok(())
    }
    fn finish_signal_input(&mut self) -> Result<(), Self::Error> {
        if self.trace.moved_signal_prefix != self.input.signal.rows.len() {
            return Err(ServiceError::Control("signal input still contains an unmoved suffix"));
        }
        self.input.signal.rows.clear();
        self.input.signal.status = 0;
        Ok(())
    }
    fn coefficient_output_count(&self) -> usize { self.state.coefficient_output.rows.len() }
    fn signal_fifo_count(&self) -> usize { self.state.signal_fifo.len() }
    fn coefficient_output_status(&self) -> u8 { self.state.coefficient_output.status }
    fn clear_pair_signal_and_set_status(&mut self, status: u8) -> Result<(), Self::Error> {
        self.trace.retired_pair_rows.try_reserve(self.state.pair_signal.rows.len())
            .map_err(ServiceError::Allocation)?;
        self.trace.retired_pair_rows.extend(self.state.pair_signal.rows.drain(..));
        self.state.pair_signal.status = status;
        Ok(())
    }
    fn reserve_pair_signal(&mut self, required: usize) -> Result<(), Self::Error> {
        self.state.pair_signal.rows.try_reserve(required).map_err(ServiceError::Allocation)
    }
    fn move_signal_to_pair(&mut self, ordinal: usize) -> Result<(), Self::Error> {
        if self.state.pair_signal.rows.len() != ordinal {
            return Err(ServiceError::Control("pair signal move is not the next ordinal"));
        }
        let slot = self.state.signal_fifo.get_mut(ordinal)
            .ok_or(ServiceError::Control("FIFO ordinal is absent"))?;
        let current = slot.as_ref().ok_or(ServiceError::Control("FIFO ordinal already moved"))?;
        if current.len() != self.state.frame as usize {
            return Err(ServiceError::SignalWidth { ordinal, expected: self.state.frame as usize, actual: current.len() });
        }
        let row = slot.take().ok_or(ServiceError::Control("FIFO move lost its row"))?;
        self.state.pair_signal.rows.push(row);
        Ok(())
    }
    fn erase_signal_prefix(&mut self, required: usize) -> Result<(), Self::Error> {
        if required > self.state.signal_fifo.len()
            || self.state.signal_fifo.iter().take(required).any(Option::is_some) {
            return Err(ServiceError::Control("FIFO erased prefix still contains owners"));
        }
        self.state.signal_fifo.drain(..required);
        self.trace.erased_signal_prefix = Some(required);
        Ok(())
    }
    fn observe_coefficient(&mut self, ordinal: usize) -> Result<(), Self::Error> {
        let length = self.state.coefficient_output.rows.get(ordinal)
            .ok_or(ServiceError::Control("observer coefficient ordinal is absent"))?.len();
        let mut current_length = i32::try_from(length)
            .map_err(|_| ServiceError::Control("observer length exceeds DWORD domain"))?;
        self.trace.observer_calls.try_reserve(1).map_err(ServiceError::Allocation)?;
        let index = self.trace.observer_calls.len();
        self.trace.observer_calls.push(ObserverCall { ordinal, initial_length: current_length,
            current_length, completed: false });
        let observer = self.observer.as_mut().ok_or(ServiceError::Control("observer owner is absent"))?;
        let row = &mut self.state.coefficient_output.rows[ordinal];
        let result = observer.coefficient_span(row, &mut current_length);
        self.trace.observer_calls[index].current_length = current_length;
        self.trace.observer_calls[index].completed = result.is_ok();

        result.map_err(ServiceError::Observer)
    }
    fn paired_call(&mut self) -> Result<(), Self::Error> {
        if self.state.kernel.config() != self.state.config {
            return Err(ServiceError::Control("initialized paired configuration changed"));
        }
        self.trace.kernel_attempted = true;
        let State { backend, kernel, coefficient_output, pair_signal, .. } = &mut self.state;
        let Backends::Ready { fft, .. } = backend else {
            return Err(ServiceError::Control("FFT owner is not ready for paired call"));
        };
        kernel.forward(coefficient_output, pair_signal, fft).map_err(ServiceError::Kernel)?;
        if !coefficient_output.rows.is_empty() || coefficient_output.status != 0
            || !pair_signal.rows.is_empty() || pair_signal.status != 0 {
            return Err(ServiceError::Control("paired return did not clear its source inputs"));
        }
        Ok(())
    }
    fn finish_observer(&mut self) -> Result<(), Self::Error> {
        self.trace.observer_finish_attempted = true;
        let observer = self.observer.as_mut().ok_or(ServiceError::Control("descriptor owner is absent"))?;
        observer.finish_descriptor().map_err(ServiceError::Observer)
    }
}
