use super::{Mask, PixelError};
use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiplyStatus { Pending, Complete, Stopped }
#[derive(Clone, Copy, Debug)]
pub struct MultiplyProgress { pub validated: bool, pub x: usize, pub y: usize }
#[must_use = "retain this partial image/mask borrow until completion or stopped failure"]
pub struct MaskMultiplier<'a> {
    pixels: &'a mut [u8],
    mask: &'a Mask,
    width: usize,
    height: usize,
    row_stride: usize,
    pixel_stride: usize,
    progress: MultiplyProgress,
    stopped: Option<PixelError>,
}
impl<'a> MaskMultiplier<'a> {
    pub fn new(pixels: &'a mut [u8], width: usize, height: usize,
        row_stride: usize, pixel_stride: usize, mask: &'a Mask) -> Self {
        Self { pixels, mask, width, height, row_stride, pixel_stride,
            progress: MultiplyProgress { validated: false, x: 0, y: 0 }, stopped: None }
    }
    pub fn progress(&self) -> MultiplyProgress { self.progress }
    pub fn partial_bytes(&self) -> &[u8] { self.pixels }
    pub fn mask(&self) -> &Mask { self.mask }
    pub fn failure(&self) -> Option<&PixelError> { self.stopped.as_ref() }
    pub fn status(&self) -> MultiplyStatus {
        if self.stopped.is_some() { MultiplyStatus::Stopped }
        else if self.progress.validated && self.progress.x == self.width { MultiplyStatus::Complete }
        else { MultiplyStatus::Pending }
    }

    pub fn advance(&mut self, pixels: NonZeroUsize) -> MultiplyStatus {
        if self.status() == MultiplyStatus::Pending {
            if let Err(error) = self.advance_inner(pixels.get()) { self.stopped = Some(error); }
        }
        self.status()
    }
    pub fn finish(mut self) -> Result<(), PixelError> {
        while self.status() == MultiplyStatus::Pending {
            if let Err(error) = self.advance_inner(usize::MAX) { self.stopped = Some(error); }
        }
        match self.stopped { Some(error) => Err(error), None => Ok(()) }
    }
    fn validate(&self) -> Result<(), PixelError> {
        if self.width != self.mask.width || self.height != self.mask.height
            || self.width == 0 || self.height == 0 { return Err(PixelError::Extent); }
        if self.pixel_stride < 4 || self.row_stride
            < self.width.checked_mul(self.pixel_stride).ok_or(PixelError::Size)? {
            return Err(PixelError::Layout);
        }
        let extent = (self.height - 1).checked_mul(self.row_stride)
            .and_then(|n| (self.width - 1).checked_mul(self.pixel_stride).and_then(|x| n.checked_add(x)))
            .and_then(|n| n.checked_add(4)).ok_or(PixelError::Size)?;
        if self.pixels.len() < extent || self.mask.bytes.len()
            != self.width.checked_mul(self.height).ok_or(PixelError::Size)? {
            return Err(PixelError::Layout);
        }
        Ok(())
    }
    fn advance_inner(&mut self, budget: usize) -> Result<(), PixelError> {
        if !self.progress.validated { self.validate()?; self.progress.validated = true; }
        let mut completed = 0usize;
        while self.progress.x < self.width && completed < budget {
            let x = self.progress.x;
            let y = self.progress.y;
            let at = y * self.row_stride + x * self.pixel_stride;
            let factor = (u32::from(self.mask.bytes[y * self.width + x]) + 127).min(255);
            let product = u32::from(self.pixels[at + 3]) * factor;
            for channel in 0..3 {
                self.pixels[at + channel] =
                    ((u32::from(self.pixels[at + channel]) * product + 32767) >> 16) as u8;
            }
            self.pixels[at + 3] = (product >> 8) as u8;
            self.progress.y += 1;
            if self.progress.y == self.height { self.progress.y = 0; self.progress.x += 1; }
            completed += 1;
        }
        Ok(())
    }
}
