const ROUND_TO_INTEGER: f64 = 6755399441055744.0;

pub(super) fn shade_radial_argb(
    bytes: &mut [u8],
    width: usize,
    height: usize,
    center: [f64; 2],
    radius_squared: f64,
    scale: f64,
    colors: &[u32],
) {
    let row_bytes = width.checked_mul(4).expect("canvas row size overflow");
    let image_bytes = row_bytes.checked_mul(height).expect("canvas image size overflow");
    assert!(image_bytes <= bytes.len(), "canvas image buffer is too short");
    assert!(width <= i32::MAX as usize && height <= i32::MAX as usize);
    assert!(!colors.is_empty() && colors.len() <= i32::MAX as usize);
    assert!(center.iter().all(|value| value.is_finite()));
    assert!(radius_squared.is_finite() && radius_squared > 0.0);
    assert!(scale.is_finite() && scale >= 0.0);
    if width == 0 || height == 0 { return; }

    #[cfg(target_arch = "x86_64")]
    {

        unsafe { shade_sse2(&mut bytes[..image_bytes], row_bytes, center, radius_squared, scale, colors); }
    }
    #[cfg(not(target_arch = "x86_64"))]
    for (y, row) in bytes[..image_bytes].chunks_exact_mut(row_bytes).enumerate() {
        let dy = y as f64 - center[1];
        let yy = dy * dy;
        for (x, pixel) in row.chunks_exact_mut(4).enumerate() {
            let dx = x as f64 - center[0];
            let squared = dx * dx + yy;
            let source = radial_color(squared, radius_squared, scale, colors);
            blend_pixel(pixel, source);
        }
    }
}

#[inline]
fn radial_color(squared: f64, radius_squared: f64, scale: f64, colors: &[u32]) -> u32 {
    if squared >= radius_squared { return colors[colors.len() - 1]; }
    let rounded = (squared.sqrt() * scale + ROUND_TO_INTEGER).to_bits() as u32 as i32;
    colors[rounded as usize]
}

#[cfg(target_arch = "x86_64")]
unsafe fn shade_sse2(
    bytes: &mut [u8],
    row_bytes: usize,
    center: [f64; 2],
    radius_squared: f64,
    scale: f64,
    colors: &[u32],
) {
    use std::arch::x86_64::*;

    unsafe {
        let center_x = _mm_set1_pd(center[0]);
        let radius = _mm_set1_pd(radius_squared);
        let factor = _mm_set1_pd(scale);
        let rounding = _mm_set1_pd(ROUND_TO_INTEGER);
        let last = colors[colors.len() - 1];
        for (y, row) in bytes.chunks_exact_mut(row_bytes).enumerate() {
            let dy = y as f64 - center[1];
            let yy = dy * dy;
            let y_squared = _mm_set1_pd(yy);
            let mut pairs = row.chunks_exact_mut(8);
            for (pair, pixels) in pairs.by_ref().enumerate() {
                let x = pair * 2;
                let positions = _mm_set_pd((x + 1) as f64, x as f64);
                let dx = _mm_sub_pd(positions, center_x);
                let squared = _mm_add_pd(_mm_mul_pd(dx, dx), y_squared);
                let outside = _mm_movemask_pd(_mm_cmpge_pd(squared, radius));
                let sources = if outside == 3 {

                    [last, last]
                } else {
                    let root = _mm_sqrt_pd(squared);
                    let rounded = _mm_add_pd(_mm_mul_pd(root, factor), rounding);
                    let mut values = [0.0_f64; 2];
                    _mm_storeu_pd(values.as_mut_ptr(), rounded);
                    std::array::from_fn(|lane| {
                        if outside & (1 << lane) != 0 { last }
                        else { colors[(values[lane].to_bits() as u32 as i32) as usize] }
                    })
                };
                let (first, second) = pixels.split_at_mut(4);
                blend_pixel(first, sources[0]);
                blend_pixel(second, sources[1]);
            }
            let tail = pairs.into_remainder();
            if !tail.is_empty() {
                let x = row_bytes / 4 - 1;
                let dx = x as f64 - center[0];
                blend_pixel(tail, radial_color(dx * dx + yy, radius_squared, scale, colors));
            }
        }
    }
}

#[inline]
fn blend_pixel(pixel: &mut [u8], source: u32) {
    let destination = u32::from_le_bytes([pixel[0], pixel[1], pixel[2], pixel[3]]);
    let inverse = 256u32.wrapping_sub(source >> 24);
    let mask = 0x00ff00ff;
    let rb = (((destination & mask).wrapping_mul(inverse) >> 8) & mask)
        .wrapping_add(source & mask);
    let ga = ((((destination >> 8) & mask).wrapping_mul(inverse) >> 8) & mask)
        .wrapping_add((source >> 8) & mask);
    let rb_overflow = (rb >> 8) & 0x00010001;
    let ga_overflow = (ga >> 8) & 0x00010001;
    let rb = (0x100u32.wrapping_sub(rb_overflow) | rb) & mask;
    let ga = ((0x100u32.wrapping_sub(ga_overflow) | ga) << 8) & 0xff00ff00;
    pixel.copy_from_slice(&(rb | ga).to_le_bytes());
}
