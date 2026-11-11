#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputExpander { up:usize, pending_zeros:usize }

impl InputExpander {
    pub fn from_source_state(up:usize,pending_zeros:usize)->Result<Self,&'static str> {
        if up==0 || up>(1<<26) || pending_zeros>=up { return Err("bounded valid source input state required"); }
        Ok(Self {up,pending_zeros})
    }
    pub fn pending_zeros(&self)->usize { self.pending_zeros }

    pub fn fill(&mut self,input:&[f64],destination:&mut[f64])->Result<usize,&'static str> {
        let initial_zeros=self.pending_zeros.min(destination.len());
        let count=destination.len()-initial_zeros;
        let groups=count/self.up;
        let remainder=count%self.up;
        let consumed=groups+usize::from(remainder!=0);
        if input.len()<consumed { return Err("source pointer span too short"); }
        destination.fill(0.0);
        self.pending_zeros-=initial_zeros;
        for (index,value) in input[..consumed].iter().enumerate() {
            destination[initial_zeros+index*self.up]=*value;
        }
        if remainder!=0 { self.pending_zeros=self.up-remainder; }
        Ok(consumed)
    }
}
