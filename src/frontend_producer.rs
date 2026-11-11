use crate::{
    Result,
    feature_projection::FeatureProjection,
    frontend_gru::FrontendGru,
    spectral_encoder::SpectralEncoder,
    temporal_mixer::{TemporalMixer, TemporalMixerStages},
};

#[path = "frontend_producer_stream.rs"]
mod incremental;
pub use incremental::{FrontendBatch, FrontendStartFailure, FrontendStream, FrontendStreamFailure};

pub struct FrontendProducer {
    encoder: SpectralEncoder,
    gru: FrontendGru,
    temporal: TemporalMixer,
    features: FeatureProjection,
}

pub struct FrontendProducerStages {
    pub recurrent384: Vec<f32>,
    pub temporal: TemporalMixerStages,
    pub right256: Vec<f32>,
    pub combined256: Vec<f32>,
    pub activated256: Vec<f32>,
    pub features_72: Vec<f32>,
}

impl FrontendProducer {
    pub fn stream_from_explicit_state(&self, hidden384: Vec<f32>, maximum_rows: usize)
        -> std::result::Result<FrontendStream<'_>, FrontendStartFailure>
    {
        FrontendStream::new(self, hidden384, maximum_rows)
    }

    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        Ok(Self {
            encoder: SpectralEncoder::from_dnni(model)?,
            gru: FrontendGru::from_dnni(model)?,
            temporal: TemporalMixer::from_dnni(model)?,
            features: FeatureProjection::from_dnni(model)?,
        })
    }

    pub fn forward_spectral256(
        &self,
        input256: &[f32],
        state384: &mut [f32],
    ) -> Result<FrontendProducerStages> {
        let recurrent384 = self.gru.forward256(input256, state384)?;
        let temporal = self.temporal.forward384(&recurrent384)?;
        let right256 = self.encoder.forward_right256(input256)?;
        let combined256 = temporal
            .left256
            .iter()
            .zip(&right256)
            .map(|(a, b)| a + b)
            .collect::<Vec<_>>();
        let head = self.features.forward256(&combined256)?;
        Ok(FrontendProducerStages {
            recurrent384,
            temporal,
            right256,
            combined256,
            activated256: head.activated256,
            features_72: head.features72,
        })
    }
}
