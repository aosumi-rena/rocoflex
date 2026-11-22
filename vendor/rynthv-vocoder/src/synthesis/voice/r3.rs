use std::{collections::HashSet, pin::Pin, sync::Arc};

use super::{archive::Archive, format::FormatContext, tree::ModuleTree, PreparedTree, Result, VoiceError};
use crate::synthesis::{
    inference::Sequence,
    stages::vocoder::{configuration::{COMPACT_CHILDREN, ROOT_TYPE}, VocoderStage, VocoderStream},
};

const HEADER: [u8; 8] = [0xff, 0, 0xca, 0x7f, 3, 0, 0, 0];
const MAX_MODEL_BYTES: usize = 256 * 1024 * 1024;
const PROBE_LIMIT: usize = 256;
const STEADY_OBSERVATIONS: usize = 8;
const CHANNELS: usize = 72;
const HOP: usize = 240;

pub struct PreparedR3 { tree: PreparedTree, compact: bool }

impl PreparedR3 {
    pub fn identifies_archive(bytes: &[u8]) -> bool {
        Archive::identifies_vocoder(bytes, &FormatContext::dnni_v3())
    }

    pub fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, String> {
        Self::parse(bytes).map_err(|e| e.to_string())
    }

    fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_MODEL_BYTES || !bytes.starts_with(&HEADER) {
            return Err(VoiceError::new("Expected a bounded standalone R3 DNNI v3 vocoder"));
        }
        let (archive, unsigned) = Archive::from_vocoder_file(Arc::from(bytes), &FormatContext::dnni_v3())?;
        let archive = Arc::new(archive);
        let mut selected = None;
        for (index, resource) in archive.resources.iter().enumerate() {
            if resource.range.len() < 20 || !resource.range.len().is_multiple_of(20) { continue; }
            let Ok(graph) = ModuleTree::parse(&archive.read(index, None)?, &archive) else { continue; };
            let root = archive.index(graph.root().resource)
                .ok_or_else(|| VoiceError::new("Vocoder graph root is absent"))?;
            if archive.prefix(root, 16)?.as_ref() != ROOT_TYPE { continue; }
            if selected.replace((resource.id, graph)).is_some() {
                return Err(VoiceError::new("Standalone R3 model contains multiple vocoder graphs"));
            }
        }
        let (graph_id, graph) = selected.ok_or_else(|| VoiceError::new("Standalone R3 model contains no vocoder graph"))?;
        let compact = graph.root().children.len() == COMPACT_CHILDREN;
        if unsigned && !compact {
            let mut used: HashSet<_> = graph.nodes.iter().map(|node| node.resource).collect();
            used.insert(graph_id);
            if used.len() != archive.resources.len() {
                return Err(VoiceError::new("Standalone R3 model contains unrelated resources"));
            }
        }
        let tree = PreparedTree::prepare_execution(archive, &graph, false)?;
        if tree.node(tree.root_index())?.type_id != ROOT_TYPE {
            return Err(VoiceError::new("Selected graph is not an R3 vocoder"));
        }
        VocoderStage::prepare(&tree)?.into_stream()?;
        Ok(Self { tree, compact })
    }

    pub fn node_count(&self) -> usize { self.tree.nodes().len() }
    pub fn is_compact(&self) -> bool { self.compact }

    pub fn stream(&self, seed: u32) -> std::result::Result<R3Stream<'_>, String> {
        R3Stream::prepare(&self.tree, seed).map_err(|e| e.to_string())
    }
}

/// Consumes one 72-channel frame. Startup latency depends on the model.
pub struct R3Stream<'a> { stream: VocoderStream<'a>, failure: Option<String> }

impl<'a> R3Stream<'a> {
    fn prepare(tree: &'a PreparedTree, seed: u32) -> Result<Self> {
        Self::prepare_mode(tree, seed, false)
    }

    pub(super) fn prepare_mode(tree: &'a PreparedTree, seed: u32, native_dds: bool) -> Result<Self> {
        let mut stage = VocoderStage::prepare(tree)?;
        if native_dds { stage.set_native_dds(); }
        let mut stream = stage.into_stream()?;
        stream.set_seed(seed)?;
        Ok(Self { stream, failure: None })
    }

