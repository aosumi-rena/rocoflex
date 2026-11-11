use std::collections::VecDeque;
use crate::{Result,
    pitch_audio_spectrum::{PitchAudioSpectrum,PitchSpectrumStream,PitchSpectrumState},
    pitch_spectral_encoder::{PitchSpectralEncoder,PitchEncoderStream,PitchEncoderState},
    pitch_temporal::{PitchTemporal,PitchTemporalState,PitchTemporalWorkspace},
    pitch_selected_head::{AdmittedPitchHead,AdmittedPitchStream,AdmittedPitchState,PitchHeadSelection,PitchRecord,IndicatorAccumulation},
    prototype_synthesis_networks::Root1AuxHeads};

pub(crate) struct SourceFrame {
    pub ordinal:u64,pub large321:Vec<f32>,pub small15:Vec<f32>,
    pub pitch:PitchRecord,pub auxiliary:f32,
}
struct Auxiliary {ordinal:u64,large:Vec<f32>,small:Vec<f32>}
pub(crate) struct SourceDiagnosticProgress {
    pub accepted_samples:usize,pub spectrum_rows:[usize;3],pub retained_samples:usize,
    pub trunk_rows:u64,pub pending_auxiliary:usize,
}
pub(crate) struct SourceProducer {
    spectrum:PitchAudioSpectrum,spectrum_state:Option<PitchSpectrumState>,
    encoder:PitchSpectralEncoder,encoder_state:Option<PitchEncoderState>,
    temporal:PitchTemporal,hidden:PitchTemporalState,temporal_work:PitchTemporalWorkspace,
    head:AdmittedPitchHead,head_state:Option<AdmittedPitchState>,aux:Root1AuxHeads,
    pending:VecDeque<Auxiliary>,trunk_rows:u64,failure:Option<String>,
}
impl SourceProducer {
    pub(crate) fn new(analysis:&[u8])->Result<Self> {
        Self::with_head(analysis,PitchHeadSelection::Head2)
    }
    pub(crate) fn with_head(analysis:&[u8],selection:PitchHeadSelection)->Result<Self> {
        let spectrum=PitchAudioSpectrum::from_dnni(analysis)?;
        let spectrum_state=spectrum.stream_with_limit(i32::MAX as usize)?.into_state();
        let encoder=PitchSpectralEncoder::from_dnni(analysis)?;
        let encoder_state=encoder.stream_with_limit(i32::MAX as usize)?.into_state();
        let head=AdmittedPitchHead::from_dnni(analysis,selection,IndicatorAccumulation::PerInputSaturatedI16)?;
        let head_state=head.stream_from_explicit_position(-1,i32::MAX as usize)?.into_state();
        let temporal=PitchTemporal::from_dnni(analysis)?;
        let temporal_work=temporal.workspace()?;
        Ok(Self {spectrum,spectrum_state:Some(spectrum_state),encoder,encoder_state:Some(encoder_state),
            temporal,hidden:PitchTemporalState::default(),temporal_work,
            head,head_state:Some(head_state),aux:Root1AuxHeads::from_dnni(analysis)?,
            pending:VecDeque::new(),trunk_rows:0,failure:None})
    }
    pub(crate) fn diagnostic_progress(&self)->Result<SourceDiagnosticProgress> {
        let state=self.spectrum_state.as_ref().ok_or("Missing diagnostic Source spectrum cursor")?;
        let (accepted_samples,spectrum_rows,retained_samples)=state.diagnostic_progress();
        Ok(SourceDiagnosticProgress {accepted_samples,spectrum_rows,retained_samples,
            trunk_rows:self.trunk_rows,pending_auxiliary:self.pending.len()})
    }

