use super::{blur, edge_table, geometry::Mask, path, rectangle};
#[path="pixels_path_mask.rs"] pub mod path_mask;
#[path="pixels_mask.rs"] pub mod mask_multiply;
pub use mask_multiply::{MaskMultiplier, MultiplyStatus};
pub use path_mask::path_mask_source_with_services;
#[derive(Debug)]
pub enum PixelError { Extent, Layout, Size, Allocation, Format, Blur(blur::BlurError) }

pub struct BitmapLayout { pub width: usize, pub height: usize, pub row_stride: usize, pub pixel_stride: usize }

pub struct SoftwareStorage {
    pub dimensions: [i32; 2], pub row_stride: usize, pub bytes: Vec<u8>,
}
impl SoftwareStorage {
    pub fn create_cleared_format2(dimensions: [i32; 2]) -> Result<Self, PixelError> {
        if dimensions.iter().any(|&x| x < 0) { return Err(PixelError::Extent); }

        let physical_width = dimensions[0].max(1) as u32;
        let packed = physical_width.checked_mul(4).ok_or(PixelError::Size)?;
        let row32 = packed.checked_add(3).ok_or(PixelError::Size)? & 0x7ffffffc;
        if row32 < packed { return Err(PixelError::Layout); }
        let row_stride = row32 as usize;
        let count = row_stride.checked_mul(dimensions[1].max(1) as usize).ok_or(PixelError::Size)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(count).map_err(|_| PixelError::Allocation)?;
        bytes.resize(count, 0);
        Ok(Self { dimensions, row_stride, bytes })
    }

    pub fn origin_descriptor_mode0(&self) -> Result<SoftwareMapDescriptor, PixelError> {
        let available_bytes = self.row_stride.checked_mul(self.dimensions[1] as usize)
            .ok_or(PixelError::Size)?;
        Ok(SoftwareMapDescriptor { byte_offset: 0, available_bytes, format: 2,
            row_stride: self.row_stride, pixel_stride: 4 })
    }
}
pub struct SoftwareMapDescriptor {
    pub byte_offset: usize, pub available_bytes: usize, pub format: i32,
    pub row_stride: usize, pub pixel_stride: usize,
}

pub fn multiply_mask(pixels: &mut [u8], width: usize, height: usize,
                     row_stride: usize, pixel_stride: usize, mask: &Mask)
    -> Result<(), PixelError> {
    match multiply_mask_scheduled(pixels, width, height, row_stride, pixel_stride, mask,
        &mut || None::<std::convert::Infallible>)? {
        None => Ok(()),
        Some((reason, _progress)) => match reason {},
    }
}
pub(super) fn multiply_mask_scheduled<C>(pixels: &mut [u8], width: usize, height: usize,
    row_stride: usize, pixel_stride: usize, mask: &Mask,
    checkpoint: &mut impl FnMut() -> Option<C>)
    -> Result<Option<(C, mask_multiply::MultiplyProgress)>, PixelError> {
    let budget = std::num::NonZeroUsize::new(height).unwrap_or(std::num::NonZeroUsize::MIN);
    let mut multiplier = MaskMultiplier::new(pixels, width, height, row_stride, pixel_stride, mask);
    loop {
        match multiplier.advance(budget) {
            MultiplyStatus::Pending => if let Some(reason) = checkpoint() {
                return Ok(Some((reason, multiplier.progress())));
            },
            MultiplyStatus::Complete | MultiplyStatus::Stopped => {
                let progress = multiplier.progress();
                multiplier.finish()?;
                return Ok(checkpoint().map(|reason| (reason, progress)));
            }
        }
    }
}
pub(super) fn blur_mapped(bytes: &mut [u8], layout: BitmapLayout, source_argument: i32)
    -> Result<(), PixelError> {
    match blur_mapped_scheduled(bytes, layout, source_argument,
        &mut || None::<std::convert::Infallible>)? {
        None => Ok(()),
        Some((reason, _partial)) => match reason {},
    }
}
pub(super) fn blur_mapped_scheduled<C>(bytes: &mut [u8], layout: BitmapLayout,
    source_argument: i32, checkpoint: &mut impl FnMut() -> Option<C>)
    -> Result<Option<(C, blur::FilterPartial)>, PixelError> {
    if layout.pixel_stride != 4 { return Err(PixelError::Layout); }
    let extent = layout.row_stride.checked_mul(layout.height).ok_or(PixelError::Size)?;
    let mapped = bytes.get_mut(..extent).ok_or(PixelError::Layout)?;
    let filtered = match blur::filter_format2_scheduled(mapped, layout.width, layout.height,
        layout.row_stride, source_argument, checkpoint).map_err(PixelError::Blur)? {
        blur::FilterOutput::Complete { horizontal: _, filtered } => filtered,
        blur::FilterOutput::Superseded { reason, partial } => return Ok(Some((reason, partial))),
    };

    mapped.copy_from_slice(&filtered);
    Ok(None)
}

pub fn path_mask_source(clip: [i32; 4], path: &path::Path, affine: [f32; 6])
    -> Result<Option<edge_table::EdgeTable>, edge_table::Error> {
    path_mask_source_with_services(clip,path,affine.map(f32::to_bits),&mut path::ellipse_source::ExistingHostValueFp)
        .map(|completed|completed.table).map_err(path_mask::Failure::into_error)
}
