use crate::Result;
use super::count_plan;
use rocoflex_r3_vocoder::{PreparedR2, R2Vocoder};

pub(super) struct LegacyVocoder {
    stream: R2Vocoder,
}

impl LegacyVocoder {
    pub(super) fn inspect(bytes: &[u8]) -> Result<(u32, usize)> {
        drop(crate::model::ModelTree::parse(bytes)?);
        let prepared = PreparedR2::from_bytes(bytes)?;
        Ok((prepared.version(), prepared.node_count()))
    }

    pub(super) fn new(bytes: &[u8], seed: u32) -> Result<Self> {
        drop(crate::model::ModelTree::parse(bytes)?);
        Ok(Self { stream: R2Vocoder::new(bytes, seed)? })
    }

    pub(super) fn prepared_count_inputs(&self) -> count_plan::Query<u64> {
        count_plan::cold(!self.stream.is_cold(), self.stream.has_failed())?;
        count_plan::checked_rows(self.stream.delay_rows())
    }

    pub(super) fn process(&mut self, raw: &[f32], output: &mut Vec<f32>) -> Result<()> {
        let row = raw.try_into().map_err(|_| "R2 vocoder needs 72 input channels")?;
        self.stream.process(row, output)
    }
}
