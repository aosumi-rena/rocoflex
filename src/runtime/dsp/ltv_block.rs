#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum LtvFftDirection {ForwardPlus1,InverseMinus1}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum LtvArithmeticPolicy {NearestEvenSeparateOperationsGradualUnderflow}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum LtvError {UnsupportedConfiguration,RowCount,Width,FftBackend}

pub trait SourceLtvFft {
    fn transform_in_place(&mut self,direction:LtvFftDirection,buffer:&mut[f32])->Result<(),LtvError>;
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SourceLtvBlockConfig {window:usize,transform:usize,hop:usize}
impl SourceLtvBlockConfig {

    pub fn bind(window:usize,transform:usize,hop:usize)->Result<Self,LtvError> {
        if hop==0||window==0||!transform.is_power_of_two()||transform>i32::MAX as usize
          ||hop>i32::MAX as usize/2||window%(2*hop)!=0||transform<=window||transform-window<=31 {
            return Err(LtvError::UnsupportedConfiguration);
        }
        Ok(Self{window,transform,hop})
    }
    pub fn transform_length(self)->usize {self.transform}
    pub fn window_length(self)->usize {self.window}
    pub fn hop(self)->usize {self.hop}
    pub fn leading_padding(self)->usize {(self.transform-self.window)/2}

    pub fn accumulate_block(self,policy:LtvArithmeticPolicy,fft:&mut dyn SourceLtvFft,
      source_window:&[f32],signal_rows:&[Vec<f32>],packed_coefficients:&[f32],
      overlap:&mut[f32])->Result<(),LtvError> {
        if source_window.len()!=self.window||packed_coefficients.len()!=self.transform||overlap.len()!=self.transform {
            return Err(LtvError::Width);
        }
        if signal_rows.len()!=self.window/self.hop{return Err(LtvError::RowCount);}
        if signal_rows.iter().any(|row|row.len()!=self.hop){return Err(LtvError::Width);}
        let mut scratch=vec![0.0f32;self.transform];
        self.accumulate_block_in_scratch(policy,fft,source_window,signal_rows,packed_coefficients,overlap,&mut scratch)
    }

    pub fn accumulate_block_in_scratch(self,_policy:LtvArithmeticPolicy,fft:&mut dyn SourceLtvFft,
      source_window:&[f32],signal_rows:&[Vec<f32>],packed_coefficients:&[f32],
      overlap:&mut[f32],scratch:&mut[f32])->Result<(),LtvError> {
        if source_window.len()!=self.window||packed_coefficients.len()!=self.transform||overlap.len()!=self.transform {
            return Err(LtvError::Width);
        }
        if scratch.len()!=self.transform{return Err(LtvError::Width);}
        if signal_rows.len()!=self.window/self.hop {return Err(LtvError::RowCount);}
        if signal_rows.iter().any(|row|row.len()!=self.hop) {return Err(LtvError::Width);}
        scratch.fill(0.0);
        let leading=self.leading_padding();
        for (row_index,row) in signal_rows.iter().enumerate() {
            for (index,&sample) in row.iter().enumerate() {
                let offset=row_index*self.hop+index;
                scratch[leading+offset]=sample*source_window[offset];
            }
        }
        fft.transform_in_place(LtvFftDirection::ForwardPlus1,scratch)?;
        scratch[0]=packed_coefficients[0]*scratch[0];
        scratch[1]=packed_coefficients[1]*scratch[1];
        for bin in 1..self.transform/2 {
            let real=scratch[2*bin];let imag=scratch[2*bin+1];
            let cr=packed_coefficients[2*bin];let ci=packed_coefficients[2*bin+1];
            scratch[2*bin]=(real*cr)-(ci*imag);
            scratch[2*bin+1]=(real*ci)+(imag*cr);
        }
        fft.transform_in_place(LtvFftDirection::InverseMinus1,scratch)?;
        let scale=2.0f32/(self.transform as f32);
        for index in 0..16 {
            let normalized=scratch[index]*scale;
            let ramp=((index+1) as f32)*0.0625f32;
            overlap[index]=(ramp*normalized)+overlap[index];
        }
        for index in 16..self.transform-16 {
            overlap[index]=(scratch[index]*scale)+overlap[index];
        }
        for index in self.transform-16..self.transform {
            let count=index-(self.transform-16)+1;
            let ramp=((count as f32)*-0.0625f32)+1.0f32;
            let normalized=scratch[index]*scale;
            overlap[index]=(ramp*normalized)+overlap[index];
        }
        Ok(())
    }
}
