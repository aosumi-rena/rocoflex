use crate::{Result, current_condition::ConditionBus, offline::{Response, io::{Input, Output}},
    prototype_controls::Snapshot};
use super::{input_gain::InputGain, live::LiveGraph};
use std::path::Path;

pub(crate) fn render(analysis_model: &Path, synthesis_model: &Path, legacy: bool, snapshot: Snapshot,
    mut input: Input, mut output: Output) -> Result<Response>
{
    let _fp = Environment::enter();
    let input_frames = input.frames;
    let parameters = snapshot.parameters;
    let bus = ConditionBus::new();
    bus.publish_canvas(snapshot)?;
    let mut graph = if legacy {
        let (analysis, synthesis) = super::read_legacy_models(analysis_model, synthesis_model)?;
        LiveGraph::with_legacy_vocoder(&analysis, &synthesis, bus.reader(), crate::prepare_quality::PrepareProfile::Highest, false)?
    } else {
        let (analysis, synthesis) = super::read_models(analysis_model, synthesis_model)?;
        LiveGraph::with_instance_policy(&analysis, &synthesis, bus.reader(),
            crate::prepare_quality::PrepareProfile::Highest, false)?
    };
    let plan = graph.prepared_count_plan()
        .map_err(|error| format!("Offline output count is unavailable: {error:?}"))?;
    let rounded = input_frames.checked_add(239).ok_or("Offline input length overflow.")? / 240 * 240;
    let padding = if input_frames == 0 { 0 } else {
        rounded.checked_sub(input_frames).and_then(|value|
            plan.delay_hops().checked_mul(240).and_then(|delay| value.checked_add(delay)))
            .ok_or("Offline context length overflow.")?
    };
    let mut gain = InputGain::new();
    let mut block = Vec::new();
    let mut rendered = Vec::new();
    let mut admitted = 0u64;
    let mut published = 0u64;
    while input.next(&mut block)? {
        admitted = admitted.checked_add(block.len() as u64).ok_or("Offline input length overflow.")?;
        render_block(&mut graph, &mut gain, &block, &mut rendered, &mut output,
            input_frames, &mut published)?;
    }
    let zeros = [0.0f32; 4096];
    let mut remaining = padding;
    while remaining > 0 {
        let count = remaining.min(zeros.len() as u64) as usize;
        render_block(&mut graph, &mut gain, &zeros[..count], &mut rendered, &mut output,
            input_frames, &mut published)?;
        admitted += count as u64;
        remaining -= count as u64;
    }
    let predicted = plan.predicted_output_samples(admitted)
        .map_err(|error| format!("Offline output length prediction failed: {error:?}"))?;
    if predicted != published || published != rounded || output.frames != input_frames {
        return Err("Offline graph output does not match the prepared sample count.".into());
    }
    output.commit()?;
    Ok(Response { version: 2, sample_rate: 48_000, channels: 1, input_frames,
        output_frames: input_frames, quality: "highest", pregain_db: parameters.pregain_db,
        pitch_semitones: parameters.pitch_semitones, appended_zero_frames: padding,
        discarded_padding_frames: published - input_frames, trimmed_latency_frames: 0 })
}

fn render_block(graph: &mut LiveGraph, gain: &mut InputGain, block: &[f32],
    rendered: &mut Vec<f32>, output: &mut Output, wanted: u64, published: &mut u64) -> Result<()>
{
    rendered.clear();
    graph.process(block, rendered, gain)?;
    *published = published.checked_add(rendered.len() as u64).ok_or("Offline publication length overflow.")?;
    let take = (wanted - output.frames).min(rendered.len() as u64) as usize;
    output.append(&rendered[..take])
}

pub(crate) struct Environment { saved: u32 }
impl Environment {
    pub(crate) fn enter() -> Self {
        unsafe {
            let saved = std::arch::x86_64::_mm_getcsr();
            std::arch::x86_64::_mm_setcsr(0x1f80);
            Self { saved }
        }
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        unsafe { std::arch::x86_64::_mm_setcsr(self.saved); }
    }
}
