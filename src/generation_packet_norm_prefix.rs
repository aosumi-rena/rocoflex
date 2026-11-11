use std::{collections::TryReserveError, ops::Range};
use super::packet_norm_math::{SourceGenerationPacketNormMath,
    SourceGenerationPacketNormStopped, SourceGenerationPacketNormTag,
    SourceGenerationRsqrtpsService};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGenerationPacketNormPrefixStage {
    PacketStorage, PacketLoad, PacketMath, PacketStore, Complete,
}

pub struct SourceGenerationPacketNormPrefixProgress {
    stage: SourceGenerationPacketNormPrefixStage,
    completed_packets: usize,
    current_packet: Option<usize>,
    current_range: Option<Range<usize>>,
    loaded_bits: Option<[u32; 4]>,
    helper_reached: bool,
}
impl SourceGenerationPacketNormPrefixProgress {
    pub fn stage(&self) -> SourceGenerationPacketNormPrefixStage { self.stage }
    pub fn completed_packets(&self) -> usize { self.completed_packets }
    pub fn completed_range(&self) -> Range<usize> { 0..self.completed_packets * 4 }
    pub fn current_packet(&self) -> Option<usize> { self.current_packet }
    pub fn current_range(&self) -> Option<Range<usize>> { self.current_range.clone() }
    pub fn loaded_bits(&self) -> Option<[u32; 4]> { self.loaded_bits }

    pub fn helper_reached(&self) -> bool { self.helper_reached }
}

pub enum SourceGenerationPacketNormPrefixCause {
    AlreadyStopped,
    Storage(TryReserveError),

    Math(SourceGenerationPacketNormStopped),
}

pub struct SourceGenerationPacketNormPrefixParts<S: SourceGenerationRsqrtpsService> {
    pub input_bits: Vec<u32>,
    pub output_prefix_bits: Vec<u32>,
    pub math: SourceGenerationPacketNormMath<S>,
    pub completed_packets: usize,
    pub processed_range: Range<usize>,
    pub remaining_range: Range<usize>,
    pub progress: SourceGenerationPacketNormPrefixProgress,
}
pub struct SourceGenerationPacketNormPrefixCompleted<S: SourceGenerationRsqrtpsService> {
    parts: SourceGenerationPacketNormPrefixParts<S>,
}
impl<S: SourceGenerationRsqrtpsService> SourceGenerationPacketNormPrefixCompleted<S> {
    pub fn input_bits(&self) -> &[u32] { &self.parts.input_bits }
    pub fn output_prefix_bits(&self) -> &[u32] { &self.parts.output_prefix_bits }
    pub fn math(&self) -> &SourceGenerationPacketNormMath<S> { &self.parts.math }
    pub fn completed_packets(&self) -> usize { self.parts.completed_packets }
    pub fn processed_range(&self) -> Range<usize> { self.parts.processed_range.clone() }
    pub fn remaining_range(&self) -> Range<usize> { self.parts.remaining_range.clone() }
    pub fn remaining_bits(&self) -> &[u32] { &self.parts.input_bits[self.parts.remaining_range.clone()] }
    pub fn progress(&self) -> &SourceGenerationPacketNormPrefixProgress { &self.parts.progress }
    pub fn into_parts(self) -> SourceGenerationPacketNormPrefixParts<S> { self.parts }
}

pub struct SourceGenerationPacketNormPrefixFailure<S: SourceGenerationRsqrtpsService> {
    input_bits: Vec<u32>,
    math: SourceGenerationPacketNormMath<S>,
    output_prefix_bits: Option<Vec<u32>>,
    progress: Option<SourceGenerationPacketNormPrefixProgress>,
    cause: SourceGenerationPacketNormPrefixCause,
}
impl<S: SourceGenerationRsqrtpsService> SourceGenerationPacketNormPrefixFailure<S> {
    pub fn input_bits(&self) -> &[u32] { &self.input_bits }
    pub fn math(&self) -> &SourceGenerationPacketNormMath<S> { &self.math }
    pub fn output_prefix_bits(&self) -> Option<&[u32]> { self.output_prefix_bits.as_deref() }
    pub fn progress(&self) -> Option<&SourceGenerationPacketNormPrefixProgress> { self.progress.as_ref() }
    pub fn cause(&self) -> &SourceGenerationPacketNormPrefixCause { &self.cause }
    pub fn processable_range(&self) -> Range<usize> { 0..(self.input_bits.len() / 4) * 4 }
    pub fn untouched_tail_range(&self) -> Range<usize> { (self.input_bits.len() / 4) * 4..self.input_bits.len() }
}

pub fn evaluate_packet_prefix_with_source_norm_math<S: SourceGenerationRsqrtpsService>(
    input_bits: Vec<u32>, mut math: SourceGenerationPacketNormMath<S>,
) -> Result<SourceGenerationPacketNormPrefixCompleted<S>, SourceGenerationPacketNormPrefixFailure<S>> {
    if math.failure().is_some() || math.is_active() {
        return Err(SourceGenerationPacketNormPrefixFailure { input_bits, math,
            output_prefix_bits: None, progress: None,
            cause: SourceGenerationPacketNormPrefixCause::AlreadyStopped });
    }
    let packet_count = input_bits.len() / 4;
    let processed = packet_count * 4;
    let mut output_prefix_bits = Vec::new();
    let mut progress = SourceGenerationPacketNormPrefixProgress {
        stage: SourceGenerationPacketNormPrefixStage::PacketStorage,
        completed_packets: 0, current_packet: None, current_range: None,
        loaded_bits: None, helper_reached: false,
    };
    for packet in 0..packet_count {
        let start = packet * 4;
        progress.stage = SourceGenerationPacketNormPrefixStage::PacketStorage;
        progress.current_packet = Some(packet);
        progress.current_range = Some(start..start + 4);
        progress.loaded_bits = None;
        progress.helper_reached = false;
        if let Err(error) = output_prefix_bits.try_reserve(4) {
            return Err(SourceGenerationPacketNormPrefixFailure { input_bits, math,
                output_prefix_bits: Some(output_prefix_bits), progress: Some(progress),
                cause: SourceGenerationPacketNormPrefixCause::Storage(error) });
        }
        progress.stage = SourceGenerationPacketNormPrefixStage::PacketLoad;

        let words = [input_bits[start], input_bits[start + 1], input_bits[start + 2], input_bits[start + 3]];
        progress.loaded_bits = Some(words);
        progress.stage = SourceGenerationPacketNormPrefixStage::PacketMath;
        progress.helper_reached = true;
        let completed = match math.evaluate(SourceGenerationPacketNormTag { packet }, words) {
            Ok(bits) => bits,
            Err(stopped) => return Err(SourceGenerationPacketNormPrefixFailure { input_bits, math,
                output_prefix_bits: Some(output_prefix_bits), progress: Some(progress),
                cause: SourceGenerationPacketNormPrefixCause::Math(stopped) }),
        };
        progress.stage = SourceGenerationPacketNormPrefixStage::PacketStore;

        output_prefix_bits.extend_from_slice(&completed);
        progress.completed_packets += 1;
    }
    progress.stage = SourceGenerationPacketNormPrefixStage::Complete;
    let remaining_range = processed..input_bits.len();
    Ok(SourceGenerationPacketNormPrefixCompleted { parts: SourceGenerationPacketNormPrefixParts {
        input_bits, output_prefix_bits, math, completed_packets: progress.completed_packets,
        processed_range: 0..processed, remaining_range, progress,
    } })
}
