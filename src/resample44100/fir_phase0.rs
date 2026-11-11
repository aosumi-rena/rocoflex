use super::fir_input::InputExpander;
use super::fir_output::OutputSelector;
use super::fir_layout::FirLayout;
use super::linear_phase_spectrum::LinearPhaseSpectrum;
use super::ooura_table_f64::{RealDft,Trig64};

pub struct Phase0Fir {
    pub layout:FirLayout,
    filter:Vec<f64>,
    input_fft:RealDft,
    output_fft:Option<RealDft>,
    history:Vec<f64>,
    current:Vec<f64>,
    previous:Vec<f64>,
    remaining:usize,
    input_state:InputExpander,
    output_state:OutputSelector,
}

impl Phase0Fir {
    pub fn from_generated_spectrum(filter:LinearPhaseSpectrum,up:usize,down:usize,input_delay:f64)->Result<Self,&'static str> {
        let layout=FirLayout::phase0(filter.integer_latency*2+1,up,down,input_delay)?;
        if filter.fft_len!=layout.fft_len || filter.packed.len()!=layout.fft_len {
            return Err("source generated filter/layout mismatch");
        }
        let input_fft=RealDft::new(layout.input_fft_len)?;
        let output_fft=if layout.input_fft_len==layout.output_fft_len { None } else {Some(RealDft::new(layout.output_fft_len)?)};
        let input_state=InputExpander::from_source_state(up,0)?;
        let output_state=OutputSelector::from_source_state(layout.fft_len as i32,down as i32,layout.initial_discard as i32,0)?;
        let n=layout.fft_len;
        let history=vec![0.0;layout.history_len];
        let remaining=layout.reset_remaining;
        Ok(Self {layout,filter:filter.packed,input_fft,output_fft,history,
                 current:vec![0.0;n],previous:vec![0.0;n],remaining,input_state,output_state})
    }

    pub fn reset(&mut self)->Result<(),&'static str> {
        self.history.fill(0.0);
        self.current[..self.layout.initial_zeros].fill(0.0);
        self.remaining=self.layout.reset_remaining;
        self.input_state=InputExpander::from_source_state(self.layout.up,0)?;
        self.output_state=OutputSelector::from_source_state(self.layout.fft_len as i32,self.layout.down as i32,self.layout.initial_discard as i32,0)?;
        Ok(())
    }

    pub fn remaining_input_domain(&self)->usize { self.remaining }
    pub fn down_phase(&self)->i32 { self.output_state.phase() }
    pub fn pending_discard(&self)->i32 { self.output_state.pending_discard() }

    fn fill(&mut self,input:&[f64],offset:usize,count:usize)->Result<usize,&'static str> {
        if self.layout.up_shift>=0 {
            if offset%self.layout.up!=0 || count%self.layout.up!=0 {
                return Err("source shortened-input alignment invariant");
            }
            let start=offset/self.layout.up;
            let consumed=count/self.layout.up;
            if input.len()<consumed { return Err("source input span too short"); }
            self.current[start..start+consumed].copy_from_slice(&input[..consumed]);
            Ok(consumed)
        } else {
            self.input_state.fill(input,&mut self.current[offset..offset+count])
        }
    }

    fn transform_block<M:Trig64>(&mut self,math:&mut M)->Result<(),&'static str> {
        let input_n=self.layout.input_fft_len;
        let input_block=if self.layout.up_shift>=0 { self.layout.block_len/self.layout.up } else { self.layout.block_len };
        let history_n=self.history.len();
        if input_block<history_n || input_block+history_n!=input_n {
            return Err("source history-copy geometry outside candidate scope");
        }
        self.current[input_block..input_n].copy_from_slice(&self.history);
        self.history.copy_from_slice(&self.current[input_block-history_n..input_block]);
        self.input_fft.transform(&mut self.current[..input_n],true,math)?;
        if self.layout.up_shift>0 {

            let twice_input=input_n*2;
            let mut reverse=input_n-1;
            for at in (input_n+2..twice_input).step_by(2) {
                self.current[at]=self.current[reverse-1];
                self.current[at+1]=f64::from_bits(self.current[reverse].to_bits()^0x8000_0000_0000_0000);
                reverse-=2;
            }
            self.current[input_n]=self.current[1];
            self.current[input_n+1]=0.0;
            self.current[1]=self.current[0];
            for shift in 1..self.layout.up_shift {
                let count=input_n<<shift;
                self.current.copy_within(0..count,count);
                self.current[count+1]=0.0;
            }
        }

        let output_n=self.layout.output_fft_len;
        for (value,coefficient) in self.current[..output_n].iter_mut().zip(&self.filter[..output_n]) {
            *value=*coefficient * *value;
        }
        if self.layout.down_shift>0 {

            let real=self.filter[output_n] * self.current[output_n];
            let imaginary=self.current[output_n+1] * self.filter[output_n+1];
            self.current[1]=real-imaginary;
        }
        let plan=if let Some(plan)=self.output_fft.as_mut() {plan} else {&mut self.input_fft};
        plan.transform(&mut self.current[..output_n],false,math)?;
        Ok(())
    }

    pub fn process<M:Trig64>(&mut self,input:&[f64],math:&mut M)->Result<Vec<f64>,&'static str> {
        let mut source_remaining=input.len().checked_mul(self.layout.up).ok_or("source expanded-count overflow")?;
        if source_remaining>i32::MAX as usize { return Err("source expanded count outside i32 scope"); }
        let mut consumed=0;
        let mut output=Vec::new();
        while source_remaining>0 {
            let filled=self.layout.block_len-self.remaining;
            if source_remaining>=self.remaining {
                let count=self.remaining;
                source_remaining-=count;
                self.remaining=self.layout.block_len;
                consumed+=self.fill(&input[consumed..],filled,count)?;
                self.transform_block(math)?;
                self.output_state.append(&self.previous,
                    filled as i32-self.layout.latency_samples as i32,count as i32,&mut output)?;
                std::mem::swap(&mut self.current,&mut self.previous);
            } else {
                self.remaining-=source_remaining;
                consumed+=self.fill(&input[consumed..],filled,source_remaining)?;
                self.output_state.append(&self.previous,
                    filled as i32-self.layout.latency_samples as i32,source_remaining as i32,&mut output)?;
                source_remaining=0;
            }
        }
        if consumed!=input.len() { return Err("source input cursor/count invariant failed"); }
        Ok(output)
    }
}
