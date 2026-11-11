use super::{allocate_zero_mask, composite_sorted_pass, domain, origin_from_sample,
    ring_sample_with_math, sort_records, Error, Mask, MaskFailure, MathServices, PassBoundary};
use super::super::producer;
use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mask16Status { Pending, Complete, Stopped }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mask16Phase { RingAndOrigin, Produce, Sort, Composite, Complete }
#[derive(Clone, Copy, Debug)]
pub struct Mask16Progress { pub next_ordinal: usize, pub phase: Mask16Phase }

#[must_use = "keep partial mask, input borrows and SAME math owner until completion or stopped failure"]
pub struct Mask16Renderer<'p, 's, 'm, M: MathServices> {
    rect: [i32; 4],
    probe: [f32; 2],
    radius: f32,
    saved_initial_zoom: f32,
    passes: &'p [PassBoundary<'s>; 16],
    math: &'m mut M,
    partial: Option<Mask>,
    progress: Mask16Progress,
    stopped: Option<MaskFailure>,
}
#[must_use = "this is a completed mask only, not a published image or native return"]
pub struct Mask16Completed<'m, M: MathServices> { mask: Mask, math: &'m mut M }
impl<'m, M: MathServices> Mask16Completed<'m, M> {
    pub fn mask(&self) -> &Mask { &self.mask }
    pub fn math(&self) -> &M { self.math }
    pub fn into_parts(self) -> (Mask, &'m mut M) { (self.mask, self.math) }
}
impl<'p, 's, 'm, M: MathServices> Mask16Renderer<'p, 's, 'm, M> {
    pub fn new(rect: [i32; 4], probe: [f32; 2], radius: f32, saved_initial_zoom: f32,
        passes: &'p [PassBoundary<'s>; 16], initial_line_pattern: u32, math: &'m mut M)
        -> Result<Self, MaskFailure> {

        domain(rect, saved_initial_zoom).map_err(|cause| MaskFailure {
            cause, partial: None, ordinal: 0, composite_failure: None })?;
        let partial = allocate_zero_mask(rect, initial_line_pattern).map_err(|cause| MaskFailure {
            cause, partial: None, ordinal: 0, composite_failure: None })?;
        Ok(Self { rect, probe, radius, saved_initial_zoom, passes, math, partial: Some(partial),
            progress: Mask16Progress { next_ordinal: 0, phase: Mask16Phase::RingAndOrigin }, stopped: None })
    }
    pub fn progress(&self) -> Mask16Progress { self.progress }
    pub fn status(&self) -> Mask16Status {
        if self.stopped.is_some() { Mask16Status::Stopped }
        else if self.progress.next_ordinal == 16 { Mask16Status::Complete }
        else { Mask16Status::Pending }
    }
    pub fn partial(&self) -> Option<&Mask> {
        match &self.stopped { Some(error) => error.partial.as_ref(), None => self.partial.as_ref() }
    }
    pub fn math(&self) -> &M { self.math }
    pub fn failure(&self) -> Option<&MaskFailure> { self.stopped.as_ref() }

    pub fn advance(&mut self, passes: NonZeroUsize) -> Mask16Status {
        if self.status() == Mask16Status::Pending { self.run_passes(passes.get()); }
        self.status()
    }
    fn run_passes(&mut self, budget: usize) {
        for _ in 0..budget {
            if self.status() != Mask16Status::Pending { return; }
            let ordinal = self.progress.next_ordinal;
            let pass = &self.passes[ordinal];
            let prepared = (|| -> Result<([f32; 2], Vec<[u32; 4]>), Error> {
                self.progress.phase = Mask16Phase::RingAndOrigin;
                let sample = ring_sample_with_math(ordinal, self.math)?;
                let origin = origin_from_sample(self.probe, self.radius, sample, self.rect, self.saved_initial_zoom)?;
                self.progress.phase = Mask16Phase::Produce;
                let mut obstacles = producer::produce_with_math(origin, pass.segments,
                    pass.segments.len(), self.math).map_err(Error::ProducerMath)?.records;
                self.progress.phase = Mask16Phase::Sort;
                sort_records(&mut obstacles)?;
                Ok((origin, obstacles))
            })();
            let (origin, obstacles) = match prepared {
                Ok(value) => value,
                Err(cause) => {
                    self.stopped = Some(MaskFailure { cause, partial: self.partial.take(), ordinal,
                        composite_failure: None });
                    return;
                }
            };
            self.progress.phase = Mask16Phase::Composite;
            let Some(mask) = self.partial.as_mut() else {
                self.stopped = Some(MaskFailure { cause: Error::Ownership, partial: None, ordinal,
                    composite_failure: None });
                return;
            };
            if let Err(failure) = composite_sorted_pass(mask, self.rect, origin,
                self.saved_initial_zoom, pass.viewport_reread_zoom, obstacles, self.math) {
                self.stopped = Some(MaskFailure { cause: Error::CompositeStopped,
                    partial: self.partial.take(), ordinal, composite_failure: Some(failure) });
                return;
            }
            self.progress.next_ordinal += 1;
            self.progress.phase = if self.progress.next_ordinal == 16 { Mask16Phase::Complete }
                else { Mask16Phase::RingAndOrigin };
        }
    }
    pub fn into_completed(mut self) -> Result<Mask16Completed<'m, M>, Self> {
        if self.status() != Mask16Status::Complete { return Err(self); }
        match self.partial.take() {
            Some(mask) => Ok(Mask16Completed { mask, math: self.math }),
            None => Err(self),
        }
    }

    pub fn into_stopped(mut self) -> Result<(MaskFailure, &'m mut M), Self> {
        match self.stopped.take() { Some(error) => Ok((error, self.math)), None => Err(self) }
    }

    pub fn into_partial_parts(mut self)
        -> Result<(Option<Mask>, Mask16Progress, &'m mut M), Self> {
        if self.status() != Mask16Status::Pending { return Err(self); }
        Ok((self.partial.take(), self.progress, self.math))
    }
    pub fn finish(mut self) -> Result<Mask, MaskFailure> {
        if self.status() == Mask16Status::Pending { self.run_passes(16); }
        if let Some(error) = self.stopped.take() { return Err(error); }
        self.partial.take().ok_or(MaskFailure { cause: Error::Ownership, partial: None,
            ordinal: self.progress.next_ordinal, composite_failure: None })
    }
}
