use crate::Result;
use super::{generator_math::{ChirpTransform,polar_response},generator_table::GeneratorTable};

const HOP:usize=240;
const WIDTH:usize=2*HOP;
const HOP_SECONDS:f32=f32::from_bits(0x3ba3_d70a);
const TWO_PI:f32=f32::from_bits(0x40c9_0fdb);

pub(crate) struct SourceGenerator {
    table:GeneratorTable,
    chirp:ChirpTransform,
    window:[f32;WIDTH],
    overlap:[f32;WIDTH],
    magnitude:[f32;WIDTH+1],
    phase:[f32;WIDTH+1],
    real:[f32;WIDTH+1],
    imaginary:[f32;WIDTH+1],
    transformed:[f32;WIDTH],
    phase_state:f64,
    first:bool,
    failure:Option<String>,
}
impl SourceGenerator {
    pub(crate) fn new(log_window_gain:f32)->Result<Self> {
        let gain=log_window_gain.exp();
        if !gain.is_finite() {return Err("Nonfinite source-generator window gain".into());}
        let window=std::array::from_fn(|k| {
            let angle=(k as f32/WIDTH as f32)*TWO_PI;
            ((1.0-angle.cos())*0.5)*gain
        });
        Ok(Self {table:GeneratorTable::new(),chirp:ChirpTransform::new(WIDTH)?,window,
            overlap:[0.0;WIDTH],magnitude:[0.0;WIDTH+1],phase:[0.0;WIDTH+1],
            real:[0.0;WIDTH+1],imaginary:[0.0;WIDTH+1],transformed:[0.0;WIDTH],
            phase_state:0.0,first:true,failure:None})
    }

    pub(crate) fn process(&mut self,raw:&[f32],output:&mut Vec<f32>)->Result<()> {
        if let Some(error)=&self.failure {return Err(error.clone());}
        let result=self.advance(raw,output);
        if let Err(error)=&result {self.failure=Some(error.clone());}
        result
    }
    fn advance(&mut self,raw:&[f32],output:&mut Vec<f32>)->Result<()> {
        if raw.len()!=72 || raw.iter().any(|value|!value.is_finite()) || raw[1]<=0.0 {
            return Err("Source generator requires a finite raw decoder row and positive pitch".into());
        }
        if !self.first {output.try_reserve(HOP).map_err(|e|e.to_string())?;}
        let pitch=raw[1];
        let advance=HOP_SECONDS*pitch;
        self.phase_state+=f64::from(advance);
        if !self.phase_state.is_finite() {return Err("Source-generator phase overflow".into());}
        let local_phase=(self.phase_state%1.0) as f32;
        self.overlap.copy_within(HOP..WIDTH,0);
        self.overlap[HOP..].fill(0.0);
        if raw[0]>=0.5 {
            let sample_rate=HOP as f32/HOP_SECONDS;
            let count=((0.5*sample_rate)/pitch).trunc();

            if !count.is_finite() || count>=2_147_483_648.0 {
                return Err("Source-generator harmonic count overflow".into());
            }
            let harmonics=count.min(WIDTH as f32) as usize;
            if harmonics>0 {
                let parameters=self.table.parameters(raw[2].clamp(0.1,2.9))?;
                let (normalization,_)=polar_response(parameters,200.0)?;
                if normalization<=0.0 {return Err("Zero source-generator response normalization".into());}
                self.magnitude.fill(0.0);self.phase.fill(0.0);
                self.real.fill(0.0);self.imaginary.fill(0.0);
                for k in 1..=harmonics {
                    let grid=(100*k) as f32;
                    let (magnitude,phase)=polar_response(parameters,grid)?;
                    let normalized=magnitude/normalization;
                    let amplitude=(((normalized/harmonics as f32)/pitch)*sample_rate)*0.1;
                    self.magnitude[k]=amplitude;
                    self.phase[k]=phase;
                }
                let phase_shift=-self.phase[1];
                for k in 1..=harmonics {
                    let index=k as f32;
                    let offset=phase_shift*index+self.phase[k];
                    let angle=((local_phase-advance)*index)*TWO_PI+offset;
                    self.real[k]=angle.cos()*self.magnitude[k];
                    self.imaginary[k]=angle.sin()*self.magnitude[k];
                }
                let rate=(pitch*(-TWO_PI))/sample_rate;
                self.chirp.real_output(&self.real[..WIDTH],&self.imaginary[..WIDTH],rate,
                    &mut self.transformed)?;
                for k in 0..WIDTH {
                    let product=self.transformed[k]*self.window[k];
                    self.overlap[k]=product+self.overlap[k];
                }
            }
        }
        if self.overlap.iter().any(|value|!value.is_finite()) {
            return Err("Nonfinite source-generator overlap output".into());
        }
        if !self.first {output.extend_from_slice(&self.overlap[..HOP]);}
        self.first=false;
        Ok(())
    }
}
