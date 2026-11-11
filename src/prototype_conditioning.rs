use crate::Result;
use super::{DenseMatrix,FloatGru};

pub struct ConditioningNetwork {
    pub gru: FloatGru,
    dct: DenseMatrix,
    frequencies: Vec<f32>, prefix: usize, period: u16, mode: u32, feature_width: usize,
    dct_values: Vec<f32>, phase_row: Vec<f32>,
    dct_work: Option<crate::quantized_matrix::QuantizedWorkspace>,
}

impl ConditioningNetwork {
    pub(super) fn from_parts(gru:FloatGru,dct:DenseMatrix,frequencies:Vec<f32>,
        prefix:usize,period:u16,mode:u32,feature_width:usize)->Result<Self> {
        if prefix + 2*frequencies.len() != gru.input_width() || prefix > dct.output_width {
            return Err("Conditioning DCT/frequency/GRU dimensions differ".into());
        }
        let mut dct_values=Vec::new();
        dct_values.try_reserve_exact(dct.output_width).map_err(|e|e.to_string())?;
        dct_values.resize(dct.output_width,0.);
        let mut phase_row=Vec::new();
        phase_row.try_reserve_exact(gru.input_width()).map_err(|e|e.to_string())?;
        let dct_work=dct.quantized_workspace_requirements()
            .map(|(input,output)|crate::quantized_matrix::QuantizedWorkspace::new(input,output)).transpose()?;
        Ok(Self {gru,dct,frequencies,prefix,period,mode,feature_width,
            dct_values,phase_row,dct_work})
    }

    pub fn condition(&mut self, feature72: &[f32], phase_word: u32) -> Result<Vec<f32>> {
        if feature72.len() != self.feature_width || feature72.iter().any(|x| !x.is_finite()) {
            return Err("Conditioning feature row width/values differ".into());
        }
        let (start, trim) = if self.mode == 0 { (3, 8) } else { (0, 1) };
        let input = feature72.get(start..start+self.feature_width-trim).ok_or("Conditioning input slice differs")?;
        self.dct.overwrite_using(input,&mut self.dct_values,self.dct_work.as_mut())?;
        self.phase_row.clear();
        self.phase_row.extend_from_slice(&self.dct_values[..self.prefix]);
        let angle = f64::from(phase_word)*std::f64::consts::TAU/f64::from(self.period);
        self.phase_row.extend(self.frequencies.iter().map(|&f| (angle*f64::from(f)).sin() as f32));
        self.phase_row.extend(self.frequencies.iter().map(|&f| (angle*f64::from(f)).cos() as f32));
        Ok(self.gru.step(&self.phase_row)?.to_vec())
    }
}

