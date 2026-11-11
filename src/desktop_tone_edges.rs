#[path = "desktop_tone_pipeline.rs"]
pub mod pipeline;
use pipeline::PathCommand;

type Point = [f32; 2];
pub type Segment = [Point; 2];

fn midpoint(a: Point, b: Point) -> Point {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}
fn distance_squared(a: Point, b: Point) -> f32 {
    let x = a[0] - b[0];
    let y = a[1] - b[1];
    x * x + y * y
}
fn approximately_equal(a: f32, b: f32) -> bool {
    let difference = (a - b).abs();
    difference <= f32::MIN_POSITIVE || a.abs().max(b.abs()) * f32::EPSILON >= difference
}

pub fn flatten(path: &[PathCommand]) -> Option<Vec<Segment>> {
    let mut segments = Vec::new();
    let mut current = [0.0; 2];
    let mut origin = current;
    let mut have_origin = false;
    for command in path {
        match *command {
            PathCommand::Move(point) => {
                if point.iter().any(|value| !value.is_finite()) {
                    return None;
                }
                current = point;
                origin = point;
                have_origin = true;
            }
            PathCommand::Line(point) => {
                if !have_origin || point.iter().any(|value| !value.is_finite()) {
                    return None;
                }
                segments.push([current, point]);
                current = point;
            }
            PathCommand::Close => {
                if !have_origin {
                    return None;
                }
                if !approximately_equal(current[0], origin[0])
                    || !approximately_equal(current[1], origin[1])
                {
                    segments.push([current, origin]);
                }
                current = origin;
            }
            PathCommand::Cubic([control1, control2, end]) => {
                if !have_origin
                    || control1
                        .iter()
                        .chain(&control2)
                        .chain(&end)
                        .any(|value| !value.is_finite())
                {
                    return None;
                }
                let mut stack = vec![[current, control1, control2, end]];
                while let Some([start, a, b, end]) = stack.pop() {
                    if segments.len() > 1_000_000 {
                        return None;
                    }
                    let ab = midpoint(start, a);
                    let bc = midpoint(b, a);
                    let cd = midpoint(b, end);
                    let abc = midpoint(ab, bc);
                    let bcd = midpoint(cd, bc);
                    let needs_split = distance_squared(abc, a) > f32::from_bits(0x3eb8_51ec)
                        || distance_squared(bcd, b) > f32::from_bits(0x3eb8_51ec);
                    let stuck = (approximately_equal(abc[0], ab[0])
                        || approximately_equal(abc[0], bc[0]))
                        && (approximately_equal(abc[1], ab[1])
                            || approximately_equal(abc[1], bc[1]))
                        && (approximately_equal(bcd[0], cd[0])
                            || approximately_equal(bcd[0], bc[0]))
                        && (approximately_equal(bcd[1], cd[1])
                            || approximately_equal(bcd[1], bc[1]));
                    if needs_split && !stuck {
                        let middle = midpoint(bcd, abc);
                        stack.push([middle, bcd, cd, end]);
                        stack.push([start, ab, abc, middle]);
                    } else {
                        segments.push([start, abc]);
                        segments.push([abc, bcd]);
                        segments.push([bcd, end]);
                    }
                }
                current = end;
            }
        }
    }
    Some(segments)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeEvent {
    pub x_fixed: i32,
    pub coverage: u8,
}

fn y_fixed(value: f32) -> i64 {
    (value * 256.0 + if value >= 0.0 { 0.5 } else { -0.5 }) as i64
}

pub fn edge_rows(segments: &[Segment], size: i32) -> Option<Vec<Vec<EdgeEvent>>> {
    if size <= 0 || size > 8192 {
        return None;
    }
    let mut rows = vec![Vec::<(i64, i64)>::new(); size as usize];
    let bound = size as i64 * 256;
    for &[start, end] in segments {
        if start.iter().chain(&end).any(|value| !value.is_finite()) {
            return None;
        }
        let y0 = y_fixed(start[1]);
        let y1 = y_fixed(end[1]);
        if y0 == y1 {
            continue;
        }
        let mut cursor = y0.min(y1).max(0);
        let last = y0.max(y1).min(bound);
        if cursor >= last {
            continue;
        }
        let slope = (end[0] - start[0]) / (end[1] - start[1]);
        if !slope.is_finite() || slope.abs() > i32::MAX as f32 - 1.0 {
            return None;
        }
        let step = (256 / (slope.abs() as i64 + 1)).max(1);
        let base_x = start[0] * 256.0;
        while cursor < last {
            let count = (last - cursor).min(step).min(256 - (cursor & 255));
            let middle = (count >> 1) + cursor - y0;
            let sampled_x = (middle as f64 * slope as f64 + base_x as f64) as i64;
            let x = sampled_x.clamp(0, bound - 1);
            let delta = if y0 > y1 { count } else { -count };
            rows[(cursor / 256) as usize].push((x, delta));
            cursor += count;
        }
    }
    Some(
        rows.into_iter()
            .map(|mut events| {
                events.sort_by_key(|event| event.0);
                let mut winding = 0i64;
                let mut normalized = Vec::new();
                let mut index = 0;
                while index < events.len() {
                    let x = events[index].0;
                    while index < events.len() && events[index].0 == x {
                        winding += events[index].1;
                        index += 1;
                    }
                    normalized.push(EdgeEvent {
                        x_fixed: x as i32,
                        coverage: winding.abs().min(255) as u8,
                    });
                }
                if let Some(last) = normalized.last_mut() {
                    last.coverage = 0;
                }
                normalized
            })
            .collect(),
    )
}

pub fn mask_alpha(rows: &[Vec<EdgeEvent>], size: i32, brush_alpha: u8) -> Option<Vec<u8>> {
    if size <= 0 || size > 8192 || rows.len() != size as usize {
        return None;
    }
    let mut output = vec![0u8; size as usize * size as usize];
    for (y, row) in rows.iter().enumerate() {
        let mut area = vec![0u32; size as usize];
        for pair in row.windows(2) {
            let mut cursor = pair[0].x_fixed;
            let end = pair[1].x_fixed;
            if cursor < 0 || end < cursor || end > size * 256 {
                return None;
            }
            while cursor < end {
                let column = cursor / 256;
                let count = (end - cursor).min(256 - (cursor & 255));
                area[column as usize] += count as u32 * pair[0].coverage as u32;
                cursor += count;
            }
        }
        for (x, area) in area.into_iter().enumerate() {
            output[y * size as usize + x] = if area >= 0xff00 {
                brush_alpha
            } else if area < 256 {
                0
            } else {
                ((brush_alpha as u32 * (area >> 8)) >> 8) as u8
            };
        }
    }
    Some(output)
}

