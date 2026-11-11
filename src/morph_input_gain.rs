use crate::Result;

pub(crate) struct InputGain { gain:f32 }
impl InputGain {
    pub(crate) fn new()->Self {Self {gain:1.}}
    pub(crate) fn value(&self)->f32 {self.gain}

    pub(crate) fn meter_block(&self,input:&[f32],previous:f32)->Option<f32> {
        if input.is_empty() || !previous.is_finite() || previous<0. {return None;}
        let _fp=InputFp::enter();
        let mut peak=0.0_f32;
        for &sample in input {
            if !sample.is_finite() {return None;}
            peak=peak.max(sample.abs());
        }
        let incoming=peak*self.gain;
        let decayed=previous*0.99_f32;
        let next=incoming.max(decayed);
        next.is_finite().then_some(next)
    }

    pub(crate) fn append(&mut self,input:&[f32],buffer:&mut Vec<f32>,db:f32)->Result<()> {
        if !db.is_finite() || input.iter().any(|v|!v.is_finite()) {
            return Err("Invalid pregain input".into());
        }
        buffer.try_reserve(input.len()).map_err(|e|format!("Cannot buffer pregain input: {e}"))?;

        let _fp=InputFp::enter();
        for &sample in input {

            let target=10.0_f64.powf(f64::from(db)/20.) as f32;
            let target_product=target*0.002_f32;
            let old_product=self.gain*0.998_f32;
            let gain=old_product+target_product;
            let scaled=sample*gain;
            if !gain.is_finite() || !scaled.is_finite() {
                return Err("Pregain produced nonfinite audio".into());
            }
            self.gain=gain;
            buffer.push(scaled);
        }
        Ok(())
    }
}

struct InputFp {saved:u32}
impl InputFp {
    fn enter()->Self {

        unsafe {
            let saved=std::arch::x86_64::_mm_getcsr();
            std::arch::x86_64::_mm_setcsr(saved|0x8040);
            Self {saved}
        }
    }
}
impl Drop for InputFp {
    fn drop(&mut self) {

        unsafe {std::arch::x86_64::_mm_setcsr(self.saved);}
    }
}
