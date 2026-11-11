use crate::Result;
const HOP:usize=240;
const TAPS:usize=512;
const EXTENT:usize=HOP+TAPS-1;
const RAMP_IN:[u32;16]=[0x00000000, 0x3d888889, 0x3e088889, 0x3e4ccccd, 0x3e888889, 0x3eaaaaab, 0x3ecccccd, 0x3eeeeeef, 0x3f088889, 0x3f19999a, 0x3f2aaaab, 0x3f3bbbbc, 0x3f4ccccd, 0x3f5dddde, 0x3f6eeeef, 0x3f800000];
const RAMP_OUT:[u32;16]=[0x3f800000, 0x3f6eeeef, 0x3f5dddde, 0x3f4ccccd, 0x3f3bbbbc, 0x3f2aaaab, 0x3f19999a, 0x3f088889, 0x3eeeeeef, 0x3ecccccd, 0x3eaaaaab, 0x3e888889, 0x3e4ccccd, 0x3e088889, 0x3d888889, 0x00000000];

struct AlternateFir {overlap:[f32;EXTENT]}
impl AlternateFir {
    fn new()->Self {Self {overlap:[0.;EXTENT]}}
    fn reset(&mut self) {self.overlap.fill(0.);}
    fn push(&mut self,signal:&[f32;HOP],first:&[f32;TAPS],second:&[f32;TAPS],
        window:&[f32;2*HOP])->Result<[f32;HOP]>
    {
        if signal.iter().chain(first).chain(second).chain(window).chain(&self.overlap)
            .any(|x|!x.is_finite()) {
            return Err("Alternate filter inputs must be finite".into());
        }

        let mut next=self.overlap;
        for i in 0..HOP {
            let first_scale=window[i+HOP]*signal[i];
            let second_scale=window[i]*signal[i];
            for j in 0..TAPS {
                let at=i+j;
                let mut a=first[j]*first_scale;
                let mut b=second[j]*second_scale;
                if at<16 {b=f32::from_bits(RAMP_IN[at])*b;}
                if i>=HOP-16 && j>=TAPS-16 {
                    a=f32::from_bits(RAMP_OUT[j-(TAPS-16)])*a;
                }
                next[at]+=a+b;
            }
        }
        if next.iter().any(|x|!x.is_finite()) {
            return Err("Alternate filter produced nonfinite audio".into());
        }
        let mut output=[0.;HOP];output.copy_from_slice(&next[..HOP]);
        next.copy_within(HOP..EXTENT,0);
        next[TAPS-1..].fill(0.);
        self.overlap=next;
        Ok(output)
    }
}

use std::collections::VecDeque;
pub(super) struct AlternateFilterStream {
    filter:AlternateFir,
    coefficients:VecDeque<[f32;TAPS]>,
    signals:VecDeque<[f32;HOP]>,
    window:[f32;2*HOP],
    feature_closed:bool,
    signal_closed:bool,
    failed:Option<String>,
}
impl AlternateFilterStream {
    pub(super) fn new()->Self {
        let mut window=[0.;2*HOP];window[HOP]=1.;
        for i in 1..HOP {
            let angle=std::f64::consts::TAU*(i as f64)/(2*HOP) as f64;
            let value=((1.-angle.cos())*0.5) as f32;
            window[i]=value;window[2*HOP-i]=value;
        }
        Self {filter:AlternateFir::new(),coefficients:VecDeque::new(),signals:VecDeque::new(),
            window,feature_closed:false,signal_closed:false,failed:None}
    }
    pub(super) fn reset(&mut self) {
        self.filter.reset();self.coefficients.clear();self.signals.clear();
        self.feature_closed=false;self.signal_closed=false;self.failed=None;
    }
    pub(super) fn push_coefficients(&mut self,rows:&[[f32;TAPS]],final_rows:bool)->Result<()> {
        if let Some(error)=&self.failed {return Err(error.clone());}
        if self.feature_closed {return Err("Alternate coefficient stream already ended".into());}
        if rows.iter().flatten().any(|x|!x.is_finite()) {return Err("Nonfinite alternate filter coefficient".into());}
        let count=rows.len().checked_add(usize::from(final_rows && (!rows.is_empty() || !self.coefficients.is_empty())))
            .ok_or("Alternate coefficient count overflow")?;
        self.coefficients.try_reserve(count).map_err(|e|e.to_string())?;
        self.coefficients.extend(rows.iter().copied());
        if final_rows {
            if let Some(last)=self.coefficients.back().copied() {self.coefficients.push_back(last);}
            self.feature_closed=true;
        }
        Ok(())
    }
    pub(super) fn push_signals(&mut self,blocks:&[[f32;HOP]],final_blocks:bool)->Result<()> {
        if let Some(error)=&self.failed {return Err(error.clone());}
        if self.signal_closed {return Err("Alternate signal stream already ended".into());}
        if blocks.iter().flatten().any(|x|!x.is_finite()) {return Err("Nonfinite alternate filter input".into());}
        self.signals.try_reserve(blocks.len()).map_err(|e|e.to_string())?;
        self.signals.extend(blocks.iter().copied());
        if final_blocks {self.signal_closed=true;}
        Ok(())
    }
    pub(super) fn available_blocks(&self)->usize {
        self.signals.len().min(self.coefficients.len().saturating_sub(1))
    }
    pub(super) fn queued_counts(&self)->(usize,usize) {
        (self.coefficients.len(),self.signals.len())
    }
    pub(super) fn drain(&mut self,output:&mut Vec<[f32;HOP]>)->Result<usize> {
        if let Some(error)=&self.failed {return Err(error.clone());}
        let count=self.available_blocks();
        output.try_reserve(count).map_err(|e|e.to_string())?;
        for _ in 0..count {
            let signal=self.signals.front().expect("count established");
            let first=self.coefficients.front().expect("count established");
            let second=self.coefficients.get(1).expect("count established");
            let block=match self.filter.push(signal,first,second,&self.window) {
                Ok(block)=>block,
                Err(error)=>{self.failed=Some(error.clone());return Err(error);}
            };
            output.push(block);self.signals.pop_front();self.coefficients.pop_front();
        }
        Ok(count)
    }
}

