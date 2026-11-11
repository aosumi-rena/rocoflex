#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub gender: f32,
    pub tone: f32,
    pub hue: f32,
}

fn byte(value: f32) -> u8 {
    value.round_ties_even() as u8
}

pub fn hsv_rgb(hue: f32, saturation: f32, value: f32) -> [u8; 3] {
    let scaled = (value * 255.0).clamp(0.0, 255.0);
    let maximum = byte(scaled);
    if saturation <= 0.0 {
        return [maximum; 3];
    }
    let s = saturation.min(1.0);
    let h = ((hue - hue.floor()) * 360.0) / 60.0;
    let fraction = h - h.floor();
    let minimum = byte((1.0 - s) * scaled);
    let decreasing = || byte(scaled * (1.0 - s * fraction));
    let increasing = || byte(scaled * (1.0 - s * (1.0 - fraction)));
    if h < 1.0 {
        [maximum, increasing(), minimum]
    } else if h < 2.0 {
        [decreasing(), maximum, minimum]
    } else if h < 3.0 {
        [minimum, maximum, increasing()]
    } else if h < 4.0 {
        [minimum, decreasing(), maximum]
    } else if h < 5.0 {
        [increasing(), minimum, maximum]
    } else {
        [maximum, minimum, decreasing()]
    }
}

pub fn rgb_hsv(rgb: [u8; 3]) -> [f32; 3] {
    let [r, g, b] = rgb.map(f32::from);
    let maximum = r.max(g).max(b);
    if maximum == 0.0 {
        return [0.0; 3];
    }
    let delta = maximum - r.min(g).min(b);
    let saturation = delta / maximum;
    let mut hue = 0.0;
    if delta != 0.0 && saturation > 0.0 {
        let inverse = 1.0 / delta;
        let red_distance = (maximum - r) * inverse;
        let green_distance = (maximum - g) * inverse;
        let blue_distance = (maximum - b) * inverse;
        let sector = if maximum == r {
            blue_distance - green_distance
        } else if maximum == g {
            (red_distance + 2.0) - blue_distance
        } else {
            (green_distance + 4.0) - red_distance
        };
        hue = sector * f32::from_bits(0x3e2aaaab);
        if hue < 0.0 {
            hue += 1.0;
        }
    }
    [hue, saturation, maximum / 255.0]
}

pub fn rgb_from_controls(controls: Controls) -> [u8; 3] {
    hsv_rgb(
        controls.hue,
        (1.0 - controls.gender) * 0.5 + 0.4,
        controls.tone * 0.5 + 0.4,
    )
}

pub fn controls_from_rgb(rgb: [u8; 3]) -> Controls {
    let [hue, saturation, value] = rgb_hsv(rgb);
    Controls {
        gender: 1.0 - ((saturation.max(0.4) - 0.4) * 2.0).min(1.0),
        tone: ((value.max(0.4) - 0.4) * 2.0).min(1.0),
        hue,
    }
}

pub fn multiply_brightness(rgb: [u8; 3], factor: f32) -> [u8; 3] {
    let [hue, saturation, value] = rgb_hsv(rgb);
    hsv_rgb(hue, saturation, (value * factor).min(1.0))
}

pub fn controls_from_pointer(
    width: i32,
    height: i32,
    x: f32,
    y: f32,
    hue: f32,
) -> Option<Controls> {
    let d = (width - 12).max(0) as f32;
    if d == 0.0 {
        return None;
    }
    let top = ((height - width) / 2 - 4) as f32;
    Some(Controls {
        gender: (((top + 6.0) + d - y) / d).clamp(0.0, 1.0),
        tone: ((x - 6.0) / d).clamp(0.0, 1.0),
        hue,
    })
}

pub struct BaseImage {
    pub pixels: Vec<[u8; 3]>,
    pub maximum_brightness: f32,
    pub brightness_scale: f32,
}

pub fn base_image(hue: f32) -> BaseImage {
    let mut pixels = vec![[0; 3]; 80 * 80];
    let mut maximum_brightness = 0.0f32;
    for x in 0..80 {
        for y in 0..80 {
            let saturation = (((y as f32 / 80.0) - 1.0) + 1.0) * 0.5 + 0.4;
            let value = (x as f32 / 80.0) * 0.5 + 0.4;
            let rgb = hsv_rgb(hue, saturation, value);
            let [r, g, b] = rgb.map(|channel| f32::from(channel) / 255.0);
            let red = (r * r) * 0.241;
            let green = (g * g) * 0.691;
            let blue = (b * b) * 0.068;
            maximum_brightness = maximum_brightness.max((blue + (red + green)).sqrt());
            pixels[y * 80 + x] = rgb;
        }
    }
    let brightness_scale = if maximum_brightness > 0.7 {
        0.7 / maximum_brightness
    } else {
        1.0
    };
    BaseImage {
        pixels,
        maximum_brightness,
        brightness_scale,
    }
}

