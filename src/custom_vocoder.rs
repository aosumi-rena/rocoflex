use crate::Result;
use super::{count_plan, legacy_vocoder::LegacyVocoder};
use rocoflex_r3_vocoder::{PreparedR3, R3Vocoder};

#[path = "r2_container.rs"]
mod r2_container;

enum Source<'a> {
    R2(&'a [u8]),
    WrappedR2(Vec<u8>),
    R3(&'a [u8]),
}

impl<'a> Source<'a> {
    fn read(bytes: &'a [u8]) -> Result<Self> {
        if !bytes.starts_with(&[0xff, 0, 0xca, 0x7f, 3, 0, 0, 0]) {
            return Ok(Self::R2(bytes));
        }
        if bytes.len() > 24 && (bytes.len() - 24).is_multiple_of(16)
            && !PreparedR3::identifies_archive(bytes)
        {
            return r2_container::unwrap(bytes).map(Self::WrappedR2);
        }
        Ok(Self::R3(bytes))
    }
}

pub(super) enum CustomVocoder {
    R2(LegacyVocoder),
    R3(R3Vocoder),
}

impl CustomVocoder {
    pub(super) fn new(bytes: &[u8], seed: u32) -> Result<Self> {
        match Source::read(bytes)? {
            Source::R2(tree) => LegacyVocoder::new(tree, seed).map(Self::R2),
            Source::WrappedR2(tree) => LegacyVocoder::new(&tree, seed).map(Self::R2),
            Source::R3(archive) => R3Vocoder::new(archive, seed).map(Self::R3),
        }
    }

    pub(super) fn inspect(bytes: &[u8]) -> Result<(u32, usize)> {
        match Source::read(bytes)? {
            Source::R2(tree) => LegacyVocoder::inspect(tree),
            Source::WrappedR2(tree) => LegacyVocoder::inspect(&tree).map(|(_, nodes)| (3, nodes)),
            Source::R3(archive) => Ok((3, R3Vocoder::new(archive, 1)?.node_count())),
        }
    }

    pub(super) fn prepared_count_inputs(&self) -> count_plan::Query<u64> {
        match self {
            Self::R2(model) => model.prepared_count_inputs(),
            Self::R3(model) => {
                count_plan::cold(!model.is_cold(), model.has_failed())?;
                count_plan::checked_rows(model.delay_rows())
            }
        }
    }

    pub(super) fn process(&mut self, row: &[f32], output: &mut Vec<f32>) -> Result<()> {
        match self {
            Self::R2(model) => model.process(row, output),
            Self::R3(model) => model.process(row.try_into().map_err(|_| "Expected 72 vocoder inputs")?, output),
        }
    }
}
