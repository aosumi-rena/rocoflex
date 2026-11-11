use super::path;
#[path="rectangle_bounds.rs"] pub mod bounds;
pub use bounds::{BoundsServices,path_transformed_edges_with_services,transformed_corners_with_services};
pub struct StateBoundary {
    pub clip_present: bool, pub flags_28_29: [u8; 2],
    pub integer_offset: [i32; 2], pub affine: [f32; 6],
}
pub enum Dispatch { EmptyClip, AxisAligned([f32; 4]), Path(path::Path) }
fn minss(a: f32, b: f32) -> f32 { if a < b { a } else { b } }
fn maxss(a: f32, b: f32) -> f32 { if a > b { a } else { b } }

pub fn dispatch(state: &StateBoundary, rectangle: [f32; 4]) -> Result<Dispatch, path::Error> {
    if !state.clip_present { return Ok(Dispatch::EmptyClip); }
    if state.flags_28_29[0] != 0 {
        return Ok(Dispatch::AxisAligned([
            state.integer_offset[0] as f32 + rectangle[0],
            state.integer_offset[1] as f32 + rectangle[1], rectangle[2], rectangle[3],
        ]));
    }
    if state.flags_28_29[1] != 0 { return path::rectangle(rectangle).map(Dispatch::Path); }
    let [x, y, w, h] = rectangle;
    let xr = w + x;
    let yb = h + y;
    let [xmin, ymin, xmax, ymax] = transformed_corners(state.affine, [x, y, xr, yb]);
    Ok(Dispatch::AxisAligned([xmin, ymin, xmax - xmin, ymax - ymin]))
}

fn transformed_corners(affine: [f32; 6], corners: [f32; 4]) -> [f32; 4] {
    match transformed_corners_with_services(affine.map(f32::to_bits),corners.map(f32::to_bits),&mut path::ellipse_source::ExistingHostValueFp) {
        Ok(completed)=>completed.words.map(f32::from_bits),Err(failure)=>match failure.failure.error {},
    }
}

#[link(name = "ucrt")]
unsafe extern "C" { fn floorf(x: f32) -> f32; fn ceilf(x: f32) -> f32; }
fn cvtt_i32(x: f32) -> i32 {
    if !x.is_finite() || x >= 2147483648.0 || x < -2147483648.0 { i32::MIN }
    else { x as i32 }
}
fn flip(x: f32) -> f32 { f32::from_bits(x.to_bits() ^ 0x80000000) }

pub fn inverse_source(matrix: [f32; 6]) -> [f32; 6] {
    let [a, b, tx, c, d, ty] = matrix;
    let determinant = (a * d) - (c * b);
    let wide = determinant as f64;
    let exceptional = wide.to_bits() & 0x7ff0000000000000 == 0x7ff0000000000000;
    let copy_original = if exceptional { determinant == 0.0 } else {
        let absolute = f64::from_bits(wide.to_bits() & 0x7fffffffffffffff);
        absolute <= f64::from_bits(0x0010000000000000)
            || absolute * f64::from_bits(0x3cb0000000000000) >= absolute
    };
    if copy_original { return matrix; }
    let reciprocal = 1.0 / wide;
    let ia = (d as f64 * reciprocal) as f32;
    let ic = (flip(c) as f64 * reciprocal) as f32;
    let ib = (flip(b) as f64 * reciprocal) as f32;
    let id = (a as f64 * reciprocal) as f32;

    let itx = (flip(tx) * ia) - (ty * ib);
    let ity = (flip(tx) * ic) - (ty * id);
    [ia, ib, itx, ic, id, ity]
}

pub fn integer_transformed_bounds(rectangle: [i32; 4], affine: [f32; 6]) -> [i32; 4] {
    let corners = [rectangle[0] as f32, rectangle[1] as f32,
        rectangle[0].wrapping_add(rectangle[2]) as f32,
        rectangle[1].wrapping_add(rectangle[3]) as f32];
    let [xmin, ymin, xmax, ymax] = transformed_corners(affine, corners);
    let xmax = (xmax - xmin) + xmin;
    let ymax = (ymax - ymin) + ymin;
    let upper = |x| if 2147483648.0f32 > x { cvtt_i32(unsafe { ceilf(x) }) } else { i32::MAX };
    let lower = |x| if x > -2147483648.0f32 { cvtt_i32(unsafe { floorf(x) }) } else { i32::MIN };
    let right = upper(xmax); let bottom = upper(ymax);
    let y = lower(ymin); let x = lower(xmin);
    [x, y, right.wrapping_sub(x), bottom.wrapping_sub(y)]
}

pub fn path_transformed_edges_source(bounds: [f32; 4], affine: [f32; 6]) -> [i32; 4] {
    match path_transformed_edges_with_services(bounds.map(f32::to_bits),affine.map(f32::to_bits),&mut path::ellipse_source::ExistingHostValueFp) {
        Ok(completed)=>completed.edges,Err(failure)=>match failure.failure.error {},
    }
}

pub fn caller_clip_bounds(state: &StateBoundary, raw_clip: [i32; 4]) -> [i32; 4] {
    if !state.clip_present { return [0; 4]; }
    if state.flags_28_29[0] != 0 {
        return [raw_clip[0].wrapping_sub(state.integer_offset[0]),
            raw_clip[1].wrapping_sub(state.integer_offset[1]), raw_clip[2], raw_clip[3]];
    }
    integer_transformed_bounds(raw_clip, inverse_source(state.affine))
}

pub enum AxisFill {
    BrushEdgeTable { rectangle: [f32; 4] },
    EmptyIntersection,
}
pub fn premultiply_argb(argb: u32) -> u32 {
    let alpha = argb >> 24;
    if alpha == 255 { return argb; }
    let channel = |shift: u32| ((((argb >> shift) & 255) * alpha + 127) >> 8) << shift;
    (alpha << 24) | channel(16) | channel(8) | channel(0)
}
pub fn gradient_axis_fill(rectangle: [f32; 4], clip_bounds: [i32; 4]) -> AxisFill {
    let clip_x = clip_bounds[0] as f32; let clip_y = clip_bounds[1] as f32;
    let left = maxss(rectangle[0], clip_x);
    let width = minss(rectangle[0] + rectangle[2], clip_bounds[2] as f32 + clip_x) - left;

    if !(width >= 0.0) { return AxisFill::EmptyIntersection; }
    let top = maxss(rectangle[1], clip_y);
    let height = minss(rectangle[1] + rectangle[3], clip_bounds[3] as f32 + clip_y) - top;
    if !(height >= 0.0) || 0.0 >= width || 0.0 >= height { return AxisFill::EmptyIntersection; }
    AxisFill::BrushEdgeTable { rectangle: [left, top, width, height] }
}

pub fn gradient_integer_intersection(rectangle: [i32; 4], clip: [i32; 4]) -> Option<[i32; 4]> {
    let left = rectangle[0].max(clip[0]); let top = rectangle[1].max(clip[1]);
    let right = rectangle[0].wrapping_add(rectangle[2]).min(clip[0].wrapping_add(clip[2]));
    let width = right.wrapping_sub(left);
    if width < 0 { return None; }
    let bottom = rectangle[1].wrapping_add(rectangle[3]).min(clip[1].wrapping_add(clip[3]));
    let height = bottom.wrapping_sub(top);
    if height < 0 || right == left || bottom == top { return None; }
    Some([left, top, width, height])
}
