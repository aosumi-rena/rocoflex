use std::collections::VecDeque;
use crate::{conditional_gate::Batch, ltv_stream::{Outputs, ExchangeFailure, LtvInputQueue}};

pub struct Parts {
    pub hop: usize, pub transform: usize,
    pub output: Batch,

    pub next_current: VecDeque<Vec<f32>>,
    pub terminal_signal: VecDeque<Vec<f32>>, pub terminal_coefficients: VecDeque<Vec<f32>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingError { Dimensions, OutputWidth, CurrentWidth, TerminalWidth }
pub struct BindingFailure { pub error: BindingError, pub parts: Parts }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Required { PriorFailure, Width, NextCurrent, TerminalSignal, TerminalCoefficient, TraceAllocation, OutputAllocation }
pub enum Error { Required(Required), Allocation { at: Required, error: std::collections::TryReserveError } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage { NextCurrent, TerminalSignal, TerminalCoefficient }
pub enum SupplyError { PriorFailure, Width, Allocation(std::collections::TryReserveError) }
pub struct SupplyFailure { pub error: SupplyError, pub storage: Storage, pub rows: VecDeque<Vec<f32>> }
pub struct Trace {
    pub retired_outputs: Vec<Batch>, pub published: usize, pub acquired_current: usize,
    pub acquired_terminal_signal: usize, pub acquired_terminal_coefficients: usize,
    pub failure: Option<Required>,
}
pub struct ValueOutputs { parts: Parts, trace: Trace }
impl ValueOutputs {

    pub fn from_explicit_parts(parts: Parts) -> Result<Self, BindingFailure> {
        let error = if parts.hop == 0 || parts.hop > i32::MAX as usize
            || parts.transform < parts.hop || parts.transform > i32::MAX as usize { Some(BindingError::Dimensions) }
            else if parts.output.rows.iter().any(|row| row.len() != parts.hop) { Some(BindingError::OutputWidth) }
            else if parts.next_current.iter().any(|row| row.len() != parts.hop) { Some(BindingError::CurrentWidth) }
            else if parts.terminal_signal.iter().any(|row| row.len() != parts.hop)
                || parts.terminal_coefficients.iter().any(|row| row.len() != parts.transform) { Some(BindingError::TerminalWidth) }
            else { None };
        if let Some(error) = error { return Err(BindingFailure { error, parts }); }
        Ok(Self { parts, trace: Trace { retired_outputs: Vec::new(), published: 0,
            acquired_current: 0, acquired_terminal_signal: 0, acquired_terminal_coefficients: 0, failure: None } })
    }
    pub fn parts(&self) -> &Parts { &self.parts }
    pub fn trace(&self) -> &Trace { &self.trace }

    pub fn append_supplied_storage(&mut self, storage: Storage, mut rows: VecDeque<Vec<f32>>)
        -> Result<(), SupplyFailure>
    {
        let width = match storage { Storage::NextCurrent | Storage::TerminalSignal => self.parts.hop,
            Storage::TerminalCoefficient => self.parts.transform };
        let error = if self.trace.failure.is_some() { Some(SupplyError::PriorFailure) }
            else if rows.iter().any(|row| row.len() != width) { Some(SupplyError::Width) }
            else { None };
        if let Some(error) = error { return Err(SupplyFailure { error, storage, rows }); }
        let destination = match storage { Storage::NextCurrent => &mut self.parts.next_current,
            Storage::TerminalSignal => &mut self.parts.terminal_signal,
            Storage::TerminalCoefficient => &mut self.parts.terminal_coefficients };
        if let Err(error) = destination.try_reserve(rows.len()) {
            return Err(SupplyFailure { error: SupplyError::Allocation(error), storage, rows });
        }
        destination.append(&mut rows);
        Ok(())
    }

    pub fn take_output(&mut self) -> Result<Batch, Error> {
        self.ready()?;
        Ok(std::mem::replace(&mut self.parts.output, Batch { rows: Vec::new(), status: 0 }))
    }
    pub fn into_parts(self) -> Result<(Parts, Trace), Self> {
        if self.trace.failure.is_some() { return Err(self); }
        Ok((self.parts, self.trace))
    }
    fn ready(&self) -> Result<(), Error> {
        if self.trace.failure.is_some() { Err(Error::Required(Required::PriorFailure)) } else { Ok(()) }
    }
    fn stop(&mut self, at: Required) -> Error {
        if self.trace.failure.is_none() { self.trace.failure = Some(at); }
        Error::Required(at)
    }
}
impl Outputs for ValueOutputs {
    type Error = Error;
    fn clear_output_and_set_status(&mut self, status: u8) -> Result<(), Error> {
        self.ready()?;
        if let Err(error) = self.trace.retired_outputs.try_reserve(1) {
            self.trace.failure = Some(Required::TraceAllocation);
            return Err(Error::Allocation { at: Required::TraceAllocation, error });
        }

        let old = std::mem::replace(&mut self.parts.output, Batch { rows: Vec::new(), status });
        self.trace.retired_outputs.push(old);
        Ok(())
    }
    fn emit_then_acquire(&mut self, completed: Vec<f32>) -> Result<Vec<f32>, ExchangeFailure<Error>> {
        if let Err(reason) = self.ready() { return Err(ExchangeFailure { reason, returned_completed: Some(completed) }); }
        if completed.len() != self.parts.hop {
            let reason = self.stop(Required::Width);
            return Err(ExchangeFailure { reason, returned_completed: Some(completed) });
        }
        if let Err(error) = self.parts.output.rows.try_reserve(1) {
            self.trace.failure = Some(Required::OutputAllocation);
            return Err(ExchangeFailure { reason: Error::Allocation { at: Required::OutputAllocation, error },
                returned_completed: Some(completed) });
        }
        self.parts.output.rows.push(completed);
        self.trace.published += 1;

        match self.parts.next_current.pop_front() {
            Some(row) => { self.trace.acquired_current += 1; Ok(row) }
            None => Err(ExchangeFailure { reason: self.stop(Required::NextCurrent), returned_completed: None }),
        }
    }
    fn acquire_terminal_row(&mut self, queue: LtvInputQueue, width: usize) -> Result<Vec<f32>, Error> {
        self.ready()?;
        let expected = match queue { LtvInputQueue::Signal40 => self.parts.hop,
            LtvInputQueue::Coefficient80 => self.parts.transform };
        if width != expected { return Err(self.stop(Required::Width)); }
        let row = match queue { LtvInputQueue::Signal40 => self.parts.terminal_signal.pop_front(),
            LtvInputQueue::Coefficient80 => self.parts.terminal_coefficients.pop_front() };
        match row {
            Some(row) => {
                match queue { LtvInputQueue::Signal40 => self.trace.acquired_terminal_signal += 1,
                    LtvInputQueue::Coefficient80 => self.trace.acquired_terminal_coefficients += 1 }

                Ok(row)
            }
            None => Err(self.stop(match queue { LtvInputQueue::Signal40 => Required::TerminalSignal,
                LtvInputQueue::Coefficient80 => Required::TerminalCoefficient })),
        }
    }
}