    pub(crate) fn cold_count_domain(&self)->super::count_plan::Query<()> {
        self.cold_count_inputs().map(|_|())
    }
    pub(crate) fn cold_count_inputs(&self)->super::count_plan::Query<super::count_plan::HeadCounts> {
        use super::count_plan::{self as count,Unavailable,Invalid,Unsupported,HeadKind};
        use crate::pitch_selected_head::PitchHeadCountError;
        if self.encoder_state.is_none() || self.head_state.is_none() {
            return Err(Unavailable::Invalid(Invalid::MissingCursor));
        }
        let p=self.diagnostic_progress().map_err(|_|Unavailable::Invalid(Invalid::MissingCursor))?;
        count::cold(p.accepted_samples!=0 || p.spectrum_rows!=[0;3] || p.retained_samples!=0
            || self.trunk_rows!=0 || !self.pending.is_empty(),self.failure.is_some())?;
        let metadata=self.head.cold_count_metadata(self.head_state.as_ref()
            .ok_or(Unavailable::Invalid(Invalid::MissingCursor))?).map_err(|why|match why {
                PitchHeadCountError::UnknownTopology=>Unavailable::Unsupported(Unsupported::UnknownTopology),
                PitchHeadCountError::MissingCursor=>Unavailable::Invalid(Invalid::MissingCursor),
                PitchHeadCountError::AdvancedCursor=>Unavailable::Unsupported(Unsupported::AdvancedCursor),
                PitchHeadCountError::FailedCursor=>Unavailable::Unsupported(Unsupported::FailedCursor),
            })?;
        let kind=match metadata.selection {PitchHeadSelection::Head0=>HeadKind::Head0,PitchHeadSelection::Head2=>HeadKind::Head2};
        count::head_counts(kind,count::checked_rows(metadata.right_rows)?)
    }
    pub(crate) fn process(&mut self,input:&[f32],output:&mut Vec<SourceFrame>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.advance(input,output);
        if let Err(error)=&result {self.failure=Some(error.clone());} result
    }
    fn advance(&mut self,input:&[f32],output:&mut Vec<SourceFrame>)->Result<()> {
        let state=self.spectrum_state.take().ok_or("Missing pitch spectrum cursor")?;
        let batch=match PitchSpectrumStream::from_state(&self.spectrum,state).push(input,false) {
            Ok(batch)=>batch,Err(failure)=>{
                self.spectrum_state=Some(failure.advanced.into_state());return Err(failure.reason);
            }
        };
        self.spectrum_state=Some(batch.state.into_state());
        let state=self.encoder_state.take().ok_or("Missing pitch encoder cursor")?;
        let encoded=match PitchEncoderStream::from_state(&self.encoder,state)
            .push([&batch.spectra[0],&batch.spectra[1],&batch.spectra[2]],false) {
            Ok(batch)=>batch,Err(failure)=>{
                self.encoder_state=Some(failure.advanced.into_state());return Err(failure.reason);
            }
        };
        self.encoder_state=Some(encoded.state.into_state());
        let Some(stages)=encoded.stages else {return Ok(());};
        if encoded.first_row as u64!=self.trunk_rows {return Err("Pitch trunk row discontinuity".into());}

        let temporal_rows=self.temporal.forward384_with_workspace(&stages.summed384,&mut self.hidden,
            &mut self.temporal_work)?.output384;
        if temporal_rows.len()%384!=0 {return Err("Partial pitch temporal row".into());}
        for row in temporal_rows.chunks_exact(384) {
            let (large,small)=self.aux.forward(row)?;
            self.pending.push_back(Auxiliary {ordinal:self.trunk_rows,large,small});
            self.trunk_rows+=1;
        }
        let state=self.head_state.take().ok_or("Missing selected-pitch cursor")?;
        let selected=match AdmittedPitchStream::from_state(&self.head,state)?.push(&temporal_rows,false) {
            Ok(batch)=>batch,Err(failure)=>{
                let (reason,advanced)=failure.into_parts();
                self.head_state=Some(advanced.into_state());return Err(reason);
            }
        };
        let (first_row,rows,advanced)=selected.into_parts();
        self.head_state=Some(advanced.into_state());
        if let Some(rows)=rows {
            if rows.records.len()!=rows.auxiliary.len() {return Err("Pitch record/auxiliary extent mismatch".into());}
            for (index,(pitch,auxiliary)) in rows.records.into_iter().zip(rows.auxiliary).enumerate() {
                let aux=self.pending.pop_front().ok_or("Missing actual auxiliary-head row")?;
                if aux.ordinal!=first_row as u64+index as u64 {
                    return Err("Selected pitch/auxiliary ordinal mismatch".into());
                }
                output.push(SourceFrame {ordinal:aux.ordinal,large321:aux.large,small15:aux.small,pitch,auxiliary});
            }
        }
        Ok(())
    }
}
