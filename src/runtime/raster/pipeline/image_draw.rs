use super::rectangle::StateBoundary;
pub enum Route { NoOp, Translation([i32; 2]), Affine([f32; 6]) }
fn abs_source(x: f32) -> f32 { f32::from_bits(x.to_bits() & 0x7fffffff) }
fn cvtt_i32(x: f32) -> i32 {
    if !x.is_finite() || x >= 2147483648.0 || x < -2147483648.0 { i32::MIN } else { x as i32 }
}

pub fn compose_source(state: &StateBoundary, requested: [f32; 6]) -> [f32; 6] {
    let [ia, ib, itx, ic, id, ity] = requested;
    if state.flags_28_29[0] != 0 {
        [ia, ib, state.integer_offset[0] as f32 + itx,
         ic, id, state.integer_offset[1] as f32 + ity]
    } else {
        let [a, b, tx, c, d, ty] = state.affine;
        [(a * ia) + (b * ic), (a * ib) + (b * id), ((a * itx) + (b * ity)) + tx,
         (ia * c) + (ic * d), (ib * c) + (id * d), ((c * itx) + (d * ity)) + ty]
    }
}
pub fn dispatch(state: &StateBoundary, brush_alpha: u8, requested: [f32; 6], quality: i32) -> Route {
    if !state.clip_present || brush_alpha == 0 { return Route::NoOp; }
    let combined = compose_source(state, requested);
    let [a, b, tx, c, d, ty] = combined;
    let threshold = f32::from_bits(0x3b03126f);
    if threshold > abs_source(b) && threshold > abs_source(c)
        && threshold > abs_source(-1.0 + a) && threshold > abs_source(-1.0 + d) {
        let fixed_x = cvtt_i32(tx * 256.0);
        let fixed_y = cvtt_i32(ty * 256.0);
        if quality == 0 || ((fixed_y | fixed_x) as u8 & 0xe0) == 0 {
            return Route::Translation([fixed_x.wrapping_add(128) >> 8, fixed_y.wrapping_add(128) >> 8]);
        }
    }

    let determinant = (a * d) - (b * c);
    if determinant == 0.0 { Route::NoOp } else { Route::Affine(combined) }
}

pub fn translated_framebuffer_rectangle(source: [i32; 2], target: [i32; 2],
    translation: [i32; 2]) -> Option<[i32; 4]> {
    let [x, y] = translation;
    let left = x.max(0); let top = y.max(0);
    let right = target[0].min(source[0].wrapping_add(x));
    let width = right.wrapping_sub(left);
    if width < 0 { return None; }
    let bottom = target[1].min(source[1].wrapping_add(y));
    let height = bottom.wrapping_sub(top);
    if height < 0 || right == left || height == 0 { return None; }
    Some([left, top, width, height])
}
