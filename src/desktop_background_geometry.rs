use super::{probe_math::spatial_builder::Segment, Source};

#[derive(Debug)]
pub struct CompletedGeometry {
    pub sources: Vec<Source>,
    pub segments: Vec<Segment>,
    pub getters: Vec<Vec<[f32; 2]>>,

    pub root_positions: Vec<[f32; 2]>,
}
