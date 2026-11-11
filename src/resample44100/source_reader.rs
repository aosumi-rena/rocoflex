use super::ordinary_44100_chain::OrdinaryMathNames;
use super::single_fir_reader::SingleFirMath;
use super::source_chain::OrdinaryChain;
use crate::wav48k::{Channel0Cursor, Metadata};
use std::collections::VecDeque;

enum SourceProvider<'a> {
    Decoded { source: &'a [f32], position: usize },
    Wave(Channel0Cursor<'a>),
}

impl<'a> SourceProvider<'a> {
    fn frames(&self) -> usize {
        match self {
            Self::Decoded { source, .. } => source.len(),
            Self::Wave(cursor) => cursor.metadata().frames,
        }
    }

    fn position(&self) -> usize {
        match self {
            Self::Decoded { position, .. } => *position,
            Self::Wave(cursor) => cursor.source_position(),
        }
    }

    fn rewound(&self) -> Self {
        match self {
            Self::Decoded { source, .. } => Self::Decoded { source: *source, position: 0 },
            Self::Wave(cursor) => {
                let mut cursor = (*cursor).clone();
                cursor.rewind();
                Self::Wave(cursor)
            }
        }
    }

    fn rewind(&mut self) {
        match self {
            Self::Decoded { position, .. } => *position = 0,
            Self::Wave(cursor) => cursor.rewind(),
        }
    }

    fn read_into(&mut self, output: &mut [f32]) -> Result<(), &'static str> {
        match self {
            Self::Decoded { source, position } => {
                let next = (*position).checked_add(output.len()).ok_or("own source input cursor overflow")?;
                let begin = (*position).min(source.len());
                let available = output.len().min(source.len() - begin);
                output[..available].copy_from_slice(&source[begin..begin + available]);
                output[available..].fill(0.0);
                *position = next;
                Ok(())
            }
            Self::Wave(cursor) => cursor.read_into(output)
                .map(|_| ())
                .map_err(|_| "own admitted WAV source range/cursor invariant failed"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct ReaderState {
    pub requested_source_position: usize,
    pub pending_fifo_words: usize,
    pub rounded_f32_words: usize,
    pub source_read_calls: usize,
    pub failure: Option<&'static str>,
    pub requires_reconstruction: bool,
}

pub struct SourceReader<'a> {
    input: SourceProvider<'a>,
    source_rate: u32,
    names: OrdinaryMathNames,
    ratio: f64,
    chain: OrdinaryChain,
    fifo: VecDeque<f32>,
    rounded_words: usize,
    read_calls: usize,
    failure: Option<&'static str>,
    prepared_source_capacity: usize,
    source_scratch: Vec<f32>,
    wide_scratch: Vec<f64>,
}

pub(super) fn frame_output_count(frames: usize, source_rate: u32) -> Result<usize, &'static str> {
    if source_rate == 0 || source_rate > 384_000 || frames == 0
        || frames > source_rate as usize * 300
    {
        return Err("own finite nonempty source<=384k and duration<=300s scope required");
    }
    if source_rate == 48_000 { return Ok(frames); }
    let duration = frames as f64 / f64::from(source_rate);
    Ok((duration * 48000.0).trunc() as usize)
}

fn output_count(source: &[f32], source_rate: u32) -> Result<usize, &'static str> {
    let count = frame_output_count(source.len(), source_rate)?;
    if source.iter().any(|value| !value.is_finite()) {
        return Err("own finite nonempty source<=384k and duration<=300s scope required");
    }
    Ok(count)
}

impl<'a> SourceReader<'a> {
    pub fn new<M: SingleFirMath>(
        source: &'a [f32], source_rate: u32, names: OrdinaryMathNames, math: &mut M,
    ) -> Result<Self, &'static str> {
        output_count(source, source_rate)?;
        Self::from_provider(SourceProvider::Decoded { source, position: 0 }, source_rate, names, math)
    }

    pub(super) fn from_wave<M: SingleFirMath>(
        source: Channel0Cursor<'a>, names: OrdinaryMathNames, math: &mut M,
    ) -> Result<Self, &'static str> {
        if source.source_position() != 0 {
            return Err("own admitted WAV cursor must start at frame0");
        }
        let rate = source.metadata().sample_rate;
        Self::from_provider(SourceProvider::Wave(source), rate, names, math)
    }

    fn from_provider<M: SingleFirMath>(
        input: SourceProvider<'a>, source_rate: u32, names: OrdinaryMathNames, math: &mut M,
    ) -> Result<Self, &'static str> {
        frame_output_count(input.frames(), source_rate)?;
        let ratio = f64::from(source_rate) / 48000.0;
        let prepared_bits = (512.0 * ratio + 6755399441055744.0_f64).to_bits();
        let prepared_source_capacity = (prepared_bits as u32) as usize + 64;
        let chain = OrdinaryChain::new(source_rate, names, math)?;
        Ok(Self {
            input, source_rate, names, ratio, chain, fifo: VecDeque::new(),
            rounded_words: 0, read_calls: 0, failure: None, prepared_source_capacity,
            source_scratch: vec![0.0; prepared_source_capacity],
            wide_scratch: vec![0.0; prepared_source_capacity],
        })
    }

    pub fn state(&self) -> ReaderState {
        ReaderState {
            requested_source_position: self.input.position(),
            pending_fifo_words: self.fifo.len(), rounded_f32_words: self.rounded_words,
            source_read_calls: self.read_calls,
            failure: self.failure.or(self.chain.failure()),
            requires_reconstruction: self.failure.is_some() || self.chain.requires_reconstruction(),
        }
    }

    pub fn restart<M: SingleFirMath>(&mut self, math: &mut M) -> Result<(), &'static str> {
        self.failure = Some("own source reader reconstruction interrupted before completion");
        let replacement = Self::from_provider(self.input.rewound(), self.source_rate, self.names, math)?;
        *self = replacement;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), &'static str> {
        if self.failure.is_some() || self.chain.requires_reconstruction() {
            return Err("own failed source reader requires full reconstruction");
        }
        self.failure = Some("own source reader reset interrupted before completion");
        if let Err(error) = self.chain.reset() {
            self.failure = Some(error);
            return Err(error);
        }
        self.input.rewind();
        self.fifo.clear();
        self.rounded_words = 0;
        self.read_calls = 0;
        self.failure = None;
        Ok(())
    }

    pub fn read_block<M: SingleFirMath>(
        &mut self, count: usize, math: &mut M,
    ) -> Result<Vec<f32>, &'static str> {
        if let Some(error) = self.failure.or(self.chain.failure()) { return Err(error); }
        if count > 512 { return Err("own ordinary prepare512 output-block scope required"); }
        let mut output = Vec::new();
        output.try_reserve_exact(count).map_err(|_| "own source output block allocation failed")?;
        output.resize(count, 0.0);
        self.read_into(&mut output, math)?;
        Ok(output)
    }

    pub fn read_into<M: SingleFirMath>(
        &mut self, output: &mut [f32], math: &mut M,
    ) -> Result<(), &'static str> {
        if let Some(error) = self.failure.or(self.chain.failure()) { return Err(error); }
        let count = output.len();
        if count > 512 { return Err("own ordinary prepare512 output-block scope required"); }
        while self.fifo.len() < count {
            let mut requested = (count - self.fifo.len()) as f64;
            requested *= self.ratio;
            requested += 64.0;
            let requested = requested.trunc() as usize;
            if requested == 0 || requested > self.prepared_source_capacity {
                self.failure = Some("source prepared-capacity/progress invariant failed");
                return Err("source prepared-capacity/progress invariant failed");
            }
            let next_calls = match self.read_calls.checked_add(1) {
                Some(value) => value,
                None => {
                    self.failure = Some("own source read-call counter overflow");
                    return Err("own source read-call counter overflow");
                }
            };
            self.failure = Some("own source provider request interrupted before completion");
            if let Err(error) = self.input.read_into(&mut self.source_scratch[..requested]) {
                self.failure = Some(error);
                return Err(error);
            }
            self.read_calls = next_calls;
            for (target, value) in self.wide_scratch[..requested].iter_mut()
                .zip(&self.source_scratch[..requested])
            {
                *target = *value as f64;
            }
            self.failure = Some("own source reader request interrupted before FIFO commit");
            let produced = match self.chain.process(&self.wide_scratch[..requested], math) {
                Ok(produced) => produced,
                Err(error) => {
                    self.failure = Some(error);
                    return Err(error);
                }
            };
            let rounded_words = match self.rounded_words.checked_add(produced.len()) {
                Some(value) => value,
                None => {
                    self.failure = Some("own source rounded-word counter overflow");
                    return Err("own source rounded-word counter overflow");
                }
            };
            if self.fifo.try_reserve(produced.len()).is_err() {
                self.failure = Some("own source FIFO allocation failed before commit");
                return Err("own source FIFO allocation failed before commit");
            }
            self.fifo.extend(produced.into_iter().map(|value| value as f32));
            self.rounded_words = rounded_words;
            self.failure = None;
        }
        let (first, second) = self.fifo.as_slices();
        let first_count = count.min(first.len());
        output[..first_count].copy_from_slice(&first[..first_count]);
        output[first_count..].copy_from_slice(&second[..count - first_count]);
        drop(self.fifo.drain(..count));
        Ok(())
    }
}

