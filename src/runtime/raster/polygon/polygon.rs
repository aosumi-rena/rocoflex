pub const OLD_FACTOR:f32=f32::from_bits(0x3f7de00d);
pub const COLOR_FACTOR:f32=f32::from_bits(0x3c07fcb9);
#[derive(Clone,Copy,Debug)]pub struct Dimensions{pub width:usize,pub height:usize,pub depth:usize,pub channels:usize}
#[derive(Debug,PartialEq,Eq)]pub enum FillError{Layout,UnsupportedVertexBranch,ArithmeticDomain,CrossingCapacity,OddCrossings}
#[derive(Debug,Default)]pub struct Trace{pub first:i32,pub before:Vec<Vec<i32>>,pub sorted:Vec<Vec<i32>>,pub spans:Vec<[usize;3]>,pub writes:Vec<(usize,u8)>}
fn sort(v:&mut[i32],mut lo:usize,hi:usize){
 while lo<hi{
  let mid=(lo+hi)/2;
  if v[lo]>v[mid]{v.swap(lo,mid)}
  if v[mid]>v[hi]{v.swap(mid,hi);if v[lo]>v[mid]{v.swap(lo,mid)}}
  if hi-lo<3{return}
  let pivot=v[mid];let mut i=lo as isize;let mut j=hi as isize;
  while i<=j{
   while v[i as usize]<pivot{i+=1}while v[j as usize]>pivot{j-=1}
   if i<=j{v.swap(i as usize,j as usize);i+=1;j-=1}
  }
  if j>lo as isize{sort(v,lo,j as usize)}lo=i as usize;
 }
}

pub fn fill(data:&mut[u8],dims:Dimensions,vertices:&[[i32;2]],color:&[u8])->Result<Trace,FillError>{
 let Dimensions{width:w,height:h,depth:d,channels:c}=dims;let n=vertices.len();
 if w==0||h==0||d==0||c==0||w>i32::MAX as usize||h>i32::MAX as usize||d>u32::MAX as usize||c>16||color.len()<c{return Err(FillError::Layout)}
 let plane=w.checked_mul(h).and_then(|x|x.checked_mul(d)).ok_or(FillError::Layout)?;if plane.checked_mul(c)!=Some(data.len()){return Err(FillError::Layout)}
 if n<4||n>i32::MAX as usize/2{return Err(FillError::UnsupportedVertexBranch)}
 let xmin=vertices.iter().map(|p|p[0]).min().unwrap();let xmax=vertices.iter().map(|p|p[0]).max().unwrap();let ymin=vertices.iter().map(|p|p[1]).min().unwrap();let ymax=vertices.iter().map(|p|p[1]).max().unwrap();
 if ymin==ymax{return Err(FillError::UnsupportedVertexBranch)}
 let sx=i64::from(xmax)-i64::from(xmin);let sy=i64::from(ymax)-i64::from(ymin);
 if sx>i32::MAX as i64/2||sy>=i32::MAX as i64||sx*sy+sy/2>i32::MAX as i64{return Err(FillError::ArithmeticDomain)}
 if n.checked_mul((sy+1)as usize).is_none_or(|x|x>i32::MAX as usize){return Err(FillError::ArithmeticDomain)}
 let mut trace=Trace::default();if xmax<0||xmin>=w as i32||ymax<0||ymin>=h as i32{return Ok(trace)}
 let first=ymin.max(0);let last=ymax.min(h as i32-1);let rows=(last-first+1)as usize;
 if n.checked_mul(rows).is_none_or(|x|x>i32::MAX as usize){return Err(FillError::Layout)}
 trace.first=first;trace.before=vec![vec![];rows];
 let mut i=0;let mut next=1;
 loop{
  let original=i;let[x0,y0]=vertices[i];let mut end=next;let mut y1=vertices[end][1];let mut following=(end+1)%n;let mut after=vertices[following][1];
  if y1==y0&&after==y0{while after==y0{end=following;following=(end+1)%n;after=vertices[following][1]}y1=vertices[end][1]}
  i=end;next=following;let x1=vertices[i][0];
  if after==y1{let mut look=following;while after==y1{look=(look+1)%n;after=vertices[look][1]}}
  let dy=y1-y0;
  if dy!=0{
   let direction=dy.signum();let denominator=dy.abs().max(1);let dx=x1-x0;let bias=(dx.signum()*denominator)/2;
   let steps=denominator-i32::from(direction==(after-y1).signum())+1;
   for k in 0..steps{
    let row=i64::from(y0)-i64::from(first)+i64::from(direction)*i64::from(k);
    if row>=0&&row<rows as i64{
     let x=((k*dx+bias)/denominator)+x0;let crossings=&mut trace.before[row as usize];
     if crossings.len()>=n{return Err(FillError::CrossingCapacity)}crossings.push(x);
    }
   }
  }
  if i<=original{break}
 }
 trace.sorted=trace.before.clone();
 for crossings in &mut trace.sorted{
  if crossings.is_empty()||crossings.len()%2!=0{return Err(FillError::OddCrossings)}let end=crossings.len()-1;sort(crossings,0,end);
 }
 for(row,crossings)in trace.sorted.iter().enumerate(){
  let mut previous_right=w as i32;
  for pair in crossings.chunks_exact(2){
   let start=(pair[0]+i32::from(pair[0]==previous_right)).max(0);previous_right=pair[1];let end=pair[1].min(w as i32-1);
   if end<start{continue}let x=start as usize;let y=first as usize+row;let length=(end-start+1)as usize;trace.spans.push([x,y,length]);
   for(channel,&value)in color.iter().take(c).enumerate(){
    let new=(value as f32)*COLOR_FACTOR;let offset=x+w*y+plane*channel;
    for at in offset..offset+length{let old=(data[at]as f32)*OLD_FACTOR;let output=((old+new)as i32)as u8;data[at]=output;trace.writes.push((at,output))}
   }
  }
 }Ok(trace)
}
