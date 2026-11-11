use super::*;
use rows::{EndValues, RowValues};

impl<S: Storage, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> graph::FractionServices for Owner<S, U, P, G, M, R, H> {
    type Error = ServiceError<S::Error>;
    fn signed_i32_to_f32_bits(&mut self, value: i32) -> Result<u32, Self::Error> {
        self.state.parts.math.signed_i32_to_f32_bits(value).map_err(ServiceError::Math)
    }
    fn addss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.state.parts.math.addss_bits(left, right).map_err(ServiceError::Math)
    }
    fn divss_bits(&mut self, left: u32, right: u32) -> Result<u32, Self::Error> {
        self.state.parts.math.divss_bits(left, right).map_err(ServiceError::Math)
    }
}
impl<S: Storage, U: ScalarUpper, P: paired_features::Frames, G: paired_features::Gaussian,
    M: Module, R: paired_input::Rows, H> graph::InterpolationServices for Owner<S, U, P, G, M, R, H> {
    type Row<'a> = RowValues<'a, S::Error, U> where Self: 'a;
    type End<'a> = EndValues<'a, S::Error> where Self: 'a;
    fn clear_first_empty(&mut self, ended: u8) -> Result<(), Self::Error> {
        let output = &mut self.state.parts.output336;
        output.status = ended;
        for row in &mut output.rows {
            self.trace.pending_output = Some(std::mem::take(row));
            self.trace.retired_output.try_reserve(1).map_err(ServiceError::Allocation)?;
            self.trace.retired_output.push(self.trace.pending_output.take().expect("retained old336 row"));
        }
        output.rows.clear(); output.status = ended; Ok(())
    }
    fn create_history_from_first(&mut self) -> Result<(), Self::Error> {
        let first = self.upstream.prepared().input().prepared().paired_features().rows.first()
            .ok_or(ServiceError::Input(0))?;
        let width = first.len(); self.trace.history_width = Some(width);
        if width > i32::MAX as usize { return Err(ServiceError::WidthDomain); }
        let p = &mut self.state.parts;
        p.width28 = width as i32;
        let next = p.storage.create_history(width).map_err(ServiceError::Create)?;
        if next.len() != width { self.trace.rejected_history = Some(next); return Err(ServiceError::Width); }
        self.trace.pending_history = p.history8.replace(next); self.trace.history_installed = true;
        if self.trace.pending_history.is_some() {
            self.trace.retired_history.try_reserve(1).map_err(ServiceError::Allocation)?;
            self.trace.retired_history.push(self.trace.pending_history.take().expect("retained prior history"));
        }
        Ok(())
    }
    fn prepare_output336(&mut self, needed: usize) -> Result<(), Self::Error> {
        self.trace.preparation = Some(needed);
        let p = &mut self.state.parts;
        p.storage.prepare336(&mut p.output336, needed, p.width28 as usize).map_err(ServiceError::Prepare)
    }
    fn set_output336_status(&mut self, ended: u8) -> Result<(), Self::Error> {
        self.state.parts.output336.status = ended; Ok(())
    }
    fn check_nonempty_arithmetic(&mut self) -> Result<(), Self::Error> {

        self.trace.arithmetic_admitted = true;
        if self.state.parts.math.failure().is_some() { return Err(ServiceError::Control("failed interpolation math owner")); }
        Ok(())
    }
    fn bind_row(&mut self, input: usize) -> Result<graph::RowBinding<Self::Row<'_>>, Self::Error> {
        self.trace.row = Some(input);
        let current = self.upstream.prepared().input().prepared().paired_features().rows.get(input)
            .ok_or(ServiceError::Input(input))?;
        let p = &mut self.state.parts;
        let width = p.width28 as usize;
        if current.len() != width { return Err(ServiceError::Width); }
        let history = p.history8.as_mut().ok_or(ServiceError::History)?;
        let first = p.seen16 == 0;
        Ok(graph::RowBinding { first, width, services: RowValues { current, history,
            output: &mut p.output336, seen: &mut p.seen16, math: &mut p.math,
            trace: &mut self.trace, error: std::marker::PhantomData } })
    }
    fn bind_end_copies(&mut self) -> Result<Self::End<'_>, Self::Error> {
        let p = &mut self.state.parts;
        let history = p.history8.as_ref().ok_or(ServiceError::History)?;
        Ok(EndValues { history, output: &mut p.output336, trace: &mut self.trace,
            error: std::marker::PhantomData })
    }
}
