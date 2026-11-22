use crate::synthesis::{
    inference::{ModelRuntime, Sequence},
    voice::{Result, VoiceError},
};

use super::{
    excitation::Excitation,
    gain_filter::GainFilter,
    interpolation::{SpectralFilterConfig, SpectralFilterStream},
    postfilter::Postfilter,
    stage::{VocoderParameters, VocoderStage},
    stream_queue::{FrameQueue, paired},
};

pub(crate) struct SignalStream {
    source: Excitation,
    gain: GainFilter,
    harmonic_filter: SpectralFilterStream,
    modulated_filter: SpectralFilterStream,
    noise_filter: SpectralFilterStream,
    postfilter: Postfilter,
    hop: usize,
    trailing_scalar: f32,
    spectra: [FrameQueue; 3],
    normalized: FrameQueue,
    source_harmonic: FrameQueue,
    source_modulated: FrameQueue,
    source_postfilter: FrameQueue,
    harmonic: FrameQueue,
    modulated: FrameQueue,
    noise: FrameQueue,
    filtered: [FrameQueue; 3],
    independent:bool,
    gain_clock:FrameQueue,
    gain_parameters:FrameQueue,
}

impl SignalStream {
    pub(crate) fn new(stage: &VocoderStage<'_>) -> Result<Self> {
        let hop = stage.condition.frame_samples;
        let period = stage.condition.frame_seconds;
        let make_filter = |spec| match spec {
            super::configuration::SpectralFilter::Module(node) =>
                SpectralFilterStream::new(SpectralFilterConfig::prepare(stage.tree, node)?, hop),
            super::configuration::SpectralFilter::Window { samples, fft_size } =>
                SpectralFilterStream::with_geometry(hop, samples, fft_size),
        };
        let mut noise_filter=make_filter(stage.modules.noise_filter)?;
        noise_filter.set_noise();
        let mut gain=GainFilter::prepare(stage.tree,stage.modules.coefficients,hop,period)?;
        if stage.compatible_dispatch || stage.editor_seed_source.is_some() || stage.streamed_frame_noise {
            gain.set_native_amplitude_limit();
        }
        let gain_channels=gain.parameter_channels();
        let mut postfilter = Postfilter::prepare(stage.tree, &stage.modules, hop, stage.config.gate_postfilter_noise)?;
        if stage.compatible_dispatch { postfilter.set_compatible_noise(); }
        if stage.streamed_frame_noise {postfilter.set_streamed_frame_noise();}
        Ok(Self {
            source: Excitation::new(hop, period, stage.config.excitation_log_gain.exp())?,
            gain,
            harmonic_filter: make_filter(stage.modules.harmonic_filter)?,
            modulated_filter: make_filter(stage.modules.third_filter)?,
            noise_filter,
            postfilter,
            hop,
            trailing_scalar: stage.config.trailing_scalar,
            spectra: std::array::from_fn(|_| FrameQueue::new(512)),
            normalized: FrameQueue::new(72),
            source_harmonic: FrameQueue::new(hop),
            source_modulated: FrameQueue::new(hop),
            source_postfilter: FrameQueue::new(hop),
            harmonic: FrameQueue::new(hop),
            modulated: FrameQueue::new(hop),
            noise: FrameQueue::new(hop),
            filtered: std::array::from_fn(|_| FrameQueue::new(hop)),
            independent:stage.independent_branches,
            gain_clock:FrameQueue::new(2),
            gain_parameters:FrameQueue::new(gain_channels),
        })
    }

