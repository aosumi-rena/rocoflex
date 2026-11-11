#[path = "desktop_paint_endpoint.rs"]
pub mod endpoint;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stop {
    pub position: f32,
    pub argb: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub gradient_end: [f32; 2],
    pub stops: Vec<Stop>,
    pub width: Option<f32>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingRecord {
    pub direction: [f32; 2],
    pub argb: u32,
    pub weight: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PointCommands {
    pub center: [f32; 2],
    pub radius: f32,
    pub argb: u32,
    pub octagon: Option<(f32, f32, u32)>,
    pub positive: Option<Line>,
    pub opposite: Option<Line>,
    pub ring_record: Option<RingRecord>,
}

fn channels(argb: u32) -> [u8; 3] {
    [(argb >> 16) as u8, (argb >> 8) as u8, argb as u8]
}
fn color(alpha: u8, rgb: [u8; 3]) -> u32 {
    u32::from(alpha) << 24 | u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2])
}

fn cvtt(value: f32) -> i32 {
    if !value.is_finite() || value < -2147483648.0 || value >= 2147483648.0 {
        i32::MIN
    } else {
        value as i32
    }
}

pub fn alpha_byte(alpha: f32) -> u8 {
    if 0.0 >= alpha {
        0
    } else if alpha >= 1.0 {
        255
    } else {
        (f64::from(alpha * 255.0) + 6755399441055744.0).to_bits() as u8
    }
}

fn hsv(argb: u32) -> [f32; 3] {
    let [r, g, b] = channels(argb).map(i32::from);
    let max = r.max(g).max(b);
    if max == 0 {
        return [0.0, 0.0, 0.0];
    }
    let delta = max - r.min(g).min(b);
    let saturation = delta as f32 / max as f32;
    let mut hue = 0.0;
    if delta != 0 && saturation > 0.0 {
        let inverse = 1.0 / delta as f32;
        let g_delta = (max - g) as f32 * inverse;
        let b_delta = (max - b) as f32 * inverse;
        let r_delta = (max - r) as f32 * inverse;
        hue = if max == r {
            b_delta - g_delta
        } else if max == g {
            (2.0 + r_delta) - b_delta
        } else {
            (g_delta + 4.0) - r_delta
        };
        hue *= f32::from_bits(0x3e2aaaab);
        if hue < 0.0 {
            hue += 1.0;
        }
    }
    [hue, saturation, max as f32 / 255.0]
}

fn from_hsv(alpha: u8, hue: f32, saturation: f32, brightness: f32) -> u32 {
    let value = (brightness * 255.0).clamp(0.0, 255.0);
    let byte = |v: f32| (f64::from(v) + 6755399441055744.0).to_bits() as u8;
    if saturation <= 0.0 {
        return color(alpha, [byte(value); 3]);
    }
    let saturation = saturation.min(1.0);
    let sector = ((hue - hue.floor()) * 360.0) / 60.0;
    let fraction = sector - sector.floor();
    let low = (1.0 - saturation) * value;
    let falling = value * (1.0 - saturation * fraction);
    let rising = value * (1.0 - saturation * (1.0 - fraction));
    let rgb = if sector < 1.0 {
        [value, rising, low]
    } else if sector < 2.0 {
        [falling, value, low]
    } else if sector < 3.0 {
        [low, value, rising]
    } else if sector < 4.0 {
        [low, falling, value]
    } else if sector < 5.0 {
        [rising, low, value]
    } else {
        [value, low, falling]
    };
    color(alpha, rgb.map(byte))
}

pub fn with_brightness(argb: u32, value: f32) -> u32 {
    let [h, s, _] = hsv(argb);
    from_hsv((argb >> 24) as u8, h, s, value)
}
pub fn with_saturation(argb: u32, value: f32) -> u32 {
    let [h, _, v] = hsv(argb);
    from_hsv((argb >> 24) as u8, h, value, v)
}

