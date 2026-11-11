use super::{Error, SqrtServices, TranslationParameters};
use super::super::{gradient::PreparedRender, image::BitmapLayout, lookup};
use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectangleStatus { Pending, Complete, Stopped }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectanglePhase { Setup, Rectangle, Row, Pixel, Complete }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelPhase { Sample, Address, Store, Advance }
#[derive(Clone, Copy, Debug)]
pub struct RectangleProgress {
    pub phase: RectanglePhase,
    pub rectangle_index: usize,
    pub active_rectangle: Option<[i32; 4]>,
    pub row: i64,
    pub bottom: i64,
    pub source_x: i32,
    pub remaining: i32,
    pub address: i64,
    pub y_squared_bits: Option<u64>,
    pub pixel_phase: PixelPhase,
    pub sample_x: Option<i32>,
    pub source: Option<u32>,
    pub completed_pixels: usize,
}

#[must_use = "retain this invocation, its partial bytes and SAME math owner across yields or failures"]
pub struct RectangleShader<'a, M: SqrtServices> {
    bytes: &'a mut [u8],
    math: &'a mut M,
    layout: BitmapLayout,
    rectangles: &'a [[i32; 4]],
    prepared: &'a PreparedRender,
    table: &'a lookup::Table,
    parameters: Option<TranslationParameters>,
    row_stride: i64,
    pixel_stride: i64,
    progress: RectangleProgress,
    stopped: Option<Error>,
}
impl<'a, M: SqrtServices> RectangleShader<'a, M> {

    pub fn new(bytes: &'a mut [u8], layout: BitmapLayout, rectangles: &'a [[i32; 4]],
        prepared: &'a PreparedRender, table: &'a lookup::Table, math: &'a mut M) -> Self {
        Self { bytes, math, layout, rectangles, prepared, table, parameters: None,
            row_stride: 0, pixel_stride: 0,
            progress: RectangleProgress { phase: RectanglePhase::Setup, rectangle_index: 0,
                active_rectangle: None, row: 0, bottom: 0, source_x: 0, remaining: 0,
                address: 0, y_squared_bits: None, pixel_phase: PixelPhase::Sample,
                sample_x: None, source: None, completed_pixels: 0 }, stopped: None }
    }
    pub fn progress(&self) -> RectangleProgress { self.progress }
    pub fn partial_bytes(&self) -> &[u8] { self.bytes }
    pub fn math(&self) -> &M { self.math }
    pub fn failure(&self) -> Option<&Error> { self.stopped.as_ref() }
    pub fn status(&self) -> RectangleStatus {
        if self.stopped.is_some() { RectangleStatus::Stopped }
        else if self.progress.phase == RectanglePhase::Complete { RectangleStatus::Complete }
        else { RectangleStatus::Pending }
    }

