#![allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Segment {
    pub xy: [f32; 4],
    pub tag: f32,
    pub owner: u64,
}
#[derive(Clone, Debug)]
pub struct Node {
    pub segments: Vec<Segment>,
    pub boxes: [[f32; 4]; 2],
    pub children: [Option<Box<Node>>; 2],
}
fn minss(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
fn maxss(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}
fn box_hit(b: [f32; 4], r: [f32; 7]) -> bool {
    let x = f32::from_bits(b[0].to_bits());
    let y = b[1];
    let right = b[2] + x;
    let bottom = b[3] + y;
    let ax = (x - r[0]) * r[4];
    let bx = (right - r[0]) * r[4];
    let ay = (y - r[1]) * r[5];
    let by = (bottom - r[1]) * r[5];
    let near = maxss(minss(ax, bx), minss(ay, by));
    let far = minss(maxss(ax, bx), maxss(ay, by));
    far >= near
}

pub fn query(node: &Node, ray: [f32; 7], owner: u64, out: &mut [f32; 3]) {
    if node.children[0].is_some() {
        for i in 0..2 {
            if box_hit(node.boxes[i], ray) {
                if let Some(child) = &node.children[i] {
                    query(child, ray, owner, out)
                }
            }
        }
        return;
    }
    for segment in &node.segments {
        let [ax, ay, bx, by] = segment.xy;
        let sx = bx - ax;
        let sy = by - ay;
        let ox = ray[0] - ax;
        let oy = ray[1] - ay;
        let den = sx * ray[3] - sy * ray[2];
        let u = (ray[3] * ox - ray[2] * oy) / den;
        if !(0. <= u && u <= 1.) {
            continue;
        }
        let t = (sy * ox - oy * sx) / den;
        if segment.owner == owner {
            if t > 0. && out[0] > t {
                out[0] = t
            }
        } else if t > 0. && out[1] > t {
            out[1] = t;
            out[0] = minss(t, out[0]);
            out[2] = segment.tag
        }
    }
}