pub fn node_color(argb: u32, positive: f32, visibility: f32, trajectory: bool) -> u32 {
    let base = if trajectory {
        argb
    } else {
        with_saturation(with_brightness(argb, 0.7), 0.7)
    };
    let w = positive.min(1.0);
    let brighten = -1.0 / (1.0 + w * if trajectory { 0.4 } else { 0.1 });
    let d = if trajectory {
        1.0 / ((visibility * -0.7 + 0.7) + 1.0)
    } else {
        1.0 / ((visibility * -0.5 + 0.5) + 1.0)
    };
    let rgb = channels(base).map(|c| {
        let intermediate = cvtt((255 - c) as f32 * brighten + 255.0) as u8;
        cvtt(f32::from(intermediate) * d) as u8
    });
    color((base >> 24) as u8, rgb)
}

pub fn trajectory_color(argb: u32, visibility_a: f32, visibility_b: f32) -> u32 {
    let alpha = (((visibility_a + visibility_b) * 0.5) * 0.8) + 0.2;
    color(alpha_byte(alpha), channels(argb))
}

pub fn source_norm(delta: [f32; 2]) -> Result<f32, &'static str> {
    if delta.iter().any(|v| v.is_infinite()) {
        return Ok(f32::INFINITY);
    }
    if delta.iter().any(|v| v.is_nan()) {
        return Ok(delta[0] * delta[0] + delta[1] * delta[1]);
    }
    let x = f64::from(delta[0]);
    let y = f64::from(delta[1]);
    let value = (y * y + x * x).sqrt();
    if value > f64::from(f32::MAX) {
        return Err("source CRT overflow callback boundary");
    }
    Ok(value as f32)
}

pub fn point_commands(
    q: [f32; 2],
    probe: [f32; 2],
    argb: u32,
    fields: [f32; 4],
    trajectory: bool,
    zoom: f64,
) -> Result<PointCommands, &'static str> {
    if q.iter()
        .chain(probe.iter())
        .chain(fields[..3].iter())
        .any(|v| !v.is_finite())
        || !zoom.is_finite()
        || zoom < 0.0
    {
        return Err("finite draw-command domain required");
    }
    let w = fields[0].min(1.0);
    let distance = source_norm([q[0] - probe[0], q[1] - probe[1]])?;
    let end = if endpoint::positive_cap_guard(distance) {
        endpoint::rescale_from_point(q, probe, 200.0, distance)
    } else {
        probe
    };
    let positive = if f64::from(w) > 0.001 {
        Some(Line {
            from: q,
            to: end,
            gradient_end: end,
            width: Some(1.0),
            stops: vec![
                Stop {
                    position: 0.0,
                    argb: color(alpha_byte(w), channels(argb)),
                },
                Stop {
                    position: 0.5,
                    argb: color(alpha_byte(w), channels(argb)),
                },
                Stop {
                    position: 1.0,
                    argb: color(alpha_byte(w.min(0.05)), channels(argb)),
                },
            ],
        })
    } else {
        None
    };
    let ring_record = positive.as_ref().map(|_| RingRecord {
        argb,
        weight: w,
        direction: [
            (q[0] - probe[0]) / (distance + 0.00001),
            (q[1] - probe[1]) / (distance + 0.00001),
        ],
    });
    let opposite = if fields[1] > 0.0 {
        let length = source_norm([q[0] - end[0], q[1] - end[1]])?;
        let n = endpoint::rescale_from_point(q, end, fields[1] * -200.0, length);
        let gradient_length = source_norm([q[0] - n[0], q[1] - n[1]])?;
        let gradient_end = endpoint::rescale_from_point(q, n, 100.0, gradient_length);
        let rgb = channels(argb).map(|c| cvtt(f32::from(c) * f32::from_bits(0x3f2aaaab)) as u8);
        Some(Line {
            from: q,
            to: n,
            gradient_end,
            width: None,
            stops: vec![
                Stop {
                    position: 0.0,
                    argb: color(alpha_byte(fields[1]), rgb),
                },
                Stop {
                    position: 1.0,
                    argb: color(0, rgb),
                },
            ],
        })
    } else {
        None
    };
    let fill = node_color(argb, w, fields[2], trajectory);
    let octagon = if trajectory {
        None
    } else {
        let original = hsv(argb)[2] * 0.7;
        let filled = hsv(fill)[2] * 0.7;
        Some((
            (16.0 * zoom) as f32,
            0.8,
            with_brightness(argb, original.max(filled)),
        ))
    };
    Ok(PointCommands {
        center: q,
        radius: (5.0 * zoom) as f32,
        argb: fill,
        octagon,
        positive,
        opposite,
        ring_record,
    })
}

