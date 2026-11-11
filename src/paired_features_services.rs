use super::*;

impl<P: Frames, G: Gaussian, M: Module, R: paired_input::Rows, H> graph::QueueServices for Owner<P, G, M, R, H> {
    type Error = ServiceError<P::Error, G::Error>;
    fn borrow_input_row(&mut self, row: usize) -> Result<(), Self::Error> {

        let source = self.input.prepared().state().parts().feature144.rows.get(row)
            .ok_or(ServiceError::Control("current feature144 row"))?;
        self.cached_row = Some(row);
        self.trace.row = Some(row); self.trace.input_width = Some(source.len());
        Ok(())
    }
    fn prepare_noise_frame(&mut self, row: usize) -> Result<(), Self::Error> {
        let fail = |error| ServiceError::Frame(graph::FrameError::Service(error));
        if self.cached_row != Some(row) || self.pending_frame.is_some() || self.noise_halted {
            return Err(fail(FrameServiceError::Control("fresh admitted feature row")));
        }
        let groups = self.state.parts.groups;
        let source = self.input.prepared().state().parts().feature144.rows.get(row)
            .ok_or_else(|| fail(FrameServiceError::Control("captured feature row")))?;
        if source.len() != groups {
            return Err(fail(FrameServiceError::InputWidth { expected: groups, actual: source.len() }));
        }
        let completed = self.completed_frames.checked_add(1)
            .ok_or_else(|| fail(FrameServiceError::CounterOverflow))?;
        self.noise_halted = true;
        let frame = self.state.parts.frames.acquire().map_err(|e| fail(FrameServiceError::Acquire(e)))?;
        let actual = frame.len();
        self.pending_frame = Some(frame); self.trace.acquired += 1;
        if actual != 2 * groups {
            return Err(fail(FrameServiceError::TargetWidth { expected: 2 * groups, actual }));
        }
        self.frame_complete = None;
        let mut work = graph::FrameWork::after_explicit_admission(groups);
        let result = {
            let mut services = FrameValues::<P::Error, G> { source,
                target: self.pending_frame.as_mut().expect("retained acquired frame"), groups,
                gaussian: &mut self.state.parts.gaussian, cache: &mut self.cache, trace: &mut self.trace,
                frames_error: std::marker::PhantomData };
            loop {
                match graph::advance_frame(&mut work, &mut services, usize::MAX) {
                    Ok(graph::FrameAdvance::Paused) => {},
                    Ok(graph::FrameAdvance::Complete(marker)) => break Ok(marker),
                    Err(error) => break Err(error),
                }
            }
        };
        self.frame_work = Some(work);
        self.frame_complete = Some(result.map_err(ServiceError::Frame)?);

        self.completed_frames = completed; self.noise_halted = false;
        Ok(())
    }
    fn append_prepared_frame(&mut self, row: usize) -> Result<(), Self::Error> {
        if self.cached_row != Some(row) || self.frame_complete.is_none() || self.pending_frame.is_none() {
            return Err(ServiceError::Control("completed frame for append"));
        }

        self.state.parts.queue400.try_reserve(1).map_err(ServiceError::Allocation)?;
        self.state.parts.queue400.push_back(self.pending_frame.take().expect("retained completed frame"));
        self.trace.appended += 1; self.cached_row = None;
        Ok(())
    }
}
struct FrameValues<'a, E, G> {
    source: &'a [f32], target: &'a mut [f32], groups: usize,
    gaussian: &'a mut G, cache: &'a mut gaussian_noise::PartnerCache, trace: &'a mut Trace,
    frames_error: std::marker::PhantomData<fn() -> E>,
}
impl<E, G: Gaussian> graph::FrameServices for FrameValues<'_, E, G> {
    type Error = FrameServiceError<E, G::Error>;
    fn copy_input_lane(&mut self, lane: usize) -> Result<(), Self::Error> {
        let bits = self.source[lane].to_bits();
        self.target[lane] = f32::from_bits(bits);
        self.trace.copied += 1; self.trace.last_bits = Some(bits); Ok(())
    }
    fn standard_gaussian_and_store(&mut self, lane: usize) -> Result<(), Self::Error> {
        let bits = self.gaussian.standard(self.cache).map_err(FrameServiceError::Gaussian)?;
        self.target[self.groups + lane] = f32::from_bits(bits);
        self.trace.gaussian_stored += 1; self.trace.last_bits = Some(bits); Ok(())
    }
}
