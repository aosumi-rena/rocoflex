#[path = "desktop_tone_edges.rs"]
pub mod edges;
use edges::pipeline;

struct FixedLine {
    current: i32,
    step: i32,
    remainder: i32,
    error: i32,
    count: i32,
}
impl FixedLine {
    fn new(start: i32, end: i32, count: i32) -> Self {
        let delta = end - start;
        let mut step = delta / count;
        let mut remainder = delta % count;
        if remainder <= 0 {
            remainder += count;
            step -= 1;
        }
        Self {
            current: start - 128,
            step,
            remainder,
            error: remainder - count,
            count,
        }
    }
    fn next(&mut self) -> i32 {
        let value = self.current;
        self.current += self.step;
        self.error += self.remainder;
        if self.error > 0 {
            self.error -= self.count;
            self.current += 1;
        }
        value
    }
}

fn sample(source: &[[u8; 4]], x_fixed: i32, y_fixed: i32) -> [u8; 4] {
    let x = x_fixed >> 8;
    let y = y_fixed >> 8;
    let x0 = x.clamp(0, 79) as usize;
    let y0 = y.clamp(0, 79) as usize;
    let x1 = (x + 1).clamp(0, 79) as usize;
    let y1 = (y + 1).clamp(0, 79) as usize;
    pipeline::bilinear_interior(
        [
            source[y0 * 80 + x0],
            source[y0 * 80 + x1],
            source[y1 * 80 + x0],
            source[y1 * 80 + x1],
        ],
        x_fixed as u8,
        y_fixed as u8,
    )
}

fn sample_span(
    source: &[[u8; 4]],
    inverse: f32,
    x: i32,
    y: i32,
    count: i32,
    target: &mut [[u8; 4]],
) {
    let center = x as f32 + 0.5;
    let end = count as f32 + center;
    let y_center = y as f32 + 0.5;
    let start_x = ((inverse * center) * 256.0) as i32;
    let end_x = ((inverse * end) * 256.0) as i32;
    let source_y = ((y_center * inverse) * 256.0) as i32;
    let mut x_line = FixedLine::new(start_x, end_x, count);
    let mut y_line = FixedLine::new(source_y, source_y, count);
    for pixel in target.iter_mut().take(count as usize) {
        *pixel = sample(source, x_line.next(), y_line.next());
    }
}

pub fn resize_opaque_80(source: &[[u8; 4]], size: i32) -> Option<Vec<[u8; 4]>> {
    if source.len() != 6400
        || source.iter().any(|pixel| pixel[3] != 255)
        || !(3..=4096).contains(&size)
    {
        return None;
    }
    if size == 80 {
        return Some(source.to_vec());
    }
    let forward = size as f32 / 80.0;
    let determinant = forward * forward;
    let inverse = (forward as f64 * (1.0f64 / determinant as f64)) as f32;
    let mut target = vec![[0; 4]; size as usize * size as usize];
    for y in 0..size {
        let row = &mut target[y as usize * size as usize..(y + 1) as usize * size as usize];
        sample_span(source, inverse, 0, y, 1, &mut row[..1]);
        sample_span(
            source,
            inverse,
            1,
            y,
            size - 2,
            &mut row[1..(size - 1) as usize],
        );
        sample_span(
            source,
            inverse,
            size - 1,
            y,
            1,
            &mut row[(size - 1) as usize..],
        );
        for channel in &mut row[(size - 1) as usize] {
            *channel = ((*channel as u32 * 254) >> 8) as u8;
        }
    }
    Some(target)
}

pub struct Stages {
    pub size: i32,
    pub base_bgra: Vec<[u8; 4]>,
    pub resized_bgra: Vec<[u8; 4]>,
    pub mask_alpha: Vec<u8>,
    pub final_bgra: Vec<[u8; 4]>,
    pub maximum_brightness: f32,
    pub brightness_scale: f32,
}

pub fn render(hue: f32, width: i32, display_scale: f32, brush_alpha: u8) -> Option<Stages> {
    let prepared = pipeline::prepare(hue, width, display_scale)?;
    let size = prepared.target_size;
    let base_bgra: Vec<_> = prepared
        .base
        .pixels
        .iter()
        .map(|&[r, g, b]| [b, g, r, 255])
        .collect();
    let resized_bgra = resize_opaque_80(&base_bgra, size)?;
    let segments = edges::flatten(&prepared.mask_path)?;
    let rows = edges::edge_rows(&segments, size)?;
    let mask_alpha = edges::mask_alpha(&rows, size, brush_alpha)?;
    let final_bgra =
        pipeline::finish_image(&resized_bgra, &mask_alpha, prepared.base.brightness_scale)?;
    Some(Stages {
        size,
        base_bgra,
        resized_bgra,
        mask_alpha,
        final_bgra,
        maximum_brightness: prepared.base.maximum_brightness,
        brightness_scale: prepared.base.brightness_scale,
    })
}