    pub fn advance(&mut self, pixels: NonZeroUsize) -> RectangleStatus {
        if self.status() != RectangleStatus::Pending { return self.status(); }
        if let Err(error) = self.advance_inner(pixels.get()) {
            self.stopped = Some(error);

        }
        self.status()
    }
    pub fn finish(mut self) -> Result<(), Error> {
        while self.status() == RectangleStatus::Pending {
            if let Err(error) = self.advance_inner(usize::MAX) { self.stopped = Some(error); }
        }
        match self.stopped { Some(error) => Err(error), None => Ok(()) }
    }
    fn advance_inner(&mut self, budget: usize) -> Result<(), Error> {
        let mut completed = 0usize;
        loop {
            match self.progress.phase {
                RectanglePhase::Setup => {
                    if !self.prepared.gradient.radial || !self.prepared.source_translation_flag {
                        return Err(Error::UnsupportedBranch);
                    }
                    if self.rectangles.is_empty() {
                        self.progress.phase = RectanglePhase::Complete;
                        return Ok(());
                    }
                    if self.layout.pixel_stride != 4 || self.layout.row_stride
                        < self.layout.width.checked_mul(4).ok_or(Error::Size)? {
                        return Err(Error::Layout);
                    }
                    self.parameters = Some(TranslationParameters::new(self.prepared, self.table, self.math)?);
                    self.row_stride = i64::try_from(self.layout.row_stride).map_err(|_| Error::Size)?;
                    self.pixel_stride = i64::try_from(self.layout.pixel_stride).map_err(|_| Error::Size)?;
                    self.progress.phase = RectanglePhase::Rectangle;
                }
                RectanglePhase::Rectangle => {
                    let Some(&rectangle) = self.rectangles.get(self.progress.rectangle_index) else {
                        self.progress.active_rectangle = None;
                        self.progress.phase = RectanglePhase::Complete;
                        return Ok(());
                    };
                    self.progress.active_rectangle = Some(rectangle);
                    let [_, y, _, height] = rectangle;

                    if height <= 0 { self.progress.rectangle_index += 1; continue; }
                    self.progress.bottom = i64::from(y).checked_add(i64::from(height)).ok_or(Error::Size)?;
                    self.progress.row = i64::from(y);
                    self.progress.phase = RectanglePhase::Row;
                }
                RectanglePhase::Row => {
                    if self.progress.row >= self.progress.bottom {
                        self.progress.rectangle_index += 1;
                        self.progress.phase = RectanglePhase::Rectangle;
                        continue;
                    }
                    let [x, _, width, _] = self.progress.active_rectangle.ok_or(Error::Unwritten)?;
                    let parameters = self.parameters.as_ref().ok_or(Error::Unwritten)?;

                    let distance = (self.progress.row as i32) as f64 - parameters.center[1];
                    self.progress.y_squared_bits = Some((distance * distance).to_bits());
                    self.progress.address = self.row_stride.checked_mul(self.progress.row)
                        .and_then(|at| self.pixel_stride.checked_mul(i64::from(x)).and_then(|dx| at.checked_add(dx)))
                        .ok_or(Error::NativeAddress)?;
                    self.progress.source_x = x;
                    self.progress.remaining = width.wrapping_add(1);
                    self.progress.phase = RectanglePhase::Pixel;
                }
                RectanglePhase::Pixel => {

                    if completed == budget { return Ok(()); }
                    self.progress.pixel_phase = PixelPhase::Sample;
                    self.progress.sample_x = Some(self.progress.source_x);
                    self.progress.source = None;
                    let parameters = self.parameters.as_ref().ok_or(Error::Unwritten)?;
                    let squared = f64::from_bits(self.progress.y_squared_bits.ok_or(Error::Unwritten)?);
                    let source = parameters.sample(self.progress.source_x, self.progress.row as i32,
                        squared, false, self.table, self.math)?;
                    self.progress.source = Some(source);
                    self.progress.source_x = self.progress.source_x.wrapping_add(1);
                    self.progress.pixel_phase = PixelPhase::Address;
                    let at = usize::try_from(self.progress.address).map_err(|_| Error::NativeAddress)?;
                    let end = at.checked_add(4).ok_or(Error::NativeAddress)?;
                    let pixel = self.bytes.get_mut(at..end).ok_or(Error::NativeAddress)?;
                    self.progress.pixel_phase = PixelPhase::Store;
                    let destination = u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]]);
                    pixel.copy_from_slice(&super::blend_full_coverage(source, destination).to_le_bytes());
                    self.progress.pixel_phase = PixelPhase::Advance;
                    self.progress.address = self.progress.address.checked_add(self.pixel_stride).ok_or(Error::NativeAddress)?;
                    self.progress.remaining = self.progress.remaining.wrapping_sub(1);
                    self.progress.completed_pixels = self.progress.completed_pixels.saturating_add(1);
                    completed += 1;
                    if self.progress.remaining <= 1 {
                        self.progress.row += 1;
                        self.progress.phase = RectanglePhase::Row;
                    }
                    if completed == budget { return Ok(()); }
                }
                RectanglePhase::Complete => return Ok(()),
            }
        }
    }
}
