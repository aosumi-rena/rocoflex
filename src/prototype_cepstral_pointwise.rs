use crate::{Result, quantized_matrix::QuantizedWorkspace};
use super::{BandOperator, CepstralBands, cepstral_silu, cepstral_relu,
    pointwise_into_using};

pub(crate) struct PointwiseCepstralBands {
    parameters: CepstralBands,
    row: Vec<f32>,
    next: Vec<f32>,
    quantized_work: Option<QuantizedWorkspace>,
}

impl CepstralBands {
    pub(crate) fn into_pointwise(self)->Result<PointwiseCepstralBands> {
        if self.input_width==0 || self.output_width==0 || self.bands.is_empty() {
            return Err("Pointwise cepstral dimensions/bands differ".into());
        }
        let mut row_width=self.input_width;
        let mut requirements:Option<(usize,usize)>=None;
        for band in &self.bands {
            if !band.range.admit(self.output_width) {
                return Err("Invalid pointwise cepstral band fades".into());
            }
            let band_width=band.range.end-band.range.start;
            let projected_width=band_width.checked_mul(2)
                .ok_or("Pointwise cepstral projected width overflow")?;
            let mut width=self.input_width;
            for operation in &band.operators {
                if let BandOperator::Conv(conv)=operation {
                    if conv.taps.len()!=1 || conv.stride!=1 || conv.padding!=0
                        || conv.dilation==0 {
                        return Err("Live cepstral convolutions require one tap/stride1/pad0".into());
                    }
                    let matrix=&conv.taps[0];
                    if conv.input_width!=width || conv.output_width==0
                        || matrix.input_width!=conv.input_width
                        || matrix.output_width!=conv.output_width
                        || conv.bias.len()!=conv.output_width {
                        return Err("Pointwise cepstral layer widths differ".into());
                    }
                    width=conv.output_width;
                    row_width=row_width.max(width);
                    if let Some((input,output))=matrix.quantized_workspace_requirements() {
                        requirements=Some(match requirements {
                            Some((previous_input,previous_output))=>
                                (previous_input.max(input),previous_output.max(output)),
                            None=>(input,output),
                        });
                    }
                }
            }
            if width!=projected_width {
                return Err("Pointwise cepstral projection must emit two contiguous band rows".into());
            }
        }
        let mut row=Vec::new();
        row.try_reserve_exact(row_width).map_err(|error|error.to_string())?;
        let mut next=Vec::new();
        next.try_reserve_exact(row_width).map_err(|error|error.to_string())?;
        let quantized_work=requirements.map(|(input,output)|
            QuantizedWorkspace::new(input,output)).transpose()?;
        Ok(PointwiseCepstralBands {parameters:self,row,next,quantized_work})
    }
}

impl PointwiseCepstralBands {

    pub(crate) fn forward(&mut self,input:&[Vec<f32>])->Result<[Vec<Vec<f32>>;2]> {
        let Self {parameters,row,next,quantized_work}=self;
        if input.iter().any(|values|values.len()!=parameters.input_width) {
            return Err("Cepstral input width differs".into());
        }
        let mut outputs:[Vec<Vec<f32>>;2]=
            std::array::from_fn(|_|Vec::with_capacity(input.len()));
        for _ in input {
            for output in &mut outputs {output.push(vec![0.;parameters.output_width]);}
        }
        for band in &parameters.bands {
            let width=band.range.end-band.range.start;
            for (index,values) in input.iter().enumerate() {
                row.clear();
                row.extend_from_slice(values);
                for operation in &band.operators {
                    match operation {
                        BandOperator::Identity=>{},
                        BandOperator::Silu=>cepstral_silu(row),
                        BandOperator::Relu=>cepstral_relu(row),
                        BandOperator::Conv(conv)=>{
                            next.resize(conv.output_width,0.);
                            next.fill(0.);
                            pointwise_into_using(conv,row,next,quantized_work.as_mut())?;
                            std::mem::swap(row,next);
                        }
                    }
                }
                if row.len()!=2*width {
                    return Err("Cepstral projected width differs".into());
                }
                for which in 0..2 {
                    band.range.blend(&row[which*width..(which+1)*width],
                        &mut outputs[which][index])
                        .map_err(|error|format!("Cepstral blend: {error:?}"))?;
                }
            }
        }
        Ok(outputs)
    }
}
