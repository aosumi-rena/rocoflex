use super::{source_pixel, Error};
use super::super::image::BitmapLayout;

#[derive(Clone, Copy)]
enum Footprint {
    Empty,
    Single { x: i32, y: i32, pixel: [u8; 4] },
    Horizontal { x: i32, y: i32, pixels: [[u8; 4]; 2] },
    Vertical { x: i32, y: i32, pixels: [[u8; 4]; 2] },
    Quad { x: i32, y: i32, pixels: [[u8; 4]; 4] },
}

pub(super) struct ImmutableTexels<'a> {
    source: &'a [u8],
    layout: &'a BitmapLayout,
    last: Footprint,
}
impl<'a> ImmutableTexels<'a> {

    pub(super) fn new(source: &'a [u8], layout: &'a BitmapLayout) -> Self {
        Self { source, layout, last: Footprint::Empty }
    }
    pub(super) fn layout(&self) -> &BitmapLayout { self.layout }
    pub(super) fn single(&mut self, x: i32, y: i32) -> Result<[u8; 4], Error> {
        if let Footprint::Single { x: old_x, y: old_y, pixel } = self.last {
            if x == old_x && y == old_y { return Ok(pixel); }
        }
        let pixel = source_pixel(self.source, self.layout, x, y)?;
        self.last = Footprint::Single { x, y, pixel };
        Ok(pixel)
    }
    pub(super) fn horizontal(&mut self, x: i32, y: i32) -> Result<[[u8; 4]; 2], Error> {
        if let Footprint::Horizontal { x: old_x, y: old_y, pixels } = self.last {
            if x == old_x && y == old_y { return Ok(pixels); }
        }
        let first = source_pixel(self.source, self.layout, x, y)?;
        let second = source_pixel(self.source, self.layout, x + 1, y)?;
        let pixels = [first, second];
        self.last = Footprint::Horizontal { x, y, pixels };
        Ok(pixels)
    }
    pub(super) fn vertical(&mut self, x: i32, y: i32) -> Result<[[u8; 4]; 2], Error> {
        if let Footprint::Vertical { x: old_x, y: old_y, pixels } = self.last {
            if x == old_x && y == old_y { return Ok(pixels); }
        }
        let first = source_pixel(self.source, self.layout, x, y)?;
        let second = source_pixel(self.source, self.layout, x, y + 1)?;
        let pixels = [first, second];
        self.last = Footprint::Vertical { x, y, pixels };
        Ok(pixels)
    }
    pub(super) fn quad(&mut self, x: i32, y: i32) -> Result<[[u8; 4]; 4], Error> {
        if let Footprint::Quad { x: old_x, y: old_y, pixels } = self.last {
            if x == old_x && y == old_y { return Ok(pixels); }
        }

        let tl = source_pixel(self.source, self.layout, x, y)?;
        let tr = source_pixel(self.source, self.layout, x + 1, y)?;
        let br = source_pixel(self.source, self.layout, x + 1, y + 1)?;
        let bl = source_pixel(self.source, self.layout, x, y + 1)?;
        let pixels = [tl, tr, br, bl];
        self.last = Footprint::Quad { x, y, pixels };
        Ok(pixels)
    }
}
