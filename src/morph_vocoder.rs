use std::collections::VecDeque;
use crate::{Result, noise::{MinStd, NormalDistribution},
    prototype_synthesis_networks::{AnalysisBank8, SynthesisBank8, SynthesisPost, NcwnNetwork, NcwnCausalStream}};

#[derive(Clone,Copy,serde::Serialize)]
pub(crate) struct VocoderDownStartupProgress {
    pub input_samples:usize,
    pub padding_samples:usize,
    pub output_rows:usize,
    pub ring_position:usize,
    pub counter:usize,
    pub finished:bool,
}
#[derive(Clone,Copy,serde::Serialize)]
pub(crate) struct VocoderUpStartupProgress {
    pub input_rows:usize,
    pub suppressed_ticks:usize,
    pub output_samples:usize,
    pub eos_samples:usize,
    pub ring_position:usize,
    pub counter:usize,
    pub finished:bool,
}
#[derive(Clone,Copy,serde::Serialize)]
pub(crate) struct VocoderStartupProgress {
    pub down:VocoderDownStartupProgress,
    pub up:VocoderUpStartupProgress,
    pub trim_remaining:usize,
    pub pending_feature_rows:usize,
    pub pending_condition_rows:usize,
    pub has_previous_condition:bool,
    pub failed:bool,
}

pub(crate) struct VocoderStream {
    down: AnalysisBank8,
    up: SynthesisBank8,
    network: NcwnCausalStream,
    post: SynthesisPost,
    post_work: Option<crate::quantized_matrix::QuantizedWorkspace>,
    features: VecDeque<[f32;16]>,
    conditions: VecDeque<[f32;72]>,
    bands: Vec<Vec<f32>>,
    projected: Vec<Vec<f32>>,
    previous: Option<[f32;72]>,
    rng: MinStd,
    gaussian: NormalDistribution,
    trim: usize,
    noise_enabled: bool,
    failure: Option<String>,
}

impl VocoderStream {

    pub(crate) fn startup_progress(&self)->VocoderStartupProgress {
        let down=self.down.progress();
        let up=self.up.progress();
        VocoderStartupProgress {
            down:VocoderDownStartupProgress {
                input_samples:down.input_samples,padding_samples:down.padding_samples,
                output_rows:down.output_rows,ring_position:down.ring_position,
                counter:down.counter,finished:down.finished,
            },
            up:VocoderUpStartupProgress {
                input_rows:up.input_rows,suppressed_ticks:up.suppressed_ticks,
                output_samples:up.output_samples,eos_samples:up.eos_samples,
                ring_position:up.ring_position,counter:up.counter,finished:up.finished,
            },
            trim_remaining:self.trim,pending_feature_rows:self.features.len(),
            pending_condition_rows:self.conditions.len(),
            has_previous_condition:self.previous.is_some(),failed:self.failure.is_some(),
        }
    }

    pub(crate) fn new(down: AnalysisBank8, up: SynthesisBank8,
        network: NcwnNetwork, post: SynthesisPost, noise_enabled: bool,
        random_seed: u32) -> Result<Self>
    {
        if network.input_width != 16 || network.condition_width != 72 || network.output_width != 64 {
            return Err("Waveform network must map 16 inputs and 72 conditions to 64 features".into());
        }

        #[cfg(target_arch="x86_64")]
        let fused_multiply_add=std::is_x86_feature_detected!("fma");
        #[cfg(not(target_arch="x86_64"))]
        let fused_multiply_add=false;
        let network=network.with_activation_arithmetic(crate::activation::ArithmeticMode {fused_multiply_add})
            .with_packed_float_convolutions()?.into_causal_stream()?;
        let post_work=post.quantized_workspace_requirements().map(|(input,output)|
            crate::quantized_matrix::QuantizedWorkspace::new(input,output)).transpose()?;
        Ok(Self { down, up, network, post, post_work,

            features: VecDeque::with_capacity(42), conditions: VecDeque::with_capacity(30),
            bands: Vec::with_capacity(30), projected: vec![vec![0.;8];30],
            previous: None, rng: MinStd::from_state(random_seed)?,
            gaussian: NormalDistribution::default(), trim: 3, noise_enabled, failure: None })
    }

    pub(crate) fn process(&mut self, signal: &[f32], condition: &[f32],
        output: &mut Vec<f32>) -> Result<()>
    {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        let result = self.advance(signal, condition, output);
        if let Err(error) = &result { self.failure = Some(error.clone()); }
        result
    }

    fn advance(&mut self, signal: &[f32], condition: &[f32], output: &mut Vec<f32>) -> Result<()> {
        if signal.len()!=240 || condition.len()!=72
            || signal.iter().chain(condition).any(|x| !x.is_finite()) {
            return Err("Waveform stage requires a finite 240-sample hop and 72-value condition".into());
        }
        let current: [f32;72] = condition.try_into().map_err(|_| "Condition width")?;
        self.bands.clear();
        self.down.process(signal, &mut self.bands)?;
        for row in self.bands.drain(..) {
            if self.trim>0 { self.trim-=1; continue; }
            if row.len()!=8 { return Err("Analysis bank did not produce eight bands".into()); }
            let mut feature=[0.;16];
            feature[..8].copy_from_slice(&row);
            for value in &mut feature[8..] {
                *value=self.gaussian.sample(&mut self.rng,0.0,1.0,true)?;
            }
            self.features.push_back(feature);
        }
        match &self.previous {
            None => { for _ in 0..15 { self.conditions.push_back(current); } }
            Some(previous) => {
                for step in 0..30 {
                    let fraction=(step as f32+0.5)/30.0;
                    self.conditions.push_back(std::array::from_fn(|i|
                        (current[i]-previous[i])*fraction+previous[i]));
                }
            }
        }
        self.previous=Some(current);

        let count=self.features.len().min(self.conditions.len());
        if count==0 { return Ok(()); }
        if count>self.projected.len() {
            return Err("Waveform stage exceeded its 240-sample row capacity".into());
        }
        for index in 0..count {
            let feature=self.features.front().ok_or("Missing waveform feature row")?;
            let condition=self.conditions.front().ok_or("Missing waveform condition row")?;
            let hidden=self.network.process_row(feature,condition)?;
            let mut row=[0.;8];
            self.post.forward_row_using(hidden,&mut row,self.post_work.as_mut())?;

            if self.noise_enabled {
                for (value,noise) in row.iter_mut().zip(&feature[8..]) {
                    let denominator=(-*value).exp()+1.0;
                    let gain=1.0/denominator;
                    *value=gain * *noise;
                }
            }
            self.projected[index].copy_from_slice(&row);
            self.features.pop_front();
            self.conditions.pop_front();
        }
        self.up.process(&self.projected[..count],output)?;
        Ok(())
    }
}
