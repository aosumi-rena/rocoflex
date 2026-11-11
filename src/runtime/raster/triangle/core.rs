#![allow(dead_code)]
#[path="../line/line.rs"]pub mod line;
#[path="../polygon/polygon.rs"]pub mod general;
#[derive(Clone,Copy)]pub struct Dimensions{pub width:usize,pub height:usize,pub depth:usize,pub channels:usize}
#[derive(Debug,PartialEq,Eq)]pub enum Error{Layout,Branch,ArithmeticDomain,EmptyQuant,NanCrtBoundary,NonFiniteInput,ConversionRange,Line,General}
#[derive(Default,Debug)]pub struct Render{pub intercepts:Vec<[i64;3]>,pub spans:Vec<[usize;3]>,pub writes:Vec<(usize,u8)>,pub pattern:u32,pub path:u32}
#[derive(Debug)]pub struct Quantized{pub vertices:Vec<[i32;2]>,pub rounded:Vec<u32>,pub floorcalls:Vec<[u32;3]>,pub intstores:Vec<[u32;2]>}
fn finite_floor_bits(bits:u32)->Result<u32,Error>{
 let magnitude=bits&0x7fffffff;let negative=bits&0x80000000!=0;
 if magnitude>=0x4b800000{return if magnitude>0x7f800000{Err(Error::NanCrtBoundary)}else{Ok(bits)}}
 if magnitude<0x3f800000{return Ok(if magnitude==0{bits}else if negative{0xbf800000}else{0})}
 let shift=150-(magnitude>>23);let chopped=bits&(u32::MAX<<shift);
 Ok(if negative&&bits!=chopped{(f32::from_bits(chopped)-1.0).to_bits()}else{chopped})
}
pub fn quantize_checked(vertices:&[[f32;2]])->Result<Quantized,Error>{
 if vertices.is_empty(){return Err(Error::EmptyQuant)}
 if vertices.len()>i32::MAX as usize/2{return Err(Error::Layout)}
 if vertices.iter().flatten().any(|x|x.is_nan()){return Err(Error::NanCrtBoundary)}
 if vertices.iter().flatten().any(|x|!x.is_finite()){return Err(Error::NonFiniteInput)}
 let n=vertices.len();let mut values=Vec::with_capacity(2*n);for a in[0,1]{for v in vertices{values.push(v[a])}}
 let mut calls=vec![];for i in(0..2*n).rev(){let sum=values[i]+0.5f32;let floor=finite_floor_bits(sum.to_bits())?;calls.push([i as u32,sum.to_bits(),floor]);values[i]=f32::from_bits(floor)}
 let rounded=values.iter().map(|x|x.to_bits()).collect();let mut ints=vec![];
 for x in values{if !(-2147483648.0..2147483648.0).contains(&x){return Err(Error::ConversionRange)}ints.push(x as i32)}
 let output=(0..n).map(|i|[ints[i],ints[n+i]]).collect();let stores=ints.iter().enumerate().map(|(i,x)|[i as u32,*x as u32]).collect();
 Ok(Quantized{vertices:output,rounded,floorcalls:calls,intstores:stores})
}
fn layout(data:&[u8],dims:Dimensions,color:&[u8])->Result<usize,Error>{
 let Dimensions{width:w,height:h,depth:d,channels:c}=dims;
 if w==0||h==0||d==0||c==0||w>i32::MAX as usize||h>i32::MAX as usize||d>u32::MAX as usize||c>16||color.len()<c{return Err(Error::Layout)}
 let plane=w.checked_mul(h).and_then(|v|v.checked_mul(d)).ok_or(Error::Layout)?;if plane.checked_mul(c)!=Some(data.len()){return Err(Error::Layout)}Ok(plane)
}
pub fn fill_triangle(data:&mut[u8],dims:Dimensions,v:[[i32;2];3],color:&[u8],pattern:u32)->Result<Render,Error>{
 let plane=layout(data,dims,color)?;let w=dims.width;let h=dims.height;
 let xmin=v.iter().map(|p|p[0]).min().unwrap();let xmax=v.iter().map(|p|p[0]).max().unwrap();let ymin=v.iter().map(|p|p[1]).min().unwrap();let ymax=v.iter().map(|p|p[1]).max().unwrap();
 let sx=i64::from(xmax)-i64::from(xmin);let sy=i64::from(ymax)-i64::from(ymin);if (sx as u128)*(sy as u128)+(sy as u128)/2>i64::MAX as u128{return Err(Error::ArithmeticDomain)}

 let(lo,hi)=if v[0][1]>v[1][1]{(v[1],v[0])}else{(v[0],v[1])};
 let(low,mid,high)=if lo[1]>v[2][1]{if hi[1]>lo[1]{(v[2],lo,hi)}else{(v[2],hi,lo)}}else if hi[1]>v[2][1]{(lo,v[2],hi)}else{(lo,hi,v[2])};
 let mut out=Render{pattern,path:2,..Render::default()};if high[1]<0||low[1]>=h as i32||xmin>=w as i32||xmax<0{return Ok(out)}
 let[xa,ya]=low.map(i64::from);let[xb,yb]=mid.map(i64::from);let[xz,yz]=high.map(i64::from);
 let dab=(yb-ya).max(1);let daz=(yz-ya).max(1);let dbz=(yz-yb).max(1);let bab=((xb-xa).signum()*dab)/2;let baz=((xz-xa).signum()*daz)/2;let bbz=((xz-xb).signum()*dbz)/2;
 let first=ya.max(0).min(h as i64-1);let last=yz.min(h as i64-1);
 for y in first..=last{
  let edge=if y<yb{((y-ya)*(xb-xa)+bab)/dab+xa}else{((y-yb)*(xz-xb)+bbz)/dbz+xb};
  let chord=((y-ya)*(xz-xa)+baz)/daz+xa;out.intercepts.push([y,edge,chord]);let left=edge.min(chord).max(0);let right=edge.max(chord).min(w as i64-1);if right<left{continue}
  let x=left as usize;let length=(right-left+1)as usize;out.spans.push([x,y as usize,length]);
  for(channel,&value)in color.iter().take(dims.channels).enumerate(){let new=(value as f32)*f32::from_bits(0x3c07fcb9);let start=x+w*y as usize+plane*channel;for at in start..start+length{let old=(data[at]as f32)*f32::from_bits(0x3f7de00d);let value=((old+new)as i32)as u8;data[at]=value;out.writes.push((at,value))}}
 }Ok(out)
}
pub fn rasterize_checked(data:&mut[u8],dims:Dimensions,vertices:&[[f32;2]],color:&[u8],pattern:&mut u32)->Result<(Quantized,Render),Error>{
 layout(data,dims,color)?;let q=quantize_checked(vertices)?;let n=q.vertices.len();
 if n<2{return Err(Error::Branch)}
 let out=if n==3{fill_triangle(data,dims,[q.vertices[0],q.vertices[1],q.vertices[2]],color,*pattern)?}
 else if n==2{
  let p=[q.vertices[0][0],q.vertices[0][1],q.vertices[1][0],q.vertices[1][1]];
  let trace=line::raster_line(Some(data),line::Dimensions{width:dims.width,height:dims.height,depth:dims.depth,channels:dims.channels},p,color,pattern).map_err(|_|Error::Line)?;
  Render{pattern:*pattern,path:1,writes:trace.writes,..Render::default()}
 }else{
  let xmin=q.vertices.iter().map(|p|p[0]).min().unwrap();let xmax=q.vertices.iter().map(|p|p[0]).max().unwrap();let ymin=q.vertices.iter().map(|p|p[1]).min().unwrap();let ymax=q.vertices.iter().map(|p|p[1]).max().unwrap();
  if ymin==ymax{
   if xmax<0||xmin>=dims.width as i32||ymax<0||ymin>=dims.height as i32{Render{pattern:*pattern,path:3,..Render::default()}}
   else{let trace=line::raster_line(Some(data),line::Dimensions{width:dims.width,height:dims.height,depth:dims.depth,channels:dims.channels},[xmin,ymin,xmax,ymin],color,pattern).map_err(|_|Error::Line)?;Render{pattern:*pattern,path:4,writes:trace.writes,..Render::default()}}
  }else{let t=general::fill(data,general::Dimensions{width:dims.width,height:dims.height,depth:dims.depth,channels:dims.channels},&q.vertices,color).map_err(|_|Error::General)?;Render{pattern:*pattern,path:3,spans:t.spans,writes:t.writes,..Render::default()}}
 };Ok((q,out))
}
