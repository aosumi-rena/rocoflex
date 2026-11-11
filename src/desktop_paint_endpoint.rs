pub fn scalar_class(value: f32) -> i16 {
    let bits = value.to_bits() & 0x7fff_ffff;
    match bits >> 23 {
        255 => {
            if bits & 0x7f_ffff == 0 {
                1
            } else {
                2
            }
        }
        0 => {
            if bits == 0 {
                0
            } else {
                -2
            }
        }
        _ => -1,
    }
}

pub fn normalize_length(length: f32) -> bool {
    if scalar_class(length) > 0 {
        length != 0.0
    } else {
        let magnitude = f32::from_bits(length.to_bits() & 0x7fff_ffff);
        if f32::MIN_POSITIVE >= magnitude {
            return false;
        }
        (magnitude * f32::EPSILON) < magnitude
    }
}

pub fn positive_cap_guard(initial_length: f32) -> bool {
    initial_length > 200.0
}

pub fn rescale_from_point(q: [f32; 2], destination: [f32; 2], span: f32, length: f32) -> [f32; 2] {
    if !normalize_length(length) {
        return q;
    }
    let dx = destination[0] - q[0];
    let dy = destination[1] - q[1];
    let ratio = span / length;
    let sy = dy * ratio;
    let sx = dx * ratio;
    let y = sy + q[1];
    let x = sx + q[0];
    [x, y]
}