    pub(crate) fn process(
        &mut self,
        runtime: &mut ModelRuntime<'_>,
        parameters: VocoderParameters,
        end: bool,
    ) -> Result<(Sequence, Sequence)> {
        if !self.independent&&parameters.normalized.frames == 0 && !end {
            return Ok((Sequence::zeros(0, self.hop)?, Sequence::zeros(0, self.hop)?));
        }
        self.normalized.push(parameters.normalized)?;
        self.harmonic_filter.push_effects(&parameters.effects);
        self.modulated_filter.push_effects(&parameters.effects);
        self.noise_filter.push_effects(&parameters.effects);
        for (queue, spectrum) in self.spectra.iter_mut().zip(parameters.spectra) {
            queue.push(spectrum)?;
        }
        let excited = self.source.process(&parameters.excitation, end)?;
        let fundamental = parameters
            .excitation
            .iter()
            .map(|frame| frame.f0_hz)
            .collect::<Vec<_>>();
        let phases = excited
            .phase_cycles
            .iter()
            .map(|phase| *phase as f32)
            .collect::<Vec<_>>();
        let excitation = Sequence::from_values(self.hop, excited.samples)?;
        self.source_harmonic.push(excitation.clone())?;
        self.source_modulated.push(excitation.clone())?;
        self.source_postfilter.push(excitation)?;
        let harmonic=if self.independent {
            self.gain_clock.push(Sequence::from_values(2,fundamental.iter().zip(&phases).flat_map(|(&f,&p)|[f,p]).collect())?)?;
            self.gain_parameters.push(parameters.harmonic_parameters)?;
            let (clock,controls)=paired(&mut self.gain_clock,&mut self.gain_parameters,end)?;
            let frequency=clock.values.chunks_exact(2).map(|row|row[0]).collect::<Vec<_>>();
            let phase=clock.values.chunks_exact(2).map(|row|row[1]).collect::<Vec<_>>();
            self.gain.render(&frequency,&phase,&controls,end)?
        }else{self.gain.render(&fundamental,&phases,&parameters.harmonic_parameters,end)?};
        self.harmonic.push(harmonic)?;

        let (source, gain) = paired(&mut self.source_modulated, &mut self.harmonic, end)?;
        self.modulated.push(Sequence::new(
            source.frames,
            self.hop,
            source
                .values
                .iter()
                .zip(&gain.values)
                .map(|(&source, &gain)| source * (self.trailing_scalar + gain))
                .collect(),
        )?)?;

        let (spectrum, source) = filter_inputs(&mut self.spectra[0], &mut self.source_harmonic, end,self.independent)?;
        self.filtered[0].push({
            if self.independent {self.harmonic_filter.process_independent(&spectrum,&source,end)?}
            else {self.harmonic_filter.process(&spectrum, &source, end)?}
        })?;
        let (spectrum, source) = filter_inputs(&mut self.spectra[2], &mut self.modulated, end,self.independent)?;
        self.filtered[2].push({
            if self.independent {self.modulated_filter.process_independent(&spectrum,&source,end)?}
            else {self.modulated_filter.process(&spectrum, &source, end)?}
        })?;

        let (condition, source) = paired(&mut self.normalized, &mut self.source_postfilter, end)?;
        self.noise
            .push(self.postfilter.process(runtime, &condition, &source, end)?)?;
        let (spectrum, noise) = filter_inputs(&mut self.spectra[1], &mut self.noise, end,self.independent)?;
        self.filtered[1].push({
            if self.independent {self.noise_filter.process_independent(&spectrum,&noise,end)?}
            else {self.noise_filter.process(&spectrum, &noise, end)?}
        })?;
        let frames = self.filtered.iter().map(FrameQueue::frames).min().unwrap();
        if end && self.filtered.iter().any(|queue| queue.frames() != frames) {
            return Err(VoiceError::new("Vocoder final PCM branches do not align"));
        }
        let mut primary = self.filtered[0].take(frames)?;
        let third = self.filtered[2].take(frames)?;
        for (first, third) in primary.values.iter_mut().zip(third.values) {
            *first += third;
        }
        Ok((primary, self.filtered[1].take(frames)?))
    }
    pub(crate) fn set_seed(&mut self, seed:u32)->Result<()> { self.postfilter.set_seed(seed) }
    pub(crate) fn append_frame_noise_seeds(&mut self,seeds:&[u64])->Result<()> {self.postfilter.append_frame_noise_seeds(seeds)}
}

fn filter_inputs(parameters:&mut FrameQueue,source:&mut FrameQueue,end:bool,independent:bool)->Result<(Sequence,Sequence)> {
    if !independent {return paired(parameters,source,end);}
    let a=parameters.frames();let b=source.frames();
    Ok((parameters.take(a)?,source.take(b)?))
}
