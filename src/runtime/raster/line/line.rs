pub const OLD_FACTOR:f32=f32::from_bits(0x3f7de00d);
pub const COLOR_FACTOR:f32=f32::from_bits(0x3c07fcb9);
#[derive(Clone,Copy,Debug)] pub struct Dimensions {pub width:usize,pub height:usize,pub depth:usize,pub channels:usize}
#[derive(Debug,PartialEq,Eq)] pub enum LineError { Layout, ArithmeticDomain }
#[derive(Debug,Default)] pub struct Trace {pub writes:Vec<(usize,u8)>,pub major_steps:usize,pub accepted:bool}

pub fn raster_line(data:Option<&mut [u8]>,dims:Dimensions,p:[i32;4],color:&[u8],pattern:&mut u32)
 ->Result<Trace,LineError>{
 let mut trace=Trace::default();
 let Some(data)=data else{return Ok(trace)};
 let Dimensions{width:w,height:h,depth:d,channels:c}=dims;
 if w==0||h==0||d==0||c==0{return Ok(trace)}
 if w>i32::MAX as usize||h>i32::MAX as usize||d>u32::MAX as usize||c>i32::MAX as usize||color.len()<c{return Err(LineError::Layout)}
 let plane=w.checked_mul(h).and_then(|x|x.checked_mul(d)).ok_or(LineError::Layout)?;
 if plane.checked_mul(c)!=Some(data.len()){return Err(LineError::Layout)}
 let [x0,y0,x1,y1]=p;
 let dx=i64::from(x1)-i64::from(x0);let dy=i64::from(y1)-i64::from(y0);

 let ax=dx.unsigned_abs();let ay=dy.unsigned_abs();
 if ax>i32::MAX as u64||ay>i32::MAX as u64||ax*ay+ax.max(ay)/2>i32::MAX as u64{return Err(LineError::ArithmeticDomain)}
 if x0.max(x1)<0||y0.max(y1)<0||x0.min(x1)>=w as i32||y0.min(y1)>=h as i32{return Ok(trace)}
 let x_major=ax>ay;
 let (a0,b0,a1,b1,major_limit,minor_limit)=if x_major{(x0,y0,x1,y1,w as i32-1,h as i32-1)}else{(y0,x0,y1,x1,h as i32-1,w as i32-1)};
 let (start,minor_origin,end,minor_delta)=if a0<=a1{(a0,b0,a1,b1-b0)}else{(a1,b1,a0,b0-b1)};
 let major_delta=end-start;
 let bias=(minor_delta.signum()*major_delta)/2;
 let divisor=major_delta.max(1);
 let first=start.clamp(0,major_limit);let last=end.clamp(0,major_limit);
 *pattern=0x80000000;trace.accepted=true;
 for major in first..=last {
  let minor=((major-start)*minor_delta+bias)/divisor+minor_origin;
  if minor>=0&&minor<=minor_limit&&*pattern!=0 {
   let (x,y)=if x_major{(major,minor)}else{(minor,major)};
   let mut offset=x as usize+y as usize*w;
   for &channel_color in color.iter().take(c) {
    let old=(data[offset] as f32)*OLD_FACTOR;
    let new=(channel_color as f32)*COLOR_FACTOR;
    let value=((new+old) as i32) as u8;
    data[offset]=value;trace.writes.push((offset,value));offset+=plane;
   }
  }
  *pattern=if *pattern<2{0x80000000}else{*pattern>>1};trace.major_steps+=1;
 }
 Ok(trace)
}
