#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectI32 {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderGlyphLayout {
    pub more_hit: RectI32,
    pub reset_hit: RectI32,
    pub more_fit: RectI32,
    pub reset_fit: RectI32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderPeerRects {
    pub component_16f0: RectI32,
    pub component_14f0: RectI32,
}

pub fn parent_rect(captured_width: i32, captured_height: i32) -> RectI32 {
    let inner_width = captured_width.wrapping_sub(0x28).max(0);
    let height = captured_height.wrapping_sub(0x16).max(0).min(0x18);
    let sign_bias = ((inner_width.wrapping_sub(0xf0) as u32) >> 31) as i32;
    let biased_difference = inner_width.wrapping_add(sign_bias).wrapping_sub(0xf0);
    let x = (biased_difference >> 1).wrapping_add(0x14);
    RectI32 { x, y: 0x0b, width: 0xf0, height }
}

pub fn settings_rect(captured_width: i32, captured_height: i32) -> RectI32 {
    let inner_width = captured_width.wrapping_sub(0x28).max(0);
    let side = inner_width.min(0x18);
    let x = inner_width.wrapping_sub(side).wrapping_add(0x14);
    let height = captured_height.wrapping_sub(0x16).max(0).min(0x18);
    RectI32 { x, y: 0x0b, width: side, height }
}

pub fn peer_rects(
    captured_width: i32,
    captured_height: i32,
    first_helper_eax: i32,
    second_helper_eax: i32,
) -> HeaderPeerRects {

    let mut width_16f0 = i32::from(first_helper_eax != 0);
    let narrow = 0x6c;
    let mut first_width = 0x80;
    if first_helper_eax == 0 {
        first_width = narrow;
    }
    width_16f0 = width_16f0.wrapping_shl(4).wrapping_add(0x50);
    let mut width_14f0 = 0x8c;
    if second_helper_eax != 0 {
        width_14f0 = first_width;
    }
    if second_helper_eax == 0 {
        width_16f0 = narrow;
    }

    let mut remaining = captured_width.wrapping_sub(0x28).max(0);
    let height = captured_height.wrapping_sub(0x16).max(0).min(0x18);
    let settings_width = 0x18_i32.min(remaining);
    remaining = remaining.wrapping_sub(settings_width);
    let first_gap = 8_i32.min(remaining);
    remaining = remaining.wrapping_sub(first_gap);
    width_16f0 = width_16f0.min(remaining);
    remaining = remaining.wrapping_sub(width_16f0);
    let second_gap = 8_i32.min(remaining);
    let mut preceding = remaining.wrapping_sub(second_gap);
    width_14f0 = width_14f0.min(preceding);

    let component_16f0 = RectI32 {
        x: remaining.wrapping_add(0x14), y: 0x0b, width: width_16f0, height,
    };
    preceding = preceding.wrapping_sub(width_14f0);
    let component_14f0 = RectI32 {
        x: preceding.wrapping_add(0x14), y: 0x0b, width: width_14f0, height,
    };
    HeaderPeerRects { component_16f0, component_14f0 }
}

pub fn layout(width: i32, height: i32) -> HeaderGlyphLayout {

    let last_width = width.min(24);
    let last_start = width.wrapping_sub(last_width);
    let penultimate_width = last_start.min(24);
    let penultimate_start = last_start.wrapping_sub(penultimate_width);
    HeaderGlyphLayout {
        more_hit: RectI32 { x: last_start, y: 0, width: last_width, height },
        reset_hit: RectI32 {
            x: penultimate_start, y: 0, width: penultimate_width, height,
        },

        more_fit: RectI32 {
            x: last_start.wrapping_add(7),
            y: 7,
            width: last_width.wrapping_sub(14).max(0),
            height: height.wrapping_sub(14).max(0),
        },

        reset_fit: RectI32 {
            x: penultimate_start.wrapping_add(8),
            y: 6,
            width: penultimate_width.wrapping_sub(12).max(0),
            height: height.wrapping_sub(12).max(0),
        },
    }
}
