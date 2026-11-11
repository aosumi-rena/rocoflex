use std::collections::VecDeque;
use crate::Result;

const INPUT_LIMIT:f32=3.0;
const INPUT_TARGET:f32=0.5;
const INPUT_REFERENCE:f32=f32::from_bits(0x3f4c_cccd);
const INPUT_PEAK_DECAY:f32=f32::from_bits(0x3727_c5ac);
const OUTPUT_PEAK_DECAY:f32=f32::from_bits(0x38d1_b717);
const RETAIN:f32=f32::from_bits(0x3f7d_70a4);
const STEP:f32=f32::from_bits(0x3c23_d70a);

const GATE_OFFSET:f32=f32::from_bits(0xbc23_d70a);

pub(crate) struct AudioLevels {
    input_peak:f32,input_reference:f32,input_gain:f32,output_envelope:f32,output_gate:f32,
    gains:VecDeque<f32>,input_scratch:Vec<f32>,gain_scratch:Vec<f32>,
    output_scratch:Vec<f32>,
}
impl AudioLevels {
    pub(crate) fn new_source_defaults()->Self {
        Self {input_peak:0.0,input_reference:INPUT_REFERENCE,input_gain:1.0,output_envelope:0.0,output_gate:1.0,
            gains:VecDeque::new(),input_scratch:Vec::new(),gain_scratch:Vec::new(),
            output_scratch:Vec::new()}
    }

    pub(crate) fn input_block(&mut self,samples:&mut[f32])->Result<&[f32]> {
        if samples.iter().any(|value|!value.is_finite()) {
            return Err("Nonfinite audio-level input".into());
        }
        self.input_scratch.clear();self.gain_scratch.clear();
        self.input_scratch.try_reserve(samples.len()).map_err(|e|e.to_string())?;
        self.gain_scratch.try_reserve(samples.len()).map_err(|e|e.to_string())?;
        self.gains.try_reserve(samples.len()).map_err(|e|e.to_string())?;
        let ratio=INPUT_TARGET/self.input_reference;
        let limited=ratio.min(INPUT_LIMIT);
        let term=limited*STEP;
        let decay=1.0_f32-INPUT_PEAK_DECAY;
        let(mut peak,mut gain)=(self.input_peak,self.input_gain);
        for &sample in samples.iter() {
            let magnitude=sample.abs();
            let maximum=if magnitude>peak {magnitude} else {peak};
            peak=decay*maximum;
            let retained=gain*RETAIN;
            gain=retained+term;
            let value=gain*sample;
            if !peak.is_finite() || !gain.is_finite() || gain<=0.0 || !value.is_finite() {
                return Err("Invalid audio-level input state or result".into());
            }
            self.input_scratch.push(value);self.gain_scratch.push(gain);
        }
        samples.copy_from_slice(&self.input_scratch);
        self.gains.extend(self.gain_scratch.iter().copied());
        self.input_peak=peak;self.input_gain=gain;
        Ok(&self.gain_scratch)
    }

    pub(crate) fn observe_pitch(&mut self,voiced:bool)->Result<()> {
        if voiced {
            let keep=1.0_f32-STEP;
            let contribution=STEP*self.input_peak;
            let retained=keep*self.input_reference;
            let reference=retained+contribution;
            if !reference.is_finite() || reference<=0.0 {
                return Err("Invalid voiced audio-level reference".into());
            }
            self.input_reference=reference;
        }
        Ok(())
    }

    pub(crate) fn output_block(&mut self,samples:&mut[f32])->Result<()> {
        if samples.len()>self.gains.len() {return Err("Audio-level gain FIFO underflow".into());}
        if samples.iter().any(|value|!value.is_finite()) {
            return Err("Nonfinite audio-level output".into());
        }
        self.output_scratch.clear();
        self.output_scratch.try_reserve(samples.len()).map_err(|e|e.to_string())?;
        let decay=1.0_f32-OUTPUT_PEAK_DECAY;
        let(mut envelope,mut gate)=(self.output_envelope,self.output_gate);
        for (&sample,&gain) in samples.iter().zip(&self.gains) {
            if !gain.is_finite() || gain<=0.0 {return Err("Invalid queued audio-level gain".into());}
            let magnitude=sample.abs();
            let maximum=if magnitude>envelope {magnitude} else {envelope};
            envelope=maximum*decay;
            let shifted=envelope+GATE_OFFSET;
            let ratio=shifted/STEP;
            let capped=if ratio<1.0 {ratio} else {1.0};
            let target=if capped>0.0 {capped} else {0.0};
            let retained=gate*RETAIN;
            let contribution=target*STEP;
            gate=retained+contribution;
            let compensated=sample/gain;
            let value=compensated*gate;
            if !envelope.is_finite() || !gate.is_finite() || !value.is_finite() {
                return Err("Invalid audio-level output state or result".into());
            }
            self.output_scratch.push(value);
        }
        samples.copy_from_slice(&self.output_scratch);
        drop(self.gains.drain(..samples.len()));
        self.output_envelope=envelope;self.output_gate=gate;
        Ok(())
    }
    pub(crate) fn pending_gains(&self)->usize {self.gains.len()}
}
