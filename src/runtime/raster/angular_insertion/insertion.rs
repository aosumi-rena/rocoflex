#[derive(Clone,Copy,Debug)]pub struct Record(pub[f32;4]);
#[derive(Debug,PartialEq)]pub enum Error{NonFinite,InvalidCapacity,Size,Allocation,ArithmeticMode}
#[derive(Default,Debug)]pub struct Output{pub records:Vec<Record>,pub trig:Vec<[u32;3]>,pub ratios:Vec<[u32;2]>,pub growth:Vec<[u32;4]>,pub states:Vec<Vec<u32>>,pub capacity:usize}
#[link(name="ucrt")]unsafe extern "C"{fn sinf(x:f32)->f32;fn cosf(x:f32)->f32;}
const SHIFT:f32=f32::from_bits(0xbfc90fdb);const EPS:f32=f32::from_bits(0xb727c5ac);
fn trig(o:&mut Output,kind:u32,a:f32)->f32{let v=unsafe{if kind==0{sinf(a)}else{cosf(a)}};o.trig.push([kind,a.to_bits(),v.to_bits()]);v}
fn ratio(o:&mut Output,kind:u32,n:f32,d:f32)->Result<f32,Error>{let v=unsafe{std::arch::x86_64::_mm_cvtss_f32(std::arch::x86_64::_mm_div_ss(std::arch::x86_64::_mm_set_ss(n),std::arch::x86_64::_mm_set_ss(d)))};o.ratios.push([kind,v.to_bits()]);if v.is_finite()||v.to_bits()==0xffc00000{Ok(v)}else{Err(Error::NonFinite)}}
fn insert(o:&mut Output,index:usize,r:Record)->Result<(),Error>{
 if o.records.len()==o.capacity{let old=o.capacity;let cap=(old+old/2).max(o.records.len()+1);if cap>i32::MAX as usize{return Err(Error::Size)}o.records.try_reserve(1).map_err(|_|Error::Allocation)?;o.growth.push([old as u32,cap as u32,o.records.len()as u32,index as u32]);o.capacity=cap;}o.records.insert(index,r);Ok(())
}
fn state(o:&mut Output,r:Record){let mut v=vec![o.records.len()as u32,o.capacity as u32];v.extend(r.0.map(f32::to_bits));for e in&o.records{v.extend(e.0.map(f32::to_bits))}o.states.push(v)}
pub fn insert_masked_x86_checked(envelope:&[Record],obstacles:&[Record],capacity:usize)->Result<Output,Error>{
 let mut mxcsr=0u32;unsafe{std::arch::asm!("stmxcsr [{}]",in(reg)&mut mxcsr,options(nostack,preserves_flags))};if mxcsr&0xffc0!=0x1f80{return Err(Error::ArithmeticMode)}
 if capacity<envelope.len(){return Err(Error::InvalidCapacity)}
 if capacity>i32::MAX as usize||envelope.len()>i32::MAX as usize||obstacles.len()>i32::MAX as usize{return Err(Error::Size)}
 if envelope.iter().chain(obstacles).any(|r|r.0.iter().any(|v|!v.is_finite())){return Err(Error::NonFinite)}
 let mut o=Output{records:envelope.to_vec(),capacity,..Output::default()};
 for &incoming in obstacles{let mut r=incoming;let end_shift=r.0[1]+SHIFT;
  loop{state(&mut o,r);if r.0[1]<=r.0[0]{break}
   let Some(index)=o.records.iter().rposition(|e|r.0[0]>=e.0[0]&&e.0[1]>r.0[0])else{break};let e=o.records[index];
   if e.0[1]<r.0[1]{
    let t=e.0[1]+SHIFT;let ct=trig(&mut o,1,t);let st=trig(&mut o,0,t);
    let ye=trig(&mut o,0,end_shift)*r.0[3];let xe=trig(&mut o,1,end_shift)*r.0[3];let start_shift=r.0[0]+SHIFT;let ys=trig(&mut o,0,start_shift)*r.0[2];let xs=trig(&mut o,1,start_shift)*r.0[2];let dx=xe-xs;let dy=ye-ys;let den=st*dx-ct*dy;let num=dx*ys-dy*xs;let d=ratio(&mut o,0,num,den)?;
    if unordered_or_le(e.0[3]+EPS,d){r.0[0]=e.0[1];r.0[2]=d;}
    else{
     let ct=trig(&mut o,1,start_shift);let st=trig(&mut o,0,start_shift);let ye=trig(&mut o,0,e.0[1]+SHIFT)*e.0[3];let xe=trig(&mut o,1,e.0[1]+SHIFT)*e.0[3];let ys=trig(&mut o,0,e.0[0]+SHIFT)*e.0[2];let xs=trig(&mut o,1,e.0[0]+SHIFT)*e.0[2];let dx=xe-xs;let dy=ye-ys;let den=st*dx-ct*dy;let num=dx*ys-dy*xs;let ed=ratio(&mut o,1,num,den)?;
     let next=Record([r.0[0],e.0[1],r.0[2],d]);r.0[0]=e.0[1];r.0[2]=d;o.records[index].0[1]=next.0[0];o.records[index].0[3]=ed;insert(&mut o,index+1,next)?;
    }
   }else{
    let ye=trig(&mut o,0,e.0[1]+SHIFT)*e.0[3];let xe=trig(&mut o,1,e.0[1]+SHIFT)*e.0[3];let ys=trig(&mut o,0,e.0[0]+SHIFT)*e.0[2];let xs=trig(&mut o,1,e.0[0]+SHIFT)*e.0[2];let dx=xe-xs;let dy=ye-ys;let t=r.0[0]+SHIFT;let ct=trig(&mut o,1,t);let st=trig(&mut o,0,t);let den=st*dx-ct*dy;let num=ys*dx-xs*dy;let d=ratio(&mut o,2,num,den)?;
    if unordered_or_le(d+EPS,r.0[2]){r.0[0]=r.0[1];}
    else{
     let ct=trig(&mut o,1,end_shift);let st=trig(&mut o,0,end_shift);let den=st*dx-ct*dy;let end_distance=ratio(&mut o,3,num,den)?;o.records[index].0[1]=r.0[0];o.records[index].0[3]=d;
     if e.0[1]>r.0[1]{insert(&mut o,index+1,Record([r.0[1],e.0[1],end_distance,e.0[3]]))?;}insert(&mut o,index+1,r)?;r.0[0]=r.0[1];
    }
   }
   if o.records[index].0[0]>=o.records[index].0[1]{o.records.remove(index);}
  }
 }
 Ok(o)
}

fn unordered_or_le(a:f32,b:f32)->bool{a.is_nan()||b.is_nan()||a<=b}
