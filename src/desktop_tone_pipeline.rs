#[path = "desktop_tone_controls.rs"]
pub mod controls;
use controls::Controls;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub width: i32,
    pub height: i32,
    pub top: i32,
    pub image: Rect,
    pub control_span: Rect,
    pub label: Rect,
    pub tone_transform: [f32; 6],
    pub random: Rect,
    pub apply: Rect,
    pub hex: Rect,
}

impl Layout {
    pub fn new(width: i32, height: i32) -> Option<Self> {
        if width <= 34 || height <= 24 {
            return None;
        }
        let middle = (height - width) / 2;
        let top = middle - 4;
        let cx = (width / 2) as f32;
        let cy = (width / 2 + middle - 4) as f32;
        let epsilon = f32::from_bits(0x333b_bd2e);
        let quarter = width / 4;
        let remaining = width - 2 * quarter;
        Some(Self {
            width,
            height,
            top,
            image: Rect {
                x: 0.0,
                y: top as f32,
                width: width as f32,
                height: width as f32,
            },
            control_span: Rect {
                x: 6.0,
                y: (top + 6) as f32,
                width: (width - 12) as f32,
                height: (width - 12) as f32,
            },
            label: Rect {
                x: 12.0,
                y: (top + width - 32) as f32,
                width: (width - 24) as f32,
                height: 32.0,
            },
            tone_transform: [
                -epsilon,
                1.0,
                (cx * epsilon - cy) + cx,
                -1.0,
                -epsilon,
                (epsilon * cy + cx) + cy,
            ],
            random: Rect {
                x: 0.0,
                y: (height - 24) as f32,
                width: quarter as f32,
                height: 24.0,
            },
            apply: Rect {
                x: (width - quarter) as f32,
                y: (height - 24) as f32,
                width: quarter as f32,
                height: 24.0,
            },
            hex: Rect {
                x: (quarter + (remaining - 100) / 2) as f32,
                y: (height - 24) as f32,
                width: 100.0,
                height: 24.0,
            },
        })
    }

    pub fn drag(&self, down: [f32; 2], current: [f32; 2], hue: f32) -> Option<Controls> {
        if down.iter().chain(&current).any(|v| !v.is_finite()) {
            return None;
        }
        let x = (down[0] as f64).round_ties_even() as i32;
        let y = (down[1] as f64).round_ties_even() as i32;
        if x < 0 || x >= self.width || y < self.top || y >= self.top + self.width {
            return None;
        }
        controls::controls_from_pointer(self.width, self.height, current[0], current[1], hue)
    }

    pub fn thumb(&self, control: Controls) -> [f32; 2] {
        let span = self.control_span.width;
        let raw_x = control.tone * span + 6.0;
        let raw_y = (1.0 - control.gender) * span + (self.top as f32 + 6.0);
        [
            raw_x.clamp(17.0, (self.width - 17) as f32),
            raw_y.clamp((self.top + 17) as f32, (self.top + self.width - 17) as f32),
        ]
    }
}

pub const THUMB_DOT_DIAMETER: f32 = 4.0;
pub const THUMB_RING_DIAMETER: f32 = 32.0;
pub const THUMB_RING_STROKE: f32 = 1.0;
pub const LABEL_FONT_HEIGHT: f32 = 15.0;
pub const LABEL_ALPHA: u8 = 0xb2;
pub const LEFT_LABEL: &str = "MASCULINE";
pub const RIGHT_LABEL: &str = "FEMININE";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    Move([f32; 2]),
    Line([f32; 2]),
    Cubic([[f32; 2]; 3]),
    Close,
}

pub fn rounded_mask_path(size: i32, display_scale: f32) -> Option<Vec<PathCommand>> {
    if size <= 0 || !display_scale.is_finite() || display_scale <= 0.0 {
        return None;
    }
    let w = size as f32;
    let radius = (w * 0.5).min(display_scale * 20.0);
    let near = radius * 0.45;
    let far = w - radius;
    let far_near = w - near;
    Some(vec![
        PathCommand::Move([0.0, radius]),
        PathCommand::Cubic([[0.0, near], [near, 0.0], [radius, 0.0]]),
        PathCommand::Line([far, 0.0]),
        PathCommand::Cubic([[far_near, 0.0], [w, near], [w, radius]]),
        PathCommand::Line([w, far]),
        PathCommand::Cubic([[w, far_near], [far_near, w], [far, w]]),
        PathCommand::Line([radius, w]),
        PathCommand::Cubic([[near, w], [0.0, far_near], [0.0, far]]),
        PathCommand::Close,
    ])
}

pub struct Prepared {
    pub base: controls::BaseImage,
    pub target_size: i32,
    pub mask_path: Vec<PathCommand>,
}

pub fn prepare(hue: f32, width: i32, display_scale: f32) -> Option<Prepared> {
    if !hue.is_finite() || width <= 0 || !display_scale.is_finite() || display_scale <= 0.0 {
        return None;
    }
    let target_size = (width as f32 * display_scale) as i32;
    Some(Prepared {
        base: controls::base_image(hue),
        target_size,
        mask_path: rounded_mask_path(target_size, display_scale)?,
    })
}

pub fn bilinear_interior(pixels: [[u8; 4]; 4], fractional_x: u8, fractional_y: u8) -> [u8; 4] {
    let x = fractional_x as u32;
    let y = fractional_y as u32;
    let weights = [(256 - x) * (256 - y), x * (256 - y), (256 - x) * y, x * y];
    std::array::from_fn(|channel| {
        let sum: u32 = pixels
            .iter()
            .zip(weights)
            .map(|(pixel, weight)| pixel[channel] as u32 * weight)
            .sum();
        ((sum + 0x8000) >> 16) as u8
    })
}

pub fn finish_pixel(premultiplied_bgra: [u8; 4], mask_alpha: u8, brightness_scale: f32) -> [u8; 4] {
    let [blue, green, red, alpha] = premultiplied_bgra;
    let unpremultiply = |channel: u8| match alpha {
        0 => 0,
        255 => channel,
        _ => ((channel as u32 * 255 / alpha as u32).min(255)) as u8,
    };
    let rgb = [
        unpremultiply(red),
        unpremultiply(green),
        unpremultiply(blue),
    ];
    let [red, green, blue] = controls::multiply_brightness(rgb, brightness_scale);
    let premultiply = |channel: u8| match mask_alpha {
        0 => 0,
        255 => channel,
        _ => ((channel as u32 * mask_alpha as u32 + 127) >> 8) as u8,
    };
    [
        premultiply(blue),
        premultiply(green),
        premultiply(red),
        mask_alpha,
    ]
}

pub fn finish_image(
    resampled_bgra: &[[u8; 4]],
    mask_alpha: &[u8],
    brightness_scale: f32,
) -> Option<Vec<[u8; 4]>> {
    if resampled_bgra.len() != mask_alpha.len()
        || !brightness_scale.is_finite()
        || !(0.0..=1.0).contains(&brightness_scale)
    {
        return None;
    }
    Some(
        resampled_bgra
            .iter()
            .zip(mask_alpha)
            .map(|(&pixel, &alpha)| finish_pixel(pixel, alpha, brightness_scale))
            .collect(),
    )
}

