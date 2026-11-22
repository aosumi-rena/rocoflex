use std::{collections::VecDeque, pin::Pin};

use super::{PreparedTree, embedded::types::VOCODER_TYPE_ID, r3::R3Stream};
use crate::synthesis::stages::vocoder::VocoderStage;

const MAX_MODEL_BYTES: usize = 256 * 1024 * 1024;
const HOP: usize = 240;
const PACKET: usize = 4;
const PROBE_LIMIT: usize = 256;
const STABLE_PACKETS: usize = 8;

pub struct PreparedR2 {
    tree: PreparedTree,
    version: u32,
}

impl PreparedR2 {
    pub fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, String> {
        if bytes.len() < 8 || bytes.len() > MAX_MODEL_BYTES
            || bytes[..4] != [0xff, 0, 0xca, 0x7f]
        {
            return Err("Expected a bounded standalone R2 DNNI vocoder".into());
        }
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if !matches!(version, 1 | 2) {
            return Err("Expected an R2 DNNI v1 or v2 vocoder".into());
        }
        let tree = PreparedTree::prepare_embedded_execution(bytes, true).map_err(|e| e.to_string())?;
        if tree.node(tree.root_index()).map_err(|e| e.to_string())?.type_id != VOCODER_TYPE_ID {
            return Err("Selected graph is not an R2 vocoder".into());
        }
        let mut stage = VocoderStage::prepare(&tree).map_err(|e| e.to_string())?;
        stage.set_native_dds();
        stage.into_stream().map_err(|e| e.to_string())?;
        Ok(Self { tree, version })
    }

    pub fn version(&self) -> u32 { self.version }
    pub fn node_count(&self) -> usize { self.tree.nodes().len() }
}

pub struct R2Vocoder {
    stream: R3Stream<'static>,
    tree: Pin<Box<PreparedTree>>,
    version: u32,
    pending: VecDeque<f32>,
    delay: usize,
    accepted: u64,
    failure: Option<String>,
}

impl R2Vocoder {
    pub fn new(bytes: &[u8], seed: u32) -> std::result::Result<Self, String> {
        let prepared = PreparedR2::from_bytes(bytes)?;
        let tree = Box::pin(prepared.tree);
        let delay = measure_delay(tree.as_ref().get_ref(), seed)?;
        // SAFETY: the stream drops before its pinned, immutable tree.
        let reference: &'static PreparedTree = unsafe { &*(tree.as_ref().get_ref() as *const PreparedTree) };
        let stream = R3Stream::prepare_mode(reference, seed, true).map_err(|e| e.to_string())?;
        Ok(Self { stream, tree, version: prepared.version, pending: VecDeque::new(),
            delay, accepted: 0, failure: None })
    }

    pub fn version(&self) -> u32 { self.version }
    pub fn node_count(&self) -> usize { self.tree.nodes().len() }
    pub fn delay_rows(&self) -> usize { self.delay }
    pub fn is_cold(&self) -> bool { self.accepted == 0 && self.failure.is_none() }
    pub fn has_failed(&self) -> bool { self.failure.is_some() }

    pub fn process(&mut self, row: &[f32; 72], out: &mut Vec<f32>) -> std::result::Result<(), String> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let begin = out.len();
        let result: std::result::Result<(), String> = (|| {
            let mut produced = Vec::new();
            self.stream.process(row, &mut produced)?;
            self.pending.extend(produced);
            if self.accepted >= self.delay as u64 {
                if self.pending.len() < HOP { return Err("R2 vocoder exhausted its prepared packet buffer".into()); }
                out.try_reserve(HOP).map_err(|e| e.to_string())?;
                out.extend(self.pending.drain(..HOP));
            }
            self.accepted = self.accepted.checked_add(1).ok_or("R2 vocoder frame count overflow")?;
            Ok(())
        })();
        if let Err(error) = &result { out.truncate(begin); self.failure = Some(error.clone()); }
        result
    }
}

fn measure_delay(tree: &PreparedTree, seed: u32) -> std::result::Result<usize, String> {
    let mut maximum = 0;
    for quiet in [false, true] {
        let mut stream = R3Stream::prepare_mode(tree, seed, true).map_err(|e| e.to_string())?;
        let mut row = [0.0; 72];
        row[0] = if quiet { 0.0 } else { 1.0 };
        row[1] = 220.0;
        row[2] = 1.0;
        if quiet { row[3..67].fill(-10.0); }
        let mut produced = Vec::new();
        let mut emitted = 0usize;
        let mut stable = 0usize;
        let mut since_packet = 0usize;
        let mut settled = false;
        for index in 0..PROBE_LIMIT {
            produced.clear();
            stream.process(&row, &mut produced)?;
            let frames = produced.len() / HOP;
            emitted = emitted.checked_add(frames).ok_or("R2 vocoder probe count overflow")?;
            let input = index + 1;
            if emitted > input { return Err("R2 vocoder emitted excess probe frames".into()); }
            maximum = maximum.max(input - emitted);
            if frames != 0 {
                stable = if frames == PACKET { stable + 1 } else { 0 };
                since_packet = 0;
            } else {
                since_packet += 1;
            }
            if stable >= STABLE_PACKETS && since_packet == PACKET - 1 {
                settled = true;
                break;
            }
        }
        if !settled { return Err("R2 vocoder packet cadence did not stabilize".into()); }
    }
    Ok(maximum)
}
