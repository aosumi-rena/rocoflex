#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct SourceCepstralBandRange{pub start:usize,pub end:usize,pub left_fade:usize,pub right_fade:usize}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ValueError{Width,NonfiniteRead,NonfiniteResult}
impl SourceCepstralBandRange{
    pub fn admit(self,width:usize)->bool{
        let Some(len)=self.end.checked_sub(self.start) else{return false;};
        len>0&&self.end<=width&&self.left_fade!=1&&self.right_fade!=1
            &&self.left_fade.checked_add(self.right_fade).is_some_and(|v|v<=len)
    }

    pub fn blend(self,source:&[f32],destination:&mut[f32])->Result<(),ValueError>{
        if !self.admit(destination.len()){return Err(ValueError::Width);}
        let len=self.end-self.start;
        if source.len()!=len{return Err(ValueError::Width);}
        if source.iter().any(|v|!v.is_finite()){return Err(ValueError::NonfiniteRead);}

        for(i,&sample)in source.iter().enumerate(){
            let old=destination[self.start+i];
            let value=if i<self.left_fade{
                let ramp=(i as f32)/((self.left_fade-1)as f32);(ramp*sample)+old
            }else if i<len-self.right_fade{sample+old}else{
                let ramp=((len-i-1)as f32)/((self.right_fade-1)as f32);(ramp*sample)+old
            };
            if !value.is_finite(){return Err(ValueError::NonfiniteResult);}
            destination[self.start+i]=value;
        }
        Ok(())
    }
}
