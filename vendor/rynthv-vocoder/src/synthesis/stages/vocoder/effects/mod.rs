use crate::synthesis::voice::Result;
use super::excitation::ExcitationFrame;
#[derive(Clone)] pub struct EffectTimeline;
#[derive(Clone,Copy,Debug)] pub struct FrameEffect;
impl EffectTimeline { pub fn frame(&self, _:usize, _:ExcitationFrame, _:f32)->FrameEffect { FrameEffect } }
impl FrameEffect { pub(crate) fn apply(self, _: &mut [[f32;2]], _:bool)->Result<()> { Ok(()) } }