    pub fn process(&mut self, row: &[f32; CHANNELS], out: &mut Vec<f32>) -> std::result::Result<(), String> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let begin = out.len();
        let result = self.advance(row, out).map_err(|e| e.to_string());
        if let Err(error) = &result { out.truncate(begin); self.failure = Some(error.clone()); }
        result
    }

    fn advance(&mut self, row: &[f32; CHANNELS], out: &mut Vec<f32>) -> Result<()> {
        if row.iter().any(|v| !v.is_finite()) || row[1] <= 0.0 {
            return Err(VoiceError::new("Vocoder requires finite input and positive F0"));
        }
        let input = Sequence::new(1, CHANNELS, row.to_vec())?;
        let pcm = self.stream.process(&input, None, false)?;
        if pcm.primary.values.len() != pcm.secondary.values.len()
            || !pcm.primary.values.len().is_multiple_of(HOP)
        {
            return Err(VoiceError::new("Vocoder output channels do not align"));
        }
        out.try_reserve(pcm.primary.values.len()).map_err(|e| VoiceError::new(e.to_string()))?;
        for (primary, secondary) in pcm.primary.values.into_iter().zip(pcm.secondary.values) {
            let sample = primary + secondary;
            if !sample.is_finite() { return Err(VoiceError::new("Vocoder emitted non-finite audio")); }
            out.push(sample);
        }
        Ok(())
    }
}

pub struct R3Vocoder {
    stream: R3Stream<'static>,
    tree: Pin<Box<PreparedTree>>,
    compact: bool,
    delay: usize,
    accepted: u64,
    failure: Option<String>,
}

impl R3Vocoder {
    pub fn new(bytes: &[u8], seed: u32) -> std::result::Result<Self, String> {
        Self::from_prepared(PreparedR3::from_bytes(bytes)?, seed)
    }

    pub fn from_prepared(model: PreparedR3, seed: u32) -> std::result::Result<Self, String> {
        Self::from_tree(model.tree, model.compact, seed, false)
    }

    pub(super) fn from_tree(tree: PreparedTree, compact: bool, seed: u32, native_dds: bool) -> std::result::Result<Self, String> {
        let tree = Box::pin(tree);
        let delay = measure_delay(tree.as_ref().get_ref(), seed, native_dds)?;
        // SAFETY: the private tree stays pinned and immutable. Field order drops the stream first.
        let reference: &'static PreparedTree = unsafe { &*(tree.as_ref().get_ref() as *const PreparedTree) };
        let stream = R3Stream::prepare_mode(reference, seed, native_dds).map_err(|e| e.to_string())?;
        Ok(Self { stream, tree, compact, delay, accepted: 0, failure: None })
    }

    pub fn node_count(&self) -> usize { self.tree.nodes().len() }
    pub fn is_compact(&self) -> bool { self.compact }
    pub fn delay_rows(&self) -> usize { self.delay }
    pub fn is_cold(&self) -> bool {
        self.accepted == 0 && self.failure.is_none() && self.stream.failure.is_none()
    }
    pub fn has_failed(&self) -> bool { self.failure.is_some() || self.stream.failure.is_some() }

    pub fn process(&mut self, row: &[f32; CHANNELS], out: &mut Vec<f32>) -> std::result::Result<(), String> {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let begin = out.len();
        let result: std::result::Result<(), String> = (|| {
            self.stream.process(row, out)?;
            let expected = if self.accepted >= self.delay as u64 { HOP } else { 0 };
            if out.len() - begin != expected { return Err("Vocoder publication differs from its prepared delay".into()); }
            self.accepted = self.accepted.checked_add(1).ok_or("Vocoder frame count overflow")?;
            Ok(())
        })();
        if let Err(error) = &result { out.truncate(begin); self.failure = Some(error.clone()); }
        result
    }
}

fn measure_delay(tree: &PreparedTree, seed: u32, native_dds: bool) -> std::result::Result<usize, String> {
    let mut stream = R3Stream::prepare_mode(tree, seed, native_dds).map_err(|e| e.to_string())?;
    let mut row = [0.0; CHANNELS];
    row[0] = 1.0; row[1] = 220.0; row[2] = 1.0;
    let mut output = Vec::with_capacity(HOP);
    let mut delay = None;
    let mut steady = 0;
    for index in 0..PROBE_LIMIT {
        output.clear();
        stream.process(&row, &mut output)?;
        match (delay, output.len()) {
            (None, 0) => {},
            (None, HOP) => { delay = Some(index); steady = 1; },
            (Some(_), HOP) => { steady += 1; },
            _ => return Err("Vocoder publication is not one frame per input after startup".into()),
        }
        if steady >= STEADY_OBSERVATIONS { return Ok(delay.expect("delay observed")); }
    }
    Err("Vocoder startup exceeds the supported measurement bound".into())
}