pub fn convert_source_channel0<M: SingleFirMath>(
    source: &[f32], source_rate: u32, names: OrdinaryMathNames, math: &mut M,
) -> Result<Vec<f32>, &'static str> {
    let count = output_count(source, source_rate)?;
    let mut reader = SourceReader::new(source, source_rate, names, math)?;
    let mut output = vec![0.0; count];
    for block in output.chunks_mut(512) {
        reader.read_into(block, math)?;
    }
    Ok(output)
}

pub(super) fn convert_wave_channel0<M: SingleFirMath>(
    mut source: Channel0Cursor<'_>, names: OrdinaryMathNames, math: &mut M,
) -> Result<(Metadata, Vec<f32>), &'static str> {
    let metadata = source.metadata().clone();
    let count = frame_output_count(metadata.frames, metadata.sample_rate)?;
    if metadata.sample_rate == 48_000 {
        if source.source_position() != 0 {
            return Err("own admitted WAV cursor must start at frame0");
        }
        let mut output = vec![0.0; count];
        for block in output.chunks_mut(512) {
            source.read_into(block).map_err(|_| "own admitted WAV source range/cursor invariant failed")?;
        }
        return Ok((metadata, output));
    }
    let mut reader = SourceReader::from_wave(source, names, math)?;
    let mut output = vec![0.0; count];
    for block in output.chunks_mut(512) {
        reader.read_into(block, math)?;
    }
    Ok((metadata, output))
}
